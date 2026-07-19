use crate::app::prelude::*;

pub(crate) fn check_c12_deformed_picking_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "SCENE-C12";
    let required: &[(&str, &[&str])] = &[
        (
            "src/geometry.rs",
            &["mod deformation;", "MissingSkinMatrices"],
        ),
        (
            "src/geometry/deformation.rs",
            &[
                "pub(crate) fn deformed_vertices",
                "let morphed = morph_weights",
                "let base_vertices = morphed.as_deref()",
                ".skinned_vertices(base_vertices, matrices)",
                "GeometryError::MissingSkinMatrices",
                "canonical_deformation_applies_morph_before_skin",
            ],
        ),
        (
            "src/render/prepare/primitives.rs",
            &[".deformed_vertices(deformation.morph_weights, deformation.skin_matrices)"],
        ),
        (
            "src/render/prepare/shadows.rs",
            &[".deformed_vertices(deformation.morph_weights, deformation.skin_matrices)"],
        ),
        (
            "src/picking.rs",
            &[
                ".deformed_vertices(scene.morph_weights(node), skin_matrices.as_deref())",
                ".deformed_vertices(None, None)",
                "invalid_skin_binding(&geometry, skin_matrices.as_deref())",
                "let (Some(a), Some(b), Some(c))",
                "vertices.get(indices[0] as usize)",
                "pub struct PickingMetrics",
                "ray_triangle_intersection_tests",
                "deformed_vertex_bytes_materialized",
                "World-space distance from the camera-ray origin",
                "negative scale can therefore reverse it",
            ],
        ),
        (
            "src/scene/picking.rs",
            &[
                "Morph targets are evaluated before skinning",
                "LookupError::InvalidSkinBinding",
                "pub fn pick_with_assets_profiled",
            ],
        ),
        (
            "docs/api.md",
            &[
                "### Picking result semantics",
                "morph targets are evaluated first, skinning second",
                "singular transform is permitted as scene",
                "collapses to zero area is not hittable",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c12_deformed_picking.rs",
        &[
            "morph_picking_uses_the_rendered_vertex_pose",
            "skin_picking_uses_the_rendered_joint_pose",
            "instance_picking_composes_distinct_root_and_instance_transforms",
            "picking_reports_world_distance_and_winding_normal_for_scaled_geometry",
            "profiled_picking_reports_intersection_and_deformation_work",
        ],
    );
}
