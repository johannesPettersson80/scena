use super::create_post_pipeline;

const SHADER: &str = include_str!("blit.wgsl");
const SRGB_SHADER: &str = concat!(
    include_str!("blit_srgb.wgsl"),
    "\n",
    include_str!("../../color_contract.wgsl")
);

pub(super) fn create_surface_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_post_pipeline(
        device,
        "scena.gpu_post.surface_blit_pipeline",
        SHADER,
        bind_group_layout,
        format,
    )
}

pub(super) fn create_srgb_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_post_pipeline(
        device,
        "scena.gpu_post.srgb_blit_pipeline",
        SRGB_SHADER,
        bind_group_layout,
        format,
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
        label: Some("scena.gpu_post.blit_pass"),
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
