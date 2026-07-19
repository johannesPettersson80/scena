use crate::app::prelude::*;

pub(crate) fn check_c06_finite_atomic_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "SCENE-C06";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene/transforms.rs",
            &[
                "fn validate_transform",
                "transform.translation.is_finite()",
                "transform.rotation.is_finite()",
                "transform.scale.is_finite()",
                "validate_transform(*transform)?",
                "LookupError::InvalidTransform",
            ],
        ),
        (
            "src/scene.rs",
            &["transforms::validate_transform(transform)?"],
        ),
        (
            "src/scene/view.rs",
            &[
                "validate_transform(world_transform)?",
                "validate_transform(transform)?",
            ],
        ),
        (
            "src/scene/instances.rs",
            &[
                "let transform = validate_transform(transform)?",
                "pub fn push_instance",
                "pub fn set_instance_transform",
            ],
        ),
        (
            "src/controls.rs",
            &[
                "if !pointer_event_is_finite(event)",
                "if !touch_event_is_finite(event)",
                "fn apply_pan_delta",
                "let view_right = Vec3::new(yaw_cos, 0.0, -yaw_sin)",
                "let view_up = Vec3::new(-yaw_sin * pitch_sin, pitch_cos, -yaw_cos * pitch_sin)",
            ],
        ),
        (
            "src/scene_host/transforms.rs",
            &[
                "struct ResolvedTransformUpdate",
                "self.preflight_instance_root_transform(*node)?",
                "for update in &resolved",
                "self.cancel_transform_transition(update.handle)",
                "set_preflighted_instance_root_transform",
                "self.set_transforms(&resolved)",
            ],
        ),
        (
            "src/scene_host/instances.rs",
            &[
                "fn preflight_instance_root_transform",
                "for entry in &binding.entries",
                "self.scene.node(entry.set_node).is_none()",
                "self.scene.instance_set(entry.set)",
                "instance_set.contains(entry.instance)",
            ],
        ),
        (
            "src/diagnostics.rs",
            &["InvalidTransform", "reason: &'static str"],
        ),
        (
            "docs/api.md",
            &[
                "Transform mutation invariant",
                "preflight the complete batch",
                "camera view-right/view-up",
            ],
        ),
        (
            "docs/errors.md",
            &["LookupError::InvalidTransform", "atomic no-op"],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c06_finite_atomic_transforms.rs",
        &[
            "orbit_pointer_touch_and_pinch_reject_non_finite_events_and_recover",
            "orbit_pan_uses_camera_right_and_up_at_cardinal_yaws_and_pitch",
            "scene_rejects_non_finite_direct_batch_alignment_and_instance_transforms_atomically",
            "scene_host_pan_matches_camera_space_directions",
            "scene_host_non_finite_camera_event_preserves_transition_and_recovers",
            "scene_host_batches_preflight_stale_and_missing_instance_roots_in_both_orders",
        ],
    );

    if let Ok(source) = fs::read_to_string(root.join("src/controls.rs")) {
        for forbidden in [
            "self.target.x -= event.delta.0",
            "self.target.y += event.delta.1",
        ] {
            if source.contains(forbidden) {
                findings.push(Finding::new(
                    RULE,
                    format!("src/controls.rs contains forbidden fixed-axis pan `{forbidden}`"),
                ));
            }
        }
    }
}
