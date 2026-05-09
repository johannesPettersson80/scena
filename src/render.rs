//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Weak;

mod build;
mod camera;
mod cpu;
mod culling;
mod gpu;
mod offscreen;
mod output;
mod prepare;
mod settings;
mod surface;

use crate::assets::{Assets, EnvironmentHandle};
use crate::diagnostics::{
    Backend, Capabilities, CapabilityReport, ChangeKind, DebugOverlay, DevicePoll, Diagnostic,
    DiagnosticCode, GpuAdapterReport, NotPreparedReason, PrepareError, RenderError, RenderOutcome,
    RendererStats,
};
use crate::geometry::Primitive;
use crate::material::Color;
use crate::picking::InteractionStyle;
use crate::platform::SurfaceKind;
use crate::scene::{CameraKey, ClippingPlane, Scene};

use self::gpu::GpuDeviceState;
pub use self::offscreen::{OffscreenTarget, PixelReadback};
use self::output::OutputTransform;
pub use self::output::Tonemapper;
pub use self::settings::{Profile, Quality, RenderMode, RendererOptions};

#[derive(Debug)]
pub struct Renderer {
    target: RasterTarget,
    prepared: Option<PreparedSceneState>,
    frame: Vec<u8>,
    fxaa_scratch: Vec<u8>,
    // CPU-only linear scene-referred straight-alpha accumulator. Stores the source of truth
    // before every pixel is ACES+sRGB encoded into `frame`.
    linear_frame: Option<Vec<Color>>,
    // CPU-only camera-space depth buffer. Lower positive values are closer to the active camera.
    depth_frame: Option<Vec<f32>>,
    stats: RendererStats,
    diagnostics: Vec<Diagnostic>,
    capabilities: Capabilities,
    gpu: Option<GpuDeviceState>,
    output: OutputTransform,
    profile: Profile,
    quality: Quality,
    render_mode: RenderMode,
    render_generation: u64,
    last_rendered_generation: Option<u64>,
    debug_overlay: DebugOverlay,
    debug_revision: u64,
    surface_lost: Option<bool>,
    context_lost: Option<bool>,
    device_lost: Option<bool>,
    hover_style: InteractionStyle,
    selection_style: InteractionStyle,
    environment: Option<EnvironmentHandle>,
    environment_revision: u64,
    target_revision: u64,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone)]
struct PreparedSceneState {
    scene: Weak<()>,
    structure_revision: u64,
    environment_revision: u64,
    target_revision: u64,
    debug_revision: u64,
    primitives: Vec<Primitive>,
    clipping_planes: Vec<ClippingPlane>,
}

/// Row-major render target dimensions used for CPU frame and accumulator indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RasterTarget {
    width: u32,
    height: u32,
    backend: Backend,
}

impl Renderer {
    pub fn prepare(&mut self, scene: &mut Scene) -> Result<(), PrepareError> {
        self.prepare_inner::<()>(scene, None)
    }

    pub fn prepare_with_assets<F>(
        &mut self,
        scene: &mut Scene,
        assets: &Assets<F>,
    ) -> Result<(), PrepareError> {
        self.prepare_inner(scene, Some(assets))
    }

