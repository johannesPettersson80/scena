#![cfg(target_arch = "wasm32")]

use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::{ClippingPlane, SectionBox};

use super::super::RasterTarget;
use super::super::camera::CameraProjection;
use super::browser_readback::{BrowserReadbackPass, encode_browser_readback_pass};
use super::depth;
use super::draw_common::{
    camera_position_uniform, identity_matrix, post_color_management_uniform, wgpu_clear_color,
};
use super::output::{OutputUniformUpload, encode_clipping_uniform, encode_output_uniform};
use super::overlays::{OverlayPasses, encode_overlay_passes};
use super::scene_color::{SceneColorPasses, encode_scene_color_passes};
use super::shadow::{self, encode_shadow_caster_pass};
use super::{
    GpuDeviceState, GpuPostPassCounts, GpuPostSettings, GpuRenderResult, labels, post, strokes,
};

impl GpuDeviceState {
    pub(in crate::render) fn render_to_surface(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        color_management: [f32; 4],
        background_color: Color,
        camera_projection: &CameraProjection,
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
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
        let mut color_management = post_color_management_uniform(color_management, post_enabled);
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
                lighting: resources.light_uniform,
                clipping_planes,
                clipping_control,
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
                1,
            ));
        }
        #[cfg(feature = "browser-probe")]
        if let Some(readback) = resources.readback.as_ref() {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("scena.browser.proof_encoder"),
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
            let post_counts = if post_enabled {
                let post_resources = resources.post.as_ref().expect("post resources exist");
                encode_scene_color_passes(
                    &mut encoder,
                    SceneColorPasses {
                        final_view: post::scene_view(post_resources),
                        final_resolve_target: None,
                        final_pipelines: post::scene_pipelines(post_resources, 1),
                        depth_view: resources.depth_prepass.as_ref().map(|d| &d.view),
                        vertex_buffer: &resources.vertex_buffer,
                        instance_buffer: &resources.instance_buffer,
                        output_bind_group: &resources.output_bind_group,
                        opaque_output_bind_group: &resources.opaque_output_bind_group,
                        draw_bind_group: &resources.draw_bind_group,
                        material_resources: &resources.material_resources,
                        draw_batches: &resources.draw_batches,
                        instance_batches: &resources.instance_batches,
                        identity_instance: resources.identity_instance,
                        transmission_view: &resources.transmission.view,
                        transmission_pipelines: resources.transmission.pipelines.refs(),
                        force_scene_color_pass: post_settings.reflections().is_some(),
                        clear_color: wgpu_clear_color(background_color),
                        base_label: "scena.browser.proof_post_scene_pass",
                        draw_submissions: &mut draw_submissions,
                    },
                );
                let (output, counts) = post::encode_chain(
                    &mut encoder,
                    &self.device,
                    &self.queue,
                    post_resources,
                    post_settings,
                    resources.depth_prepass.as_ref(),
                    &mut draw_submissions,
                )?;
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
                        label: "scena.browser.proof_overlay_final_pass",
                        draw_submissions: &mut draw_submissions,
                    },
                );
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
                        depth_view: resources.depth_prepass.as_ref().map(|d| &d.view),
                        vertex_buffer: &resources.vertex_buffer,
                        output_bind_group: &resources.output_bind_group,
                        opaque_output_bind_group: &resources.opaque_output_bind_group,
                        draw_bind_group: &resources.draw_bind_group,
                        material_resources: &resources.material_resources,
                        stroke_resources: resources.strokes.as_ref(),
                        label_resources: resources.labels.as_ref(),
                        draw_batches: &resources.draw_batches,
                        instance_buffer: &resources.instance_buffer,
                        instance_batches: &resources.instance_batches,
                        identity_instance: resources.identity_instance,
                        transmission: &resources.transmission,
                        clear_color: wgpu_clear_color(background_color),
                        draw_submissions: &mut draw_submissions,
                    },
                );
                GpuPostPassCounts::default()
            };
            self.queue.submit(Some(encoder.finish()));
            return Ok(GpuRenderResult {
                submitted: true,
                post_counts,
                draw_submissions,
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
        encode_scene_color_passes(
            &mut encoder,
            SceneColorPasses {
                final_view,
                final_resolve_target: None,
                final_pipelines,
                depth_view: resources.depth_prepass.as_ref().map(|d| &d.view),
                vertex_buffer: &resources.vertex_buffer,
                instance_buffer: &resources.instance_buffer,
                output_bind_group: &resources.output_bind_group,
                opaque_output_bind_group: &resources.opaque_output_bind_group,
                draw_bind_group: &resources.draw_bind_group,
                material_resources: &resources.material_resources,
                draw_batches: &resources.draw_batches,
                instance_batches: &resources.instance_batches,
                identity_instance: resources.identity_instance,
                transmission_view: &resources.transmission.view,
                transmission_pipelines: resources.transmission.pipelines.refs(),
                force_scene_color_pass: post_settings.reflections().is_some(),
                clear_color: wgpu_clear_color(background_color),
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
                &mut draw_submissions,
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
                    post::BloomFxaaToViewInputs {
                        output,
                        target_view: &surface_view,
                        pipeline: surface_bloom_fxaa_pipeline,
                        config: bloom_config,
                        draw_submissions: &mut draw_submissions,
                    },
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
                    &mut draw_submissions,
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
                    &mut draw_submissions,
                );
            }
            let stroke_pipeline = match resources.strokes.as_ref() {
                Some(stroke_resources) => Some(strokes::surface_pipeline(stroke_resources).ok_or(
                    RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    },
                )?),
                None => None,
            };
            let label_pipeline = match resources.labels.as_ref() {
                Some(label_resources) => Some(labels::surface_pipeline(label_resources).ok_or(
                    RenderError::GpuResourcesNotPrepared {
                        backend: target.backend,
                    },
                )?),
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
            self.queue.submit(Some(encoder.finish()));
            surface_output.present();
            return Ok(GpuRenderResult {
                submitted: true,
                post_counts: counts,
                draw_submissions,
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
                    instance_buffer: &resources.instance_buffer,
                    output_bind_group: &resources.output_bind_group,
                    opaque_output_bind_group: &resources.opaque_output_bind_group,
                    draw_bind_group: &resources.draw_bind_group,
                    material_resources: &resources.material_resources,
                    stroke_resources: resources.strokes.as_ref(),
                    label_resources: resources.labels.as_ref(),
                    draw_batches: &resources.draw_batches,
                    instance_batches: &resources.instance_batches,
                    identity_instance: resources.identity_instance,
                    transmission: &resources.transmission,
                    clear_color: wgpu_clear_color(background_color),
                    draw_submissions: &mut draw_submissions,
                },
            );
        }
        self.queue.submit(Some(encoder.finish()));
        surface_output.present();
        Ok(GpuRenderResult {
            submitted: true,
            post_counts,
            draw_submissions,
        })
    }
}
