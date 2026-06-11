use super::{create_post_pipeline, create_texture_bind_group};

const SHADER: &str = include_str!("bloom.wgsl");

pub(super) fn create_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_post_pipeline(
        device,
        "scena.gpu_post.bloom_pipeline",
        SHADER,
        bind_group_layout,
        wgpu::TextureFormat::Rgba8Unorm,
    )
}

pub(super) fn encode(
    encoder: &mut wgpu::CommandEncoder,
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    pipeline: &wgpu::RenderPipeline,
    source_view: &wgpu::TextureView,
    target_view: &wgpu::TextureView,
) {
    let bind_group = create_texture_bind_group(
        device,
        bind_group_layout,
        source_view,
        uniform,
        "scena.gpu_post.bloom_bind_group",
    );
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
        label: Some("scena.gpu_post.bloom_pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}
