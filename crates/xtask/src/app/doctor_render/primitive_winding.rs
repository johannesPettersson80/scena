use crate::app::prelude::*;

pub(crate) fn check_c05_primitive_winding_contract(root: &Path, findings: &mut Vec<Finding>) {
    let rule = "C05-OUTWARD-PRIMITIVE-WINDING";
    require_contains(
        root,
        findings,
        rule,
        "src/geometry/primitive_meshes.rs",
        &[
            "triangle_normal(p0, tip, p1)",
            "indices.extend_from_slice(&[base, base + 2, base + 1]);",
            "triangle_normal(face[0], face[2], face[1])",
            "indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);",
        ],
    );
    forbid_contains(
        root,
        findings,
        rule,
        "src/geometry/primitive_meshes.rs",
        &[
            "triangle_normal(p0, p1, tip)",
            "triangle_normal(face[0], face[1], face[2])",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/geometry/primitive_meshes/tests.rs",
        &[
            "cone_faces_are_outward_from_computed_geometry_truth",
            "wedge_faces_are_outward_from_computed_geometry_truth",
            "computed_face_normal",
            "cone_and_wedge_dimensions_do_not_encode_transform_sign_or_scale",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "tests/placeholder_regression.rs",
        &[
            "cone_and_wedge_default_culling_show_the_near_exterior",
            "single-sided default culling must render the same near exterior",
        ],
    );
}
