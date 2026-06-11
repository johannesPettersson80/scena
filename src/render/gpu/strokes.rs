use crate::render::prepare::PreparedStrokeSegment;

use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::vertices::DrawUniformValue;

const SHADER: &str = include_str!("strokes.wgsl");
const QUAD_VERTEX_BYTE_LEN: usize = 2 * std::mem::size_of::<f32>();
const INSTANCE_BYTE_LEN: usize = 11 * std::mem::size_of::<f32>();
const POST_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const QUAD_VERTICES: [[f32; 2]; 4] = [[-1.0, 0.0], [1.0, 0.0], [-1.0, 1.0], [1.0, 1.0]];
const QUAD_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x2,
    offset: 0,
    shader_location: 0,
}];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
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
        format: wgpu::VertexFormat::Float32x4,
        offset: 6 * std::mem::size_of::<f32>() as u64,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 10 * std::mem::size_of::<f32>() as u64,
        shader_location: 4,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StrokeDrawBatch {
    pub(super) start_instance: u32,
    pub(super) instance_count: u32,
    pub(super) draw_uniform_index: u32,
}

#[derive(Debug)]
pub(super) struct StrokeResources {
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    instance_capacity: usize,
    pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    surface_pipeline: Option<wgpu::RenderPipeline>,
    post_pipeline: wgpu::RenderPipeline,
    pub(super) batches: Vec<StrokeDrawBatch>,
}

pub(super) struct StrokePass<'a> {
    pub(super) view: &'a wgpu::TextureView,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) resources: &'a StrokeResources,
    pub(super) pipeline: &'a wgpu::RenderPipeline,
    pub(super) label: &'static str,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) struct StrokeResourceDescriptor<'a> {
    pub(super) target_format: wgpu::TextureFormat,
    pub(super) surface_format: Option<wgpu::TextureFormat>,
    pub(super) output_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) draw_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) depth_compare: Option<wgpu::CompareFunction>,
    pub(super) retained_strokes: &'a [PreparedStrokeSegment],
    pub(super) batches: Vec<StrokeDrawBatch>,
}

pub(super) fn create_resources(
    device: &wgpu::Device,
    descriptor: StrokeResourceDescriptor<'_>,
) -> StrokeResources {
    let quad_bytes = encode_quad_vertices();
    let quad_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_strokes.quad_vertices"),
        size: quad_bytes.len() as u64,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: true,
    });
    {
        let mut mapped = quad_vertex_buffer.slice(..).get_mapped_range_mut();
        mapped.copy_from_slice(&quad_bytes);
    }
    quad_vertex_buffer.unmap();

    let instance_bytes = encode_instances(descriptor.retained_strokes);
    let instance_capacity = descriptor.retained_strokes.len();
    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.gpu_strokes.instances"),
        size: instance_bytes.len().max(4) as u64,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: true,
    });
    if !instance_bytes.is_empty() {
        let mut mapped = instance_buffer.slice(..).get_mapped_range_mut();
        mapped.copy_from_slice(&instance_bytes);
    }
    instance_buffer.unmap();

    let pipeline = create_pipeline(
        device,
        descriptor.target_format,
        descriptor.output_bind_group_layout,
        descriptor.draw_bind_group_layout,
        descriptor.depth_compare,
        "scena.gpu_strokes.pipeline",
    );
    let surface_pipeline = descriptor.surface_format.map(|format| {
        create_pipeline(
            device,
            format,
            descriptor.output_bind_group_layout,
            descriptor.draw_bind_group_layout,
            descriptor.depth_compare,
            "scena.gpu_strokes.surface_pipeline",
        )
    });
    let post_pipeline = create_pipeline(
        device,
        POST_COLOR_FORMAT,
        descriptor.output_bind_group_layout,
        descriptor.draw_bind_group_layout,
        descriptor.depth_compare,
        "scena.gpu_strokes.post_pipeline",
    );

    StrokeResources {
        quad_vertex_buffer,
        instance_buffer,
        instance_capacity,
        pipeline,
        surface_pipeline,
        post_pipeline,
        batches: descriptor.batches,
    }
}

pub(super) fn create_draw_batches(
    strokes: &[PreparedStrokeSegment],
    draw_uniforms: &mut Vec<DrawUniformValue>,
) -> Vec<StrokeDrawBatch> {
    let mut batches: Vec<StrokeDrawBatch> = Vec::new();
    for stroke in strokes {
        let draw_uniform_index = draw_uniform_index(draw_uniforms, stroke);
        let start_instance = stroke.original_segment_index();
        if let Some(last) = batches.last_mut()
            && last.draw_uniform_index == draw_uniform_index
            && last.start_instance.saturating_add(last.instance_count) == start_instance
        {
            last.instance_count = last.instance_count.saturating_add(1);
            continue;
        }
        batches.push(StrokeDrawBatch {
            start_instance,
            instance_count: 1,
            draw_uniform_index,
        });
    }
    batches
}

pub(super) fn encode_pass(encoder: &mut wgpu::CommandEncoder, inputs: StrokePass<'_>) {
    if inputs.resources.batches.is_empty() {
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
    pass.set_vertex_buffer(0, inputs.resources.quad_vertex_buffer.slice(..));
    pass.set_vertex_buffer(1, inputs.resources.instance_buffer.slice(..));
    for batch in &inputs.resources.batches {
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
        pass.draw(
            0..4,
            batch.start_instance..batch.start_instance.saturating_add(batch.instance_count),
        );
        *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
    }
}

pub(super) const fn pipeline(resources: &StrokeResources) -> &wgpu::RenderPipeline {
    &resources.pipeline
}

#[allow(dead_code)]
pub(super) fn surface_pipeline(resources: &StrokeResources) -> Option<&wgpu::RenderPipeline> {
    resources.surface_pipeline.as_ref()
}

pub(super) const fn post_pipeline(resources: &StrokeResources) -> &wgpu::RenderPipeline {
    &resources.post_pipeline
}

fn create_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.gpu_strokes.shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let dummy_material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.gpu_strokes.material_dummy"),
        entries: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.gpu_strokes.pipeline_layout"),
        bind_group_layouts: &[
            Some(output_bind_group_layout),
            Some(&dummy_material_layout),
            Some(draw_bind_group_layout),
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

fn draw_uniform_index(
    draw_uniforms: &mut Vec<DrawUniformValue>,
    stroke: &PreparedStrokeSegment,
) -> u32 {
    let value = DrawUniformValue {
        world_from_model: stroke.world_from_model(),
        normal_from_model: identity_matrix4(),
        tint: stroke.tint(),
    };
    match draw_uniforms.iter().position(|existing| *existing == value) {
        Some(existing) => existing as u32,
        None => {
            draw_uniforms.push(value);
            (draw_uniforms.len() - 1) as u32
        }
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

fn encode_instances(strokes: &[PreparedStrokeSegment]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(strokes.len() * INSTANCE_BYTE_LEN);
    for stroke in strokes {
        for value in [
            stroke.start().x,
            stroke.start().y,
            stroke.start().z,
            stroke.end().x,
            stroke.end().y,
            stroke.end().z,
            stroke.color().r,
            stroke.color().g,
            stroke.color().b,
            stroke.color().a,
            stroke.width_px(),
        ] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn stroke_shader_expands_instanced_segments_in_vertex_shader() {
        let shader = include_str!("strokes.wgsl");
        assert!(
            shader.contains("clip_segment_to_near")
                && shader.contains("quad.side_along")
                && shader.contains("camera.viewport_near_far"),
            "GPU strokes must use a dedicated vertex-shader expansion path with near-plane clipping and viewport-scaled width"
        );
    }
}
