use crate::app::prelude::*;

#[test]
fn c19_doctor_rejects_a_wrapped_cylinder_side_seam() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c19-primitive-uv-seams");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/geometry/primitive_meshes.rs",
        "src/geometry/primitive_meshes/tests.rs",
        "src/render/prepare/tangents.rs",
        "src/scene_host/recipe/authoring/geometry/projection.rs",
        "tests/c19_primitive_uv_seams.rs",
        "README.md",
        "docs/rendering.md",
        "docs/api.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C19 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C19 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c19_primitive_uv_seam_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let geometry = fixture_root.join("src/geometry/primitive_meshes.rs");
    let source = fs::read_to_string(&geometry).expect("C19 primitive source reads");
    let mutated = source.replacen(
        "let side_row = segments + 1;",
        "let side_row = segments;",
        1,
    );
    assert_ne!(source, mutated, "C19 mutation must restore a wrapped seam");
    fs::write(geometry, mutated).expect("C19 geometry mutation writes");
    findings.clear();
    check_c19_primitive_uv_seam_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C19-PRIMITIVE-UV-SEAMS"
                && finding.message.contains("let side_row = segments + 1;")
        }),
        "restoring the wrapped cylinder seam must fail doctor: {findings:?}",
    );
}
