use crate::app::prelude::*;

#[test]
fn c18_doctor_rejects_undeprecated_panicking_polyline_wrapper() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c18-fallible-polyline");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/geometry.rs",
        "tests/m1_geometry_materials.rs",
        "tests/scene_recipe_contracts.rs",
        "src/scene_host/recipe/authoring/geometry/construction.rs",
        "docs/api.md",
        "docs/specs/public-api.md",
        "README.md",
        "tests/m5_release.rs",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C18 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C18 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c18_fallible_polyline_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let geometry = fixture_root.join("src/geometry.rs");
    let source = fs::read_to_string(&geometry).expect("C18 geometry source reads");
    let deprecated =
        "#[deprecated(note = \"use GeometryDesc::try_polyline for untrusted or runtime input\")]\n";
    let mutated = source.replacen(deprecated, "", 1);
    assert_ne!(source, mutated, "C18 mutation must remove deprecation");
    fs::write(geometry, mutated).expect("C18 geometry mutation writes");
    findings.clear();
    check_c18_fallible_polyline_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C18-FALLIBLE-POLYLINE" && finding.message.contains("#[deprecated(note")
        }),
        "restoring an endorsed panicking wrapper must fail doctor: {findings:?}",
    );
}
