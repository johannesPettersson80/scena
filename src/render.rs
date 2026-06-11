//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::{cell::Cell, marker::PhantomData, sync::Weak};

mod background;
mod build;
mod camera;
mod color_contract;
mod cpu;
mod culling;
mod environment_cache;
mod exposure;
mod gpu;
mod offscreen;
mod output;
mod prepare;
mod prepare_lifecycle;
mod reporting;
mod settings;
mod surface;

use self::prepare::PreparedPrimitive;
use crate::assets::EnvironmentHandle;
use crate::diagnostics::{
    Backend, Capabilities, ChangeKind, DebugOverlay, DevicePoll, Diagnostic, GpuAdapterReport,
    NotPreparedReason, OutputColorSpace, RenderError, RenderOutcome, RendererStats,
};
use crate::material::Color;
use crate::picking::InteractionStyle;
use crate::platform::SurfaceKind;
use crate::scene::{CameraKey, ClippingPlane, Scene, SceneDirtyState};

pub use self::background::Background;
pub use self::exposure::{
    AutoExposureConfig, AutoExposureResult, estimate_auto_exposure_from_linear_colors,
    estimate_auto_exposure_from_srgb8,
};
use self::gpu::GpuDeviceState;
pub use self::offscreen::{OffscreenTarget, PixelReadback};
use self::output::OutputTransform;
pub use self::output::{
    AntiAliasing, OrderIndependentTransparencyConfig, PostBloomConfig,
    ScreenSpaceAmbientOcclusionConfig, Tonemapper,
};
#[doc(hidden)]
pub use self::prepare::precompute_environment_sidecar;
pub use self::settings::{Profile, Quality, RenderMode, RendererOptions};

#[derive(Debug)]
pub struct Renderer {
    target: RasterTarget,
    prepared: Option<PreparedSceneState>,
    frame: Vec<u8>,
    fxaa_scratch: Vec<u8>,
    bloom_scratch: Vec<u8>,
    oit_scratch: Vec<cpu::OitAccumPixel>,
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
    anti_aliasing: AntiAliasing,
    order_independent_transparency: Option<OrderIndependentTransparencyConfig>,
    screen_space_ambient_occlusion: Option<ScreenSpaceAmbientOcclusionConfig>,
    bloom: Option<PostBloomConfig>,
    profile: Profile,
    quality: Quality,
    render_mode: RenderMode,
    output_color_space: OutputColorSpace,
    render_generation: u64,
    last_rendered_generation: Option<u64>,
    last_rendered_frame: Option<RenderedFrameState>,
    debug_overlay: DebugOverlay,
    debug_revision: u64,
    surface_lost: Option<bool>,
    context_lost: Option<bool>,
    device_lost: Option<bool>,
    hover_style: InteractionStyle,
    selection_style: InteractionStyle,
    environment: Option<EnvironmentHandle>,
    environment_lighting_cache: environment_cache::EnvironmentLightingCache,
    background_color: Color,
    auto_exposure: Option<AutoExposureConfig>,
    last_auto_exposure: Option<AutoExposureResult>,
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
    dynamic_template_prepares: u64,
    draw_uniform_only_updates: u64,
}

