#![cfg(target_arch = "wasm32")]

use crate::diagnostics::RenderError;
use crate::material::Color;
#[cfg(feature = "browser-probe")]
use wasm_bindgen::JsValue;
#[cfg(feature = "browser-probe")]
use wasm_bindgen_futures::JsFuture;

use super::super::RasterTarget;
use super::super::camera::CameraProjection;
use super::browser_readback::{BrowserReadbackPass, encode_browser_readback_pass};
use super::depth;
use super::draw_common::{
    camera_position_uniform, identity_matrix, post_color_management_uniform, wgpu_clear_color,
};
use super::output::{OutputUniformUpload, encode_output_uniform};
use super::scene_color::{SceneColorPasses, encode_scene_color_passes};
use super::shadow::encode_shadow_caster_pass;
use super::{GpuDeviceState, GpuPostPassCounts, GpuPostSettings, GpuRenderResult, post, strokes};

impl GpuDeviceState {
    pub(in crate::render) fn render_to_surface(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        color_management: [f32; 4],
        background_color: Color,
        camera_projection: &CameraProjection,
        post_settings: GpuPostSettings,
    ) -> Result<GpuRenderResult, RenderError> {
        let Some(resources) = self.resources.as_mut() else {
            return self.render_empty_surface(target, background_color);
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
                Some(surface.config.format),
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
        #[cfg(feature = "browser-probe")]
        if let Some(readback) = resources.readback.as_ref() {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("scena.browser.proof_encoder"),
                });
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
            let post_counts = if post_enabled {
                let post_resources = resources.post.as_ref().expect("post resources exist");
                encode_scene_color_passes(
                    &mut encoder,
                    SceneColorPasses {
                        final_view: post::scene_view(post_resources),
                        final_pipeline: &post_resources.scene_pipeline,
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
                        base_label: "scena.browser.proof_post_scene_pass",
                    },
                );
                if let Some(stroke_resources) = resources.strokes.as_ref() {
                    strokes::encode_pass(
                        &mut encoder,
                        strokes::StrokePass {
                            view: post::scene_view(post_resources),
                            depth_view: resources
                                .depth_prepass
                                .as_ref()
                                .map(|depth_prepass| &depth_prepass.view),
                            output_bind_group: &resources.output_bind_group,
                            draw_bind_group: &resources.draw_bind_group,
                            resources: stroke_resources,
                            pipeline: strokes::post_pipeline(stroke_resources),
                            label: "scena.browser.proof_stroke_post_scene_pass",
                        },
                    );
                }
                let (output, counts) = post::encode_chain(
                    &mut encoder,
                    &self.device,
                    &self.queue,
                    post_resources,
                    post_settings,
                    resources.depth_prepass.as_ref(),
                )?;
                post::copy_output_to_buffer(
                    &mut encoder,
                    post_resources,
                    output,
                    &readback.buffer,
                    readback.padded_bytes_per_row,
                );
                counts
            } else {
                encode_browser_readback_pass(
                    &mut encoder,
                    BrowserReadbackPass {
                        target,
                        readback,
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
                        transmission: &resources.transmission,
                        clear_color: wgpu_clear_color(background_color),
                    },
                );
                GpuPostPassCounts::default()
            };
            self.queue.submit(Some(encoder.finish()));
            return Ok(GpuRenderResult {
                submitted: true,
                post_counts,
            });
        }
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
                label: Some("scena.browser.encoder"),
            });
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
                "scena.browser.post_scene_pass",
            )
        } else {
            (
                &surface_view,
                &resources.surface_pipeline,
                "scena.browser.render_pass",
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
        if let Some(stroke_resources) = resources.strokes.as_ref() {
            let stroke_pipeline = if post_enabled {
                strokes::post_pipeline(stroke_resources)
            } else {
                strokes::pipeline(stroke_resources)
            };
            strokes::encode_pass(
                &mut encoder,
                strokes::StrokePass {
                    view: final_view,
                    depth_view: resources
                        .depth_prepass
                        .as_ref()
                        .map(|depth_prepass| &depth_prepass.view),
                    output_bind_group: &resources.output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    resources: stroke_resources,
                    pipeline: stroke_pipeline,
                    label: "scena.browser.stroke_scene_pass",
                },
            );
        }
        let post_counts = if post_enabled {
            let post_resources = resources.post.as_ref().expect("post resources exist");
            let bloom_fxaa_to_surface = post_settings
                .uses_fxaa()
                .then(|| post_settings.bloom())
                .flatten();
            let render_fxaa_to_surface =
                post_settings.uses_fxaa() && bloom_fxaa_to_surface.is_none();
            let chain_settings = if bloom_fxaa_to_surface.is_some() {
                post_settings.without_bloom_and_fxaa()
            } else if render_fxaa_to_surface {
                post_settings.without_fxaa()
            } else {
                post_settings
            };
            let (output, mut counts) = post::encode_chain(
                &mut encoder,
                &self.device,
                &self.queue,
                post_resources,
                chain_settings,
                resources.depth_prepass.as_ref(),
            )?;
            if let Some(bloom_config) = bloom_fxaa_to_surface {
                let Some(surface_bloom_fxaa_pipeline) =
                    post::surface_bloom_fxaa_pipeline(post_resources)
                else {
                    return Err(RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    });
                };
                post::encode_bloom_fxaa_to_view(
                    &mut encoder,
                    &self.queue,
                    post_resources,
                    output,
                    &surface_view,
                    surface_bloom_fxaa_pipeline,
                    bloom_config,
                );
                counts.bloom = 1;
                counts.fxaa = 1;
            } else if render_fxaa_to_surface {
                let Some(surface_fxaa_pipeline) = post::surface_fxaa_pipeline(post_resources)
                else {
                    return Err(RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    });
                };
                post::encode_fxaa_to_view(
                    &mut encoder,
                    &self.queue,
                    post_resources,
                    output,
                    &surface_view,
                    surface_fxaa_pipeline,
                );
                counts.fxaa = 1;
            } else {
                let Some(surface_blit_pipeline) = post::surface_blit_pipeline(post_resources)
                else {
                    return Err(RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    });
                };
                post::encode_blit_to_view(
                    &mut encoder,
                    post_resources,
                    output,
                    &surface_view,
                    surface_blit_pipeline,
                );
            }
            self.queue.submit(Some(encoder.finish()));
            surface_output.present();
            return Ok(GpuRenderResult {
                submitted: true,
                post_counts: counts,
            });
        } else {
            GpuPostPassCounts::default()
        };
        if !post_enabled && let Some(readback) = resources.readback.as_ref() {
            encode_browser_readback_pass(
                &mut encoder,
                BrowserReadbackPass {
                    target,
                    readback,
                    depth_view: None,
                    vertex_buffer: &resources.vertex_buffer,
                    output_bind_group: &resources.output_bind_group,
                    opaque_output_bind_group: &resources.opaque_output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    material_resources: &resources.material_resources,
                    draw_batches: &resources.draw_batches,
                    transmission: &resources.transmission,
                    clear_color: wgpu_clear_color(background_color),
                },
            );
        }
        self.queue.submit(Some(encoder.finish()));
        surface_output.present();
        Ok(GpuRenderResult {
            submitted: true,
            post_counts,
        })
    }

    fn render_empty_surface(
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

    #[cfg(feature = "browser-probe")]
    pub(in crate::render) async fn browser_probe_readback_rgba8(
        &mut self,
        target: RasterTarget,
    ) -> Result<Option<Vec<u8>>, JsValue> {
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
                        &JsValue::from_str(&format!("browser proof readback failed: {error:?}")),
                    );
                }
            });
        });
        JsFuture::from(promise).await?;
        let mapped = slice.get_mapped_range();
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
        Ok(Some(frame))
    }
}
