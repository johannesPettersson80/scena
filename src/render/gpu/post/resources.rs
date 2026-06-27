use super::super::super::RasterTarget;
use super::super::material_bindings::MaterialTextureBindingMode;
use super::super::pipeline::{GPU_COLOR_FORMAT, create_unlit_pipeline_set};
use super::types::PostResources;
use super::{blit, bloom, bloom_fxaa, dof, fxaa, ssao, ssr};

pub(super) const POST_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const POST_UNIFORM_BYTE_LEN: u64 = 32;

#[allow(clippy::too_many_arguments)]
pub(in crate::render::gpu) fn create_resources(
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
    let scene_pipelines = create_unlit_pipeline_set(
        device,
        POST_COLOR_FORMAT,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        texture_binding_mode,
        depth_compare,
        1,
    );
    let scene_msaa4_pipelines = create_unlit_pipeline_set(
        device,
        POST_COLOR_FORMAT,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        texture_binding_mode,
        depth_compare,
        4,
    );
    let output_blit_pipeline =
        blit::create_srgb_pipeline(device, &texture_bind_group_layout, GPU_COLOR_FORMAT);
    let surface_blit_pipeline = surface_format
        .map(|format| blit::create_surface_pipeline(device, &texture_bind_group_layout, format));
    let surface_fxaa_pipeline = surface_format
        .map(|format| fxaa::create_surface_pipeline(device, &texture_bind_group_layout, format));
    let surface_bloom_fxaa_pipeline = surface_format.map(|format| {
        bloom_fxaa::create_surface_pipeline(device, &texture_bind_group_layout, format)
    });
    let fxaa_pipeline = fxaa::create_pipeline(device, &texture_bind_group_layout);
    let ssr_pipeline = ssr::create_pipeline(device, &texture_bind_group_layout);
    let bloom_pipeline = bloom::create_pipeline(device, &texture_bind_group_layout);
    let ssao_pipeline = ssao::create_pipeline(device, &ssao_bind_group_layout);
    let depth_of_field_pipeline = dof::create_pipeline(device, &ssao_bind_group_layout);

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
        scene_pipelines,
        scene_msaa4_pipelines,
        scene_msaa8_pipelines: None,
        output_blit_pipeline,
        surface_blit_pipeline,
        surface_fxaa_pipeline,
        surface_bloom_fxaa_pipeline,
        fxaa_pipeline,
        ssr_pipeline,
        bloom_pipeline,
        ssao_pipeline,
        depth_of_field_pipeline,
    }
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

fn create_texture_bind_group(
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
