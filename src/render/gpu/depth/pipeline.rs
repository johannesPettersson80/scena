//! Render pipeline construction for the plain depth prepass.

use super::DEPTH_COLOR_FORMAT;
use crate::render::gpu::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN};
use crate::render::gpu::pipeline::SCENA_FRONT_FACE;
use crate::render::gpu::vertices::{VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

pub(super) fn create_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_compare: wgpu::CompareFunction,
    depth_color_enabled: bool,
    double_sided: bool,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    let label = if double_sided {
        "scena.m2.depth_prepass_pipeline.double_sided"
    } else {
        "scena.m2.depth_prepass_pipeline.single_sided"
    };
    let color_targets = depth_color_enabled
        .then_some(Some(wgpu::ColorTargetState {
            format: DEPTH_COLOR_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }))
        .into_iter()
        .collect::<Vec<_>>();
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                vertex_buffer,
                wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_BYTE_LEN as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES,
                },
            ],
        },
        primitive: wgpu::PrimitiveState {
            front_face: SCENA_FRONT_FACE,
            cull_mode: (!double_sided).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(color_compare),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(if depth_color_enabled {
                "fs_main"
            } else {
                "fs_depth_only"
            }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &color_targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}
