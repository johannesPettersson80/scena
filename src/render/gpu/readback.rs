#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::GpuPreparedResources;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn encode_copy_target_to_readback(
    encoder: &mut wgpu::CommandEncoder,
    resources: &GpuPreparedResources,
    target: RasterTarget,
) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &resources.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &resources.readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(resources.padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn map_readback_to_frame(
    device: &wgpu::Device,
    resources: &GpuPreparedResources,
    target: RasterTarget,
    frame: &mut Vec<u8>,
) -> Result<(), RenderError> {
    let readback = resources.readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    readback.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|_| RenderError::GpuReadback {
            backend: target.backend,
        })?;
    receiver
        .recv()
        .map_err(|_| RenderError::GpuReadback {
            backend: target.backend,
        })?
        .map_err(|_| RenderError::GpuReadback {
            backend: target.backend,
        })?;

    let mapped = readback.get_mapped_range();
    if frame.len() != target.byte_len() {
        frame.resize(target.byte_len(), 0);
    }
    for row in 0..target.height as usize {
        let source_start = row * resources.padded_bytes_per_row as usize;
        let source_end = source_start + resources.unpadded_bytes_per_row as usize;
        let target_start = row * resources.unpadded_bytes_per_row as usize;
        let target_end = target_start + resources.unpadded_bytes_per_row as usize;
        frame[target_start..target_end].copy_from_slice(&mapped[source_start..source_end]);
    }
    drop(mapped);
    resources.readback.unmap();
    Ok(())
}
