use std::cell::Cell;
use std::rc::Rc;

use crate::diagnostics::{Backend, RenderError};
use crate::material::Color;

use super::super::RasterTarget;
use super::GpuDeviceState;

const GRID: u32 = 16;
const SAMPLE_COUNT: usize = (GRID * GRID) as usize;
const STRIDE: u64 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;
const BUFFER_SIZE: u64 = SAMPLE_COUNT as u64 * STRIDE;

pub(in crate::render) struct BrowserAutoExposureSample {
    pub(in crate::render) colors: Vec<Color>,
    pub(in crate::render) width: u32,
    pub(in crate::render) height: u32,
    pub(in crate::render) source_target: RasterTarget,
}

struct Pending {
    status: Rc<Cell<i8>>,
    target: RasterTarget,
    sequence: u64,
}

pub(super) struct Submission {
    slot: usize,
    target: RasterTarget,
    sequence: u64,
}

pub(super) struct BrowserAutoExposureMeter {
    buffers: [wgpu::Buffer; 2],
    pending: [Option<Pending>; 2],
    next_slot: usize,
    next_sequence: u64,
    last_applied_sequence: Option<u64>,
}

impl BrowserAutoExposureMeter {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let buffers = std::array::from_fn(|slot| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(if slot == 0 {
                    "scena.browser.auto_exposure_meter.0"
                } else {
                    "scena.browser.auto_exposure_meter.1"
                }),
                size: BUFFER_SIZE,
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
    ) -> Option<Submission> {
        if target.width == 0 || target.height == 0 {
            return None;
        }
        let slot = (0..2)
            .map(|offset| (self.next_slot + offset) % 2)
            .find(|slot| self.pending[*slot].is_none())?;
        self.next_slot = (slot + 1) % 2;
        for sy in 0..GRID {
            for sx in 0..GRID {
                let index = u64::from(sy * GRID + sx);
                let x = (((u64::from(sx) * 2 + 1) * u64::from(target.width)) / u64::from(GRID * 2))
                    .min(u64::from(target.width - 1)) as u32;
                let y = (((u64::from(sy) * 2 + 1) * u64::from(target.height)) / u64::from(GRID * 2))
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
                            offset: index * STRIDE,
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
        Some(Submission {
            slot,
            target,
            sequence,
        })
    }

    pub(super) fn begin_mapping(&mut self, submission: Submission) {
        let status = Rc::new(Cell::new(0));
        let callback_status = Rc::clone(&status);
        self.buffers[submission.slot]
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                callback_status.set(if result.is_ok() { 1 } else { -1 });
            });
        self.pending[submission.slot] = Some(Pending {
            status,
            target: submission.target,
            sequence: submission.sequence,
        });
    }

    fn try_finish(
        &mut self,
        backend: Backend,
    ) -> Result<Option<BrowserAutoExposureSample>, RenderError> {
        let selected = self
            .pending
            .iter()
            .enumerate()
            .filter_map(|(slot, pending)| {
                pending.as_ref().and_then(|pending| {
                    (pending.status.get() == 1
                        && self
                            .last_applied_sequence
                            .is_none_or(|last| pending.sequence > last))
                    .then_some((slot, pending.sequence))
                })
            })
            .max_by_key(|(_, sequence)| *sequence)
            .map(|(slot, _)| slot);
        let mut sample = None;
        for slot in 0..2 {
            let status = self.pending[slot]
                .as_ref()
                .map_or(0, |pending| pending.status.get());
            if status < 0 {
                return Err(RenderError::GpuReadback { backend });
            }
            if status != 1 {
                continue;
            }
            let pending = self.pending[slot]
                .take()
                .expect("completed browser meter is pending");
            if selected == Some(slot) {
                let mapped = self.buffers[slot].slice(..).get_mapped_range();
                let mut colors = Vec::with_capacity(SAMPLE_COUNT);
                for index in 0..SAMPLE_COUNT {
                    let offset = index * STRIDE as usize;
                    let channel = |byte_offset: usize| {
                        half::f16::from_bits(u16::from_le_bytes([
                            mapped[offset + byte_offset],
                            mapped[offset + byte_offset + 1],
                        ]))
                        .to_f32()
                    };
                    colors.push(Color::from_linear_rgba(
                        channel(0),
                        channel(2),
                        channel(4),
                        channel(6),
                    ));
                }
                drop(mapped);
                self.last_applied_sequence = Some(pending.sequence);
                sample = Some(BrowserAutoExposureSample {
                    colors,
                    width: GRID,
                    height: GRID,
                    source_target: pending.target,
                });
            }
            self.buffers[slot].unmap();
        }
        Ok(sample)
    }
}

impl GpuDeviceState {
    pub(in crate::render) fn auto_exposure_meter_supported(&self) -> bool {
        self.resources
            .as_ref()
            .and_then(|resources| resources.post.as_ref())
            .is_some()
    }

    pub(in crate::render) fn poll_auto_exposure_meter(
        &mut self,
        backend: Backend,
    ) -> Result<Option<BrowserAutoExposureSample>, RenderError> {
        self.browser_auto_exposure_meter.try_finish(backend)
    }
}
