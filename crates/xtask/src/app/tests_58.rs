use crate::app::prelude::*;

#[test]
fn c21_doctor_rejects_cad_inspection_without_oriented_studio_rig() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c21-cad-lighting");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena/recipe/cad_inspection/view.rs",
        "tests/scena_cli_recipe.rs",
        "docs/guides/llm-app-builder.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C21 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C21 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c21_cad_inspection_lighting(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let view = fixture_root.join("src/bin/scena/recipe/cad_inspection/view.rs");
    let source = fs::read_to_string(&view).expect("C21 view source reads");
    let mutated = source.replacen("\"kind\": \"studio_rig\"", "\"kind\": \"directional\"", 1);
    assert_ne!(source, mutated, "C21 mutation must remove studio rig kind");
    fs::write(view, mutated).expect("C21 view mutation writes");
    findings.clear();
    check_c21_cad_inspection_lighting(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C21-CAD-INSPECTION-LIGHTING"
                && finding.message.contains("\"kind\": \"studio_rig\"")
        }),
        "removing the oriented rig must fail doctor: {findings:?}",
    );
}
