use crate::app::prelude::*;

pub(crate) fn check_c08_animation_basis_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C08-Z-UP-ANIMATION-BASIS";

    require_contains(
        root,
        findings,
        RULE,
        "src/scene/import/options.rs",
        &[
            "fn convert_animation_rotation",
            "interpolation == AnimationInterpolation::CubicSpline && output_index % 3 != 1",
            "convert_rotation_derivative(value)",
            "normalize_quat(conjugate_quat(basis, rotation))",
            "z_up_rotation_animation_uses_the_static_transform_basis",
            "z_up_cubic_rotation_converts_derivative_tangents_without_normalizing_them",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/animation.rs",
        &[
            "H: FnMut(AnimationInterpolation, usize, Quat) -> Quat",
            ".map(|(index, value)| map_quat(self.interpolation, index, value))",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene/import/animation_bindings.rs",
        &["options.convert_animation_rotation(interpolation, index, value)"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/assets/gltf/z_up_animated_rotation.gltf",
        &["AnimatedZUp", "LinearZ", "StepZ", "CubicZ", "CUBICSPLINE"],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/m3b_gltf_animation.rs",
        &["z_up_rotation_animation_preserves_rest_pose_and_world_axis_trajectory"],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/dynamic_transform_parity.rs",
        &["z_up_imported_rotation_frame_matches_cpu_and_gpu_after_basis_conversion"],
    );
    for (path, needle) in [
        ("docs/assets.md", "cubic-spline"),
        ("docs/guides/units-axes-handedness.md", "CUBICSPLINE"),
        ("CHANGELOG.md", "cubic-spline"),
        ("docs/release-notes/v1.8.0.md", "cubic-spline"),
    ] {
        require_contains(root, findings, RULE, path, &[needle]);
    }
}
