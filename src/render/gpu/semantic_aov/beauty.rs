use crate::render::RasterTarget;

use super::{BYTES_PER_PIXEL, FORMAT, SemanticAovResources, align_to, extent};

#[derive(Debug)]
pub(super) struct Target {
    pub(super) target: RasterTarget,
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(super) msaa_texture: Option<wgpu::Texture>,
    pub(super) msaa_view: Option<wgpu::TextureView>,
    pub(super) readback: wgpu::Buffer,
    pub(super) padded_bytes_per_row: u32,
    pub(super) unpadded_bytes_per_row: u32,
    pub(super) sample_count: u32,
    pub(super) valid: bool,
}

pub(super) fn create(device: &wgpu::Device, target: RasterTarget, sample_count: u32) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.beauty_semantic.resolved"),
        size: extent(target),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let msaa_texture = (sample_count > 1).then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scena.beauty_semantic.msaa"),
            size: extent(target),
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    });
    let msaa_view = msaa_texture
        .as_ref()
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.beauty_semantic.readback"),
        size: u64::from(padded_bytes_per_row).saturating_mul(u64::from(target.height)),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    Target {
        target,
        texture,
        view,
        msaa_texture,
        msaa_view,
        readback,
        padded_bytes_per_row,
        unpadded_bytes_per_row,
        sample_count,
        valid: false,
    }
}

pub(super) fn attachment_views(
    resources: &SemanticAovResources,
    target: RasterTarget,
) -> Option<(&wgpu::TextureView, Option<&wgpu::TextureView>)> {
    if resources.beauty.target != target {
        return None;
    }
    match resources.beauty.msaa_view.as_ref() {
        Some(msaa_view) => Some((msaa_view, Some(&resources.beauty.view))),
        None => Some((&resources.beauty.view, None)),
    }
}

pub(super) fn encode_copy(encoder: &mut wgpu::CommandEncoder, semantic: &SemanticAovResources) {
    if !semantic.beauty.valid {
        return;
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &semantic.beauty.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &semantic.beauty.readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(semantic.beauty.padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        extent(semantic.beauty.target),
    );
}
