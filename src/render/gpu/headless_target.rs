use super::super::RasterTarget;
use super::pipeline::{BYTES_PER_PIXEL, GPU_COLOR_FORMAT};
use super::stats::align_to;

pub(super) struct HeadlessTargetResources {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) readback: [wgpu::Buffer; 2],
    pub(super) padded_bytes_per_row: u32,
    pub(super) unpadded_bytes_per_row: u32,
}

pub(super) fn create(device: &wgpu::Device, target: RasterTarget) -> HeadlessTargetResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.headless_gpu.target"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: GPU_COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = std::array::from_fn(|slot| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(if slot == 0 {
                "scena.headless_gpu.readback.0"
            } else {
                "scena.headless_gpu.readback.1"
            }),
            size: u64::from(padded_bytes_per_row) * u64::from(target.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    });
    HeadlessTargetResources {
        texture,
        view,
        readback,
        padded_bytes_per_row,
        unpadded_bytes_per_row,
    }
}
