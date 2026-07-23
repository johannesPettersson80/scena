use crate::app::prelude::*;

#[test]
fn c10_doctor_rejects_cache_lookup_that_discards_the_active_policy() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c10-semantic-scene-cache");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/assets.rs",
        "src/assets/load.rs",
        "src/assets/load/options.rs",
        "src/assets/scene_cache.rs",
        "src/assets/scene_loading.rs",
        "tests/m8_assets_materials_ecosystem.rs",
        "docs/assets.md",
        "docs/schema-contracts.md",
        "docs/api.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C10 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C10 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c10_cache_policy_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let loading = fixture_root.join("src/assets/scene_loading.rs");
    let source = fs::read_to_string(&loading).expect("C10 scene-loading source reads");
    let mutated = source.replace(
        "storage.cached_scene(&path, options.clone())",
        "storage.cached_scene(&path, AssetLoadOptions::default())",
    );
    assert_ne!(
        source, mutated,
        "C10 mutation must discard the caller's active cache policy"
    );
    fs::write(loading, mutated).expect("C10 scene-loading mutation writes");
    findings.clear();
    check_c10_cache_policy_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C10-SEMANTIC-SCENE-CACHE-POLICY"
                && finding
                    .message
                    .contains("storage.cached_scene(&path, options.clone())")
        }),
        "discarding the active cache policy must fail doctor: {findings:?}"
    );
}
