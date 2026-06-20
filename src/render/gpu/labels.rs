use crate::render::prepare::PreparedLabelAtlas;

const FINAL_SHADER: &str = include_str!("labels.wgsl");
const ENCODED_SHADER: &str = include_str!("labels_encoded.wgsl");
const QUAD_VERTEX_BYTE_LEN: usize = 2 * std::mem::size_of::<f32>();
const INSTANCE_FLOATS: usize = 23;
const INSTANCE_BYTE_LEN: usize = INSTANCE_FLOATS * std::mem::size_of::<f32>();
const POST_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const QUAD_VERTICES: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
const QUAD_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x2,
    offset: 0,
    shader_location: 0,
}];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 8] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 3 * std::mem::size_of::<f32>() as u64,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 6 * std::mem::size_of::<f32>() as u64,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 9 * std::mem::size_of::<f32>() as u64,
        shader_location: 4,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 10 * std::mem::size_of::<f32>() as u64,
        shader_location: 5,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 14 * std::mem::size_of::<f32>() as u64,
        shader_location: 6,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 18 * std::mem::size_of::<f32>() as u64,
        shader_location: 7,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 22 * std::mem::size_of::<f32>() as u64,
        shader_location: 8,
    },
];

#[derive(Debug)]
pub(super) struct LabelResources {
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    instance_capacity: usize,
    #[allow(dead_code)]
    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    flat_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    surface_pipeline: Option<wgpu::RenderPipeline>,
    surface_flat_pipeline: Option<wgpu::RenderPipeline>,
    post_pipeline: wgpu::RenderPipeline,
    instance_count: u32,
}

pub(super) struct LabelPass<'a> {
    pub(super) view: &'a wgpu::TextureView,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) resources: &'a LabelResources,
    pub(super) pipeline: &'a wgpu::RenderPipeline,
    pub(super) label: &'static str,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) struct LabelResourceDescriptor<'a> {
    pub(super) target_format: wgpu::TextureFormat,
    pub(super) surface_format: Option<wgpu::TextureFormat>,
    pub(super) output_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) depth_compare: Option<wgpu::CompareFunction>,
    pub(super) labels: &'a PreparedLabelAtlas,
}

pub(super) fn create_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    descriptor: LabelResourceDescriptor<'_>,
) -> LabelResources {
    let quad_bytes = encode_quad_vertices();
    let quad_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_labels.quad_vertices"),
        size: quad_bytes.len() as u64,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: true,
    });
    {
        let mut mapped = quad_vertex_buffer.slice(..).get_mapped_range_mut();
        mapped.copy_from_slice(&quad_bytes);
    }
    quad_vertex_buffer.unmap();

    let instance_bytes = encode_instances(descriptor.labels);
    let instance_capacity = descriptor.labels.quads().len();
    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_labels.instances"),
        size: instance_bytes.len().max(4) as u64,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: true,
    });
    if !instance_bytes.is_empty() {
        let mut mapped = instance_buffer.slice(..).get_mapped_range_mut();
        mapped.copy_from_slice(&instance_bytes);
    }
    instance_buffer.unmap();

    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.gpu_labels.atlas_texture"),
        size: wgpu::Extent3d {
            width: descriptor.labels.width(),
            height: descriptor.labels.height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        descriptor.labels.rgba8(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * descriptor.labels.width()),
            rows_per_image: Some(descriptor.labels.height()),
        },
        wgpu::Extent3d {
            width: descriptor.labels.width(),
            height: descriptor.labels.height(),
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let atlas_layout = create_atlas_bind_group_layout(device);
    let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.gpu_labels.atlas_bind_group"),
        layout: &atlas_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&atlas_view),
        }],
    });
    let pipeline = create_pipeline(
        device,
        descriptor.target_format,
        descriptor.output_bind_group_layout,
        &atlas_layout,
        descriptor.depth_compare,
        shader_for_format(descriptor.target_format),
        "scena.gpu_labels.pipeline",
    );
    let flat_pipeline = create_pipeline(
        device,
        descriptor.target_format,
        descriptor.output_bind_group_layout,
        &atlas_layout,
        None,
        shader_for_format(descriptor.target_format),
        "scena.gpu_labels.flat_pipeline",
    );
    let surface_pipeline = descriptor.surface_format.map(|format| {
        create_pipeline(
            device,
            format,
            descriptor.output_bind_group_layout,
            &atlas_layout,
            descriptor.depth_compare,
            shader_for_format(format),
            "scena.gpu_labels.surface_pipeline",
        )
    });
    let surface_flat_pipeline = descriptor.surface_format.map(|format| {
        create_pipeline(
            device,
            format,
            descriptor.output_bind_group_layout,
            &atlas_layout,
            None,
            shader_for_format(format),
            "scena.gpu_labels.surface_flat_pipeline",
        )
    });
    let post_pipeline = create_pipeline(
        device,
        POST_COLOR_FORMAT,
        descriptor.output_bind_group_layout,
        &atlas_layout,
        None,
        ENCODED_SHADER,
        "scena.gpu_labels.post_pipeline",
    );
    LabelResources {
        quad_vertex_buffer,
        instance_buffer,
        instance_capacity,
        atlas_texture,
        atlas_bind_group,
        pipeline,
        flat_pipeline,
        surface_pipeline,
        surface_flat_pipeline,
        post_pipeline,
        instance_count: descriptor.labels.quads().len() as u32,
    }
}

