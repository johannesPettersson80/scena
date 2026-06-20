//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::{cell::Cell, marker::PhantomData};

#[cfg(feature = "inspection")]
pub mod animation_introspection;
#[cfg(feature = "inspection")]
pub mod appearance;
mod background;
mod build;
mod camera;
mod color_contract;
mod cpu;
mod cpu_labels;
mod cpu_render;
mod cpu_strokes;
mod culling;
mod environment_cache;
mod exposure;
mod gpu;
#[cfg(feature = "inspection")]
pub mod introspection;
mod offscreen;
mod output;
mod prepare;
mod prepare_lifecycle;
mod prepare_retained;
#[cfg(feature = "inspection")]
pub mod quality;
mod reporting;
#[cfg(feature = "inspection")]
mod screen_bounds;
mod settings;
// PreparedSceneState stores clipping_planes: Vec<ClippingPlane> in state.rs.
mod state;
mod surface;
mod target;
#[cfg(feature = "inspection")]
pub mod visibility_diagnosis;
#[cfg(feature = "inspection")]
pub mod visual_repair;

use crate::assets::EnvironmentHandle;
use crate::diagnostics::{
    Capabilities, ChangeKind, DevicePoll, Diagnostic, GpuAdapterReport, NotPreparedReason,
    OutputColorSpace, RenderError, RenderOutcome, RendererStats,
};
use crate::material::Color;
use crate::scene::{CameraKey, ClippingPlane, Scene, SectionBox};

pub use self::background::Background;
pub use self::exposure::{
    AutoExposureConfig, AutoExposureResult, estimate_auto_exposure_from_linear_colors,
    estimate_auto_exposure_from_srgb8,
};
use self::gpu::GpuDeviceState;
pub use self::offscreen::{OffscreenTarget, PixelReadback};
use self::output::OutputTransform;
pub use self::output::{
    AntiAliasing, OrderIndependentTransparencyConfig, PostBloomConfig, ReconstructionFilter,
    ScreenSpaceAmbientOcclusionConfig, Tonemapper,
};
#[doc(hidden)]
pub use self::prepare::precompute_environment_sidecar;
pub use self::settings::{Profile, Quality, RenderMode, RendererOptions};
use self::state::{PreparedSceneState, RenderedFrameState};
use self::target::{RasterTarget, backend_for_attached_surface, validate_target_size};

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
    supersample_factor: u32,
    reconstruction_filter: ReconstructionFilter,
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
    surface_lost: Option<bool>,
    context_lost: Option<bool>,
    device_lost: Option<bool>,
    environment: Option<EnvironmentHandle>,
    environment_lighting_cache: environment_cache::EnvironmentLightingCache,
    background_color: Color,
    auto_exposure: Option<AutoExposureConfig>,
    last_auto_exposure: Option<AutoExposureResult>,
    environment_revision: u64,
    target_revision: u64,
    prepare_telemetry: PrepareTelemetry,
    #[cfg(not(target_arch = "wasm32"))]
    _headless_gpu_test_guard: Option<build::HeadlessGpuTestSupportGuard>,
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

        let gpu_target = if self.gpu.is_some() {
            Some(self.gpu_render_target()?)
        } else {
            None
        };
        let camera_projection =
            camera::CameraProjection::from_scene(scene, camera, gpu_target.unwrap_or(self.target))?;
        let primitive_count = prepared_triangle_alias_count(self.prepared_state(scene)?);
        let mut auto_exposure_attempted = false;
        let mut gpu_draw_submissions = 0;
        loop {
            if self.gpu.is_some() {
                let (clipping_planes, section_box) = {
                    let prepared = self.prepared_state(scene)?;
                    (prepared.clipping_planes.clone(), prepared.section_box)
                };
                let gpu_result = self.draw_gpu(
                    gpu_target.expect("GPU render target exists when GPU is active"),
                    &camera_projection,
                    &clipping_planes,
                    section_box,
                )?;
                gpu_draw_submissions = gpu_result.draw_submissions;
                self.stats.order_independent_transparency_passes = 0;
                self.stats.ambient_occlusion_passes = gpu_result.post_counts.ambient_occlusion;
                self.stats.bloom_passes = gpu_result.post_counts.bloom;
                self.stats.fxaa_passes = gpu_result.post_counts.fxaa;
            } else {
                let cpu_projection =
                    camera::CameraProjection::from_scene(scene, camera, self.target)?;
                self.draw_cpu(scene, camera, &cpu_projection)?;
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
        self.stats.instances = self
            .prepared_state(scene)
            .map(|prepared| {
                prepared
                    .instances
                    .iter()
                    .map(|set| set.instances().len() as u64)
                    .sum()
            })
            .unwrap_or(0);
        self.stats.gpu_draw_submissions = gpu_draw_submissions;
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
        target: RasterTarget,
        camera_projection: &camera::CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
    ) -> Result<gpu::GpuRenderResult, RenderError> {
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
            let scale = if target == self.target {
                1
            } else {
                self.supersample_factor
            };
            let mut supersample_frame = Vec::new();
            let frame = if scale > 1 {
                &mut supersample_frame
            } else {
                &mut self.frame
            };
            let result = gpu.render_to_frame(
                target,
                self.output.exposure_ev(),
                self.output.color_management_uniform(),
                self.background_color,
                camera_projection,
                clipping_planes,
                section_box,
                frame,
                post_settings,
            )?;
            if scale > 1 {
                cpu_render::downsample_rgba8_reconstruction_filter(
                    target,
                    scale,
                    supersample_frame.as_slice(),
                    self.target,
                    &mut self.frame,
                    self.reconstruction_filter,
                );
            }
            if result.submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            // self.stats.gpu_culling_dispatches stays at 0 — the empty culling
            // kernel was deleted in commit a311fcd. The public counter is kept
            // for API stability and will be repurposed when a real culling
            // kernel lands in a future v1.x.
            Ok(result)
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
                clipping_planes,
                section_box,
                post_settings,
            )?;
            if result.submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
            Ok(result)
        }
    }

    fn gpu_render_target(&self) -> Result<RasterTarget, RenderError> {
        let scale = if self.target.backend == crate::diagnostics::Backend::HeadlessGpu {
            self.supersample_factor
        } else {
            1
        };
        self::target::validate_supersample_target(self.target, scale)
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

        Ok(prepared)
    }
}

fn prepared_triangle_alias_count(prepared: &PreparedSceneState) -> u64 {
    let primitive_triangles = prepared.primitives.len() as u64;
    let instance_triangles = prepared
        .instances
        .iter()
        .map(|set| (set.primitives().len() as u64).saturating_mul(set.instances().len() as u64))
        .sum::<u64>();
    primitive_triangles.saturating_add(instance_triangles)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod phase4_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod phase5_tests;
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
