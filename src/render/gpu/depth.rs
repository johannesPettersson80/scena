use super::super::RasterTarget;
use super::vertices::{VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

const DEPTH_PREPASS_SHADER: &str = r#"
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> @builtin(position) vec4<f32> {
    return vec4<f32>(in.position, 1.0);
}
"#;

#[derive(Debug)]
pub(super) struct DepthPrepassResources {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
}

pub(super) fn create_depth_prepass_resources(
    device: &wgpu::Device,
    target: RasterTarget,
) -> DepthPrepassResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.m2.depth_prepass"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m2.depth_prepass_shader"),
        source: wgpu::ShaderSource::Wgsl(DEPTH_PREPASS_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m2.depth_prepass_pipeline_layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.m2.depth_prepass_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_buffer],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    });

    DepthPrepassResources {
        texture,
        view,
        pipeline,
    }
}

pub(super) fn encode_depth_prepass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &DepthPrepassResources,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
        view: &resources.view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scena.m2.depth_prepass"),
        color_attachments: &[],
        depth_stencil_attachment: depth_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&resources.pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.draw(0..vertex_count, 0..1);
}

impl Drop for DepthPrepassResources {
    fn drop(&mut self) {
        let _ = &self.texture;
    }
}
