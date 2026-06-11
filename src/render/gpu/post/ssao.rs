use super::create_post_pipeline;

const SHADER: &str = include_str!("ssao.wgsl");

pub(super) fn create_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_post_pipeline(
        device,
        "scena.gpu_post.ssao_pipeline",
        SHADER,
        bind_group_layout,
        wgpu::TextureFormat::Rgba8Unorm,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    encoder: &mut wgpu::CommandEncoder,
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    pipeline: &wgpu::RenderPipeline,
    source_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    target_view: &wgpu::TextureView,
    draw_submissions: &mut u64,
) {
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.gpu_post.ssao_bind_group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(depth_view),
            },
        ],
    });
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
        label: Some("scena.gpu_post.ssao_pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
    *draw_submissions = draw_submissions.saturating_add(1);
}
