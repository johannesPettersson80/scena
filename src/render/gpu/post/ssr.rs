use super::{create_post_pipeline, resources::POST_COLOR_FORMAT};
use crate::render::gpu::shader_manifest::ShaderVariantId;

pub(in crate::render::gpu) const SHADER: &str = include_str!("ssr.wgsl");

pub(super) fn create_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    create_post_pipeline(
        device,
        "scena.gpu_post.ssr_pipeline",
        ShaderVariantId::PostSsr,
        pipeline_layout,
        POST_COLOR_FORMAT,
    )
}

pub(super) fn encode(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    target_view: &wgpu::TextureView,
    draw_submissions: &mut u64,
) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: target_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scena.gpu_post.ssr_pass"),
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
