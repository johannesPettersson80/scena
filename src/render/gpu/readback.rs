#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::{GpuDeviceState, GpuPreparedResources};

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::render) struct PendingGpuReadback {
    slot: usize,
    order: usize,
    target: RasterTarget,
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn encode_copy_target_to_readback(
    encoder: &mut wgpu::CommandEncoder,
    resources: &GpuPreparedResources,
    target: RasterTarget,
) {
    encode_copy_target_to_readback_slot(encoder, resources, target, 0);
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_copy_target_to_readback_slot(
    encoder: &mut wgpu::CommandEncoder,
    resources: &GpuPreparedResources,
    target: RasterTarget,
    slot: usize,
) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &resources.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &resources.readback[slot],
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
    let readback = resources.readback[0].slice(..);
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
    resources.readback[0].unmap();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
impl GpuDeviceState {
    pub(in crate::render) fn begin_async_readback(
        &mut self,
        target: RasterTarget,
        slot: usize,
        order: usize,
    ) -> Result<PendingGpuReadback, RenderError> {
        let Some(resources) = self.resources.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        if resources.target != target || slot >= resources.readback.len() {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.gpu.async_readback_encoder"),
            });
        encode_copy_target_to_readback_slot(&mut encoder, resources, target, slot);
        self.queue.submit(Some(encoder.finish()));
        let readback = resources.readback[slot].slice(..);
        let (sender, receiver) = mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        Ok(PendingGpuReadback {
            slot,
            order,
            target,
            receiver,
        })
    }

    pub(in crate::render) fn finish_async_readback(
        &mut self,
        pending: PendingGpuReadback,
    ) -> Result<(usize, Vec<u8>), RenderError> {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|_| RenderError::GpuReadback {
                backend: pending.target.backend,
            })?;
        pending
            .receiver
            .recv()
            .map_err(|_| RenderError::GpuReadback {
                backend: pending.target.backend,
            })?
            .map_err(|_| RenderError::GpuReadback {
                backend: pending.target.backend,
            })?;
        let Some(resources) = self.resources.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: pending.target.backend,
            });
        };
        let buffer = &resources.readback[pending.slot];
        let mapped = buffer.slice(..).get_mapped_range();
        let mut frame = vec![0; pending.target.byte_len()];
        copy_mapped_rows(resources, pending.target, &mapped, &mut frame);
        drop(mapped);
        buffer.unmap();
        Ok((pending.order, frame))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_mapped_rows(
    resources: &GpuPreparedResources,
    target: RasterTarget,
    mapped: &[u8],
    frame: &mut [u8],
) {
    for row in 0..target.height as usize {
        let source_start = row * resources.padded_bytes_per_row as usize;
        let source_end = source_start + resources.unpadded_bytes_per_row as usize;
        let target_start = row * resources.unpadded_bytes_per_row as usize;
        let target_end = target_start + resources.unpadded_bytes_per_row as usize;
        frame[target_start..target_end].copy_from_slice(&mapped[source_start..source_end]);
    }
}
