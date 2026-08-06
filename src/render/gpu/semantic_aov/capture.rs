use crate::diagnostics::RenderError;
use crate::render::camera::CameraProjection;
use crate::render::semantic_aov::RawSemanticAovCapture;
use crate::scene::{ClippingPlane, SectionBox};

use super::super::{GpuDeviceState, GpuPreparedResources};
use super::{SemanticAovResources, encode_beauty_copy, encode_copies, encode_pass};
use crate::render::RasterTarget;
use crate::render::gpu::draw_common::{camera_position_uniform, identity_matrix};
use crate::render::gpu::output::{
    OutputUniformUpload, encode_clipping_uniform, encode_output_uniform,
};

pub(super) fn decode_capture(
    semantic: &SemanticAovResources,
    near_far: [f32; 2],
    frames: [Vec<u8>; 3],
    beauty_frame: Option<(RasterTarget, Vec<u8>)>,
) -> RawSemanticAovCapture {
    let [near, far] = near_far;
    let pixels = semantic.target.pixel_len();
    let mut id_indices = Vec::with_capacity(pixels);
    let mut depth_meters = Vec::with_capacity(pixels);
    let mut world_normals = Vec::with_capacity(pixels);
    for pixel in 0..pixels {
        let offset = pixel * 4;
        let id = u32::from(frames[0][offset])
            | (u32::from(frames[0][offset + 1]) << 8)
            | (u32::from(frames[0][offset + 2]) << 16);
        id_indices.push(id);
        let depth_code = u32::from(frames[1][offset])
            | (u32::from(frames[1][offset + 1]) << 8)
            | (u32::from(frames[1][offset + 2]) << 16);
        depth_meters.push(if id == 0 || depth_code == 0 {
            f32::INFINITY
        } else {
            near + (depth_code.saturating_sub(1) as f32 / 16_777_214.0) * (far - near)
        });
        if id == 0 {
            world_normals.push([0.0; 3]);
        } else {
            let mut normal = [0.0; 3];
            for component in 0..3 {
                normal[component] = frames[2][offset + component] as f32 / 255.0 * 2.0 - 1.0;
            }
            let length = normal.iter().map(|value| value * value).sum::<f32>().sqrt();
            if length > f32::EPSILON {
                normal.iter_mut().for_each(|value| *value /= length);
            }
            world_normals.push(normal);
        }
    }
    RawSemanticAovCapture {
        width: semantic.target.width,
        height: semantic.target.height,
        near,
        far,
        id_indices,
        beauty_id_indices: beauty_frame
            .map(|(source_target, frame)| decode_beauty_ids(semantic, source_target, &frame)),
        depth_meters,
        world_normals,
        legend: semantic.legend.clone(),
        exclusions: semantic.exclusions,
    }
}

fn decode_beauty_ids(
    semantic: &SemanticAovResources,
    source_target: RasterTarget,
    frame: &[u8],
) -> Vec<u32> {
    let valid_indices = semantic
        .legend
        .iter()
        .map(|entry| entry.palette_index)
        .collect::<std::collections::BTreeSet<_>>();
    let mut ids = vec![0; semantic.target.pixel_len()];
    for y in 0..semantic.target.height {
        let source_y = nearest_resolved_coordinate(y, source_target.height, semantic.target.height);
        for x in 0..semantic.target.width {
            let source_x =
                nearest_resolved_coordinate(x, source_target.width, semantic.target.width);
            let source_offset = source_target
                .pixel_index(source_x, source_y)
                .saturating_mul(4);
            let Some(pixel) = frame.get(source_offset..source_offset + 4) else {
                continue;
            };
            // A partially-covered MSAA edge resolves to alpha below 255. Drop
            // it instead of decoding an averaged palette value as an identity.
            if pixel[3] != u8::MAX {
                continue;
            }
            let id = u32::from(pixel[0]) | (u32::from(pixel[1]) << 8) | (u32::from(pixel[2]) << 16);
            if valid_indices.contains(&id) {
                ids[semantic.target.pixel_index(x, y)] = id;
            }
        }
    }
    ids
}

