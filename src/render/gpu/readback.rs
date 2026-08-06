#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

use crate::diagnostics::RenderError;
#[cfg(not(target_arch = "wasm32"))]
use crate::material::Color;

use super::super::RasterTarget;
use super::{GpuDeviceState, GpuPreparedResources};

#[cfg(not(target_arch = "wasm32"))]
const AUTO_EXPOSURE_GRID: u32 = 16;
#[cfg(not(target_arch = "wasm32"))]
pub(super) const AUTO_EXPOSURE_SAMPLE_COUNT: usize =
    (AUTO_EXPOSURE_GRID * AUTO_EXPOSURE_GRID) as usize;
#[cfg(not(target_arch = "wasm32"))]
const AUTO_EXPOSURE_SAMPLE_STRIDE: u64 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;
#[cfg(not(target_arch = "wasm32"))]
const AUTO_EXPOSURE_BUFFER_SIZE: u64 =
    AUTO_EXPOSURE_SAMPLE_COUNT as u64 * AUTO_EXPOSURE_SAMPLE_STRIDE;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(super) struct GpuAutoExposureMeter {
    buffers: [wgpu::Buffer; 2],
    pending: [Option<PendingAutoExposureMeter>; 2],
    next_slot: usize,
    next_sequence: u64,
    last_applied_sequence: Option<u64>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct PendingAutoExposureMeter {
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    format: wgpu::TextureFormat,
    sequence: u64,
    target: RasterTarget,
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct AutoExposureMeterSubmission {
    slot: usize,
    format: wgpu::TextureFormat,
    sequence: u64,
    target: RasterTarget,
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::render) struct AutoExposureMeterSample {
    pub(in crate::render) colors: Vec<Color>,
    pub(in crate::render) width: u32,
    pub(in crate::render) height: u32,
    pub(in crate::render) source_target: RasterTarget,
}

#[cfg(not(target_arch = "wasm32"))]
impl GpuAutoExposureMeter {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let buffers = std::array::from_fn(|slot| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(if slot == 0 {
                    "scena.gpu.auto_exposure_meter.0"
                } else {
                    "scena.gpu.auto_exposure_meter.1"
                }),
                size: AUTO_EXPOSURE_BUFFER_SIZE,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        });
        Self {
            buffers,
            pending: [None, None],
            next_slot: 0,
            next_sequence: 0,
            last_applied_sequence: None,
        }
    }

    pub(super) fn encode_copy(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        target: RasterTarget,
        format: wgpu::TextureFormat,
    ) -> Option<AutoExposureMeterSubmission> {
        if target.width == 0 || target.height == 0 || !meter_format_supported(format) {
            return None;
        }
        let slot = (0..self.pending.len())
            .map(|offset| (self.next_slot + offset) % self.pending.len())
            .find(|slot| self.pending[*slot].is_none())?;
        self.next_slot = (slot + 1) % self.pending.len();
        for sample_y in 0..AUTO_EXPOSURE_GRID {
            for sample_x in 0..AUTO_EXPOSURE_GRID {
                let index = (sample_y * AUTO_EXPOSURE_GRID + sample_x) as u64;
                let x = ((u64::from(sample_x) * 2 + 1) * u64::from(target.width)
                    / u64::from(AUTO_EXPOSURE_GRID * 2))
                .min(u64::from(target.width - 1)) as u32;
                let y = ((u64::from(sample_y) * 2 + 1) * u64::from(target.height)
                    / u64::from(AUTO_EXPOSURE_GRID * 2))
                .min(u64::from(target.height - 1)) as u32;
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x, y, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &self.buffers[slot],
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: index * AUTO_EXPOSURE_SAMPLE_STRIDE,
                            bytes_per_row: None,
                            rows_per_image: None,
                        },
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        Some(AutoExposureMeterSubmission {
            slot,
            format,
            sequence,
            target,
        })
    }

    pub(super) fn begin_mapping(&mut self, submission: AutoExposureMeterSubmission) {
        let slice = self.buffers[submission.slot].slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.pending[submission.slot] = Some(PendingAutoExposureMeter {
            receiver,
            format: submission.format,
            sequence: submission.sequence,
            target: submission.target,
        });
    }

    pub(super) fn try_finish(
        &mut self,
        device: &wgpu::Device,
        backend: crate::diagnostics::Backend,
    ) -> Result<Option<AutoExposureMeterSample>, RenderError> {
        device
            .poll(wgpu::PollType::Poll)
            .map_err(|_| RenderError::GpuReadback { backend })?;
        let mut completed = [None; 2];
        for (slot, completed_sequence) in completed.iter_mut().enumerate() {
            let Some(pending) = self.pending[slot].as_ref() else {
                continue;
            };
            match pending.receiver.try_recv() {
                Ok(Ok(())) => *completed_sequence = Some(pending.sequence),
                Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(RenderError::GpuReadback { backend });
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let selected = select_completed_meter_slot(&completed, self.last_applied_sequence);
        // Retire every completed slot. A superseded or stale sample is
        // discarded rather than applied, and unmapping it returns the buffer
        // to the pool instead of blocking later submissions.
        let mut sample = None;
        for (slot, sequence) in completed.into_iter().enumerate() {
            let Some(sequence) = sequence else {
                continue;
            };
            let pending = self.pending[slot]
                .take()
                .expect("completed auto-exposure meter remains pending");
            if selected == Some(slot) {
                let mapped = self.buffers[slot].slice(..).get_mapped_range();
                let mut colors = Vec::with_capacity(AUTO_EXPOSURE_SAMPLE_COUNT);
                for index in 0..AUTO_EXPOSURE_SAMPLE_COUNT {
                    let offset = index * AUTO_EXPOSURE_SAMPLE_STRIDE as usize;
                    if pending.format == wgpu::TextureFormat::Rgba16Float {
                        let component = |byte_offset: usize| {
                            half::f16::from_bits(u16::from_le_bytes([
                                mapped[offset + byte_offset],
                                mapped[offset + byte_offset + 1],
                            ]))
                            .to_f32()
                        };
                        colors.push(Color::from_linear_rgba(
                            component(0),
                            component(2),
                            component(4),
                            component(6),
                        ));
                    } else {
                        let pixel = &mapped[offset..offset + 4];
                        let rgba = if matches!(
                            pending.format,
                            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
                        ) {
                            [pixel[2], pixel[1], pixel[0], pixel[3]]
                        } else {
                            [pixel[0], pixel[1], pixel[2], pixel[3]]
                        };
                        let color = if pending.format.is_srgb() {
                            Color::from_srgb_u8(rgba[0], rgba[1], rgba[2])
                        } else {
                            Color::from_linear_rgba(
                                f32::from(rgba[0]) / 255.0,
                                f32::from(rgba[1]) / 255.0,
                                f32::from(rgba[2]) / 255.0,
                                f32::from(rgba[3]) / 255.0,
                            )
                        };
                        colors.push(Color::from_linear_rgba(
                            color.r,
                            color.g,
                            color.b,
                            f32::from(rgba[3]) / 255.0,
                        ));
                    }
                }
                drop(mapped);
                self.last_applied_sequence = Some(sequence);
                sample = Some(AutoExposureMeterSample {
                    colors,
                    width: AUTO_EXPOSURE_GRID,
                    height: AUTO_EXPOSURE_GRID,
                    source_target: pending.target,
                });
            }
            self.buffers[slot].unmap();
        }
        Ok(sample)
    }
}

/// Chooses which completed meter slot to consume.
///
/// Slots complete asynchronously and out of order, so slot index carries no
/// temporal meaning. Consuming by index can apply an older sample after a
/// newer one has already been applied, stepping exposure backward.
///
/// `completed[slot]` is the submission sequence of a slot whose mapping has
/// finished, or `None` if that slot is empty or still pending.
#[cfg(not(target_arch = "wasm32"))]
fn select_completed_meter_slot(
    completed: &[Option<u64>],
    last_applied_sequence: Option<u64>,
) -> Option<usize> {
    completed
        .iter()
        .enumerate()
        .filter_map(|(slot, sequence)| sequence.map(|sequence| (slot, sequence)))
        .filter(|(_, sequence)| last_applied_sequence.is_none_or(|applied| *sequence > applied))
        .max_by_key(|(_, sequence)| *sequence)
        .map(|(slot, _)| slot)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn meter_format_supported(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb
            | wgpu::TextureFormat::Rgba16Float
    )
}

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
    pub(in crate::render) fn auto_exposure_meter_supported(&self) -> bool {
        self.resources
            .as_ref()
            .and_then(|resources| resources.post.as_ref())
            .is_some_and(|_| meter_format_supported(super::post::scene_color_format()))
    }

    pub(in crate::render) fn poll_auto_exposure_meter(
        &mut self,
        backend: crate::diagnostics::Backend,
    ) -> Result<Option<AutoExposureMeterSample>, RenderError> {
        self.auto_exposure_meter.try_finish(&self.device, backend)
    }

    #[cfg(any(feature = "scene-host", test))]
    pub(in crate::render) fn read_scene_linear_rgba32f(
        &mut self,
        backend: crate::diagnostics::Backend,
    ) -> Result<(RasterTarget, Vec<[f32; 4]>), RenderError> {
        let resources = self
            .resources
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared { backend })?;
        let post = resources
            .post
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared { backend })?;
        let readback = post
            .linear_scene_readback
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared { backend })?;
        let target = post.target;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.gpu.scene_linear_readback_encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &post.scene_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(readback.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = readback.buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|_| RenderError::GpuReadback { backend })?;
        receiver
            .recv()
            .map_err(|_| RenderError::GpuReadback { backend })?
            .map_err(|_| RenderError::GpuReadback { backend })?;

        let mapped = slice.get_mapped_range();
        let mut rgba32f = Vec::with_capacity(target.pixel_len());
        for row in 0..target.height as usize {
            let row_start = row * readback.padded_bytes_per_row as usize;
            for column in 0..target.width as usize {
                let offset = row_start + column * 8;
                rgba32f.push(std::array::from_fn(|component| {
                    let byte = offset + component * 2;
                    half::f16::from_bits(u16::from_le_bytes([mapped[byte], mapped[byte + 1]]))
                        .to_f32()
                }));
            }
        }
        drop(mapped);
        readback.buffer.unmap();
        Ok((target, rgba32f))
    }

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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::select_completed_meter_slot;

    #[test]
    fn newest_completed_meter_sample_wins_regardless_of_slot_index() {
        // Slot 1 carries the newer submission. Consuming slot 0 first would
        // apply a stale frame's exposure after a newer one was already
        // available, which reads as exposure stepping backward.
        assert_eq!(
            select_completed_meter_slot(&[Some(5), Some(7)], None),
            Some(1),
        );
        assert_eq!(
            select_completed_meter_slot(&[Some(7), Some(5)], None),
            Some(0),
        );
    }

    #[test]
    fn meter_samples_older_than_the_last_applied_sequence_are_rejected() {
        // A slot that completes late must never be applied once a newer
        // sample has already driven exposure.
        assert_eq!(select_completed_meter_slot(&[Some(3), None], Some(6)), None);
        assert_eq!(
            select_completed_meter_slot(&[Some(3), Some(9)], Some(6)),
            Some(1),
        );
        assert_eq!(select_completed_meter_slot(&[None, None], Some(6)), None);
    }
}