    fn prepare_inner<F>(
        &mut self,
        scene: &mut Scene,
        assets: Option<&Assets<F>>,
    ) -> Result<(), PrepareError> {
        self.poll_device();
        self.diagnostics.clear();
        validate_target_size(self.target.width, self.target.height).map_err(|()| {
            PrepareError::InvalidTargetSize {
                width: self.target.width,
                height: self.target.height,
            }
        })?;
        let mut diagnostics = prepare::collect_precision_diagnostics(scene, self.target.backend);
        diagnostics.extend(prepare::collect_camera_visibility_diagnostics(
            scene,
            self.target,
        ));
        let environment_desc = match self.environment {
            Some(environment) => {
                let Some(assets) = assets else {
                    return Err(PrepareError::EnvironmentAssetsRequired { environment });
                };
                Some(
                    assets
                        .environment(environment)
                        .ok_or(PrepareError::EnvironmentNotFound { environment })?,
                )
            }
            None => None,
        };
        let environment_prepare_stats =
            prepare::collect_environment_prepare_stats(environment_desc.as_ref());
        let environment_count = u64::from(environment_desc.is_some());
        let lighting_stats = prepare::collect_lighting_stats(scene, self.target.backend)?;
        let environment_lighting = prepare::collect_environment_lighting(environment_desc.as_ref());
        let gpu_light_uniform = prepare::collect_gpu_light_uniform(
            scene,
            scene.origin_shift(),
            environment_desc.as_ref(),
        );
        let active_camera_projection = scene.active_camera().and_then(|camera| {
            camera::CameraProjection::from_scene(scene, camera, self.target).ok()
        });
        let backend_material_slots = if self.gpu.is_some() {
            prepare::collect_backend_material_slots(scene, assets)
        } else {
            Vec::new()
        };
        let backend_sampled_base_color_textures = backend_material_slots
            .iter()
            .filter_map(|slot| slot.base_color.as_ref().map(|texture| texture.handle))
            .collect::<Vec<_>>();
        let backend_material_handles = backend_material_slots
            .iter()
            .map(|slot| slot.handle)
            .collect::<Vec<_>>();
        let prepared_scene = prepare::collect_prepared_primitives(
            self.target,
            scene,
            assets,
            active_camera_projection.as_ref(),
            &backend_sampled_base_color_textures,
            &backend_material_handles,
            environment_lighting,
        )?;
        let light_from_world = prepared_scene.light_from_world;
        let culled_primitives =
            culling::cull_cpu_frustum(prepared_scene.primitives, active_camera_projection.as_ref());
        let primitives = culled_primitives.visible;
        let depth_stats = prepare::collect_depth_prepass_stats(&primitives, self.target.backend);
        let logical_stats =
            prepare::collect_logical_resource_stats(scene, assets, environment_count);
        self.stats.materials = logical_stats.materials;
        self.stats.material_bindings = logical_stats.material_bindings;
        self.stats.material_texture_bindings = logical_stats.material_texture_bindings;
        self.stats.material_sampler_bindings = logical_stats.material_sampler_bindings;
        self.stats.environments = logical_stats.environments;
        self.stats.environment_cubemaps = environment_prepare_stats.cubemaps;
        self.stats.environment_prefilter_passes = environment_prepare_stats.prefilter_passes;
        self.stats.environment_brdf_luts = environment_prepare_stats.brdf_luts;
        self.stats.live_logical_handles = logical_stats.live_logical_handles;
        self.stats.shadow_maps = lighting_stats.shadow_maps;
        self.stats.depth_prepass_passes = depth_stats.passes;
        self.stats.depth_prepass_draws = depth_stats.draws;
        self.stats.directional_shadow_map_resolution =
            lighting_stats.directional_shadow_map_resolution;
        self.stats.directional_shadow_pcf_kernel = lighting_stats.directional_shadow_pcf_kernel;
        self.stats.culled_objects = culled_primitives.culled;
        if let Some(gpu) = &mut self.gpu {
            gpu.prepare(
                self.target,
                &primitives,
                lighting_stats,
                gpu_light_uniform,
                light_from_world,
                depth_stats,
                &backend_material_slots,
            );
            let stats = gpu.prepared_resource_stats();
            let pending_destructions = gpu.pending_destructions();
            self.stats.buffers = stats.buffers;
            self.stats.textures = logical_stats.textures;
            self.stats.render_targets = stats.render_targets;
            self.stats.pipelines = stats.pipelines;
            self.stats.bind_groups = stats.bind_groups;
            self.stats.shader_modules = stats.shader_modules;
            self.stats.pending_destructions = pending_destructions;
            self.stats.approximate_gpu_memory_bytes = (stats.approximate_gpu_memory_bytes > 0)
                .then_some(stats.approximate_gpu_memory_bytes);
        } else {
            self.stats.textures = logical_stats.textures;
        }
        self.prepared = Some(PreparedSceneState {
            scene: scene.identity(),
            structure_revision: scene.structure_revision(),
            environment_revision: self.environment_revision,
            target_revision: self.target_revision,
            debug_revision: self.debug_revision,
            primitives,
            clipping_planes: scene.active_clipping_plane_values().collect(),
        });
        self.render_generation = self.render_generation.saturating_add(1);
        self.last_rendered_generation = None;
        self.diagnostics = diagnostics;
        Ok(())
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RenderOutcome, RenderError> {
        self.loss_error()?;
        self.prepared_state(scene)?;
        if scene.camera(camera).is_none() {
            return Err(RenderError::CameraNotFound(camera));
        }

        if self.render_mode == RenderMode::OnChange
            && self.last_rendered_generation == Some(self.render_generation)
        {
            self.stats.skipped_frames = self.stats.skipped_frames.saturating_add(1);
            return Ok(RenderOutcome {
                width: self.target.width,
                height: self.target.height,
                draw_calls: 0,
                primitives: 0,
                skipped: true,
            });
        }

        let primitives = self.prepared_state(scene)?.primitives.clone();
        let clipping_planes = self.prepared_state(scene)?.clipping_planes.clone();
        let camera_projection = camera::CameraProjection::from_scene(scene, camera, self.target)?;
        let primitive_count = primitives.len() as u64;
        if self.gpu.is_some() {
            self.draw_gpu(&camera_projection)?;
        } else {
            let linear_frame = self
                .linear_frame
                .as_mut()
                .expect("CPU renderer owns a linear accumulator");
            let depth_frame = self
                .depth_frame
                .as_mut()
                .expect("CPU renderer owns a depth buffer");
            let mut cpu_frame = cpu::CpuFrame::new(
                self.target,
                self.output,
                linear_frame,
                depth_frame,
                &mut self.frame,
            );
            cpu::clear_cpu(&mut cpu_frame, Color::BLACK);
            for primitive in &primitives {
                cpu::draw_primitive_cpu(
                    &mut cpu_frame,
                    primitive,
                    &clipping_planes,
                    &camera_projection,
                );
            }
        }
        self.stats.fxaa_passes =
            output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch);
        self.poll_device();

        self.stats.frames_rendered = self.stats.frames_rendered.saturating_add(1);
        self.stats.draw_calls = primitive_count;
        self.stats.triangles = primitive_count;
        self.stats.primitives = primitive_count;
        self.last_rendered_generation = Some(self.render_generation);

        Ok(RenderOutcome {
            width: self.target.width,
            height: self.target.height,
            draw_calls: primitive_count,
            primitives: primitive_count,
            skipped: false,
        })
    }

