use crate::app::prelude::*;

#[test]
fn c09_doctor_rejects_reload_that_uses_the_immutable_texture_policy() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c09-transactional-reload");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/assets/texture.rs",
        "src/assets/texture_reload.rs",
        "src/assets/scene_loading.rs",
        "src/assets/load.rs",
        "src/lib.rs",
        "src/assets/gltf.rs",
        "src/assets/gltf/textures.rs",
        "tests/round_d_asset_hot_reload.rs",
        "docs/assets.md",
        "docs/api.md",
        "docs/guides/easy-scene-setup.md",
        "docs/errors.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C09 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C09 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c09_transactional_reload_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let loading = fixture_root.join("src/assets/scene_loading.rs");
    let source = fs::read_to_string(&loading).expect("C09 scene-loading source reads");
    let mutated = source.replacen(
        "TextureCacheUpdatePolicy::ReplaceChangedSource,",
        "TextureCacheUpdatePolicy::Immutable,",
        1,
    );
    assert_ne!(
        source, mutated,
        "C09 mutation must disable source replacement for explicit reload"
    );
    fs::write(loading, mutated).expect("C09 scene-loading mutation writes");
    findings.clear();
    check_c09_transactional_reload_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C09-TRANSACTIONAL-ASSET-RELOAD"
                && finding.message.contains("ReplaceChangedSource")
        }),
        "restoring immutable reload behavior must fail doctor: {findings:?}"
    );
}
