#![cfg(target_arch = "wasm32")]

use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::{ClippingPlane, SectionBox};

use super::super::camera::CameraProjection;
use super::super::{RasterTarget, RenderReadbackMode};
use super::browser_readback::{
    BrowserReadbackPass, encode_browser_readback_pass, encode_texture_readback_copy,
};
use super::depth;
use super::draw_common::{
    camera_position_uniform, identity_matrix, target_color_management_uniform,
    wgpu_clear_color_for_target,
};
use super::output::{OutputUniformUpload, encode_clipping_uniform, encode_output_uniform};
use super::overlays::{OverlayPasses, encode_overlay_passes};
use super::scene_color::{SceneColorPasses, encode_scene_color_passes};
use super::shadow::{self, encode_shadow_caster_pass};
use super::{
    GpuDeviceState, GpuPostPassCounts, GpuPostSettings, GpuRenderResult, labels, post,
    semantic_aov, strokes, surface_frame,
};

impl GpuDeviceState {
    pub(in crate::render) fn render_to_surface(
        &mut self,
        target: RasterTarget,
        readback_mode: RenderReadbackMode,
        exposure_ev: f32,
        color_management: [f32; 4],
        white_balance: [f32; 4],
        background_color: Color,
        camera_projection: &CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
        post_settings: GpuPostSettings,
        auto_exposure_meter: bool,
    ) -> Result<GpuRenderResult, RenderError> {
        if let Some(error) = self.runtime_fault.render_error(target.backend) {
            return Err(error);
        }
        let Some(resources) = self.resources.as_mut() else {
            return self.render_empty_surface(target, background_color);
        };
        if let Some(semantic) = resources.semantic_aov.as_mut() {
            semantic_aov::invalidate_beauty_witness(semantic);
        }
        if resources.target != target {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let Some(surface_format) = self.surface.as_ref().map(|surface| surface.config.format)
        else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        let post_enabled = post_settings.enabled();
        let tonemapper_mode = color_management[0];
        let scene_format = if post_enabled {
            post::scene_color_format()
        } else {
            surface_format
        };
        if post_enabled
            && !resources
                .post
                .as_ref()
                .is_some_and(|post| post::resources_match(post, target))
        {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let mut color_management = target_color_management_uniform(color_management, scene_format);
        if let Some(reflections) = post_settings.reflections() {
            color_management[2] = reflections.strength();
            color_management[3] = reflections.roughness();
        }
        let (clipping_planes, clipping_control) =
            encode_clipping_uniform(clipping_planes, section_box);
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
                white_balance,
                lighting: resources.light_uniform,
                clipping_planes,
                clipping_control,
            }),
        );
        if resources
            .depth_prepass
            .as_ref()
            .is_some_and(|depth_prepass| {
                depth_prepass.depth_color_enabled() != post_settings.needs_depth_color()
            })
        {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        #[cfg(feature = "browser-probe")]
        if readback_mode == RenderReadbackMode::Synchronous
            && let Some(result) = super::draw_surface_probe::render_browser_probe(
                &self.device,
                &self.queue,
                resources,
                target,
                background_color,
                post_settings,
                exposure_ev,
                tonemapper_mode,
                white_balance,
            )?
        {
            return Ok(result);
        }
        let surface_frame::SurfaceFrameAcquisition {
            output: surface_output,
            skip: surface_skip,
            reconfigure_after_present,
            mut reconfigurations,
            retries: surface_acquire_retries,
        } = surface_frame::acquire_surface_frame(
            self.surface.as_mut(),
            &self.adapter,
            &self.device,
            target,
        )?;
        if surface_skip.is_some() {
            self.refresh_browser_canvas_output_color_space(target.backend);
            return Ok(GpuRenderResult {
                surface_skip,
                surface_reconfigurations: reconfigurations,
                surface_acquire_retries,
                ..GpuRenderResult::default()
            });
        }
        let surface_output = surface_output.expect("attached browser surface acquired a frame");
        let surface_view = surface_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.browser.encoder"),
            });
        let mut draw_submissions = 0;
        encode_shadow_caster_pass(
            &mut encoder,
            &resources.shadow_caster,
            shadow::ShadowCasterPassInputs {
                vertex_buffer: &resources.vertex_buffer,
                instance_buffer: &resources.instance_buffer,
                draw_bind_group: &resources.draw_bind_group,
                draw_batches: &resources.draw_batches,
                instance_batches: &resources.instance_batches,
                identity_instance: resources.identity_instance,
                draw_submissions: &mut draw_submissions,
            },
        );
        if let Some(depth_prepass) = &resources.depth_prepass {
            depth::encode_depth_prepass(
                &mut encoder,
                depth_prepass,
                depth::DepthPrepassInputs {
                    vertex_buffer: &resources.vertex_buffer,
                    instance_buffer: &resources.instance_buffer,
                    camera_bind_group: &resources.output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    draw_batches: &resources.draw_batches,
                    instance_batches: &resources.instance_batches,
                    identity_instance: resources.identity_instance,
                    draw_submissions: &mut draw_submissions,
                },
            );
        }
        let surface_readback = (readback_mode == RenderReadbackMode::Synchronous)
            .then_some(resources.readback.as_ref())
            .flatten()
            .filter(|_| {
                self.surface.as_ref().is_some_and(|surface| {
                    surface.config.usage.contains(wgpu::TextureUsages::COPY_SRC)
                })
            });
        if readback_mode == RenderReadbackMode::Synchronous
            && !post_enabled
            && surface_readback.is_none()
            && let Some(readback) = resources.readback.as_ref()
        {
            // Encode the independent capture pass before the visible surface
            // color pass. Both consume the prepared depth prepass; encoding
            // capture after surface color can reuse depth written for the
            // presented frame and reject every repeated fragment.
            encode_browser_readback_pass(
                &mut encoder,
                BrowserReadbackPass {
                    target,
                    readback,
                    readback_pipelines: readback
                        .pipelines
                        .as_ref()
                        .map(super::pipeline::MeshPipelineSet::refs)
                        .unwrap_or_else(|| resources.surface_pipeline.refs()),
                    depth_view: resources.depth_prepass.as_ref().map(|depth| &depth.view),
                    vertex_buffer: &resources.vertex_buffer,
                    instance_buffer: &resources.instance_buffer,
                    output_bind_group: &resources.output_bind_group,
                    opaque_output_bind_group: &resources.opaque_output_bind_group,
                    reflection_probe_output_bind_groups: &resources
                        .reflection_probe_output_bind_groups,
                    reflection_probe_opaque_output_bind_groups: &resources
                        .reflection_probe_opaque_output_bind_groups,
                    draw_bind_group: &resources.draw_bind_group,
                    material_resources: &resources.material_resources,
                    stroke_resources: resources.strokes.as_ref(),
                    label_resources: resources.labels.as_ref(),
                    draw_batches: &resources.draw_batches,
                    instance_batches: &resources.instance_batches,
                    identity_instance: resources.identity_instance,
                    transmission: &resources.transmission,
                    clear_color: wgpu_clear_color_for_target(background_color, readback.format),
                    draw_submissions: &mut draw_submissions,
                },
            );
        }
        let post_resources = resources.post.as_ref();
        let (final_view, final_pipelines, base_label) = if post_enabled {
            let post_resources = post_resources.expect("post resources were created above");
            (
                post::scene_view(post_resources),
                post::scene_pipelines(post_resources, 1),
                "scena.browser.post_scene_pass",
            )
        } else {
            (
                &surface_view,
                resources.surface_pipeline.refs(),
                "scena.browser.render_pass",
            )
        };
        let (semantic_view, semantic_resolve_target) = resources
            .semantic_aov
            .as_ref()
            .and_then(|semantic| semantic_aov::beauty_attachment_views(semantic, target))
            .map_or((None, None), |(view, resolve_target)| {
                (Some(view), resolve_target)
            });
        let beauty_witness_encoded = semantic_view.is_some();
        encode_scene_color_passes(
            &mut encoder,
            SceneColorPasses {
                final_view,
                final_resolve_target: None,
                semantic_view,
                semantic_resolve_target,
                final_pipelines,
                depth_view: resources.depth_prepass.as_ref().map(|d| &d.view),
                vertex_buffer: &resources.vertex_buffer,
                instance_buffer: &resources.instance_buffer,
                output_bind_group: &resources.output_bind_group,
                opaque_output_bind_group: &resources.opaque_output_bind_group,
                reflection_probe_output_bind_groups: &resources.reflection_probe_output_bind_groups,
                reflection_probe_opaque_output_bind_groups: &resources
                    .reflection_probe_opaque_output_bind_groups,
                draw_bind_group: &resources.draw_bind_group,
                material_resources: &resources.material_resources,
                draw_batches: &resources.draw_batches,
                instance_batches: &resources.instance_batches,
                identity_instance: resources.identity_instance,
                transmission_view: &resources.transmission.view,
                transmission_pipelines: resources
                    .transmission
                    .pipelines
                    .as_ref()
                    .map(super::pipeline::MeshPipelineSet::refs),
                force_scene_color_pass: post_settings.reflections().is_some(),
                clear_color: wgpu_clear_color_for_target(background_color, scene_format),
                base_label,
                draw_submissions: &mut draw_submissions,
            },
        );
        if !post_enabled {
            encode_overlay_passes(
                &mut encoder,
                OverlayPasses {
                    view: final_view,
                    depth_view: resources.depth_prepass.as_ref().map(|d| &d.view),
                    output_bind_group: &resources.output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    stroke_resources: resources.strokes.as_ref(),
                    stroke_pipeline: resources.strokes.as_ref().map(strokes::pipeline),
                    label_resources: resources.labels.as_ref(),
                    label_pipeline: resources.labels.as_ref().map(labels::pipeline),
                    label: "scena.browser.overlay_scene_pass",
                    draw_submissions: &mut draw_submissions,
                },
            );
        }
        let post_counts = if post_enabled {
            let post_resources = resources.post.as_ref().expect("post resources exist");
            let renderer_readback = (readback_mode == RenderReadbackMode::Synchronous
                && surface_readback.is_none())
            .then_some(resources.readback.as_ref())
            .flatten();
            let (output, counts) = post::encode_chain(
                &mut encoder,
                &self.queue,
                post_resources,
                post_settings,
                resources.depth_prepass.as_ref(),
                &mut draw_submissions,
            )?;
            if let Some(readback) = renderer_readback {
                encode_overlay_passes(
                    &mut encoder,
                    OverlayPasses {
                        view: post::output_view(post_resources, output),
                        depth_view: None,
                        output_bind_group: &resources.output_bind_group,
                        draw_bind_group: &resources.draw_bind_group,
                        stroke_resources: resources.strokes.as_ref(),
                        stroke_pipeline: resources.strokes.as_ref().map(strokes::post_pipeline),
                        label_resources: resources.labels.as_ref(),
                        label_pipeline: resources.labels.as_ref().map(labels::post_pipeline),
                        label: "scena.browser.capture_overlay_final_pass",
                        draw_submissions: &mut draw_submissions,
                    },
                );
                let readback_pipeline = post::readback_blit_pipeline(post_resources).ok_or(
                    RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    },
                )?;
                post::encode_blit_to_view(
                    &mut encoder,
                    &self.queue,
                    post_resources,
                    output,
                    &readback.view,
                    readback_pipeline,
                    2.0_f32.powf(exposure_ev),
                    tonemapper_mode,
                    white_balance,
                    &mut draw_submissions,
                );
                encode_texture_readback_copy(&mut encoder, &readback.texture, readback, target);
            }
            let Some(surface_blit_pipeline) = post::surface_blit_pipeline(post_resources) else {
                return Err(RenderError::GpuResourcesNotPrepared {
                    backend: target.backend,
                });
            };
            post::encode_blit_to_view(
                &mut encoder,
                &self.queue,
                post_resources,
                output,
                &surface_view,
                surface_blit_pipeline,
                2.0_f32.powf(exposure_ev),
                tonemapper_mode,
                white_balance,
                &mut draw_submissions,
            );
            if renderer_readback.is_none() {
                let stroke_pipeline = match resources.strokes.as_ref() {
                    Some(stroke_resources) => {
                        Some(strokes::surface_pipeline(stroke_resources).ok_or(
                            RenderError::GpuResourcesNotPrepared {
                                backend: target.backend,
                            },
                        )?)
                    }
                    None => None,
                };
                let label_pipeline = match resources.labels.as_ref() {
                    Some(label_resources) => {
                        Some(labels::surface_pipeline(label_resources).ok_or(
                            RenderError::GpuResourcesNotPrepared {
                                backend: target.backend,
                            },
                        )?)
                    }
                    None => None,
                };
                encode_overlay_passes(
                    &mut encoder,
                    OverlayPasses {
                        view: &surface_view,
                        depth_view: resources.depth_prepass.as_ref().map(|d| &d.view),
                        output_bind_group: &resources.output_bind_group,
                        draw_bind_group: &resources.draw_bind_group,
                        stroke_resources: resources.strokes.as_ref(),
                        stroke_pipeline,
                        label_resources: resources.labels.as_ref(),
                        label_pipeline,
                        label: "scena.browser.overlay_final_surface_pass",
                        draw_submissions: &mut draw_submissions,
                    },
                );
            }
            if let Some(readback) = surface_readback {
                encode_texture_readback_copy(
                    &mut encoder,
                    &surface_output.texture,
                    readback,
                    target,
                );
            }
            let meter_submission = if auto_exposure_meter {
                self.browser_auto_exposure_meter.encode_copy(
                    &mut encoder,
                    &post_resources.scene_texture,
                    target,
                )
            } else {
                None
            };
            let meter_submitted = meter_submission.is_some();
            self.queue.submit(Some(encoder.finish()));
            if beauty_witness_encoded && let Some(semantic) = resources.semantic_aov.as_mut() {
                semantic_aov::mark_beauty_witness_written(semantic);
            }
            if let Some(submission) = meter_submission {
                self.browser_auto_exposure_meter.begin_mapping(submission);
            }
            surface_output.present();
            if reconfigure_after_present && let Some(surface) = self.surface.as_mut() {
                let change = surface_frame::refresh_surface_configuration(
                    surface,
                    &self.adapter,
                    &self.device,
                    target,
                );
                reconfigurations = reconfigurations.saturating_add(1);
                self.refresh_browser_canvas_output_color_space(target.backend);
                if change.requires_reprepare() {
                    return Err(RenderError::SurfaceConfigurationChanged {
                        backend: target.backend,
                    });
                }
            }
            return Ok(GpuRenderResult {
                submitted: true,
                post_counts: counts,
                draw_submissions,
                auto_exposure_meter_submissions: u64::from(meter_submitted),
                auto_exposure_meter_samples: u64::from(meter_submitted) * 256,
                surface_reconfigurations: reconfigurations,
                surface_acquire_retries,
                ..GpuRenderResult::default()
            });
        } else {
            GpuPostPassCounts::default()
        };
        if let Some(readback) = surface_readback {
            encode_texture_readback_copy(&mut encoder, &surface_output.texture, readback, target);
        }
        self.queue.submit(Some(encoder.finish()));
        if beauty_witness_encoded && let Some(semantic) = resources.semantic_aov.as_mut() {
            semantic_aov::mark_beauty_witness_written(semantic);
        }
        surface_output.present();
        if reconfigure_after_present && let Some(surface) = self.surface.as_mut() {
            let change = surface_frame::refresh_surface_configuration(
                surface,
                &self.adapter,
                &self.device,
                target,
            );
            reconfigurations = reconfigurations.saturating_add(1);
            self.refresh_browser_canvas_output_color_space(target.backend);
            if change.requires_reprepare() {
                return Err(RenderError::SurfaceConfigurationChanged {
                    backend: target.backend,
                });
            }
        }
        Ok(GpuRenderResult {
            submitted: true,
            post_counts,
            draw_submissions,
            surface_reconfigurations: reconfigurations,
            surface_acquire_retries,
            ..GpuRenderResult::default()
        })
    }
}
