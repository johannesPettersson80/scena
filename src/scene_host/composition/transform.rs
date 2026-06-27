use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs, round3};
use super::helpers::resolve_target_handles;
use crate::{
    Quat, SceneCompositionCheckV1, SceneInspectionReportV1, SceneRecipeBuildV1,
    SceneRecipeExpectV1, SceneRecipeTargetV1, Transform, Vec3,
};

const DEFAULT_TRANSLATION_TOLERANCE: f32 = 0.001;
const DEFAULT_SCALE_TOLERANCE: f32 = 0.001;
const DEFAULT_ROTATION_TOLERANCE_DEGREES: f32 = 0.05;

pub(super) fn composition_transform_checks(
    manifest: &SceneRecipeBuildV1,
    inspection: &SceneInspectionReportV1,
    expect: Option<&SceneRecipeExpectV1>,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let Some(expect) = expect else {
        return checks;
    };
    for expectation in &expect.expect_transform {
        let id = format!("expect_transform.{}", expectation.id);
        let handles = resolve_target_handles(&expectation.target, manifest);
        if handles.is_empty() {
            checks.push(error_check(
                id,
                "placement",
                "transform_target_unresolved",
                Some(expectation.id.clone()),
                Vec::new(),
                observed_pairs([("target", json!(target_summary(&expectation.target)))]),
                (
                    "transform expectation target did not resolve to a manifest handle",
                    "target an authored node or imported node path from the build manifest",
                ),
            ));
            continue;
        }

        let handle_count = handles.len();
        for handle in handles {
            let check_id = if handle_count == 1 {
                id.clone()
            } else {
                format!("{id}.{handle}")
            };
            let Some(node) = inspection.node_by_handle(handle) else {
                checks.push(error_check(
                    check_id,
                    "placement",
                    "transform_target_not_inspected",
                    Some(expectation.id.clone()),
                    vec![handle],
                    observed_pairs([("handle", json!(handle))]),
                    (
                        "transform expectation target is absent from the inspection report",
                        "rerun inspection after building the recipe or remove the stale transform expectation",
                    ),
                ));
                continue;
            };

            let actual = node.world_transform;
            let expected_translation = expectation.translation.map(vec3_from_f64);
            let expected_scale = expectation.scale.map(vec3_from_f64);
            let expected_rotation = expectation.rotation_degrees.map(rotation_from_degrees);
            let translation_delta = expected_translation
                .map(|expected| max_abs_vec3(actual.translation - expected))
                .unwrap_or(0.0);
            let scale_delta = expected_scale
                .map(|expected| max_abs_vec3(actual.scale - expected))
                .unwrap_or(0.0);
            let rotation_delta = expected_rotation
                .map(|expected| rotation_delta_degrees(actual.rotation, expected))
                .unwrap_or(0.0);
            let translation_tolerance = expectation
                .translation_tolerance
                .map_or(DEFAULT_TRANSLATION_TOLERANCE, |value| value as f32);
            let scale_tolerance = expectation
                .scale_tolerance
                .map_or(DEFAULT_SCALE_TOLERANCE, |value| value as f32);
            let rotation_tolerance = expectation
                .rotation_tolerance_degrees
                .map_or(DEFAULT_ROTATION_TOLERANCE_DEGREES, |value| value as f32);
            let translation_ok =
                expected_translation.is_none_or(|_| translation_delta <= translation_tolerance);
            let scale_ok = expected_scale.is_none_or(|_| scale_delta <= scale_tolerance);
            let rotation_ok =
                expected_rotation.is_none_or(|_| rotation_delta <= rotation_tolerance);
            let observed = observed_pairs([
                ("target", json!(target_summary(&expectation.target))),
                ("actual_translation", json!(vec3_json(actual.translation))),
                ("actual_scale", json!(vec3_json(actual.scale))),
                ("actual_rotation_xyzw", json!(quat_json(actual.rotation))),
                (
                    "expected_translation",
                    json!(expected_translation.map(vec3_json)),
                ),
                ("expected_scale", json!(expected_scale.map(vec3_json))),
                (
                    "expected_rotation_degrees",
                    json!(expectation.rotation_degrees),
                ),
                ("translation_delta", json!(round3(translation_delta))),
                ("scale_delta", json!(round3(scale_delta))),
                ("rotation_delta_degrees", json!(round3(rotation_delta))),
                (
                    "translation_tolerance",
                    json!(round3(translation_tolerance)),
                ),
                ("scale_tolerance", json!(round3(scale_tolerance))),
                (
                    "rotation_tolerance_degrees",
                    json!(round3(rotation_tolerance)),
                ),
            ]);

            if translation_ok && scale_ok && rotation_ok {
                checks.push(checked_check(
                    check_id,
                    "placement",
                    "transform_conformance_satisfied",
                    Some(expectation.id.clone()),
                    vec![handle],
                    observed,
                    (
                        "declared transform expectation matches the inspected world transform",
                        "no action needed",
                    ),
                ));
            } else {
                checks.push(error_check(
                    check_id,
                    "placement",
                    "transform_conformance_mismatch",
                    Some(expectation.id.clone()),
                    vec![handle],
                    observed,
                    (
                        "declared transform expectation does not match the inspected world transform",
                        "fix the node transform, parent transform, placement directive, or transform expectation",
                    ),
                ));
            }
        }
    }
    checks
}

fn vec3_from_f64(values: [f64; 3]) -> Vec3 {
    Vec3::new(values[0] as f32, values[1] as f32, values[2] as f32)
}

fn rotation_from_degrees(values: [f64; 3]) -> Quat {
    Transform::IDENTITY
        .rotate_x_deg(values[0] as f32)
        .rotate_y_deg(values[1] as f32)
        .rotate_z_deg(values[2] as f32)
        .rotation
}

fn max_abs_vec3(value: Vec3) -> f32 {
    value.x.abs().max(value.y.abs()).max(value.z.abs())
}

fn rotation_delta_degrees(actual: Quat, expected: Quat) -> f32 {
    let actual = normalized_or_identity(actual);
    let expected = normalized_or_identity(expected);
    let dot = actual.dot(expected).abs().clamp(0.0, 1.0);
    (2.0 * dot.acos()).to_degrees()
}

fn normalized_or_identity(value: Quat) -> Quat {
    if value.length_squared().is_finite() && value.length_squared() > f32::EPSILON {
        value.normalize()
    } else {
        Quat::IDENTITY
    }
}

fn vec3_json(value: Vec3) -> [f32; 3] {
    [round3(value.x), round3(value.y), round3(value.z)]
}

fn quat_json(value: Quat) -> [f32; 4] {
    [
        round3(value.x),
        round3(value.y),
        round3(value.z),
        round3(value.w),
    ]
}

fn target_summary(target: &SceneRecipeTargetV1) -> String {
    match target {
        SceneRecipeTargetV1::Node { id } => format!("node:{id}"),
        SceneRecipeTargetV1::Import { id } => format!("import:{id}"),
        SceneRecipeTargetV1::World { .. } => "world".to_owned(),
    }
}
