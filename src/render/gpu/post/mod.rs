use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::depth;
use super::pipeline::{UnlitPipelines, create_unlit_pipeline_set};
use super::shader_manifest::{ShaderVariantId, create_shader_module};
#[cfg(target_arch = "wasm32")]
use crate::render::PostBloomConfig;

pub(super) mod blit;
pub(super) mod bloom;
pub(super) mod bloom_fxaa;
pub(super) mod dof;
pub(super) mod fxaa;
mod pipeline_helpers;
mod resources;
pub(super) mod ssao;
pub(super) mod ssr;
#[cfg(test)]
mod tests;
mod types;

pub(super) use blit::{
    LINEAR_TARGET_SHADER as BLIT_LINEAR_SHADER, SRGB_BYTE_TARGET_SHADER as BLIT_SRGB_BYTE_SHADER,
};
pub(super) use bloom::SHADER as BLOOM_SHADER;
pub(super) use bloom_fxaa::{
    LINEAR_TARGET_SHADER as BLOOM_FXAA_LINEAR_SHADER,
    SRGB_BYTE_TARGET_SHADER as BLOOM_FXAA_SRGB_BYTE_SHADER,
};
pub(super) use dof::SHADER as DOF_SHADER;
pub(super) use fxaa::{
    LINEAR_TARGET_SHADER as FXAA_LINEAR_SHADER, SRGB_BYTE_TARGET_SHADER as FXAA_SRGB_BYTE_SHADER,
};
use pipeline_helpers::{bind_group, depth_bind_group, view, write_uniform};
#[allow(unused_imports)]
pub(super) use pipeline_helpers::{
    create_post_pipeline, create_post_pipeline_with_shader, create_post_shader,
    output_blit_pipeline, readback_blit_pipeline, surface_blit_pipeline,
    surface_bloom_fxaa_pipeline, surface_fxaa_pipeline,
};
pub(super) use resources::{create_resources, resource_stats};
pub(super) use ssao::SHADER as SSAO_SHADER;
pub(super) use ssr::SHADER as SSR_SHADER;
pub(in crate::render::gpu) use types::PostResources;
pub(in crate::render) use types::{GpuOutputPlan, GpuPostPassCounts, GpuPostSettings};
use types::{POST_UNIFORM_BYTE_LEN, PostChainOutput, PostTextureSlot, PostUniformSlot};

pub(super) fn resources_match(resources: &PostResources, target: RasterTarget) -> bool {
    resources.target == target
}

pub(super) fn scene_pipelines(resources: &PostResources, sample_count: u32) -> UnlitPipelines<'_> {
    match sample_count {
        4 => resources.scene_msaa4_pipelines.refs(),
        8 => resources
            .scene_msaa8_pipelines
            .as_ref()
            .expect("post MSAA8 pipelines must be prepared before encoding")
            .refs(),
        _ => resources.scene_pipelines.refs(),
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) fn ensure_scene_msaa8_pipelines(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    triangle_shader: &wgpu::ShaderModule,
    resources: &mut PostResources,
    target: RasterTarget,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    semantic_aov_capture_enabled: bool,
    material_features: super::material_uniform::MaterialShaderFeatures,
) -> Result<(), RenderError> {
    if resources.scene_msaa8_pipelines.is_some() {
        return Ok(());
    }
    if !super::msaa::texture_format_supports_sample_count(device, adapter, scene_color_format(), 8)
    {
        return Err(RenderError::UnsupportedSampleCount {
            backend: target.backend,
            requested: 8,
            maximum: super::msaa::max_supported_sample_count(
                device,
                adapter,
                &[scene_color_format()],
            ),
        });
    }
    resources.scene_msaa8_pipelines = Some(create_unlit_pipeline_set(
        device,
        triangle_shader,
        scene_color_format(),
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        depth_compare,
        8,
        semantic_aov_capture_enabled.then_some(super::semantic_aov::FORMAT),
        material_features,
    ));
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) const fn scene_color_format() -> wgpu::TextureFormat {
    resources::POST_COLOR_FORMAT
}

pub(super) const fn scene_view(resources: &PostResources) -> &wgpu::TextureView {
    &resources.scene_view
}

#[allow(dead_code)]
pub(super) fn output_view(
    resources: &PostResources,
    output: PostChainOutput,
) -> &wgpu::TextureView {
    view(resources, output.slot)
}