#[derive(Debug, Clone)]
struct PreparedSceneState {
    scene: Weak<()>,
    structure_revision: u64,
    transform_revision: u64,
    appearance_revision: u64,
    visibility_revision: u64,
    environment_revision: u64,
    target_revision: u64,
    debug_revision: u64,
    retained_primitives: Vec<PreparedPrimitive>,
    primitives: Vec<PreparedPrimitive>,
    clipping_planes: Vec<ClippingPlane>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderedFrameState {
    dirty_state: SceneDirtyState,
    camera: CameraKey,
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

        let dirty_state = scene.dirty_state();
        if self.render_mode == RenderMode::OnChange
            && self.last_rendered_generation == Some(self.render_generation)
            && self
                .last_rendered_frame
                .is_some_and(|state| state.matches(dirty_state, camera))
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
        let mut auto_exposure_attempted = false;
        loop {
            let gpu_post_counts = if self.gpu.is_some() {
                let post_counts = self.draw_gpu(&camera_projection)?;
                self.stats.order_independent_transparency_passes = 0;
                Some(post_counts)
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
                self.stats.order_independent_transparency_passes =
                    if let Some(config) = self.order_independent_transparency {
                        cpu::clear_order_independent_transparency(&mut self.oit_scratch);
                        for primitive in &primitives {
                            if cpu::primitive_needs_order_independent_transparency(primitive) {
                                cpu::draw_order_independent_transparency_cpu(
                                    &mut cpu_frame,
                                    primitive,
                                    &clipping_planes,
                                    &camera_projection,
                                    &mut self.oit_scratch,
                                    config,
                                );
                            } else {
                                cpu::draw_primitive_cpu(
                                    &mut cpu_frame,
                                    primitive,
                                    &clipping_planes,
                                    &camera_projection,
                                );
                            }
                        }
                        cpu::resolve_order_independent_transparency_cpu(
                            &mut cpu_frame,
                            &self.oit_scratch,
                        )
                    } else {
                        for primitive in &primitives {
                            cpu::draw_primitive_cpu(
                                &mut cpu_frame,
                                primitive,
                                &clipping_planes,
                                &camera_projection,
                            );
                        }
                        0
                    };
                None
            };
            if let Some(post_counts) = gpu_post_counts {
                self.stats.ambient_occlusion_passes = post_counts.ambient_occlusion;
                self.stats.bloom_passes = post_counts.bloom;
                self.stats.fxaa_passes = post_counts.fxaa;
            } else {
                self.stats.ambient_occlusion_passes = match (
                    self.screen_space_ambient_occlusion,
                    self.depth_frame.as_ref(),
                ) {
                    (Some(config), Some(depth_frame)) => {
                        output::apply_screen_space_ambient_occlusion_rgba8(
                            self.target,
                            &mut self.frame,
                            depth_frame,
                            config,
                        )
                    }
                    _ => 0,
                };
                self.stats.bloom_passes = self.bloom.map_or(0, |bloom| {
                    output::apply_bloom_rgba8(
                        self.target,
                        &mut self.frame,
                        &mut self.bloom_scratch,
                        bloom,
                    )
                });
                self.stats.fxaa_passes = match self.anti_aliasing {
                    AntiAliasing::None => 0,
                    AntiAliasing::Fxaa => output::apply_fxaa_rgba8(
                        self.target,
                        &mut self.frame,
                        &mut self.fxaa_scratch,
                    ),
                };
            }
            if auto_exposure_attempted || !self.apply_managed_auto_exposure_after_render() {
                break;
            }
            auto_exposure_attempted = true;
        }
        self.poll_device();

        self.stats.frames_rendered = self.stats.frames_rendered.saturating_add(1);
        self.stats.draw_calls = primitive_count;
        self.stats.triangles = primitive_count;
        self.stats.primitives = primitive_count;
        self.last_rendered_generation = Some(self.render_generation);
        self.last_rendered_frame = Some(RenderedFrameState {
            dirty_state,
            camera,
        });

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

    pub(crate) fn rendered_frame_state(&self) -> Option<RenderedFrameState> {
        self.last_rendered_frame
    }

    pub(crate) fn clear_rendered_frame(&mut self) {
        self.last_rendered_generation = None;
        self.last_rendered_frame = None;
    }

    pub fn has_gpu_device(&self) -> bool {
        self.gpu.is_some()
    }

    fn draw_gpu(
        &mut self,
        camera_projection: &camera::CameraProjection,
    ) -> Result<gpu::GpuPostPassCounts, RenderError> {
        let post_settings = gpu::GpuPostSettings::new(
            self.anti_aliasing,
            self.bloom,
            self.screen_space_ambient_occlusion,
        );
        #[cfg(not(target_arch = "wasm32"))]
        {
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            let result = gpu.render_to_frame(
                self.target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.background_color,
                camera_projection,
                &mut self.frame,
                post_settings,
            )?;
            if result.submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            // self.stats.gpu_culling_dispatches stays at 0 — the empty culling
            // kernel was deleted in commit a311fcd. The public counter is kept
            // for API stability and will be repurposed when a real culling
            // kernel lands in a future v1.x.
            Ok(result.post_counts)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            let result = gpu.render_to_surface(
                self.target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.background_color,
                camera_projection,
                post_settings,
            )?;
            if result.submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            Ok(result.post_counts)
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

        let current_revision = scene.appearance_revision();
        if prepared.appearance_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.appearance_revision,
                    current_revision,
                    change: ChangeKind::Appearance,
                },
            });
        }

        let current_revision = scene.visibility_revision();
        if prepared.visibility_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.visibility_revision,
                    current_revision,
                    change: ChangeKind::Visibility,
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

impl RenderedFrameState {
    pub(crate) const fn dirty_state(self) -> SceneDirtyState {
        self.dirty_state
    }

    pub(crate) const fn camera(self) -> CameraKey {
        self.camera
    }

    fn matches(self, dirty_state: SceneDirtyState, camera: CameraKey) -> bool {
        self.dirty_state.structure_revision == dirty_state.structure_revision
            && self.dirty_state.transform_revision == dirty_state.transform_revision
            && self.dirty_state.appearance_revision == dirty_state.appearance_revision
            && self.dirty_state.visibility_revision == dirty_state.visibility_revision
            && self.dirty_state.interaction_revision == dirty_state.interaction_revision
            && self.camera == camera
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod post_quality_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod post_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

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
