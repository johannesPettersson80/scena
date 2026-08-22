use crate::diagnostics::RenderError;
use crate::material::Color;

use super::super::RasterTarget;
use super::browser_readback::{BrowserReadbackPass, encode_browser_readback_pass};
use super::draw_common::wgpu_clear_color_for_target;
use super::overlays::{OverlayPasses, encode_overlay_passes};
use super::scene_color::{SceneColorPasses, encode_scene_color_passes};
use super::shadow::{self, encode_shadow_caster_pass};
use super::{
    GpuPostPassCounts, GpuPostSettings, GpuPreparedResources, GpuRenderResult, depth, labels, post,
    semantic_aov, strokes,
};

pub(super) fn render_browser_probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &GpuPreparedResources,
    target: RasterTarget,
    background_color: Color,
    post_settings: GpuPostSettings,
    exposure_ev: f32,
    tonemapper_mode: f32,
    white_balance: [f32; 4],
) -> Result<Option<GpuRenderResult>, RenderError> {
    let Some(readback) = resources.readback.as_ref() else {
        return Ok(None);
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
    let post_counts = if post_settings.enabled() {
        let post_resources = resources.post.as_ref().expect("post resources exist");
        let (semantic_view, semantic_resolve_target) = resources
            .semantic_aov
            .as_ref()
            .and_then(|semantic| semantic_aov::beauty_attachment_views(semantic, target))
            .map_or((None, None), |(view, resolve_target)| {
                (Some(view), resolve_target)
            });
        encode_scene_color_passes(
            &mut encoder,
            SceneColorPasses {
                final_view: post::scene_view(post_resources),
                final_resolve_target: None,
                semantic_view,
                semantic_resolve_target,
                final_pipelines: post::scene_pipelines(post_resources, 1),
                depth_view: resources.depth_prepass.as_ref().map(|depth| &depth.view),
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
                clear_color: wgpu_clear_color_for_target(
                    background_color,
                    post::scene_color_format(),
                ),
                base_label: "scena.browser.proof_post_scene_pass",
                draw_submissions: &mut draw_submissions,
            },
        );
        let (output, counts) = post::encode_chain(
            &mut encoder,
            queue,
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
        let readback_pipeline = post::readback_blit_pipeline(post_resources).ok_or(
            RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            },
        )?;
        post::encode_blit_to_view(
            &mut encoder,
            queue,
            post_resources,
            output,
            &readback.view,
            readback_pipeline,
            2.0_f32.powf(exposure_ev),
            tonemapper_mode,
            white_balance,
            &mut draw_submissions,
        );
        super::browser_readback::encode_texture_readback_copy(
            &mut encoder,
            &readback.texture,
            readback,
            target,
        );
        counts
    } else {
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
                output_bind_group: &resources.output_bind_group,
                opaque_output_bind_group: &resources.opaque_output_bind_group,
                reflection_probe_output_bind_groups: &resources.reflection_probe_output_bind_groups,
                reflection_probe_opaque_output_bind_groups: &resources
                    .reflection_probe_opaque_output_bind_groups,
                draw_bind_group: &resources.draw_bind_group,
                material_resources: &resources.material_resources,
                stroke_resources: resources.strokes.as_ref(),
                label_resources: resources.labels.as_ref(),
                draw_batches: &resources.draw_batches,
                instance_buffer: &resources.instance_buffer,
                instance_batches: &resources.instance_batches,
                identity_instance: resources.identity_instance,
                transmission: &resources.transmission,
                clear_color: wgpu_clear_color_for_target(background_color, readback.format),
                draw_submissions: &mut draw_submissions,
            },
        );
        GpuPostPassCounts::default()
    };
    queue.submit(Some(encoder.finish()));
    Ok(Some(GpuRenderResult {
        submitted: true,
        post_counts,
        draw_submissions,
        ..GpuRenderResult::default()
    }))
}
