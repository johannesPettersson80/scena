#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;
use crate::diagnostics::RenderError;
use crate::material::Color;

use super::super::RasterTarget;
use super::super::camera::CameraProjection;
use super::GpuDeviceState;
use super::depth;
use super::output::{OutputUniformUpload, encode_output_uniform};
use super::pipeline::{UnlitPass, encode_unlit_pass};
use super::shadow::encode_shadow_caster_pass;
#[cfg(target_arch = "wasm32")]
use super::webgl2;

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::render) fn render_to_frame(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        color_management: [f32; 4],
        background_color: Color,
        camera_projection: &CameraProjection,
        frame: &mut Vec<u8>,
    ) -> Result<bool, RenderError> {
        let Some(resources) = self.resources.as_ref() else {
            frame.resize(target.byte_len(), 0);
            frame.fill(0);
            return Ok(false);
        };
        if resources.target != target {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        self.queue.write_buffer(
            &resources.output_uniform,
            0,
            &encode_output_uniform(OutputUniformUpload {
                exposure_ev,
                view_from_world: camera_projection
                    .view_from_world_matrix()
                    .unwrap_or_else(identity_matrix),
                clip_from_view: camera_projection
                    .clip_from_view_matrix()
                    .unwrap_or_else(identity_matrix),
                clip_from_world: camera_projection
                    .clip_from_world_matrix()
                    .unwrap_or_else(identity_matrix),
                light_from_world: resources.light_from_world,
                camera_position: camera_position_uniform(camera_projection),
                viewport: [target.width as f32, target.height as f32],
                near_far: camera_projection.near_far(),
                color_management,
                lighting: resources.light_uniform,
            }),
        );
        let surface_output =
            self.surface
                .as_ref()
                .and_then(|surface| match surface.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(output)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(output) => Some(output),
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded
                    | wgpu::CurrentSurfaceTexture::Outdated
                    | wgpu::CurrentSurfaceTexture::Lost
                    | wgpu::CurrentSurfaceTexture::Validation => None,
                });
        let surface_view = surface_output.as_ref().map(|output| {
            output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.headless_gpu.encoder"),
            });
        // Phase 1B step 2: shadow caster pass writes the directional shadow
        // map BEFORE the unlit pass so the fragment shader can sample it.
        // No-op if no shadow-casting directional light exists.
        encode_shadow_caster_pass(
            &mut encoder,
            &resources.shadow_caster,
            &resources.vertex_buffer,
            &resources.draw_bind_group,
            &resources.draw_batches,
        );
        if let Some(depth_prepass) = &resources.depth_prepass {
            depth::encode_depth_prepass(
                &mut encoder,
                depth_prepass,
                &resources.vertex_buffer,
                &resources.output_bind_group,
                &resources.draw_bind_group,
                &resources.draw_batches,
            );
        }
        encode_unlit_pass(
            &mut encoder,
            UnlitPass {
                view: &resources.view,
                depth_view: resources
                    .depth_prepass
                    .as_ref()
                    .map(|depth_prepass| &depth_prepass.view),
                vertex_buffer: &resources.vertex_buffer,
                output_bind_group: &resources.output_bind_group,
                draw_bind_group: &resources.draw_bind_group,
                material_resources: &resources.material_resources,
                draw_batches: &resources.draw_batches,
                pipeline: &resources.offscreen_pipeline,
                clear_color: wgpu_clear_color(background_color),
                label: "scena.headless_gpu.render_pass",
            },
        );
        if let (Some(surface_view), Some(surface_pipeline)) =
            (surface_view.as_ref(), resources.surface_pipeline.as_ref())
        {
            encode_unlit_pass(
                &mut encoder,
                UnlitPass {
                    view: surface_view,
                    depth_view: resources
                        .depth_prepass
                        .as_ref()
                        .map(|depth_prepass| &depth_prepass.view),
                    vertex_buffer: &resources.vertex_buffer,
                    output_bind_group: &resources.output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    material_resources: &resources.material_resources,
                    draw_batches: &resources.draw_batches,
                    pipeline: surface_pipeline,
                    clear_color: wgpu_clear_color(background_color),
                    label: "scena.surface.render_pass",
                },
            );
        }
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
        self.queue.submit(Some(encoder.finish()));
        if let Some(surface_output) = surface_output {
            surface_output.present();
        }

        let readback = resources.readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
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

        Ok(true)
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::render) fn render_to_surface(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        color_management: [f32; 4],
        background_color: Color,
        camera_projection: &CameraProjection,
    ) -> Result<bool, RenderError> {
        let Some(resources) = self.resources.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        if resources.target != target {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let Some(surface) = self.surface.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        if target.backend == Backend::WebGl2 {
            let Some(canvas) = self.browser_canvas.as_ref() else {
                return Err(RenderError::GpuResourcesNotPrepared {
                    backend: target.backend,
                });
            };
            webgl2::render_canvas(
                &mut self.webgl2_render_cache,
                canvas,
                &resources.webgl2_vertices,
                &resources.draw_batches,
                &resources.draw_uniforms,
                &camera_projection
                    .view_from_world_matrix()
                    .unwrap_or_else(identity_matrix),
                &camera_projection
                    .clip_from_view_matrix()
                    .unwrap_or_else(identity_matrix),
                &camera_projection
                    .clip_from_world_matrix()
                    .unwrap_or_else(identity_matrix),
                camera_position_uniform(camera_projection),
                [target.width as f32, target.height as f32],
                camera_projection.near_far(),
                webgl2_clear_color(background_color),
                2.0_f32.powf(exposure_ev),
                color_management,
                resources.light_uniform,
            )
            .map_err(|_| RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?;
            return Ok(true);
        }
        self.queue.write_buffer(
            &resources.output_uniform,
            0,
            &encode_output_uniform(OutputUniformUpload {
                exposure_ev,
                view_from_world: camera_projection
                    .view_from_world_matrix()
                    .unwrap_or_else(identity_matrix),
                clip_from_view: camera_projection
                    .clip_from_view_matrix()
                    .unwrap_or_else(identity_matrix),
                clip_from_world: camera_projection
                    .clip_from_world_matrix()
                    .unwrap_or_else(identity_matrix),
                light_from_world: resources.light_from_world,
                camera_position: camera_position_uniform(camera_projection),
                viewport: [target.width as f32, target.height as f32],
                near_far: camera_projection.near_far(),
                color_management,
                lighting: resources.light_uniform,
            }),
        );
        let surface_output = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(false),
        };
        let surface_view = surface_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.browser.encoder"),
            });
        // Phase 1B step 2 (wasm32 mirror of native): shadow caster pass writes
        // the directional shadow map BEFORE the unlit pass so the fragment
        // shader can sample it. No-op if no shadow-casting directional light
        // exists.
        encode_shadow_caster_pass(
            &mut encoder,
            &resources.shadow_caster,
            &resources.vertex_buffer,
            &resources.draw_bind_group,
            &resources.draw_batches,
        );
        if let Some(depth_prepass) = &resources.depth_prepass {
            depth::encode_depth_prepass(
                &mut encoder,
                depth_prepass,
                &resources.vertex_buffer,
                &resources.output_bind_group,
                &resources.draw_bind_group,
                &resources.draw_batches,
            );
        }
        encode_unlit_pass(
            &mut encoder,
            UnlitPass {
                view: &surface_view,
                depth_view: resources
                    .depth_prepass
                    .as_ref()
                    .map(|depth_prepass| &depth_prepass.view),
                vertex_buffer: &resources.vertex_buffer,
                output_bind_group: &resources.output_bind_group,
                draw_bind_group: &resources.draw_bind_group,
                material_resources: &resources.material_resources,
                draw_batches: &resources.draw_batches,
                pipeline: &resources.surface_pipeline,
                clear_color: wgpu_clear_color(background_color),
                label: "scena.browser.surface_pass",
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        surface_output.present();
        Ok(true)
    }
}

fn wgpu_clear_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: clear_channel_f64(color.r),
        g: clear_channel_f64(color.g),
        b: clear_channel_f64(color.b),
        a: clear_channel_f64(color.a),
    }
}

fn clear_channel_f64(channel: f32) -> f64 {
    channel.clamp(0.0, 1.0) as f64
}

#[cfg(target_arch = "wasm32")]
fn webgl2_clear_color(color: Color) -> [f32; 4] {
    [
        color.r.clamp(0.0, 1.0),
        color.g.clamp(0.0, 1.0),
        color.b.clamp(0.0, 1.0),
        color.a.clamp(0.0, 1.0),
    ]
}

fn camera_position_uniform(camera_projection: &CameraProjection) -> [f32; 3] {
    let position = camera_projection.camera_position();
    [position.x, position.y, position.z]
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}
