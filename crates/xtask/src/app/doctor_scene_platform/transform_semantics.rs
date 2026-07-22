use crate::app::prelude::*;

pub(crate) fn check_c16_transform_scale_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C16-TRANSFORM-SCALE-SEMANTICS";

    require_contains(
        root,
        findings,
        RULE,
        "src/scene/math.rs",
        &[
            "pub const fn with_scale(mut self, scale: Vec3)",
            "pub const fn with_uniform_scale(mut self, scale: f32)",
            "pub const fn scale_by(mut self, scale: f32)",
            "self.scale.x * scale",
            "self.scale.y * scale",
            "self.scale.z * scale",
            "repeated calls compose in call order",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c16_transform_scale_semantics.rs",
        &[
            "scale_by_composes_multiplicatively_without_resetting_other_components",
            "scale_setters_and_composition_are_order_explicit",
            "rotation_helpers_and_scale_by_each_compose_in_call_order",
        ],
    );
    for (path, needles) in [
        (
            "docs/api.md",
            &[
                "Transform builder order is explicit",
                "Transform::with_uniform_scale",
                "migrating code that intentionally depended",
            ][..],
        ),
        (
            "docs/guides/migrating-from-threejs.md",
            &["## Compose Or Replace Transform Scale", "multiplyScalar"][..],
        ),
        (
            "docs/specs/public-api.md",
            &["Transform::with_scale(Vec3)", "v1.8.0 replacement behavior"][..],
        ),
        (
            "README.md",
            &["Transform builder names distinguish replacement from composition"][..],
        ),
        (
            "examples/layers_visibility.rs",
            &["with_uniform_scale(0.5)"][..],
        ),
        (
            "tests/m5_release.rs",
            &[
                "Transform::with_scale",
                "Transform::with_uniform_scale",
                "Transform::scale_by",
            ][..],
        ),
        (
            "CHANGELOG.md",
            &["Make `Transform::scale_by` compose multiplicatively"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["`Transform::scale_by` replaced existing scale"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