fn nearest_resolved_coordinate(output: u32, source_size: u32, output_size: u32) -> u32 {
    let numerator = u64::from(output)
        .saturating_mul(2)
        .saturating_add(1)
        .saturating_mul(u64::from(source_size));
    let denominator = u64::from(output_size).saturating_mul(2).max(1);
    (numerator / denominator).min(u64::from(source_size.saturating_sub(1))) as u32
}

fn copy_rows(
    target: RasterTarget,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    mapped: &[u8],
) -> Vec<u8> {
    let mut frame = vec![0; target.byte_len()];
    for row in 0..target.height as usize {
        let source = row * padded_bytes_per_row as usize;
        let destination = row * unpadded_bytes_per_row as usize;
        frame[destination..destination + unpadded_bytes_per_row as usize]
            .copy_from_slice(&mapped[source..source + unpadded_bytes_per_row as usize]);
    }
    frame
}

pub(super) fn write_camera_uniform(
    queue: &wgpu::Queue,
    resources: &GpuPreparedResources,
    projection: &CameraProjection,
    target: RasterTarget,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
) {
    let (clipping_planes, clipping_control) = encode_clipping_uniform(clipping_planes, section_box);
    queue.write_buffer(
        &resources.output_uniform,
        0,
        &encode_output_uniform(OutputUniformUpload {
            exposure_ev: 0.0,
            view_from_world: projection
                .view_from_world_matrix()
                .unwrap_or_else(identity_matrix),
            clip_from_view: projection
                .clip_from_view_matrix()
                .unwrap_or_else(identity_matrix),
            clip_from_world: projection
                .clip_from_world_matrix()
                .unwrap_or_else(identity_matrix),
            light_from_world: resources.light_from_world,
            camera_position: camera_position_uniform(projection),
            viewport: [target.width as f32, target.height as f32],
            near_far: projection.near_far(),
            color_management: [0.0; 4],
            white_balance: [1.0, 1.0, 1.0, 0.0],
            lighting: resources.light_uniform,
            clipping_planes,
            clipping_control,
        }),
    );
}

#[cfg(not(target_arch = "wasm32"))]
impl GpuDeviceState {
    pub(in crate::render) fn capture_semantic_aov(
        &mut self,
        target: RasterTarget,
        projection: &CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
    ) -> Result<RawSemanticAovCapture, RenderError> {
        let resources = self
            .resources
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?;
        let semantic = resources
            .semantic_aov
            .as_ref()
            .filter(|semantic| semantic.target == target)
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?;
        write_camera_uniform(
            &self.queue,
            resources,
            projection,
            target,
            clipping_planes,
            section_box,
        );
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.semantic_aov.encoder"),
            });
        encode_pass(&mut encoder, resources, semantic);
        encode_copies(&mut encoder, semantic);
        encode_beauty_copy(&mut encoder, semantic);
        self.queue.submit(Some(encoder.finish()));
        let (sender, receiver) = std::sync::mpsc::channel();
        for target in &semantic.targets {
            let sender = sender.clone();
            target
                .readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
        }
        if semantic.beauty.valid {
            let sender = sender.clone();
            semantic
                .beauty
                .readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
        }
        drop(sender);
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|_| RenderError::GpuReadback {
                backend: target.backend,
            })?;
        for result in receiver {
            result.map_err(|_| RenderError::GpuReadback {
                backend: target.backend,
            })?;
        }
        let frames = std::array::from_fn(|slot| {
            let mapped = semantic.targets[slot].readback.slice(..).get_mapped_range();
            let frame = copy_rows(
                semantic.target,
                semantic.padded_bytes_per_row,
                semantic.unpadded_bytes_per_row,
                &mapped,
            );
            drop(mapped);
            semantic.targets[slot].readback.unmap();
            frame
        });
        let beauty_frame = if semantic.beauty.valid {
            let mapped = semantic.beauty.readback.slice(..).get_mapped_range();
            let frame = copy_rows(
                semantic.beauty.target,
                semantic.beauty.padded_bytes_per_row,
                semantic.beauty.unpadded_bytes_per_row,
                &mapped,
            );
            drop(mapped);
            semantic.beauty.readback.unmap();
            Some((semantic.beauty.target, frame))
        } else {
            None
        };
        Ok(decode_capture(
            semantic,
            projection.near_far(),
            frames,
            beauty_frame,
        ))
    }
}

