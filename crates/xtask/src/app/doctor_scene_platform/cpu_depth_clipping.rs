use crate::app::prelude::*;

pub(crate) fn check_full_review_cpu_depth_clipping_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    const RULE: &str = "FULL-REVIEW-C13-CPU-DEPTH-CLIPPING";

    require_contains(
        root,
        findings,
        RULE,
        "src/render/cpu_geometry.rs",
        &[
            "MAX_CLIPPED_VERTICES",
            "project_clipped_triangle",
            "clip_depth_plane(&polygon, polygon_len, &mut scratch, near, true)",
            "clip_depth_plane(&polygon, polygon_len, &mut scratch, far, false)",
            "normal: mix_vec3",
            "tex_coord0:",
            "tangent: mix_vec3",
            "tangent_handedness: mix_f32",
            "shadow_visibility: mix_f32",
            "camera.interpolation_weights(projected, affine)",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/render/cpu_geometry.rs",
        &[
            "clipped_intersections_interpolate_the_complete_vertex_payload",
            "clipping_preserves_source_winding_for_every_generated_triangle",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c13_depth_clipping_parity.rs",
        &["close_camera_near_clip_matches_cpu_and_gpu_rendered_output"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/c13_depth_clipping_parity.rs",
        &[
            "SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS",
            "\\\"release_evidence\\\": false",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/cpu_render/row_bands.rs",
        &[
            "projected_primitives",
            "project_clipped_primitive(primitive, target, camera)",
            "projected.row_bounds()",
        ],
    );
    for path in [
        "src/render/cpu.rs",
        "src/render/cpu_transmission.rs",
        "src/render/semantic_aov.rs",
    ] {
        require_contains(
            root,
            findings,
            RULE,
            path,
            &["projected.triangles()", "perspective_weights"],
        );
    }
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c13_cpu_depth_clipping.rs",
        &[
            "cpu_triangle_crossing_near_plane_is_clipped_instead_of_dropped",
            "cpu_triangle_crossing_far_plane_is_clipped_instead_of_dropped",
            "cpu_triangle_spanning_near_and_far_planes_keeps_the_depth_slab",
            "cpu_depth_clipping_accepts_vertices_exactly_on_each_plane",
            "cpu_depth_clipping_rejects_empty_and_degenerate_results_without_artifacts",
            "cpu_large_triangle_spanning_the_camera_clips_to_finite_screen_bounds",
            "cpu_oit_triangle_crossing_near_plane_is_clipped_and_resolved",
            "near_crossing_triangle_keeps_scene_picking_identity",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/render/semantic_aov.rs",
        &["semantic_aov_preserves_identity_depth_and_normals_across_near_clip"],
    );
    for (path, needle) in [
        (
            "README.md",
            "CPU headless triangles are clipped against both camera depth planes",
        ),
        ("docs/headless-rendering.md", "## CPU camera-depth clipping"),
        (
            "docs/rendering.md",
            "CPU depth-slab clipping happens before perspective division",
        ),
        (
            "CHANGELOG.md",
            "Clip CPU triangles against both camera depth planes",
        ),
        (
            "docs/release-notes/v1.8.0.md",
            "published v1.8.0 CPU rasterizer discarded a whole triangle",
        ),
    ] {
        require_contains(root, findings, RULE, path, &[needle]);
    }
}
