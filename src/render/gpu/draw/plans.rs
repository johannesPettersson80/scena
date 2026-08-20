use crate::diagnostics::RenderError;
use crate::render::RasterTarget;
use crate::render::camera::CameraProjection;
use crate::render::prepare::PreparedGpuLightUniform;

use super::super::draw_common::{
    camera_position_uniform, identity_matrix, target_color_management_uniform,
};
use super::super::output::OutputUniformUpload;
use super::super::{
    GpuDeviceState, GpuPostPassCounts, GpuPreparedResources, GpuRenderResult, post, readback,
    surface_frame,
};

pub(super) struct NativeFrameResultInputs {
    pub(super) reconfigure_after_present: bool,
    pub(super) reconfigurations: u64,
    pub(super) surface_acquire_retries: u64,
    pub(super) post_counts: GpuPostPassCounts,
    pub(super) draw_submissions: u64,
    pub(super) native_scene_color_passes: u64,
    pub(super) readback: bool,
    pub(super) meter_submitted: bool,
}

impl GpuDeviceState {
    pub(super) fn finalize_native_frame(
        &mut self,
        target: RasterTarget,
        mut inputs: NativeFrameResultInputs,
    ) -> Result<GpuRenderResult, RenderError> {
        if inputs.reconfigure_after_present
            && let Some(surface) = self.surface.as_mut()
        {
            let change = surface_frame::refresh_surface_configuration(
                surface,
                &self.adapter,
                &self.device,
                target,
            );
            inputs.reconfigurations = inputs.reconfigurations.saturating_add(1);
            if change.requires_reprepare() {
                return Err(RenderError::SurfaceConfigurationChanged {
                    backend: target.backend,
                });
            }
        }
        let readback = u64::from(inputs.readback);
        let meter_submitted = u64::from(inputs.meter_submitted);
        Ok(GpuRenderResult {
            submitted: true,
            post_counts: inputs.post_counts,
            draw_submissions: inputs.draw_submissions,
            native_scene_color_passes: inputs.native_scene_color_passes,
            readback_copies: readback,
            readback_bytes_copied: readback.saturating_mul(target.byte_len() as u64),
            map_requests: readback,
            blocking_polls: readback,
            blocking_waits: readback,
            cpu_frame_copy_bytes: readback.saturating_mul(target.byte_len() as u64),
            auto_exposure_meter_submissions: meter_submitted,
            auto_exposure_meter_samples: meter_submitted
                .saturating_mul(readback::AUTO_EXPOSURE_SAMPLE_COUNT as u64),
            surface_skip: None,
            surface_reconfigurations: inputs.reconfigurations,
            surface_acquire_retries: inputs.surface_acquire_retries,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeSceneTargetPlan {
    OffscreenOnly,
    OffscreenAndSurface,
    DirectSurface,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeDepthSource {
    Scene,
    ResolvedOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NativeSurfaceDepthPlan {
    pub(super) scene: NativeDepthSource,
    pub(super) overlay: NativeDepthSource,
}

pub(super) const fn native_surface_depth_plan(sample_count: u32) -> NativeSurfaceDepthPlan {
    NativeSurfaceDepthPlan {
        scene: NativeDepthSource::Scene,
        overlay: if sample_count > 1 {
            NativeDepthSource::ResolvedOverlay
        } else {
            NativeDepthSource::Scene
        },
    }
}

pub(super) fn native_depth_view(
    resources: &GpuPreparedResources,
    source: NativeDepthSource,
) -> Option<&wgpu::TextureView> {
    match source {
        NativeDepthSource::Scene => resources.depth_prepass.as_ref().map(|depth| &depth.view),
        NativeDepthSource::ResolvedOverlay => resources
            .overlay_depth_prepass
            .as_ref()
            .map(|depth| &depth.view),
    }
}

pub(super) const fn native_scene_target_plan(
    has_surface: bool,
    post_enabled: bool,
    readback: bool,
) -> NativeSceneTargetPlan {
    if post_enabled {
        NativeSceneTargetPlan::Post
    } else if has_surface && !readback {
        NativeSceneTargetPlan::DirectSurface
    } else if has_surface {
        NativeSceneTargetPlan::OffscreenAndSurface
    } else {
        NativeSceneTargetPlan::OffscreenOnly
    }
}

pub(super) fn native_scene_color_format(
    post_enabled: bool,
    surface_format: Option<wgpu::TextureFormat>,
    readback: bool,
) -> wgpu::TextureFormat {
    if post_enabled {
        post::scene_color_format()
    } else if !readback && let Some(surface_format) = surface_format {
        surface_format
    } else {
        super::super::pipeline::GPU_COLOR_FORMAT
    }
}

pub(super) fn native_color_management(
    color_management: [f32; 4],
    scene_format: wgpu::TextureFormat,
    surface_format: Option<wgpu::TextureFormat>,
    reflections: Option<[f32; 2]>,
) -> ([f32; 4], Option<[f32; 4]>) {
    let mut scene = target_color_management_uniform(color_management, scene_format);
    let mut surface =
        surface_format.map(|format| target_color_management_uniform(color_management, format));
    if let Some([strength, roughness]) = reflections {
        scene[2] = strength;
        scene[3] = roughness;
        if let Some(surface) = surface.as_mut() {
            surface[2] = strength;
            surface[3] = roughness;
        }
    }
    (scene, surface)
}

/// Builds the per-target output uniform while keeping native draw submission
/// focused on pass scheduling rather than camera/output serialization.
#[allow(clippy::too_many_arguments)]
pub(super) fn native_output_uniform_upload(
    target: RasterTarget,
    exposure_ev: f32,
    color_management: [f32; 4],
    white_balance: [f32; 4],
    camera_projection: &CameraProjection,
    light_from_world: [f32; 16],
    lighting: PreparedGpuLightUniform,
    clipping_planes: [[f32; 4]; super::super::output::MAX_OUTPUT_CLIPPING_PLANES],
    clipping_control: [f32; 4],
) -> OutputUniformUpload {
    OutputUniformUpload {
        exposure_ev,
        view_from_world: camera_projection
            .view_from_world_matrix()
            .unwrap_or_else(identity_matrix),
        clip_from_view: camera_projection
            .clip_from_view_matrix()
            .unwrap_or_else(identity_matrix),
        clip_from_world: camera_projection
            .clip_from_world_matrix()
            .unwrap_or_else(identity_matrix),
        light_from_world,
        camera_position: camera_position_uniform(camera_projection),
        viewport: [target.width as f32, target.height as f32],
        near_far: camera_projection.near_far(),
        color_management,
        white_balance,
        lighting,
        clipping_planes,
        clipping_control,
    }
}

pub(super) fn validate_native_sample_count(
    state: &GpuDeviceState,
    target: RasterTarget,
    scene_format: wgpu::TextureFormat,
    sample_count: u32,
) -> Result<(), RenderError> {
    let maximum =
        state.max_supported_sample_count_cached(&[scene_format, wgpu::TextureFormat::Depth32Float]);
    if sample_count > maximum {
        return Err(RenderError::UnsupportedSampleCount {
            backend: target.backend,
            requested: sample_count,
            maximum,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        NativeDepthSource, NativeSceneTargetPlan, native_scene_color_format,
        native_scene_target_plan, native_surface_depth_plan,
    };

    #[test]
    fn direct_native_surface_uses_its_actual_format_for_output_transfer() {
        assert_eq!(
            native_scene_color_format(false, Some(wgpu::TextureFormat::Bgra8Unorm), false),
            wgpu::TextureFormat::Bgra8Unorm,
        );
        assert_eq!(
            native_scene_color_format(false, Some(wgpu::TextureFormat::Bgra8UnormSrgb), false),
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );
        assert_eq!(
            native_scene_color_format(false, Some(wgpu::TextureFormat::Bgra8Unorm), true),
            super::super::super::pipeline::GPU_COLOR_FORMAT,
            "readback keeps the scene pass in the offscreen capture format",
        );
    }

    #[test]
    fn present_only_without_post_targets_the_surface_once() {
        assert_eq!(
            native_scene_target_plan(true, false, false),
            NativeSceneTargetPlan::DirectSurface
        );
        assert_eq!(
            native_scene_target_plan(true, false, true),
            NativeSceneTargetPlan::OffscreenAndSurface
        );
        assert_eq!(
            native_scene_target_plan(false, false, false),
            NativeSceneTargetPlan::OffscreenOnly
        );
        assert_eq!(
            native_scene_target_plan(true, true, false),
            NativeSceneTargetPlan::Post
        );
    }

    #[test]
    fn msaa_surface_scene_and_resolved_overlays_use_matching_depth_samples() {
        let single_sample = native_surface_depth_plan(1);
        assert_eq!(single_sample.scene, NativeDepthSource::Scene);
        assert_eq!(single_sample.overlay, NativeDepthSource::Scene);

        let msaa4 = native_surface_depth_plan(4);
        assert_eq!(msaa4.scene, NativeDepthSource::Scene);
        assert_eq!(msaa4.overlay, NativeDepthSource::ResolvedOverlay);
    }
}
