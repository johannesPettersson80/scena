use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::{ClippingPlane, SectionBox};

use super::super::RasterTarget;
use super::super::camera::CameraProjection;
use super::depth;
use super::draw_common::{
    camera_position_uniform, identity_matrix, post_color_management_uniform, wgpu_clear_color,
};
use super::draw_overlays::{
    encode_offscreen_overlay_pass, encode_surface_overlay_pass, resolved_depth_view,
};
use super::output::{OutputUniformUpload, encode_clipping_uniform, encode_output_uniform};
use super::pipeline::GPU_COLOR_FORMAT;
use super::scene_color::{SceneColorPasses, encode_scene_color_passes};
use super::shadow::{self, encode_shadow_caster_pass};
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
        clipping_planes: &[ClippingPlane],
        section_box: Option<SectionBox>,
        frame: &mut Vec<u8>,
        post_settings: GpuPostSettings,
        readback: bool,
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
        let sample_count = post_settings.sample_count();
        let post_enabled = post_settings.enabled();
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
        let mut color_management = post_color_management_uniform(color_management, post_enabled);
        if let Some(reflections) = post_settings.reflections() {
            color_management[2] = reflections.strength();
            color_management[3] = reflections.roughness();
        }
        let scene_format = if post_enabled {
            post::scene_color_format()
        } else {
            GPU_COLOR_FORMAT
        };
        let max_supported_sample_count = super::msaa::max_supported_sample_count(
            &self.device,
            &self.adapter,
            &[scene_format, wgpu::TextureFormat::Depth32Float],
        );
        if sample_count > max_supported_sample_count {
            return Err(RenderError::UnsupportedSampleCount {
                backend: target.backend,
                requested: sample_count,
                maximum: max_supported_sample_count,
            });
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
        if resources
            .depth_prepass
            .as_ref()
            .is_some_and(|depth_prepass| {
                depth_prepass.depth_color_enabled() != post_settings.needs_depth_color()
                    || depth_prepass.sample_count() != sample_count
            })
        {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
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
        let mut draw_submissions = 0;
        // Phase 1B step 2: shadow caster pass writes the directional shadow
        // map BEFORE the unlit pass so the fragment shader can sample it.
        // No-op if no shadow-casting directional light exists.
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
        if sample_count > 1 {
            if resources.depth_prepass.is_some() && resources.overlay_depth_prepass.is_none() {
                return Err(RenderError::GpuResourcesNotPrepared {
                    backend: target.backend,
                });
            }
            if let Some(overlay_depth_prepass) = &resources.overlay_depth_prepass {
                depth::encode_depth_prepass(
                    &mut encoder,
                    overlay_depth_prepass,
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
        }
        if sample_count > 1
            && !resources.msaa_color.as_ref().is_some_and(|msaa| {
                msaa.target == target
                    && msaa.format == scene_format
                    && msaa.sample_count == sample_count
            })
        {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let post_resources = resources.post.as_ref();
        let (final_view, final_pipelines, base_label) = if post_enabled {
            let post_resources = post_resources.expect("post resources were created above");
            (
                post::scene_view(post_resources),
                post::scene_pipelines(post_resources, sample_count),
                "scena.headless_gpu.post_scene_pass",
            )
        } else {
            (
                &resources.view,
                super::msaa::offscreen_pipelines_for_sample_count(resources, sample_count),
                "scena.headless_gpu.render_pass",
            )
        };
        let msaa_view = resources.msaa_color.as_ref().map(|msaa| &msaa.view);
        let scene_attachment_view = msaa_view.unwrap_or(final_view);
        let scene_resolve_target = msaa_view.map(|_| final_view);
        let resolved_depth_view = resolved_depth_view(resources, sample_count);
        encode_scene_color_passes(
            &mut encoder,
            SceneColorPasses {
                final_view: scene_attachment_view,
                final_resolve_target: scene_resolve_target,
                final_pipelines,
                depth_view: resources
                    .depth_prepass
                    .as_ref()
                    .map(|depth_prepass| &depth_prepass.view),
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
            encode_offscreen_overlay_pass(
                &mut encoder,
                resources,
                final_view,
                resolved_depth_view,
                "scena.gpu_overlay.offscreen_pass",
                &mut draw_submissions,
            );
        }
        let post_output = if post_enabled {
            let post_resources = resources.post.as_ref().expect("post resources exist");
            let (output, post_counts) = post::encode_chain(
                &mut encoder,
                &self.queue,
                post_resources,
                post_settings,
                resources.depth_prepass.as_ref(),
                &mut draw_submissions,
            )?;
            post::encode_blit_to_view(
                &mut encoder,
                post_resources,
                output,
                &resources.view,
                post::output_blit_pipeline(post_resources),
                &mut draw_submissions,
            );
            encode_offscreen_overlay_pass(
                &mut encoder,
                resources,
                &resources.view,
                resolved_depth_view,
                "scena.gpu_overlay.post_final_offscreen_pass",
                &mut draw_submissions,
            );
            if let Some(surface_view) = surface_view.as_ref() {
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
                    surface_view,
                    surface_blit_pipeline,
                    &mut draw_submissions,
                );
                encode_surface_overlay_pass(
                    &mut encoder,
                    resources,
                    surface_view,
                    resolved_depth_view,
                    "scena.gpu_overlay.post_final_surface_pass",
                    target,
                    &mut draw_submissions,
                )?;
            }
            if readback {
                super::readback::encode_copy_target_to_readback(&mut encoder, resources, target);
            }
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
                    final_resolve_target: None,
                    final_pipelines: surface_pipeline.refs(),
                    depth_view: resolved_depth_view,
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
                    force_scene_color_pass: false,
                    clear_color: wgpu_clear_color(background_color),
                    base_label: "scena.surface.render_pass",
                    draw_submissions: &mut draw_submissions,
                },
            );
            encode_surface_overlay_pass(
                &mut encoder,
                resources,
                surface_view,
                resolved_depth_view,
                "scena.gpu_overlay.surface_pass",
                target,
                &mut draw_submissions,
            )?;
        }
        if readback && !post_enabled {
            super::readback::encode_copy_target_to_readback(&mut encoder, resources, target);
        }
        self.queue.submit(Some(encoder.finish()));
        if let Some(surface_output) = surface_output {
            surface_output.present();
        }
        if readback {
            super::readback::map_readback_to_frame(&self.device, resources, target, frame)?;
        }

        Ok(GpuRenderResult {
            submitted: true,
            post_counts: post_output
                .map(|(_, counts)| counts)
                .unwrap_or_else(GpuPostPassCounts::default),
            draw_submissions,
            readback_copies: u64::from(readback),
            readback_bytes_copied: u64::from(readback).saturating_mul(target.byte_len() as u64),
            map_requests: u64::from(readback),
            blocking_polls: u64::from(readback),
            blocking_waits: u64::from(readback),
            cpu_frame_copy_bytes: u64::from(readback).saturating_mul(target.byte_len() as u64),
        })
    }
}
