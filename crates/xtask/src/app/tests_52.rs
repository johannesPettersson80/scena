use crate::app::prelude::*;

#[test]
fn c15_doctor_rejects_removed_paired_basis_validation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c15-marker-transforms");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/assets/gltf/transform.rs",
        "src/assets/gltf/anchors.rs",
        "src/assets/gltf/connectors.rs",
        "src/assets/gltf/nodes.rs",
        "tests/c15_gltf_marker_transform_contracts.rs",
        "docs/guides/authoring-gltf-anchors-connectors.md",
        "docs/assets.md",
        "docs/errors.md",
        "README.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C15 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C15 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c15_marker_transform_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let policy = fixture_root.join("src/assets/gltf/transform.rs");
    let source = fs::read_to_string(&policy).expect("C15 transform policy reads");
    let mutated = source.replace("if has_forward != has_up", "if false");
    assert_ne!(
        source, mutated,
        "C15 mutation must disable paired-basis validation"
    );
    fs::write(policy, mutated).expect("C15 transform policy mutation writes");
    findings.clear();
    check_c15_marker_transform_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C15-GLTF-MARKER-TRANSFORMS"
                && finding.message.contains("has_forward != has_up")
        }),
        "removing paired-basis validation must fail doctor: {findings:?}",
    );
}
