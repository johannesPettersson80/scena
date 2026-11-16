use crate::app::prelude::*;

pub(crate) fn check_c14_gltf_semantic_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C14-GLTF-SEMANTIC-HANDLING";

    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/meshes/flat_normals.rs",
        &[
            "expand_missing_flat_normals",
            "try_normalize()",
            "const DEGENERATE_TRIANGLE_NORMAL: Vec3 = Vec3::Y;",
            "Affected::record(&mut degenerate_triangles, triangle_index)",
            "copy_optional(",
            "GeometryMorphTarget::new_with_semantics",
            "GeometrySkin::new",
        ],
    );
    forbid_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/meshes.rs",
        &["unwrap_or_else(|| vec![Vec3::new(0.0, 0.0, 1.0)"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/material_extensions.rs",
        &[
            "validate_material_texture_coordinates",
            "validate_texture_info_tex_coords_value",
            "key.ends_with(\"Texture\")",
            "requests texCoord {tex_coord}; scena supports only TEXCOORD_0",
            "validate_texture_transform_tex_coords_value",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/meshes/skin_influences.rs",
        &[
            "JOINTS_1/WEIGHTS_1 require JOINTS_0/WEIGHTS_0",
            ".total_cmp(&left.weight)",
            "then(left.ordinal.cmp(&right.ordinal))",
            "truncated_vertices += 1",
            "selected[3].weight / selected_sum",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/meshes.rs",
        &[
            "read_joints(source, primitive, 1)",
            "read_weights(source, primitive, 1)",
            "AssetLoadWarning::ComputedFlatNormals",
            "AssetLoadWarning::DegenerateTriangleNormals",
            "AssetLoadWarning::SkinInfluencesTruncated",
            "AssetLoadWarning::SkinVerticesUnweighted",
            "reject_skin_sets_above_one",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/nodes.rs",
        &[
            "fn parse_node_meshes",
            "let Some(weights) = node.weights()",
            "weights.len() != target_count",
            "mesh.morph_weights = weights.to_vec()",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/load/warnings.rs",
        &[
            "ComputedFlatNormals",
            "DegenerateTriangleNormals",
            "affected_triangles",
            "first_triangle",
            "SkinInfluencesTruncated",
            "SkinVerticesUnweighted",
            "first_vertex",
            "source_influences",
            "retained_influences",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c04_gltf_deformation_contracts.rs",
        &[
            "missing_normals_are_computed_per_face_for_indexed_and_nonindexed_meshes",
            "missing_normals_on_degenerate_triangles_use_a_reported_fallback_normal",
            "a_primitive_made_only_of_degenerate_triangles_still_loads",
            "computed_flat_normals_are_recorded_in_the_asset_load_report",
            "every_material_texture_slot_rejects_unsupported_texcoord_one_explicitly",
            "secondary_skin_influences_select_the_strongest_four_and_report_degradation",
            "selected_skin_joint_outside_the_bound_skin_fails_predictably",
            "shared_mesh_nodes_apply_distinct_morph_overrides_before_animation",
            "node_morph_override_width_must_match_every_primitive",
            "khronos_simple_skin_uses_computed_normals_through_skinning_and_cpu_render",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c14_gltf_semantic_parity.rs",
        &["khronos_missing_normal_skin_matches_cpu_and_gpu_rendered_output"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/c14_gltf_semantic_parity.rs",
        &[
            "SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS",
            "\\\"release_evidence\\\": false",
        ],
    );
    for (path, needles) in [
        (
            "docs/assets.md",
            &[
                "computed_flat_normals",
                "degenerate_triangle_normals",
                "skin_influences_truncated",
                "skin_vertices_unweighted",
                "TEXCOORD_0",
            ][..],
        ),
        (
            "docs/errors.md",
            &[
                "glTF texture requests `texCoord` 1",
                "Node morph-weight count",
            ][..],
        ),
        (
            "README.md",
            &["secondary skin sets are reduced to the strongest four"][..],
        ),
        (
            "CHANGELOG.md",
            &["Complete common glTF mesh semantics without silent substitution"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["ignored ordinary texture-info UV-set requests"][..],
        ),
        (
            "tests/assets/stable-contracts/asset_load_report.v1.json",
            &[
                "computed_flat_normals",
                "degenerate_triangle_normals",
                "skin_influences_truncated",
                "skin_vertices_unweighted",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