pub(super) fn encode_pass(encoder: &mut wgpu::CommandEncoder, inputs: LabelPass<'_>) {
    if inputs.resources.instance_count == 0 {
        return;
    }
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: inputs.view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    });
    let depth_stencil_attachment =
        inputs
            .depth_view
            .map(|view| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(inputs.label),
        color_attachments: &[color_attachment],
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(inputs.pipeline);
    pass.set_bind_group(0, inputs.output_bind_group, &[]);
    pass.set_bind_group(1, &inputs.resources.atlas_bind_group, &[]);
    pass.set_vertex_buffer(0, inputs.resources.quad_vertex_buffer.slice(..));
    pass.set_vertex_buffer(1, inputs.resources.instance_buffer.slice(..));
    pass.draw(0..4, 0..inputs.resources.instance_count);
    *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
}

pub(super) const fn pipeline(resources: &LabelResources) -> &wgpu::RenderPipeline {
    &resources.pipeline
}

pub(super) const fn flat_pipeline(resources: &LabelResources) -> &wgpu::RenderPipeline {
    &resources.flat_pipeline
}

#[allow(dead_code)]
pub(super) fn surface_pipeline(resources: &LabelResources) -> Option<&wgpu::RenderPipeline> {
    resources.surface_pipeline.as_ref()
}

#[allow(dead_code)]
pub(super) fn surface_flat_pipeline(resources: &LabelResources) -> Option<&wgpu::RenderPipeline> {
    resources.surface_flat_pipeline.as_ref()
}

pub(super) const fn post_pipeline(resources: &LabelResources) -> &wgpu::RenderPipeline {
    &resources.post_pipeline
}

fn create_atlas_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.gpu_labels.atlas_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    atlas_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    shader_source: &'static str,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.gpu_labels.shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.gpu_labels.pipeline_layout"),
        bind_group_layouts: &[
            Some(output_bind_group_layout),
            Some(atlas_bind_group_layout),
        ],
        immediate_size: 0,
    });
    let quad_vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: QUAD_VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &QUAD_ATTRIBUTES,
    };
    let instance_buffer = wgpu::VertexBufferLayout {
        array_stride: INSTANCE_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &INSTANCE_ATTRIBUTES,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[quad_vertex_buffer, instance_buffer],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: depth_compare.map(|depth_compare| wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(depth_compare),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

const fn shader_for_format(format: wgpu::TextureFormat) -> &'static str {
    match format {
        wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb => FINAL_SHADER,
        _ => ENCODED_SHADER,
    }
}

fn encode_quad_vertices() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(QUAD_VERTICES.len() * QUAD_VERTEX_BYTE_LEN);
    for vertex in QUAD_VERTICES {
        for value in vertex {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

fn encode_instances(labels: &PreparedLabelAtlas) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(labels.quads().len() * INSTANCE_BYTE_LEN);
    for quad in labels.quads() {
        let color = quad.final_color();
        for value in [
            quad.anchor().x,
            quad.anchor().y,
            quad.anchor().z,
            quad.right().x,
            quad.right().y,
            quad.right().z,
            quad.up().x,
            quad.up().y,
            quad.up().z,
            quad.world_units_per_px(),
            quad.rect_px()[0],
            quad.rect_px()[1],
            quad.rect_px()[2],
            quad.rect_px()[3],
            quad.uv_rect()[0],
            quad.uv_rect()[1],
            quad.uv_rect()[2],
            quad.uv_rect()[3],
            color.r,
            color.g,
            color.b,
            color.a,
            if quad.solid_coverage() { 1.0 } else { 0.0 },
        ] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    #[test]
    fn label_shader_uses_manual_bilinear_atlas_sampling() {
        let shader = include_str!("labels.wgsl");
        assert!(
            shader.contains("manual_bilinear_coverage")
                && shader.contains("textureLoad")
                && shader.contains(
                    "clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)) * vec2<f32>(dims) - vec2<f32>(0.5)"
                ),
            "GPU labels must use the dedicated atlas path with manual texel-space bilinear sampling"
        );
    }
}
