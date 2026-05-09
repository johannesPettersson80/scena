use super::materials::MaterialTextureResources;
use super::output::{DRAW_UNIFORM_ENTRY_STRIDE, GPU_TRIANGLE_SHADER};
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

#[cfg(not(target_arch = "wasm32"))]
pub(super) const BYTES_PER_PIXEL: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
pub(super) const GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub(super) struct UnlitPass<'a> {
    pub(super) view: &'a wgpu::TextureView,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) material_resources: &'a [MaterialTextureResources],
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) pipeline: &'a wgpu::RenderPipeline,
    pub(super) label: &'static str,
}

pub(super) fn encode_unlit_pass(encoder: &mut wgpu::CommandEncoder, inputs: UnlitPass<'_>) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: inputs.view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
    pass.set_vertex_buffer(0, inputs.vertex_buffer.slice(..));
    let Some(fallback_material) = inputs.material_resources.first() else {
        return;
    };
    for batch in inputs.draw_batches {
        let material = inputs
            .material_resources
            .get(batch.material_slot as usize)
            .unwrap_or(fallback_material);
        pass.set_bind_group(1, &material.bind_group, &[]);
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..1,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_unlit_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m0.unlit_triangle"),
        source: wgpu::ShaderSource::Wgsl(GPU_TRIANGLE_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m0.pipeline_layout"),
        bind_group_layouts: &[
            Some(output_bind_group_layout),
            Some(material_bind_group_layout),
            Some(draw_bind_group_layout),
        ],
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

#[cfg(test)]
mod tests {
    #[test]
    fn unlit_pipeline_source_wires_depth_state_into_visible_color_pass() {
        let source = include_str!("pipeline.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("pipeline implementation precedes tests");
        assert!(
            implementation.contains("RenderPassDepthStencilAttachment")
                && implementation.contains("depth_stencil: depth_compare.map"),
            "visible GPU color pass must use the prepared depth buffer when one exists"
        );
    }

    #[test]
    fn unlit_pipeline_binds_material_group_for_fragment_sampling() {
        let source = include_str!("pipeline.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("pipeline implementation precedes tests");
        assert!(
            implementation.contains("material_bind_group_layout")
                && implementation.contains("material_resources")
                && implementation.contains("pass.set_bind_group(1, &material.bind_group"),
            "visible GPU color pass must bind material resources, not only camera uniforms"
        );
    }
}
