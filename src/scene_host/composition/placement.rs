use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs, round3};
use super::helpers::{resolve_target_handles, world_bounds_for_handle};
use crate::{
    SceneCompositionCheckV1, SceneInspectionReportV1, SceneRecipeBuildV1, SceneRecipeExpectV1,
};

const DEFAULT_GROUND_TOLERANCE: f32 = 0.01;

pub(super) fn composition_ground_contact_checks(
    expect: Option<&SceneRecipeExpectV1>,
    manifest: &SceneRecipeBuildV1,
    inspection: &SceneInspectionReportV1,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let Some(expect) = expect else {
        return checks;
    };
    for expectation in &expect.expect_grounded {
        let target_id = Some(expectation.id.clone());
        let handles = resolve_target_handles(&expectation.target, manifest);
        let plane_y = expectation.plane_y.unwrap_or(0.0) as f32;
        let tolerance = expectation
            .tolerance
            .map(|value| value as f32)
            .unwrap_or(DEFAULT_GROUND_TOLERANCE);
        if handles.is_empty() {
            checks.push(error_check(
                format!("expect_grounded.{}", expectation.id),
                "placement",
                "ground_target_unresolved",
                target_id,
                Vec::new(),
                observed_pairs([("plane_y", json!(round3(plane_y)))]),
                (
                    "grounded expectation target did not resolve to any built node handles",
                    "fix the expectation target id or remove the stale grounded expectation",
                ),
            ));
            continue;
        }

        let mut missing = Vec::new();
        let mut observed_bounds = Vec::new();
        for handle in handles {
            let Some(bounds) = world_bounds_for_handle(inspection, handle) else {
                missing.push(handle);
                continue;
            };
            let min_y = bounds.min.y;
            let delta = (min_y - plane_y).abs();
            observed_bounds.push(json!({
                "handle": handle,
                "min_y": round3(min_y),
                "plane_y": round3(plane_y),
                "delta": round3(delta),
                "tolerance": round3(tolerance)
            }));
            if delta > tolerance {
                missing.push(handle);
            }
        }
        if missing.is_empty() {
            checks.push(checked_check(
                format!("expect_grounded.{}", expectation.id),
                "placement",
                "ground_contact_present",
                target_id,
                observed_bounds
                    .iter()
                    .filter_map(|value| value["handle"].as_u64())
                    .collect(),
                observed_pairs([("ground_contacts", json!(observed_bounds))]),
                (
                    "declared grounded target touches the expected ground plane within tolerance",
                    "no action needed",
                ),
            ));
        } else {
            checks.push(error_check(
                format!("expect_grounded.{}", expectation.id),
                "placement",
                "ground_contact_missing",
                target_id,
                missing,
                observed_pairs([("ground_contacts", json!(observed_bounds))]),
                (
                    "declared grounded target does not touch the expected ground plane",
                    "ground the node, adjust plane_y/tolerance, or remove the grounded expectation for intentionally floating content",
                ),
            ));
        }
    }
    checks
}
