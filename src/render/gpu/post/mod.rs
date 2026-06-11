use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::depth;
use super::material_bindings::MaterialTextureBindingMode;
use super::pipeline::create_unlit_pipeline;
use crate::render::{AntiAliasing, PostBloomConfig};

mod blit;
mod bloom;
mod bloom_fxaa;
mod copy;
mod fxaa;
mod ssao;
#[cfg(test)]
mod tests;
mod types;

pub(in crate::render::gpu) use types::PostResources;
pub(in crate::render) use types::{GpuPostPassCounts, GpuPostSettings};
use types::{PostChainOutput, PostTextureSlot};

#[cfg(any(not(target_arch = "wasm32"), feature = "browser-probe"))]
pub(super) use copy::copy_output_to_buffer;

const POST_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const POST_UNIFORM_BYTE_LEN: u64 = 32;

#[allow(clippy::too_many_arguments)]
pub(super) fn create_resources(
    device: &wgpu::Device,
    target: RasterTarget,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    texture_binding_mode: MaterialTextureBindingMode,
    depth_compare: Option<wgpu::CompareFunction>,
    surface_format: Option<wgpu::TextureFormat>,
) -> PostResources {
    let scene = create_post_texture(device, target, "scena.gpu_post.scene_encoded_srgb");
    let ping = create_post_texture(device, target, "scena.gpu_post.ping");
    let pong = create_post_texture(device, target, "scena.gpu_post.pong");
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_post.uniform"),
        size: POST_UNIFORM_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scena.gpu_post.texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
    let ssao_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scena.gpu_post.ssao_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
    let texture_bind_groups = [
        create_texture_bind_group(
            device,
            &texture_bind_group_layout,
            &scene.1,
            &uniform,
            "scena.gpu_post.scene_bind_group",
        ),
        create_texture_bind_group(
            device,
            &texture_bind_group_layout,
            &ping.1,
            &uniform,
            "scena.gpu_post.ping_bind_group",
        ),
        create_texture_bind_group(
            device,
            &texture_bind_group_layout,
            &pong.1,
            &uniform,
            "scena.gpu_post.pong_bind_group",
        ),
    ];
    let scene_pipeline = create_unlit_pipeline(
        device,
        POST_COLOR_FORMAT,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        texture_binding_mode,
        depth_compare,
    );
    let surface_blit_pipeline = surface_format
        .map(|format| blit::create_surface_pipeline(device, &texture_bind_group_layout, format));
    let surface_fxaa_pipeline = surface_format
        .map(|format| fxaa::create_surface_pipeline(device, &texture_bind_group_layout, format));
    let surface_bloom_fxaa_pipeline = surface_format.map(|format| {
        bloom_fxaa::create_surface_pipeline(device, &texture_bind_group_layout, format)
    });
    let fxaa_pipeline = fxaa::create_pipeline(device, &texture_bind_group_layout);
    let bloom_pipeline = bloom::create_pipeline(device, &texture_bind_group_layout);
    let ssao_pipeline = ssao::create_pipeline(device, &ssao_bind_group_layout);

    PostResources {
        target,
        scene_texture: scene.0,
        scene_view: scene.1,
        ping_texture: ping.0,
        ping_view: ping.1,
        pong_texture: pong.0,
        pong_view: pong.1,
        uniform,
        ssao_bind_group_layout,
        texture_bind_groups,
        scene_pipeline,
        surface_blit_pipeline,
        surface_fxaa_pipeline,
        surface_bloom_fxaa_pipeline,
        fxaa_pipeline,
        bloom_pipeline,
        ssao_pipeline,
    }
}

pub(super) fn resources_match(resources: &PostResources, target: RasterTarget) -> bool {
    resources.target == target
}

pub(super) const fn scene_view(resources: &PostResources) -> &wgpu::TextureView {
    &resources.scene_view
}

pub(super) fn encode_chain(
    encoder: &mut wgpu::CommandEncoder,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &PostResources,
    settings: GpuPostSettings,
    depth_prepass: Option<&depth::DepthPrepassResources>,
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
) {
    blit::encode(
        encoder,
        pipeline,
        bind_group(resources, output.slot),
        target_view,
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
    );
}

#[allow(dead_code)]
pub(super) fn encode_bloom_fxaa_to_view(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    output: PostChainOutput,
    target_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    config: PostBloomConfig,
) {
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
    bloom_fxaa::encode(
        encoder,
        pipeline,
        bind_group(resources, output.slot),
        target_view,
    );
}

pub(super) fn surface_blit_pipeline(resources: &PostResources) -> Option<&wgpu::RenderPipeline> {
    resources.surface_blit_pipeline.as_ref()
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

pub(super) fn create_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    uniform: &wgpu::Buffer,
    label: &'static str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

fn create_post_texture(
    device: &wgpu::Device,
    target: RasterTarget,
    label: &'static str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: POST_COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
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
