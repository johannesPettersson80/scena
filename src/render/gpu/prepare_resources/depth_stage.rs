//! Depth-derived resources the native frame builds before its colour pass.

use crate::render::RasterTarget;
use crate::render::gpu::GpuOutputPlan;
use crate::render::gpu::alpha_mask::AlphaMaskPipelineInputs;
use crate::render::gpu::ambient_occlusion::AmbientOcclusionResources;
use crate::render::gpu::depth::{self, DepthPrepassResources};
use crate::render::prepare::PreparedDepthStats;

pub(super) struct DepthStage {
    pub(super) depth_prepass: Option<DepthPrepassResources>,
    /// Single-sampled depth for overlays and occlusion when the colour pass
    /// is multisampled.
    pub(super) overlay_depth_prepass: Option<DepthPrepassResources>,
    pub(super) ambient_occlusion: AmbientOcclusionResources,
}

pub(super) struct DepthStageInputs<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) target: RasterTarget,
    pub(super) depth_stats: PreparedDepthStats,
    pub(super) output_plan: GpuOutputPlan,
    pub(super) sample_count: u32,
    pub(super) output_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) draw_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) alpha_mask: Option<AlphaMaskPipelineInputs<'a>>,
}

pub(super) fn create_depth_stage(inputs: DepthStageInputs<'_>) -> DepthStage {
    let create = |reversed_z, depth_color_enabled, sample_count| {
        depth::create_depth_prepass_resources(
            inputs.device,
            inputs.target,
            reversed_z,
            inputs.output_bind_group_layout,
            inputs.draw_bind_group_layout,
            depth_color_enabled,
            sample_count,
            inputs.alpha_mask,
        )
    };
    let depth_prepass = (inputs.depth_stats.passes > 0).then(|| {
        create(
            inputs.depth_stats.reversed_z,
            inputs.output_plan.depth_color_enabled(),
            inputs.sample_count,
        )
    });
    let overlay_depth_prepass = depth_prepass
        .as_ref()
        .filter(|_| inputs.sample_count > 1)
        .map(|depth_prepass| create(depth_prepass.reversed_z(), false, 1));
    // Occlusion reads single-sampled depth: the overlay prepass when the
    // colour pass is multisampled, otherwise the prepass itself.
    let occlusion_depth = overlay_depth_prepass.as_ref().or(depth_prepass.as_ref());
    let ambient_occlusion = AmbientOcclusionResources::create(
        inputs.device,
        inputs.target,
        occlusion_depth
            .filter(|_| inputs.output_plan.ambient_occlusion_enabled())
            .map(|depth_prepass| &depth_prepass.view),
    );
    DepthStage {
        depth_prepass,
        overlay_depth_prepass,
        ambient_occlusion,
    }
}
