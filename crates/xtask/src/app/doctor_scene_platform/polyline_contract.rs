use crate::app::prelude::*;

pub(crate) fn check_c18_fallible_polyline_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C18-FALLIBLE-POLYLINE";

    require_contains(
        root,
        findings,
        RULE,
        "src/geometry.rs",
        &[
            "#[deprecated(note = \"use GeometryDesc::try_polyline for untrusted or runtime input\")]",
            "pub fn polyline(points: &[Vec3]) -> Self",
            "pub fn try_polyline(points: &[Vec3]) -> Result<Self, GeometryError>",
            "GeometryError::PolylineTooShort",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/m1_geometry_materials.rs",
        &["fallible_polyline_rejects_zero_or_one_point_without_unwinding"],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/scene_recipe_contracts.rs",
        &["scene_recipe_polyline_validation_and_build_reject_zero_or_one_point"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene_host/recipe/authoring/geometry/construction.rs",
        &["GeometryDesc::try_polyline(&points)"],
    );
    for (path, needles) in [
        (
            "docs/api.md",
            &[
                "GeometryDesc::try_polyline",
                "GeometryError::PolylineTooShort",
            ][..],
        ),
        (
            "docs/specs/public-api.md",
            &["GeometryDesc::polyline", "deprecated compatibility wrapper"][..],
        ),
        ("README.md", &["Fallible geometry construction"][..]),
        ("tests/m5_release.rs", &["GeometryDesc::try_polyline"][..]),
        (
            "CHANGELOG.md",
            &["Deprecate the panicking `GeometryDesc::polyline`"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["panicking `GeometryDesc::polyline` wrapper"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