#[cfg(target_arch = "wasm32")]
impl GpuDeviceState {
    pub(in crate::render) async fn capture_semantic_aov(
        &mut self,
        target: RasterTarget,
        projection: &CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
    ) -> Result<RawSemanticAovCapture, RenderError> {
        use wasm_bindgen::JsValue;
        use wasm_bindgen_futures::JsFuture;

        let resources = self
            .resources
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?;
        let semantic = resources
            .semantic_aov
            .as_ref()
            .filter(|semantic| semantic.target == target)
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?;
        if target.backend == crate::Backend::WebGl2 {
            return super::webgl2::capture(
                self,
                resources,
                semantic,
                target,
                projection,
                clipping_planes,
                section_box,
            );
        }
        write_camera_uniform(
            &self.queue,
            resources,
            projection,
            target,
            clipping_planes,
            section_box,
        );
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.semantic_aov.browser_encoder"),
            });
        encode_pass(&mut encoder, resources, semantic);
        encode_copies(&mut encoder, semantic);
        encode_beauty_copy(&mut encoder, semantic);
        self.queue.submit(Some(encoder.finish()));
        let mut frames = [Vec::new(), Vec::new(), Vec::new()];
        for (slot, target_resource) in semantic.targets.iter().enumerate() {
            let slice = target_resource.readback.slice(..);
            let promise = js_sys::Promise::new(&mut |resolve, reject| {
                let resolve = resolve.clone();
                let reject = reject.clone();
                slice.map_async(wgpu::MapMode::Read, move |result| match result {
                    Ok(()) => {
                        let _ = resolve.call0(&JsValue::UNDEFINED);
                    }
                    Err(error) => {
                        let _ = reject.call1(
                            &JsValue::UNDEFINED,
                            &JsValue::from_str(&format!("semantic AOV readback failed: {error:?}")),
                        );
                    }
                });
            });
            JsFuture::from(promise)
                .await
                .map_err(|_| RenderError::GpuReadback {
                    backend: target.backend,
                })?;
            let mapped = slice.get_mapped_range();
            frames[slot] = copy_rows(
                semantic.target,
                semantic.padded_bytes_per_row,
                semantic.unpadded_bytes_per_row,
                &mapped,
            );
            drop(mapped);
            target_resource.readback.unmap();
        }
        let beauty_frame = if semantic.beauty.valid {
            let slice = semantic.beauty.readback.slice(..);
            let promise = js_sys::Promise::new(&mut |resolve, reject| {
                let resolve = resolve.clone();
                let reject = reject.clone();
                slice.map_async(wgpu::MapMode::Read, move |result| match result {
                    Ok(()) => {
                        let _ = resolve.call0(&JsValue::UNDEFINED);
                    }
                    Err(error) => {
                        let _ = reject.call1(
                            &JsValue::UNDEFINED,
                            &JsValue::from_str(&format!(
                                "beauty semantic witness readback failed: {error:?}"
                            )),
                        );
                    }
                });
            });
            JsFuture::from(promise)
                .await
                .map_err(|_| RenderError::GpuReadback {
                    backend: target.backend,
                })?;
            let mapped = slice.get_mapped_range();
            let frame = copy_rows(
                semantic.beauty.target,
                semantic.beauty.padded_bytes_per_row,
                semantic.beauty.unpadded_bytes_per_row,
                &mapped,
            );
            drop(mapped);
            semantic.beauty.readback.unmap();
            Some((semantic.beauty.target, frame))
        } else {
            None
        };
        Ok(decode_capture(
            semantic,
            projection.near_far(),
            frames,
            beauty_frame,
        ))
    }
}
