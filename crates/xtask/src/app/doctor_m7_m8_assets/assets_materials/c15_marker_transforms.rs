use crate::app::prelude::*;

pub(crate) fn check_c15_marker_transform_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C15-GLTF-MARKER-TRANSFORMS";

    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/transform.rs",
        &[
            "pub(super) fn parse_marker_transform",
            "if has_forward != has_up",
            "must not be parallel",
            "must contain exactly 16 finite numbers",
            "must be an affine transform",
            "must be decomposable as translation",
            "scale.abs().min_element() <= f32::EPSILON",
            "quaternion must be normalized",
            "fn finite_f32",
        ],
    );
    for (path, collection) in [
        ("src/assets/gltf/anchors.rs", "anchors"),
        ("src/assets/gltf/connectors.rs", "connectors"),
    ] {
        require_contains(
            root,
            findings,
            RULE,
            path,
            &[
                "path: &AssetPath",
                "Result<Vec<SceneAsset",
                "parse_marker_transform",
                "AssetError::Parse",
                &format!("extras.scena.{collection}"),
                "invalid_reason: None",
            ],
        );
    }
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/nodes.rs",
        &[
            "anchors: parse_node_anchors(path, &node)?",
            "connectors: parse_node_connectors(path, &node)?",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c15_gltf_marker_transform_contracts.rs",
        &[
            "anchor_and_connector_basis_vectors_fail_closed",
            "anchor_and_connector_trs_fail_closed",
            "anchor_and_connector_matrices_fail_closed",
            "valid_marker_basis_loads_without_changing_authored_units",
            "valid_marker_matrix_preserves_translation_rotation_and_scale",
        ],
    );
    for (path, needles) in [
        (
            "docs/guides/authoring-gltf-anchors-connectors.md",
            &[
                "paired, finite, nonzero, nonparallel `forward` and `up`",
                "Invalid marker transforms abort glTF loading",
            ][..],
        ),
        (
            "docs/assets.md",
            &[
                "shared extras transform grammar",
                "Invalid metadata fails loading",
            ][..],
        ),
        (
            "docs/errors.md",
            &["Invalid glTF anchor/connector transform extras"][..],
        ),
        (
            "README.md",
            &["anchor or connector TRS, basis, and matrix extras"][..],
        ),
        (
            "CHANGELOG.md",
            &[
                "Validate glTF anchor and connector extras",
                "contract. Nonfinite/zero/parallel basis vectors",
            ][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["invalid forward/up basis or malformed matrix"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
