#![cfg(target_arch = "wasm32")]

use crate::diagnostics::RenderError;
use crate::material::Color;
#[cfg(any(feature = "browser-probe", feature = "scene-host"))]
use wasm_bindgen::JsValue;
#[cfg(any(feature = "browser-probe", feature = "scene-host"))]
use wasm_bindgen_futures::JsFuture;

use super::super::RasterTarget;
use super::draw_common::wgpu_clear_color;
use super::{GpuDeviceState, GpuRenderResult};

impl GpuDeviceState {
    pub(in crate::render) fn render_empty_surface(
        &mut self,
        target: RasterTarget,
        background_color: Color,
    ) -> Result<GpuRenderResult, RenderError> {
        let Some(surface) = self.surface.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        let surface_output = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(GpuRenderResult::default()),
        };
        let surface_view = surface_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.browser.empty_surface_encoder"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scena.browser.empty_surface_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu_clear_color(background_color)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.queue.submit(Some(encoder.finish()));
        surface_output.present();
        Ok(GpuRenderResult::default())
    }

    #[cfg(any(feature = "browser-probe", feature = "scene-host"))]
    pub(in crate::render) async fn browser_readback_rgba8(
        &mut self,
        target: RasterTarget,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        if target.backend == crate::Backend::WebGl2 {
            let canvas = self.browser_canvas.as_ref().ok_or_else(|| {
                JsValue::from_str("renderer-owned WebGL2 readback requires its attached canvas")
            })?;
            return super::browser_readback::read_webgl2_canvas_rgba8(canvas, target).map(Some);
        }
        let Some(resources) = self.resources.as_ref() else {
            return Ok(None);
        };
        let Some(readback) = resources.readback.as_ref() else {
            return Ok(None);
        };
        if resources.target != target {
            return Err(JsValue::from_str(&format!(
                "browser proof readback resources were prepared for {:?}, not {:?}",
                resources.target, target
            )));
        }
        let slice = readback.buffer.slice(..);
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
                            "renderer-owned WebGPU readback failed: {error:?}"
                        )),
                    );
                }
            });
        });
        JsFuture::from(promise).await?;
        let mapped = slice.get_mapped_range();
        let surface_copy_format = self.surface.as_ref().and_then(|surface| {
            surface
                .config
                .usage
                .contains(wgpu::TextureUsages::COPY_SRC)
                .then_some(surface.config.format)
        });
        let mut frame = vec![0; target.byte_len()];
        for row in 0..target.height as usize {
            let source_start = row * readback.padded_bytes_per_row as usize;
            let source_end = source_start + readback.unpadded_bytes_per_row as usize;
            let target_start = row * readback.unpadded_bytes_per_row as usize;
            let target_end = target_start + readback.unpadded_bytes_per_row as usize;
            frame[target_start..target_end].copy_from_slice(&mapped[source_start..source_end]);
        }
        drop(mapped);
        readback.buffer.unmap();
        if matches!(
            surface_copy_format,
            Some(wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb)
        ) {
            for pixel in frame.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        Ok(Some(frame))
    }
}
