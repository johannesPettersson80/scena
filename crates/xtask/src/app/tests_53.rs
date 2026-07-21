use crate::app::prelude::*;

#[test]
fn c16_doctor_rejects_scale_by_replacement_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c16-transform-scale");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene/math.rs",
        "tests/c16_transform_scale_semantics.rs",
        "docs/api.md",
        "docs/guides/migrating-from-threejs.md",
        "docs/specs/public-api.md",
        "README.md",
        "examples/layers_visibility.rs",
        "tests/m5_release.rs",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C16 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C16 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c16_transform_scale_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let policy = fixture_root.join("src/scene/math.rs");
    let source = fs::read_to_string(&policy).expect("C16 transform source reads");
    let mutated = source.replace("self.scale.x * scale", "scale");
    assert_ne!(source, mutated, "C16 mutation must remove X composition");
    fs::write(policy, mutated).expect("C16 transform mutation writes");
    findings.clear();
    check_c16_transform_scale_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C16-TRANSFORM-SCALE-SEMANTICS"
                && finding.message.contains("self.scale.x * scale")
        }),
        "restoring setter behavior must fail doctor: {findings:?}",
    );
}
