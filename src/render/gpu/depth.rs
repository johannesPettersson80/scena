use super::super::RasterTarget;
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

const DEPTH_PREPASS_SHADER: &str = r#"
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct CameraUniform {
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    exposure_padding: vec4<f32>,
};

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

@vertex
fn vs_main(in: VertexIn) -> @builtin(position) vec4<f32> {
    // Use the same matrix multiplication path as the color pass so depth
    // values are bit-identical. On low-precision WebGL2 drivers (Pi 5 V3D),
    // computing `clip_from_view * view_from_world * world_from_model * pos`
    // here while the color pass computes `clip_from_view * view_from_world *
    // (world_from_model * pos)` makes most color-pass fragments fail the
    // LessEqual depth test by a single ULP, producing a mostly-black render.
    let world_position = draw.world_from_model * vec4<f32>(in.position, 1.0);
    return camera.clip_from_world * world_position;
}
"#;

#[derive(Debug)]
pub(super) struct DepthPrepassResources {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    clear_depth: f32,
    pub(super) color_compare: wgpu::CompareFunction,
}

pub(super) fn create_depth_prepass_resources(
    device: &wgpu::Device,
    target: RasterTarget,
    reversed_z: bool,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
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
    // Depth prepass binds camera at @group(0) and draw uniform at @group(2)
    // — material bind group is unused but the pipeline layout matches the
    // unlit pipeline so the same vertex buffer + draw indices apply.
    let dummy_material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.m2.depth_prepass_material_dummy"),
        entries: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m2.depth_prepass_pipeline_layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(&dummy_material_layout),
            Some(draw_bind_group_layout),
        ],
        immediate_size: 0,
    });
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    let color_compare = if reversed_z {
        wgpu::CompareFunction::GreaterEqual
    } else {
        wgpu::CompareFunction::LessEqual
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
            depth_compare: Some(color_compare),
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
        clear_depth: if reversed_z { 0.0 } else { 1.0 },
        color_compare,
    }
}

pub(super) fn encode_depth_prepass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &DepthPrepassResources,
    vertex_buffer: &wgpu::Buffer,
    camera_bind_group: &wgpu::BindGroup,
    draw_bind_group: &wgpu::BindGroup,
    draw_batches: &[PrimitiveDrawBatch],
) {
    let depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
        view: &resources.view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(resources.clear_depth),
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
    pass.set_bind_group(0, camera_bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    for batch in draw_batches {
        if !batch.depth_prepass_eligible {
            continue;
        }
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, draw_bind_group, &[draw_offset]);
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..1,
        );
    }
}

impl Drop for DepthPrepassResources {
    fn drop(&mut self) {
        let _ = &self.texture;
    }
}
