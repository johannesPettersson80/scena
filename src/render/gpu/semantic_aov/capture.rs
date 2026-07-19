use crate::diagnostics::RenderError;
use crate::render::camera::CameraProjection;
use crate::render::semantic_aov::RawSemanticAovCapture;
use crate::scene::{ClippingPlane, SectionBox};

use super::super::GpuDeviceState;
use super::{SemanticAovResources, encode_copies, encode_pass, write_camera_uniform};
use crate::render::RasterTarget;

pub(super) fn decode_capture(
    semantic: &SemanticAovResources,
    near_far: [f32; 2],
    frames: [Vec<u8>; 3],
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
        depth_meters,
        world_normals,
        legend: semantic.legend.clone(),
        exclusions: semantic.exclusions,
    }
}

fn copy_rows(semantic: &SemanticAovResources, mapped: &[u8]) -> Vec<u8> {
    let mut frame = vec![0; semantic.target.byte_len()];
    for row in 0..semantic.target.height as usize {
        let source = row * semantic.padded_bytes_per_row as usize;
        let destination = row * semantic.unpadded_bytes_per_row as usize;
        frame[destination..destination + semantic.unpadded_bytes_per_row as usize]
            .copy_from_slice(&mapped[source..source + semantic.unpadded_bytes_per_row as usize]);
    }
    frame
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
            let frame = copy_rows(semantic, &mapped);
            drop(mapped);
            semantic.targets[slot].readback.unmap();
            frame
        });
        Ok(decode_capture(semantic, projection.near_far(), frames))
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
            frames[slot] = copy_rows(semantic, &mapped);
            drop(mapped);
            target_resource.readback.unmap();
        }
        Ok(decode_capture(semantic, projection.near_far(), frames))
    }
}
