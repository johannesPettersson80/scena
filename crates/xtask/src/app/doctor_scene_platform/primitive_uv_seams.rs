use crate::app::prelude::*;

pub(crate) fn check_c19_primitive_uv_seam_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C19-PRIMITIVE-UV-SEAMS";

    require_contains(
        root,
        findings,
        RULE,
        "src/geometry/primitive_meshes.rs",
        &[
            "Builds a closed cylinder with a duplicated side seam at `u = 1`",
            "for (ring, y) in [-half_height, half_height].into_iter().enumerate() {\n            for segment in 0..=segments {",
            "let theta = if segment == segments",
            "let side_row = segments + 1;",
            "[(segment + 1) as f32 / segments as f32, 1.0]",
            "The final face ends at `u = 1`",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/geometry/primitive_meshes/tests.rs",
        &["(\"cylinder\", GeometryDesc::cylinder(0.10, 0.22, 12), 52, 144)"],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c19_primitive_uv_seams.rs",
        &[
            "cylinder_side_rows_duplicate_the_u1_seam_without_changing_caps_or_indices",
            "cone_last_face_uses_a_distinct_u1_base_vertex_and_local_tip_uv",
            "rendered_last_cylinder_quad_crosses_only_its_local_checker_boundary",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/c19_primitive_uv_seams.rs",
        &[
            "known_bad_uvs[2][0] = 0.0",
            "corrected_transitions < wrapped_transitions",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/render/prepare/tangents.rs",
        &["generated_cylinder_and_cone_seams_keep_finite_local_tangents"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene_host/recipe/authoring/geometry/projection.rs",
        &[
            "checked_mul(segments, 4, \"cylinder vertices\")?,\n                        4,",
            "projected_geometry_counts_match_authored_primitive_builders",
        ],
    );
    for (path, needles) in [
        ("README.md", &["duplicated `u=1` seam vertices"][..]),
        (
            "docs/rendering.md",
            &["Generated cylinder and cone sides duplicate the closing vertex at `u=1`"][..],
        ),
        ("docs/api.md", &["emit seam-safe side UVs"][..]),
        (
            "CHANGELOG.md",
            &["Duplicate generated cylinder and cone side seam vertices at `u=1`"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["generated cylinder and cone side UVs wrapped the final"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
