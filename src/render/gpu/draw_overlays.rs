use crate::diagnostics::RenderError;

use super::overlays::{OverlayPasses, encode_overlay_passes};
use super::{GpuPreparedResources, labels, strokes};
use crate::render::RasterTarget;

pub(super) fn resolved_depth_view(
    resources: &GpuPreparedResources,
    sample_count: u32,
) -> Option<&wgpu::TextureView> {
    if sample_count == 1 {
        resources.depth_prepass.as_ref().map(|depth| &depth.view)
    } else {
        resources
            .overlay_depth_prepass
            .as_ref()
            .map(|depth| &depth.view)
    }
}

pub(super) fn encode_offscreen_overlay_pass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &GpuPreparedResources,
    view: &wgpu::TextureView,
    depth_view: Option<&wgpu::TextureView>,
    label: &'static str,
    draw_submissions: &mut u64,
) {
    encode_overlay_passes(
        encoder,
        OverlayPasses {
            view,
            depth_view,
            output_bind_group: &resources.output_bind_group,
            draw_bind_group: &resources.draw_bind_group,
            stroke_resources: resources.strokes.as_ref(),
            stroke_pipeline: resources.strokes.as_ref().map(|resources| {
                if depth_view.is_some() {
                    strokes::pipeline(resources)
                } else {
                    strokes::flat_pipeline(resources)
                }
            }),
            label_resources: resources.labels.as_ref(),
            label_pipeline: resources.labels.as_ref().map(|resources| {
                if depth_view.is_some() {
                    labels::pipeline(resources)
                } else {
                    labels::flat_pipeline(resources)
                }
            }),
            label,
            draw_submissions,
        },
    );
}

pub(super) fn encode_surface_overlay_pass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &GpuPreparedResources,
    view: &wgpu::TextureView,
    depth_view: Option<&wgpu::TextureView>,
    label: &'static str,
    target: RasterTarget,
    draw_submissions: &mut u64,
) -> Result<(), RenderError> {
    let stroke_pipeline = match resources.strokes.as_ref() {
        Some(stroke_resources) => {
            let pipeline = if depth_view.is_some() {
                strokes::surface_pipeline(stroke_resources)
            } else {
                strokes::surface_flat_pipeline(stroke_resources)
            };
            Some(pipeline.ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?)
        }
        None => None,
    };
    let label_pipeline = match resources.labels.as_ref() {
        Some(label_resources) => {
            let pipeline = if depth_view.is_some() {
                labels::surface_pipeline(label_resources)
            } else {
                labels::surface_flat_pipeline(label_resources)
            };
            Some(pipeline.ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?)
        }
        None => None,
    };
    encode_overlay_passes(
        encoder,
        OverlayPasses {
            view,
            depth_view,
            output_bind_group: &resources.output_bind_group,
            draw_bind_group: &resources.draw_bind_group,
            stroke_resources: resources.strokes.as_ref(),
            stroke_pipeline,
            label_resources: resources.labels.as_ref(),
            label_pipeline,
            label,
            draw_submissions,
        },
    );
    Ok(())
}
