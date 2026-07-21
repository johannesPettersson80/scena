use crate::diagnostics::{RenderOutcome, RendererStats};

use super::super::{RasterTarget, gpu};

pub(super) fn record_surface_result(
    stats: &mut RendererStats,
    target: RasterTarget,
    result: gpu::GpuRenderResult,
) -> Option<RenderOutcome> {
    stats.surface_reconfigurations = stats
        .surface_reconfigurations
        .saturating_add(result.surface_reconfigurations);
    stats.surface_acquire_retries = stats
        .surface_acquire_retries
        .saturating_add(result.surface_acquire_retries);
    let reason = result.surface_skip?;
    stats.skipped_frames = stats.skipped_frames.saturating_add(1);
    match reason {
        gpu::SurfaceFrameSkipReason::Timeout => {
            stats.surface_timeout_skips = stats.surface_timeout_skips.saturating_add(1);
        }
        gpu::SurfaceFrameSkipReason::Occluded => {
            stats.surface_occluded_skips = stats.surface_occluded_skips.saturating_add(1);
        }
    }
    Some(RenderOutcome {
        width: target.width,
        height: target.height,
        draw_calls: 0,
        primitives: 0,
        skipped: true,
    })
}