pub(super) fn encode_chain(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    settings: GpuPostSettings,
    depth_prepass: Option<&depth::DepthPrepassResources>,
    draw_submissions: &mut u64,
) -> Result<(PostChainOutput, GpuPostPassCounts), RenderError> {
    let mut current = PostTextureSlot::Scene;
    let mut next = PostTextureSlot::Ping;
    let mut counts = GpuPostPassCounts::default();

    if let Some(config) = settings.reflections {
        write_uniform(
            encoder,
            queue,
            resources,
            PostUniformSlot::Reflections,
            [
                resources.target.width as f32,
                resources.target.height as f32,
                0.0,
                0.0,
                config.strength(),
                config.roughness(),
                config.horizon_fraction(),
                config.fade(),
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        );
        ssr::encode(
            encoder,
            &resources.ssr_pipeline,
            bind_group(resources, current),
            view(resources, next),
            draw_submissions,
        );
        current = next;
        next = next.alternate();
        counts.screen_space_reflections = 1;
    }

    if let Some(config) = settings.ambient_occlusion {
        let Some(depth_prepass) = depth_prepass else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: resources.target.backend,
            });
        };
        write_uniform(
            encoder,
            queue,
            resources,
            PostUniformSlot::AmbientOcclusion,
            [
                resources.target.width as f32,
                resources.target.height as f32,
                config.radius_px() as f32,
                config.intensity(),
                config.depth_threshold(),
                if depth_prepass.reversed_z() { 1.0 } else { 0.0 },
                depth_prepass.clear_depth(),
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        );
        let Some(_depth_color_view) = depth_prepass.color_view() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: resources.target.backend,
            });
        };
        ssao::encode(
            encoder,
            &resources.ssao_pipeline,
            depth_bind_group(resources, current)?,
            view(resources, next),
            draw_submissions,
        );
        current = next;
        next = next.alternate();
        counts.ambient_occlusion = 1;
    }

    if let Some(config) = settings.depth_of_field() {
        let Some(depth_prepass) = depth_prepass else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: resources.target.backend,
            });
        };
        let physical = config.physical_parameters();
        let depth = config.depth_parameters();
        write_uniform(
            encoder,
            queue,
            resources,
            PostUniformSlot::DepthOfField,
            [
                resources.target.width as f32,
                resources.target.height as f32,
                config.focus_depth(),
                f32::from(config.radius_px()),
                physical[0],
                physical[1],
                physical[2],
                physical[3],
                depth[0],
                depth[1],
                depth[2],
                depth_prepass.clear_depth(),
            ],
        );
        let Some(_depth_color_view) = depth_prepass.color_view() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: resources.target.backend,
            });
        };
        dof::encode(
            encoder,
            &resources.depth_of_field_pipeline,
            depth_bind_group(resources, current)?,
            view(resources, next),
            draw_submissions,
        );
        current = next;
        next = next.alternate();
        counts.depth_of_field = 1;
    }

    if let Some(config) = settings.bloom {
        write_uniform(
            encoder,
            queue,
            resources,
            PostUniformSlot::Bloom,
            [
                resources.target.width as f32,
                resources.target.height as f32,
                srgb8_threshold_to_linear(config.threshold_srgb()),
                config.intensity(),
                config.radius_px() as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        );
        bloom::encode(
            encoder,
            &resources.bloom_pipeline,
            bind_group(resources, current),
            view(resources, next),
            draw_submissions,
        );
        current = next;
        next = next.alternate();
        counts.bloom = 1;
    }

    if settings.anti_aliasing.uses_post_fxaa() {
        write_uniform(
            encoder,
            queue,
            resources,
            PostUniformSlot::Fxaa,
            [
                resources.target.width as f32,
                resources.target.height as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        );
        fxaa::encode(
            encoder,
            &resources.fxaa_pipeline,
            bind_group(resources, current),
            view(resources, next),
            draw_submissions,
        );
        current = next;
        counts.fxaa = 1;
    }

    Ok((PostChainOutput { slot: current }, counts))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode_blit_to_view(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    output: PostChainOutput,
    target_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    exposure_scale: f32,
    tonemapper_mode: f32,
    white_balance: [f32; 4],
    draw_submissions: &mut u64,
) {
    write_uniform(
        encoder,
        queue,
        resources,
        PostUniformSlot::Surface,
        [
            resources.target.width as f32,
            resources.target.height as f32,
            0.0,
            0.0,
            exposure_scale,
            tonemapper_mode,
            0.0,
            0.0,
            white_balance[0],
            white_balance[1],
            white_balance[2],
            white_balance[3],
        ],
    );
    blit::encode(
        encoder,
        pipeline,
        bind_group(resources, output.slot),
        target_view,
        draw_submissions,
    );
}

#[allow(dead_code)]
pub(super) fn encode_fxaa_to_view(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    output: PostChainOutput,
    target_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    draw_submissions: &mut u64,
) {
    write_uniform(
        encoder,
        queue,
        resources,
        PostUniformSlot::Surface,
        [
            resources.target.width as f32,
            resources.target.height as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            1.0,
            1.0,
            0.0,
        ],
    );
    fxaa::encode(
        encoder,
        pipeline,
        bind_group(resources, output.slot),
        target_view,
        draw_submissions,
    );
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(super) fn encode_bloom_fxaa_to_view(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    inputs: BloomFxaaToViewInputs<'_>,
) {
    write_uniform(
        encoder,
        queue,
        resources,
        PostUniformSlot::Surface,
        [
            resources.target.width as f32,
            resources.target.height as f32,
            srgb8_threshold_to_linear(inputs.config.threshold_srgb()),
            inputs.config.intensity(),
            inputs.config.radius_px() as f32,
            0.0,
            0.0,
            0.0,
            1.0,
            1.0,
            1.0,
            0.0,
        ],
    );
    bloom_fxaa::encode(
        encoder,
        inputs.pipeline,
        bind_group(resources, inputs.output.slot),
        inputs.target_view,
        inputs.draw_submissions,
    );
}

fn srgb8_threshold_to_linear(value: u8) -> f32 {
    crate::render::color_contract::srgb_channel_to_linear(f32::from(value) / 255.0)
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(super) struct BloomFxaaToViewInputs<'a> {
    pub(super) output: PostChainOutput,
    pub(super) target_view: &'a wgpu::TextureView,
    pub(super) pipeline: &'a wgpu::RenderPipeline,
    pub(super) config: PostBloomConfig,
    pub(super) draw_submissions: &'a mut u64,
}
