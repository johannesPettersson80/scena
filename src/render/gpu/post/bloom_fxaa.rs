use super::create_post_pipeline;
use crate::render::gpu::shader_manifest::{ShaderVariantId, variant_for_srgb_target};

pub(in crate::render::gpu) const LINEAR_TARGET_SHADER: &str = concat!(
    include_str!("bloom_fxaa.wgsl"),
    "\n",
    include_str!("post_output_linear.wgsl")
);
pub(in crate::render::gpu) const SRGB_BYTE_TARGET_SHADER: &str = concat!(
    include_str!("bloom_fxaa.wgsl"),
    "\n",
    include_str!("post_output_srgb.wgsl"),
    "\n",
    include_str!("../../color_contract.wgsl")
);

pub(super) fn create_surface_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_post_pipeline(
        device,
        "scena.gpu_post.surface_bloom_fxaa_pipeline",
        shader_for_target(format),
        pipeline_layout,
        format,
    )
}

const fn shader_for_target(format: wgpu::TextureFormat) -> ShaderVariantId {
    variant_for_srgb_target(
        format,
        ShaderVariantId::PostBloomFxaaLinear,
        ShaderVariantId::PostBloomFxaaSrgbBytes,
    )
}

#[cfg(target_arch = "wasm32")]
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
        label: Some("scena.gpu_post.bloom_fxaa_surface_pass"),
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
