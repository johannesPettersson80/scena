use super::super::super::RasterTarget;
use super::super::pipeline::{GPU_COLOR_FORMAT, create_unlit_pipeline_set};
use super::super::stats::GpuResourceStats;
use super::types::{
    LinearSceneReadbackResources, POST_UNIFORM_BYTE_LEN, POST_UNIFORM_SLOT_COUNT, PostResources,
};
use super::{blit, bloom, bloom_fxaa, dof, fxaa, ssao, ssr};

pub(super) const POST_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[allow(clippy::too_many_arguments)]
pub(in crate::render::gpu) fn create_resources(
    device: &wgpu::Device,
    triangle_shader: &wgpu::ShaderModule,
    target: RasterTarget,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    surface_format: Option<wgpu::TextureFormat>,
    readback_format: Option<wgpu::TextureFormat>,
    depth_color_view: Option<&wgpu::TextureView>,
    semantic_aov_capture_enabled: bool,
    scene_linear_capture_enabled: bool,
) -> PostResources {
    let scene = create_post_texture(device, target, "scena.gpu_post.scene_linear_sampling");
    let ping = create_post_texture(device, target, "scena.gpu_post.ping");
    let pong = create_post_texture(device, target, "scena.gpu_post.pong");
    let linear_scene_readback = scene_linear_capture_enabled.then(|| {
        let unpadded_bytes_per_row = target.width.saturating_mul(8);
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            .saturating_mul(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        LinearSceneReadbackResources {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scena.gpu_post.scene_linear_readback"),
                size: u64::from(padded_bytes_per_row).saturating_mul(u64::from(target.height)),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            padded_bytes_per_row,
        }
    });
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_post.uniform"),
        size: POST_UNIFORM_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let uniform_staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_post.uniform_staging"),
        size: POST_UNIFORM_BYTE_LEN * POST_UNIFORM_SLOT_COUNT,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
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
    let texture_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.gpu_post.texture_pipeline_layout"),
        bind_group_layouts: &[Some(&texture_bind_group_layout)],
        immediate_size: 0,
    });
    let depth_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.gpu_post.depth_pipeline_layout"),
        bind_group_layouts: &[Some(&ssao_bind_group_layout)],
        immediate_size: 0,
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
    let depth_texture_bind_groups = depth_color_view.map(|depth_view| {
        [
            create_depth_texture_bind_group(
                device,
                &ssao_bind_group_layout,
                &scene.1,
                depth_view,
                &uniform,
                "scena.gpu_post.scene_depth_bind_group",
            ),
            create_depth_texture_bind_group(
                device,
                &ssao_bind_group_layout,
                &ping.1,
                depth_view,
                &uniform,
                "scena.gpu_post.ping_depth_bind_group",
            ),
            create_depth_texture_bind_group(
                device,
                &ssao_bind_group_layout,
                &pong.1,
                depth_view,
                &uniform,
                "scena.gpu_post.pong_depth_bind_group",
            ),
        ]
    });
    let scene_pipelines = create_unlit_pipeline_set(
        device,
        triangle_shader,
        POST_COLOR_FORMAT,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        depth_compare,
        1,
        semantic_aov_capture_enabled.then_some(super::super::semantic_aov::FORMAT),
    );
    let scene_msaa4_pipelines = create_unlit_pipeline_set(
        device,
        triangle_shader,
        POST_COLOR_FORMAT,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        depth_compare,
        4,
        semantic_aov_capture_enabled.then_some(super::super::semantic_aov::FORMAT),
    );
    let output_blit_pipeline =
        blit::create_target_pipeline(device, &texture_pipeline_layout, GPU_COLOR_FORMAT);
    let readback_blit_pipeline = readback_format
        .map(|format| blit::create_target_pipeline(device, &texture_pipeline_layout, format));
    let surface_blit_pipeline = surface_format
        .map(|format| blit::create_surface_pipeline(device, &texture_pipeline_layout, format));
    let surface_bloom_fxaa_pipeline = surface_format.map(|format| {
        bloom_fxaa::create_surface_pipeline(device, &texture_pipeline_layout, format)
    });
    let (fxaa_pipeline, surface_fxaa_pipeline) =
        fxaa::create_pipelines(device, &texture_pipeline_layout, surface_format);
    let ssr_pipeline = ssr::create_pipeline(device, &texture_pipeline_layout);
    let bloom_pipeline = bloom::create_pipeline(device, &texture_pipeline_layout);
    let ssao_pipeline = ssao::create_pipeline(device, &depth_pipeline_layout);
    let depth_of_field_pipeline = dof::create_pipeline(device, &depth_pipeline_layout);

    PostResources {
        target,
        scene_texture: scene.0,
        scene_view: scene.1,
        linear_scene_readback,
        ping_texture: ping.0,
        ping_view: ping.1,
        pong_texture: pong.0,
        pong_view: pong.1,
        uniform,
        uniform_staging,
        texture_bind_groups,
        depth_texture_bind_groups,
        scene_pipelines,
        scene_msaa4_pipelines,
        scene_msaa8_pipelines: None,
        output_blit_pipeline,
        readback_blit_pipeline,
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

pub(in crate::render::gpu) fn resource_stats(resources: &PostResources) -> GpuResourceStats {
    let mesh_pipelines = 4 + u64::from(resources.scene_msaa8_pipelines.is_some()) * 2;
    let optional_surface_pipelines = u64::from(resources.surface_blit_pipeline.is_some())
        + u64::from(resources.readback_blit_pipeline.is_some())
        + u64::from(resources.surface_fxaa_pipeline.is_some())
        + u64::from(resources.surface_bloom_fxaa_pipeline.is_some());
    let pipelines = mesh_pipelines + optional_surface_pipelines + 6;
    // The mesh pipelines share the device-owned triangle module. The FXAA
    // target and optional surface pipelines share one post shader module.
    let shader_modules =
        optional_surface_pipelines + 6 - u64::from(resources.surface_fxaa_pipeline.is_some());
    GpuResourceStats {
        buffers: 2 + u64::from(resources.linear_scene_readback.is_some()),
        textures: 3,
        render_targets: 3,
        pipelines,
        bind_groups: resources.texture_bind_groups.len() as u64
            + resources
                .depth_texture_bind_groups
                .as_ref()
                .map_or(0, |groups| groups.len() as u64),
        shader_modules,
        shader_module_creations: shader_modules,
        approximate_gpu_memory_bytes: GpuResourceStats::target_bytes(resources.target, 8, 1)
            .saturating_mul(3)
            .saturating_add(POST_UNIFORM_BYTE_LEN)
            .saturating_add(POST_UNIFORM_BYTE_LEN.saturating_mul(POST_UNIFORM_SLOT_COUNT))
            .saturating_add(
                resources
                    .linear_scene_readback
                    .as_ref()
                    .map_or(0, |readback| {
                        u64::from(readback.padded_bytes_per_row)
                            .saturating_mul(u64::from(resources.target.height))
                    }),
            ),
        ..GpuResourceStats::default()
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

fn create_depth_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(depth_view),
            },
        ],
    })
}