    pub fn gpu_adapter_report(&self) -> Option<GpuAdapterReport> {
        self.gpu.as_ref().map(GpuDeviceState::adapter_report)
    }

    pub fn capability_report(&self) -> CapabilityReport {
        CapabilityReport::new(self.capabilities, self.gpu_adapter_report())
    }

    pub fn render_active(&mut self, scene: &Scene) -> Result<RenderOutcome, RenderError> {
        self.prepared_state(scene)?;
        let camera = scene.active_camera().ok_or(RenderError::NoActiveCamera)?;
        self.render(scene, camera)
    }

    pub fn diagnose_scene(&self, scene: &Scene) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if scene.active_camera().is_none() {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MissingActiveCamera,
                "scene has no active camera",
                "call Scene::add_default_camera or Scene::set_active_camera before rendering",
            ));
        }
        diagnostics.extend(prepare::collect_camera_projection_diagnostics(scene));
        diagnostics.extend(prepare::collect_camera_visibility_diagnostics(
            scene,
            self.target,
        ));

        if scene.visible_drawable_count() == 0 {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::InvisibleScene,
                "scene has no visible drawables for the active camera",
                "check node visibility, parent visibility, camera layer masks, or add a mesh/renderable node",
            ));
        }

        if scene.light_nodes().count() == 0 && self.environment.is_none() {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::MissingLightingOrEnvironment,
                "scene has no active light nodes and no renderer environment",
                "call renderer.set_environment for image-based lighting or add a scene light for lit materials",
            ));
        }

        diagnostics
    }

    pub fn diagnose_scene_with_assets<F>(
        &self,
        scene: &Scene,
        assets: &Assets<F>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = self.diagnose_scene(scene);
        diagnostics.extend(prepare::collect_asset_camera_visibility_diagnostics(
            scene,
            self.target,
            assets,
        ));
        diagnostics
    }

    pub fn frame_rgba8(&self) -> &[u8] {
        &self.frame
    }

    pub fn stats(&self) -> RendererStats {
        self.stats
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn poll_device(&mut self) -> DevicePoll {
        let before = self.stats.pending_destructions;
        let (destroyed_resources, gpu_polled) = self
            .gpu
            .as_mut()
            .map(|gpu| gpu.poll_device())
            .unwrap_or((before, false));
        let after = self
            .gpu
            .as_ref()
            .map(|gpu| gpu.pending_destructions())
            .unwrap_or(0);
        self.stats.pending_destructions = after;
        DevicePoll {
            pending_destructions_before: before,
            pending_destructions_after: after,
            destroyed_resources,
            gpu_polled,
        }
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub fn has_gpu_device(&self) -> bool {
        self.gpu.is_some()
    }

    fn draw_gpu(
        &mut self,
        camera_projection: &camera::CameraProjection,
    ) -> Result<(), RenderError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            let submitted = gpu.render_to_frame(
                self.target,
                self.output.exposure_ev(),
                camera_projection,
                &mut self.frame,
            )?;
            if submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            // self.stats.gpu_culling_dispatches stays at 0 — the empty culling
            // kernel was deleted in commit a311fcd. The public counter is kept
            // for API stability and will be repurposed when a real culling
            // kernel lands in a future v1.x.
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            if gpu.render_to_surface(self.target, self.output.exposure_ev(), camera_projection)? {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            Ok(())
        }
    }

    fn prepared_state(&self, scene: &Scene) -> Result<&PreparedSceneState, RenderError> {
        let prepared = self.prepared.as_ref().ok_or(RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared,
        })?;

        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::DifferentScene,
            });
        }

        let current_revision = scene.structure_revision();
        if prepared.structure_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.structure_revision,
                    current_revision,
                    change: ChangeKind::SceneStructure,
                },
            });
        }

        if prepared.environment_revision != self.environment_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::EnvironmentChanged {
                    prepared_revision: prepared.environment_revision,
                    current_revision: self.environment_revision,
                    change: ChangeKind::Environment,
                },
            });
        }

        if prepared.target_revision != self.target_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::TargetChanged {
                    prepared_revision: prepared.target_revision,
                    current_revision: self.target_revision,
                    change: ChangeKind::RenderTarget,
                },
            });
        }

        if prepared.debug_revision != self.debug_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::RendererChanged {
                    prepared_revision: prepared.debug_revision,
                    current_revision: self.debug_revision,
                    change: ChangeKind::DebugOverlay,
                },
            });
        }

        Ok(prepared)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(gpu) = &mut self.gpu {
            gpu.release_prepared_resources();
            let _ = gpu.poll_device();
        }
    }
}

impl RasterTarget {
    fn pixel_len(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    fn byte_len(self) -> usize {
        self.pixel_len() * 4
    }

    fn pixel_index(self, x: u32, y: u32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }
}

pub(super) fn backend_for_attached_surface(kind: SurfaceKind) -> Backend {
    match kind {
        SurfaceKind::NativeWindow => Backend::NativeSurface,
        SurfaceKind::BrowserWebGpuCanvas => Backend::WebGpu,
        SurfaceKind::BrowserWebGl2Canvas => Backend::WebGl2,
    }
}

pub(super) fn validate_target_size(width: u32, height: u32) -> Result<(), ()> {
    if width == 0 || height == 0 {
        Err(())
    } else {
        Ok(())
    }
}
