//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::{cell::Cell, marker::PhantomData, sync::Weak};

mod build;
mod camera;
mod color_contract;
mod cpu;
mod culling;
mod environment_cache;
mod gpu;
mod offscreen;
mod output;
mod prepare;
mod prepare_lifecycle;
mod reporting;
mod settings;
mod surface;

use crate::assets::EnvironmentHandle;
use crate::diagnostics::{
    Backend, Capabilities, CapabilityReport, ChangeKind, DebugOverlay, DevicePoll, Diagnostic,
    GpuAdapterReport, NotPreparedReason, RenderError, RenderOutcome, RendererStats,
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
    environment_lighting_cache: Option<environment_cache::EnvironmentLightingCache>,
    background_color: Color,
    environment_revision: u64,
    target_revision: u64,
    prepare_telemetry: PrepareTelemetry,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PrepareTelemetry {
    full_prepares: u64,
    prepared_primitive_collections: u64,
    static_gpu_resource_rebuilds: u64,
}

#[derive(Debug, Clone)]
struct PreparedSceneState {
    scene: Weak<()>,
    structure_revision: u64,
    transform_revision: u64,
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

        let camera_projection = camera::CameraProjection::from_scene(scene, camera, self.target)?;
        let primitive_count = self.prepared_state(scene)?.primitives.len() as u64;
        if self.gpu.is_some() {
            self.draw_gpu(&camera_projection)?;
        } else {
            let (primitives, clipping_planes) = {
                let prepared = self.prepared_state(scene)?;
                (
                    prepared.primitives.clone(),
                    prepared.clipping_planes.clone(),
                )
            };
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
            cpu::clear_cpu(&mut cpu_frame, self.background_color);
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

    pub fn frame_rgba8(&self) -> &[u8] {
        &self.frame
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
                self.output.color_management_uniform(),
                self.background_color,
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
            if gpu.render_to_surface(
                self.target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.background_color,
                camera_projection,
            )? {
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

        let current_revision = scene.transform_revision();
        if prepared.transform_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.transform_revision,
                    current_revision,
                    change: ChangeKind::Transform,
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

#[cfg(test)]
impl Renderer {
    fn prepare_telemetry_for_test(&self) -> PrepareTelemetry {
        self.prepare_telemetry
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::assets::Assets;
    use crate::diagnostics::DebugOverlay;
    use crate::geometry::GeometryDesc;
    use crate::material::{Color, MaterialDesc};
    use crate::platform::SurfaceEvent;
    use crate::scene::{DirectionalLight, NodeKey, Scene, Transform, Vec3};

    use super::Renderer;

    #[test]
    fn transform_only_gpu_prepare_recollects_primitives_for_visual_correctness() {
        let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
            return;
        };
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
        let material =
            assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
        let mut scene = Scene::new();
        scene.add_default_camera().expect("camera inserts");
        let moving = scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
            .add()
            .expect("first mesh inserts");
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
            .add()
            .expect("second mesh inserts");

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("initial GPU prepare succeeds");
        let first = renderer.prepare_telemetry_for_test();

        scene
            .set_transform(moving, Transform::at(Vec3::new(-0.15, 0.0, 0.0)))
            .expect("mesh transform updates");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("transform-only GPU prepare succeeds");
        let second = renderer.prepare_telemetry_for_test();

        assert_eq!(
            second.prepared_primitive_collections,
            first.prepared_primitive_collections + 1,
            "transform-only GPU prepares must use the canonical prepared primitive order until a visual-equivalence gate proves a fast path safe"
        );
    }

    #[test]
    fn target_change_rejects_transform_only_gpu_template_reuse() {
        let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
            return;
        };
        let (assets, mut scene, moving) = gpu_template_scene();

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("initial GPU prepare succeeds");
        let first = renderer.prepare_telemetry_for_test();

        renderer
            .handle_surface_event(SurfaceEvent::Resize {
                width: 24,
                height: 16,
            })
            .expect("target resizes");
        scene
            .set_transform(moving, Transform::at(Vec3::new(-0.15, 0.0, 0.0)))
            .expect("mesh transform updates");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("target-changed prepare succeeds");
        let second = renderer.prepare_telemetry_for_test();

        assert!(
            second.prepared_primitive_collections > first.prepared_primitive_collections,
            "target changes must force a full prepare instead of a dynamic draw-template update"
        );
    }

    #[test]
    fn environment_and_debug_changes_reject_transform_only_gpu_template_reuse() {
        let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
            return;
        };
        let (assets, mut scene, moving) = gpu_template_scene();

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("initial GPU prepare succeeds");
        let first = renderer.prepare_telemetry_for_test();

        renderer.set_environment(assets.default_environment());
        scene
            .set_transform(moving, Transform::at(Vec3::new(-0.15, 0.0, 0.0)))
            .expect("mesh transform updates");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("environment-changed prepare succeeds");
        let second = renderer.prepare_telemetry_for_test();

        assert!(
            second.prepared_primitive_collections > first.prepared_primitive_collections,
            "environment changes must force a full prepare"
        );
        renderer.set_debug_overlay(DebugOverlay::Wireframe);
        scene
            .set_transform(moving, Transform::at(Vec3::new(0.0, 0.0, 0.0)))
            .expect("mesh transform updates again");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("debug-changed prepare succeeds");
        let third = renderer.prepare_telemetry_for_test();

        assert!(
            third.prepared_primitive_collections > second.prepared_primitive_collections,
            "debug draw-shape changes must force a full prepare"
        );
    }

    #[test]
    fn shadow_state_change_rejects_transform_only_gpu_template_reuse() {
        let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
            return;
        };
        let (assets, mut scene, _moving) = gpu_template_scene();

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("initial GPU prepare succeeds");
        let first = renderer.prepare_telemetry_for_test();

        scene
            .directional_light(DirectionalLight::default().with_shadows(true))
            .add()
            .expect("shadowed light inserts");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("shadow-state-changed prepare succeeds");
        let second = renderer.prepare_telemetry_for_test();

        assert!(
            second.prepared_primitive_collections > first.prepared_primitive_collections,
            "shadow pass eligibility changes must force a full prepare"
        );
        assert_eq!(
            renderer.stats().shadow_maps,
            1,
            "shadow pass must stay enabled after the fallback full prepare"
        );
    }

    fn gpu_template_scene() -> (Assets, Scene, NodeKey) {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
        let material =
            assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
        let mut scene = Scene::new();
        scene.add_default_camera().expect("camera inserts");
        let moving = scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
            .add()
            .expect("first mesh inserts");
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
            .add()
            .expect("second mesh inserts");
        (assets, scene, moving)
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
