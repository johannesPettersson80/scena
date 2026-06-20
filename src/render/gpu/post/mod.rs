use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::depth;
use super::pipeline::UnlitPipelines;
use crate::render::AntiAliasing;
#[cfg(target_arch = "wasm32")]
use crate::render::PostBloomConfig;

mod blit;
mod bloom;
mod bloom_fxaa;
mod copy;
mod fxaa;
mod resources;
mod ssao;
#[cfg(test)]
mod tests;
mod types;

pub(super) use resources::create_resources;
pub(in crate::render::gpu) use types::PostResources;
pub(in crate::render) use types::{GpuPostPassCounts, GpuPostSettings};
use types::{PostChainOutput, PostTextureSlot};

#[cfg(any(not(target_arch = "wasm32"), feature = "browser-probe"))]
#[allow(unused_imports)]
pub(super) use copy::copy_output_to_buffer;

pub(super) fn resources_match(resources: &PostResources, target: RasterTarget) -> bool {
    resources.target == target
}

pub(super) fn scene_pipelines(resources: &PostResources) -> UnlitPipelines<'_> {
    resources.scene_pipelines.refs()
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
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &PostResources,
    settings: GpuPostSettings,
    depth_prepass: Option<&depth::DepthPrepassResources>,
    draw_submissions: &mut u64,
) -> Result<(PostChainOutput, GpuPostPassCounts), RenderError> {
    let mut current = PostTextureSlot::Scene;
    let mut next = PostTextureSlot::Ping;
    let mut counts = GpuPostPassCounts::default();

    if let Some(config) = settings.ambient_occlusion {
        let Some(depth_prepass) = depth_prepass else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: resources.target.backend,
            });
        };
        write_uniform(
            queue,
            resources,
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
        let Some(depth_color_view) = depth_prepass.color_view() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: resources.target.backend,
            });
        };
        ssao::encode(
            encoder,
            device,
            &resources.ssao_bind_group_layout,
            &resources.uniform,
            &resources.ssao_pipeline,
            view(resources, current),
            depth_color_view,
            view(resources, next),
            draw_submissions,
        );
        current = next;
        next = next.alternate();
        counts.ambient_occlusion = 1;
    }

    if let Some(config) = settings.bloom {
        write_uniform(
            queue,
            resources,
            [
                resources.target.width as f32,
                resources.target.height as f32,
                config.threshold_srgb() as f32 / 255.0,
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

    if matches!(settings.anti_aliasing, AntiAliasing::Fxaa) {
        write_uniform(
            queue,
            resources,
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
        queue,
        resources,
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
        queue,
        resources,
        [
            resources.target.width as f32,
            resources.target.height as f32,
            inputs.config.threshold_srgb() as f32 / 255.0,
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
    bind_group_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.gpu_post.pipeline_layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
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

fn write_uniform(queue: &wgpu::Queue, resources: &PostResources, values: [f32; 8]) {
    queue.write_buffer(&resources.uniform, 0, bytemuck::cast_slice(&values));
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

#[allow(dead_code)]
fn texture(resources: &PostResources, slot: PostTextureSlot) -> &wgpu::Texture {
    match slot {
        PostTextureSlot::Scene => &resources.scene_texture,
        PostTextureSlot::Ping => &resources.ping_texture,
        PostTextureSlot::Pong => &resources.pong_texture,
    }
}
