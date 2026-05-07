use super::output::GPU_TRIANGLE_SHADER;
use super::vertices::{VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

#[cfg(not(target_arch = "wasm32"))]
pub(super) const BYTES_PER_PIXEL: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
pub(super) const GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub(super) fn encode_unlit_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    vertex_buffer: &wgpu::Buffer,
    output_bind_group: &wgpu::BindGroup,
    vertex_count: u32,
    pipeline: &wgpu::RenderPipeline,
    label: &'static str,
) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, output_bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    if vertex_count > 0 {
        pass.draw(0..vertex_count, 0..1);
    }
}

pub(super) fn create_unlit_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m0.unlit_triangle"),
        source: wgpu::ShaderSource::Wgsl(GPU_TRIANGLE_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m0.pipeline_layout"),
        bind_group_layouts: &[Some(output_bind_group_layout)],
        immediate_size: 0,
    });
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.m0.unlit_triangle_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_buffer],
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
