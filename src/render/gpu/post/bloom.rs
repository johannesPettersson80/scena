use super::types::{PostResources, PostTextureSlot};
use super::{
    bind_group, create_post_pipeline_with_shader, create_post_shader, resources::POST_COLOR_FORMAT,
    texture, view,
};
use crate::render::gpu::shader_manifest::ShaderVariantId;

pub(in crate::render::gpu) const SHADER: &str = concat!(
    include_str!("bloom.wgsl"),
    "\n",
    include_str!("bloom_kernel.wgsl")
);

/// Creates the two bloom pipelines from one shader module: the bright pass
/// with the horizontal blur, and the vertical blur that is added to the frame.
pub(super) fn create_pipelines(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let shader = create_post_shader(
        device,
        "scena.gpu_post.bloom_shader",
        ShaderVariantId::PostBloom,
    );
    let bright = create_post_pipeline_with_shader(
        device,
        "scena.gpu_post.bloom_pipeline",
        &shader,
        pipeline_layout,
        POST_COLOR_FORMAT,
    );
    // The composite keeps the frame's alpha and adds the blurred highlights.
    let additive = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    };
    let composite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.gpu_post.bloom_composite_pipeline"),
        layout: Some(pipeline_layout),
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
            entry_point: Some("fs_composite"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: POST_COLOR_FORMAT,
                blend: Some(additive),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    (bright, composite)
}

/// Encodes bloom and returns the slot that holds the frame afterwards.
///
/// The blurred highlights go to one scratch slot and are then added onto the
/// frame. The scene slot is never written, because exposure metering and
/// linear capture read it after the chain; a frame that still lives there is
/// copied to the other scratch slot first.
pub(super) fn encode(
    encoder: &mut wgpu::CommandEncoder,
    resources: &PostResources,
    source: PostTextureSlot,
    draw_submissions: &mut u64,
) -> PostTextureSlot {
    let (blur, output) = match source {
        PostTextureSlot::Scene | PostTextureSlot::Pong => {
            (PostTextureSlot::Ping, PostTextureSlot::Pong)
        }
        PostTextureSlot::Ping => (PostTextureSlot::Pong, PostTextureSlot::Ping),
    };
    encode_pass(
        encoder,
        &resources.bloom_pipeline,
        bind_group(resources, source),
        view(resources, blur),
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        "scena.gpu_post.bloom_pass",
        draw_submissions,
    );
    if source != output {
        encoder.copy_texture_to_texture(
            texture(resources, source).as_image_copy(),
            texture(resources, output).as_image_copy(),
            wgpu::Extent3d {
                width: resources.target.width,
                height: resources.target.height,
                depth_or_array_layers: 1,
            },
        );
    }
    encode_pass(
        encoder,
        &resources.bloom_composite_pipeline,
        bind_group(resources, blur),
        view(resources, output),
        wgpu::LoadOp::Load,
        "scena.gpu_post.bloom_composite_pass",
        draw_submissions,
    );
    output
}

fn encode_pass(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    target_view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
    label: &'static str,
    draw_submissions: &mut u64,
) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: target_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load,
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
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
    *draw_submissions = draw_submissions.saturating_add(1);
}
