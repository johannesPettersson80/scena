use crate::app::prelude::*;

#[test]
fn c08_doctor_rejects_unconverted_imported_rotation_animation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c08-animation-basis");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene/import/options.rs",
        "src/animation.rs",
        "src/scene/import/animation_bindings.rs",
        "tests/assets/gltf/z_up_animated_rotation.gltf",
        "tests/m3b_gltf_animation.rs",
        "tests/dynamic_transform_parity.rs",
        "docs/assets.md",
        "docs/guides/units-axes-handedness.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C08 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C08 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c08_animation_basis_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let bindings = fixture_root.join("src/scene/import/animation_bindings.rs");
    let source = fs::read_to_string(&bindings).expect("C08 animation bindings read");
    let mutated = source.replace(
        "options.convert_animation_rotation(interpolation, index, value)",
        "value",
    );
    assert_ne!(
        source, mutated,
        "C08 mutation must bypass imported rotation conversion"
    );
    fs::write(bindings, mutated).expect("C08 animation bindings mutation writes");
    findings.clear();
    check_c08_animation_basis_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C08-Z-UP-ANIMATION-BASIS"
                && finding.message.contains("convert_animation_rotation")
        }),
        "bypassing imported quaternion conversion must fail doctor: {findings:?}"
    );
}
