use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::depth;
use super::pipeline::{UnlitPipelines, create_unlit_pipeline_set};
#[cfg(target_arch = "wasm32")]
use crate::render::PostBloomConfig;

mod blit;
mod bloom;
mod bloom_fxaa;
mod copy;
mod dof;
mod fxaa;
mod resources;
mod ssao;
mod ssr;
#[cfg(test)]
mod tests;
mod types;

pub(super) use resources::{create_resources, resource_stats};
pub(in crate::render::gpu) use types::PostResources;
pub(in crate::render) use types::{GpuOutputPlan, GpuPostPassCounts, GpuPostSettings};
use types::{POST_UNIFORM_BYTE_LEN, PostChainOutput, PostTextureSlot, PostUniformSlot};

#[allow(unused_imports)]
pub(super) use copy::copy_output_to_buffer;

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
                config.aperture_f_stop(),
                if depth_prepass.reversed_z() { 1.0 } else { 0.0 },
                depth_prepass.clear_depth(),
                0.0,
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

pub(super) fn encode_blit_to_view(
    encoder: &mut wgpu::CommandEncoder,
    resources: &PostResources,
    output: PostChainOutput,
    target_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    draw_submissions: &mut u64,
) {
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
pub(super) struct BloomFxaaToViewInputs<'a> {
    pub(super) output: PostChainOutput,
    pub(super) target_view: &'a wgpu::TextureView,
    pub(super) pipeline: &'a wgpu::RenderPipeline,
    pub(super) config: PostBloomConfig,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) fn surface_blit_pipeline(resources: &PostResources) -> Option<&wgpu::RenderPipeline> {
    resources.surface_blit_pipeline.as_ref()
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) const fn output_blit_pipeline(resources: &PostResources) -> &wgpu::RenderPipeline {
    &resources.output_blit_pipeline
}

#[allow(dead_code)]
pub(super) fn surface_fxaa_pipeline(resources: &PostResources) -> Option<&wgpu::RenderPipeline> {
    resources.surface_fxaa_pipeline.as_ref()
}

#[allow(dead_code)]
pub(super) fn surface_bloom_fxaa_pipeline(
    resources: &PostResources,
) -> Option<&wgpu::RenderPipeline> {
    resources.surface_bloom_fxaa_pipeline.as_ref()
}

pub(super) fn create_post_pipeline(
    device: &wgpu::Device,
    label: &'static str,
    shader_source: &'static str,
    pipeline_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = create_post_shader(device, label, shader_source);
    create_post_pipeline_with_shader(device, label, &shader, pipeline_layout, format)
}

pub(super) fn create_post_shader(
    device: &wgpu::Device,
    label: &'static str,
    shader_source: &'static str,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    })
}

pub(super) fn create_post_pipeline_with_shader(
    device: &wgpu::Device,
    label: &'static str,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn write_uniform(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    slot: PostUniformSlot,
    values: [f32; 8],
) {
    let offset = slot.byte_offset();
    queue.write_buffer(
        &resources.uniform_staging,
        offset,
        bytemuck::cast_slice(&values),
    );
    encoder.copy_buffer_to_buffer(
        &resources.uniform_staging,
        offset,
        &resources.uniform,
        0,
        POST_UNIFORM_BYTE_LEN,
    );
}

fn view(resources: &PostResources, slot: PostTextureSlot) -> &wgpu::TextureView {
    match slot {
        PostTextureSlot::Scene => &resources.scene_view,
        PostTextureSlot::Ping => &resources.ping_view,
        PostTextureSlot::Pong => &resources.pong_view,
    }
}

fn bind_group(resources: &PostResources, slot: PostTextureSlot) -> &wgpu::BindGroup {
    match slot {
        PostTextureSlot::Scene => &resources.texture_bind_groups[0],
        PostTextureSlot::Ping => &resources.texture_bind_groups[1],
        PostTextureSlot::Pong => &resources.texture_bind_groups[2],
    }
}

fn depth_bind_group(
    resources: &PostResources,
    slot: PostTextureSlot,
) -> Result<&wgpu::BindGroup, RenderError> {
    let groups = resources.depth_texture_bind_groups.as_ref().ok_or(
        RenderError::GpuResourcesNotPrepared {
            backend: resources.target.backend,
        },
    )?;
    Ok(match slot {
        PostTextureSlot::Scene => &groups[0],
        PostTextureSlot::Ping => &groups[1],
        PostTextureSlot::Pong => &groups[2],
    })
}

#[allow(dead_code)]
fn texture(resources: &PostResources, slot: PostTextureSlot) -> &wgpu::Texture {
    match slot {
        PostTextureSlot::Scene => &resources.scene_texture,
        PostTextureSlot::Ping => &resources.ping_texture,
        PostTextureSlot::Pong => &resources.pong_texture,
    }
}
