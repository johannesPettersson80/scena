#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

use crate::diagnostics::RenderError;
use crate::material::Color;

use super::super::RasterTarget;
use super::super::camera::CameraProjection;
use super::depth;
use super::draw_common::{
    camera_position_uniform, identity_matrix, post_color_management_uniform, wgpu_clear_color,
};
use super::output::{OutputUniformUpload, encode_output_uniform};
use super::scene_color::{SceneColorPasses, encode_scene_color_passes};
use super::shadow::encode_shadow_caster_pass;
use super::{GpuDeviceState, GpuPostPassCounts, GpuPostSettings, GpuRenderResult, post};

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::render) fn render_to_frame(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        color_management: [f32; 4],
        background_color: Color,
        camera_projection: &CameraProjection,
        frame: &mut Vec<u8>,
        post_settings: GpuPostSettings,
    ) -> Result<GpuRenderResult, RenderError> {
        let Some(resources) = self.resources.as_mut() else {
            frame.resize(target.byte_len(), 0);
            frame.fill(0);
            return Ok(GpuRenderResult::default());
        };
        if resources.target != target {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let post_enabled = post_settings.enabled();
        if post_enabled
            && !resources
                .post
                .as_ref()
                .is_some_and(|post| post::resources_match(post, target))
        {
            resources.post = Some(post::create_resources(
                &self.device,
                target,
                &resources.output_bind_group_layout,
                &resources.material_bind_group_layout,
                &resources.draw_bind_group_layout,
                resources.texture_binding_mode,
                resources.depth_compare,
                self.surface.as_ref().map(|surface| surface.config.format),
            ));
        }
        let color_management = post_color_management_uniform(color_management, post_enabled);
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
        let rebuild_depth_prepass = resources.depth_prepass.as_ref().and_then(|depth_prepass| {
            (depth_prepass.depth_color_enabled() != post_settings.needs_depth_color())
                .then(|| depth_prepass.reversed_z())
        });
        if let Some(reversed_z) = rebuild_depth_prepass {
            resources.depth_prepass = Some(depth::create_depth_prepass_resources(
                &self.device,
                target,
                reversed_z,
                &resources.output_bind_group_layout,
                &resources.draw_bind_group_layout,
                post_settings.needs_depth_color(),
            ));
        }
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
        let post_resources = resources.post.as_ref();
        let (final_view, final_pipeline, base_label) = if post_enabled {
            let post_resources = post_resources.expect("post resources were created above");
            (
                post::scene_view(post_resources),
                &post_resources.scene_pipeline,
                "scena.headless_gpu.post_scene_pass",
            )
        } else {
            (
                &resources.view,
                &resources.offscreen_pipeline,
                "scena.headless_gpu.render_pass",
            )
        };
        encode_scene_color_passes(
            &mut encoder,
            SceneColorPasses {
                final_view,
                final_pipeline,
                depth_view: resources
                    .depth_prepass
                    .as_ref()
                    .map(|depth_prepass| &depth_prepass.view),
                vertex_buffer: &resources.vertex_buffer,
                output_bind_group: &resources.output_bind_group,
                opaque_output_bind_group: &resources.opaque_output_bind_group,
                draw_bind_group: &resources.draw_bind_group,
                material_resources: &resources.material_resources,
                draw_batches: &resources.draw_batches,
                transmission_view: &resources.transmission.view,
                transmission_pipeline: &resources.transmission.pipeline,
                clear_color: wgpu_clear_color(background_color),
                base_label,
            },
        );
        let post_output = if post_enabled {
            let post_resources = resources.post.as_ref().expect("post resources exist");
            let (output, post_counts) = post::encode_chain(
                &mut encoder,
                &self.device,
                &self.queue,
                post_resources,
                post_settings,
                resources.depth_prepass.as_ref(),
            )?;
            if let Some(surface_view) = surface_view.as_ref() {
                let Some(surface_blit_pipeline) = post::surface_blit_pipeline(post_resources)
                else {
                    return Err(RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    });
                };
                post::encode_blit_to_view(
                    &mut encoder,
                    &self.device,
                    post_resources,
                    output,
                    surface_view,
                    surface_blit_pipeline,
                );
            }
            post::copy_output_to_buffer(
                &mut encoder,
                post_resources,
                output,
                &resources.readback,
                resources.padded_bytes_per_row,
            );
            Some((output, post_counts))
        } else {
            None
        };
        if !post_enabled
            && let (Some(surface_view), Some(surface_pipeline)) =
                (surface_view.as_ref(), resources.surface_pipeline.as_ref())
        {
            encode_scene_color_passes(
                &mut encoder,
                SceneColorPasses {
                    final_view: surface_view,
                    final_pipeline: surface_pipeline,
                    depth_view: resources
                        .depth_prepass
                        .as_ref()
                        .map(|depth_prepass| &depth_prepass.view),
                    vertex_buffer: &resources.vertex_buffer,
                    output_bind_group: &resources.output_bind_group,
                    opaque_output_bind_group: &resources.opaque_output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    material_resources: &resources.material_resources,
                    draw_batches: &resources.draw_batches,
                    transmission_view: &resources.transmission.view,
                    transmission_pipeline: &resources.transmission.pipeline,
                    clear_color: wgpu_clear_color(background_color),
                    base_label: "scena.surface.render_pass",
                },
            );
        }
        if !post_enabled {
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

        Ok(GpuRenderResult {
            submitted: true,
            post_counts: post_output
                .map(|(_, counts)| counts)
                .unwrap_or_else(GpuPostPassCounts::default),
        })
    }
}
