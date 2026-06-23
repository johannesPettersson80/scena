use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs, round3};
use super::helpers::{resolve_target_handles, world_bounds_for_handle};
use crate::geometry::Aabb;
use crate::{
    SceneCompositionCheckV1, SceneInspectionReportV1, SceneRecipeBuildV1, SceneRecipeExpectV1,
};

const DEFAULT_SEPARATION_TOLERANCE: f32 = 0.001;

pub(super) fn composition_separation_checks(
    expect: Option<&SceneRecipeExpectV1>,
    manifest: &SceneRecipeBuildV1,
    inspection: &SceneInspectionReportV1,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let Some(expect) = expect else {
        return checks;
    };
    for expectation in &expect.expect_separation {
        let id = format!("expect_separation.{}", expectation.id);
        let target_id = Some(expectation.id.clone());
        let a_handles = resolve_target_handles(&expectation.a, manifest);
        let b_handles = resolve_target_handles(&expectation.b, manifest);
        let affected = combined_handles(&a_handles, &b_handles);
        if a_handles.is_empty() || b_handles.is_empty() {
            checks.push(error_check(
                id,
                "placement",
                "separation_target_unresolved",
                target_id,
                affected,
                observed_pairs([
                    ("a_handles", json!(a_handles)),
                    ("b_handles", json!(b_handles)),
                ]),
                (
                    "separation expectation did not resolve both targets",
                    "fix the target ids or remove the stale separation expectation",
                ),
            ));
            continue;
        }

        let a_bounds = union_world_bounds(inspection, &a_handles);
        let b_bounds = union_world_bounds(inspection, &b_handles);
        let (Some(a_bounds), Some(b_bounds)) = (a_bounds, b_bounds) else {
            checks.push(error_check(
                id,
                "placement",
                "separation_bounds_unavailable",
                target_id,
                affected,
                observed_pairs([
                    ("a_handles", json!(a_handles)),
                    ("b_handles", json!(b_handles)),
                ]),
                (
                    "separation expectation could not derive world-space bounds for both targets",
                    "use bounded geometry targets or remove the separation expectation",
                ),
            ));
            continue;
        };

        let min_gap = expectation.min_gap.map_or(0.0, |value| value as f32);
        let tolerance = expectation
            .tolerance
            .map_or(DEFAULT_SEPARATION_TOLERANCE, |value| value as f32);
        let clearance = signed_aabb_clearance(a_bounds, b_bounds);
        let passes = clearance + tolerance >= min_gap;
        let observed = observed_pairs([
            ("a_handles", json!(a_handles)),
            ("b_handles", json!(b_handles)),
            ("a_bounds", json!(bounds_json(a_bounds))),
            ("b_bounds", json!(bounds_json(b_bounds))),
            ("clearance_meters", json!(round3(clearance))),
            ("min_gap_meters", json!(round3(min_gap))),
            ("tolerance_meters", json!(round3(tolerance))),
        ]);
        if passes {
            checks.push(checked_check(
                id,
                "placement",
                "separation_conformance_satisfied",
                target_id,
                affected,
                observed,
                (
                    "declared targets satisfy the requested world-space separation",
                    "no action needed",
                ),
            ));
        } else {
            checks.push(error_check(
                id,
                "placement",
                "separation_conformance_mismatch",
                target_id,
                affected,
                observed,
                (
                    "declared targets intersect or are closer than the requested separation",
                    "move or scale one target, reduce min_gap only if the proximity is intentional, or remove the expectation",
                ),
            ));
        }
    }
    checks
}

fn union_world_bounds(inspection: &SceneInspectionReportV1, handles: &[u64]) -> Option<Aabb> {
    let mut bounds: Option<Aabb> = None;
    for handle in handles {
        let Some(handle_bounds) = world_bounds_for_handle(inspection, *handle) else {
            continue;
        };
        bounds = Some(match bounds {
            Some(bounds) => bounds.union(handle_bounds),
            None => handle_bounds,
        });
    }
    bounds
}

fn signed_aabb_clearance(a: Aabb, b: Aabb) -> f32 {
    let gaps = [
        signed_axis_gap(a.min.x, a.max.x, b.min.x, b.max.x),
        signed_axis_gap(a.min.y, a.max.y, b.min.y, b.max.y),
        signed_axis_gap(a.min.z, a.max.z, b.min.z, b.max.z),
    ];
    let max_positive_gap = gaps
        .iter()
        .copied()
        .filter(|gap| *gap >= 0.0)
        .fold(f32::NEG_INFINITY, f32::max);
    if max_positive_gap.is_finite() {
        max_positive_gap
    } else {
        gaps.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }
}

fn signed_axis_gap(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> f32 {
    if a_max < b_min {
        b_min - a_max
    } else if b_max < a_min {
        a_min - b_max
    } else {
        -(a_max.min(b_max) - a_min.max(b_min))
    }
}

fn bounds_json(bounds: Aabb) -> serde_json::Value {
    json!({
        "min": [round3(bounds.min.x), round3(bounds.min.y), round3(bounds.min.z)],
        "max": [round3(bounds.max.x), round3(bounds.max.y), round3(bounds.max.z)]
    })
}

fn combined_handles(left: &[u64], right: &[u64]) -> Vec<u64> {
    let mut handles = left.to_vec();
    handles.extend(right);
    handles.sort_unstable();
    handles.dedup();
    handles
}
