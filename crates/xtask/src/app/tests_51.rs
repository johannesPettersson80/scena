use crate::app::prelude::*;

#[test]
fn c14_doctor_rejects_removed_ordinary_texture_coordinate_validation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c14-gltf-semantics");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/assets/gltf/meshes/flat_normals.rs",
        "src/assets/gltf/meshes/skin_influences.rs",
        "src/assets/gltf/meshes.rs",
        "src/assets/gltf/material_extensions.rs",
        "src/assets/gltf.rs",
        "src/assets/gltf/nodes.rs",
        "src/assets/load/warnings.rs",
        "tests/c04_gltf_deformation_contracts.rs",
        "tests/c14_gltf_semantic_parity.rs",
        "docs/assets.md",
        "docs/errors.md",
        "README.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
        "tests/assets/stable-contracts/asset_load_report.v1.json",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C14 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C14 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c14_gltf_semantic_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let policy = fixture_root.join("src/assets/gltf/material_extensions.rs");
    let source = fs::read_to_string(&policy).expect("C14 UV policy reads");
    let mutated = source.replace("key.ends_with(\"Texture\")", "key.is_empty()");
    assert_ne!(source, mutated, "C14 mutation must disable slot discovery");
    fs::write(policy, mutated).expect("C14 UV policy mutation writes");
    findings.clear();
    check_c14_gltf_semantic_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C14-GLTF-SEMANTIC-HANDLING"
                && finding.message.contains("key.ends_with")
        }),
        "removing ordinary texture-coordinate validation must fail doctor: {findings:?}",
    );
}
