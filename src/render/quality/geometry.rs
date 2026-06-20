use crate::SceneRecipeQualityGeometryV1;

use super::metrics::geometry_edge_metrics;
use super::types::{RenderQualityCheckV1, RenderQualityRegion};
use super::{ThresholdCheck, push_threshold_check};

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
