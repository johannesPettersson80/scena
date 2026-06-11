use super::{PostChainOutput, PostResources, texture};

#[allow(dead_code)]
pub(in crate::render::gpu) fn copy_output_to_buffer(
    encoder: &mut wgpu::CommandEncoder,
    resources: &PostResources,
    output: PostChainOutput,
    buffer: &wgpu::Buffer,
    padded_bytes_per_row: u32,
) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: texture(resources, output.slot),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: resources.target.width,
            height: resources.target.height,
            depth_or_array_layers: 1,
        },
    );
}
