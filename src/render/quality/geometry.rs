use crate::SceneRecipeQualityGeometryV1;

use super::metrics::pixel_luminance;
use super::types::round3;
use super::types::{RenderQualityCheckV1, RenderQualityGeometryEdgeMetrics, RenderQualityRegion};
use super::{ThresholdCheck, push_threshold_check};

pub fn geometry_edge_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> RenderQualityGeometryEdgeMetrics {
    if width < 3 || height < 3 {
        return RenderQualityGeometryEdgeMetrics {
            intermediate_edge_fraction: 0.0,
            edge_candidate_fraction: 0.0,
        };
    }
    let x0 = region.x.max(1);
    let y0 = region.y.max(1);
    let x1 = region
        .x
        .saturating_add(region.width)
        .min(width.saturating_sub(1));
    let y1 = region
        .y
        .saturating_add(region.height)
        .min(height.saturating_sub(1));
    let mut edge_candidates = 0usize;
    let mut intermediate = 0usize;
    let mut inspected = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            inspected = inspected.saturating_add(1);
            let mut min_luminance = f32::INFINITY;
            let mut max_luminance = f32::NEG_INFINITY;
            for sy in y.saturating_sub(1)..=y.saturating_add(1).min(height.saturating_sub(1)) {
                for sx in x.saturating_sub(1)..=x.saturating_add(1).min(width.saturating_sub(1)) {
                    let luminance = pixel_luminance(rgba8, width, sx, sy).unwrap_or(0.0);
                    min_luminance = min_luminance.min(luminance);
                    max_luminance = max_luminance.max(luminance);
                }
            }
            let contrast = max_luminance - min_luminance;
            // Product renders often use low-contrast studio palettes. A fixed
            // 0.25 cutoff missed real sampled silhouettes on light-gray
            // subjects over neutral-gray backgrounds, so keep the candidate
            // floor low while still rejecting sub-visible noise/tonemap drift.
            if contrast < 0.08 {
                continue;
            }
            edge_candidates = edge_candidates.saturating_add(1);
            let center = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            let normalized = ((center - min_luminance) / contrast).clamp(0.0, 1.0);
            if (0.02..0.98).contains(&normalized) {
                intermediate = intermediate.saturating_add(1);
            }
        }
    }
    RenderQualityGeometryEdgeMetrics {
        intermediate_edge_fraction: round3(if edge_candidates == 0 {
            0.0
        } else {
            intermediate as f32 / edge_candidates as f32
        }),
        edge_candidate_fraction: round3(if inspected == 0 {
            0.0
        } else {
            edge_candidates as f32 / inspected as f32
        }),
    }
}

pub fn evaluate_geometry_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    expectation: SceneRecipeQualityGeometryV1,
) -> Vec<RenderQualityCheckV1> {
    let metrics = geometry_edge_metrics(rgba8, width, height, region);
    let min_intermediate = expectation.min_intermediate_edge_fraction.unwrap_or(0.05) as f32;
    let mut checks = Vec::new();
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id,
            code: "geometry_missing_antialiasing",
            severity: "error",
            region,
            observed_key: "intermediate_edge_fraction",
            observed: metrics.intermediate_edge_fraction,
            threshold_key: "min_intermediate_edge_fraction",
            threshold: min_intermediate,
            fails: metrics.edge_candidate_fraction <= 0.0
                || metrics.intermediate_edge_fraction < min_intermediate,
            fix_hint: "enable msaa4 or msaa8 for GPU renders, or CPU sample AA, so geometry silhouettes have intermediate edge coverage",
        },
    );
    checks
}
