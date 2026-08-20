use super::*;

#[cfg(not(target_arch = "wasm32"))]
impl GpuDeviceState {
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
}
