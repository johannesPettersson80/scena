use crate::app::prelude::*;

pub(crate) fn check_c09_transactional_reload_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C09-TRANSACTIONAL-ASSET-RELOAD";

    require_contains(
        root,
        findings,
        RULE,
        "src/assets/texture.rs",
        &[
            "mod texture_reload;",
            "pub(crate) use texture_reload::TextureCacheUpdatePolicy;",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/texture_reload.rs",
        &[
            "enum TextureCacheUpdatePolicy",
            "Immutable",
            "ReplaceChangedSource",
            "fn replace_changed_source_bytes",
            "self.provenance == incoming_provenance",
        ],
    );
    let loading_path = "src/assets/scene_loading.rs";
    if let Ok(source) = fs::read_to_string(root.join(loading_path)) {
        let replacement_boundaries = source
            .matches("TextureCacheUpdatePolicy::ReplaceChangedSource")
            .count();
        if replacement_boundaries != 2 {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{loading_path} must select ReplaceChangedSource for the fetched and retained-source reload paths; found {replacement_boundaries} boundaries"
                ),
            ));
        }
    }
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/scene_loading.rs",
        &[
            "pub async fn reload_scene_with_report",
            "fn reload_scene_report_inner",
            ".with_strict_textures(true)",
            ".with_strict_external_resources(true)",
            "TextureCacheUpdatePolicy::ReplaceChangedSource",
            "GltfImportPolicy {",
            "storage.replace_cached_scene(",
        ],
    );
    // The reload texture policy is scoped to the import transaction. Assigning
    // it on the shared storage would let an interrupted reload leave it behind.
    forbid_contains(
        root,
        findings,
        RULE,
        "src/assets/scene_loading.rs",
        &["storage.texture_cache_update_policy ="],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/load.rs",
        &[
            "pub struct AssetReloadError",
            "previous_asset_preserved",
            "impl std::error::Error for AssetReloadError",
        ],
    );
    require_contains(root, findings, RULE, "src/lib.rs", &["AssetReloadError"]);
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf.rs",
        &[
            "let mut transaction = storage.clone();",
            "transaction.texture_cache_update_policy = policy.texture_cache_update_policy;",
            "transaction.texture_cache_update_policy = storage.texture_cache_update_policy;",
            "*storage = transaction;",
        ],
    );
    // A panic under the storage lock must not disable the store for later
    // callers; the transaction above is what makes recovering the guard sound.
    require_contains(
        root,
        findings,
        RULE,
        "src/assets.rs",
        &["self.storage.clear_poison();", "poisoned.into_inner()"],
    );
    forbid_contains(
        root,
        findings,
        RULE,
        "src/assets.rs",
        &["asset storage mutex should not be poisoned"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/textures.rs",
        &[
            "TextureCacheUpdatePolicy::Immutable",
            "TextureCacheUpdatePolicy::ReplaceChangedSource",
            ".replace_changed_source_bytes",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/round_d_asset_hot_reload.rs",
        &[
            "reload_scene_replaces_changed_external_texture_at_the_same_path",
            "reload_scene_updates_every_shared_texture_consumer_once",
            "ordinary_load_keeps_texture_provenance_immutable_until_explicit_reload",
            "reload_scene_replaces_external_buffer_and_keeps_last_complete_version_on_failure",
            "reload_scene_uses_content_addressed_identity_for_changed_embedded_texture",
        ],
    );
    for (path, needle) in [
        ("docs/assets.md", "Transactional reload"),
        ("docs/api.md", "AssetReloadError"),
        (
            "docs/guides/easy-scene-setup.md",
            "Explicit reload is transactional",
        ),
        (
            "docs/errors.md",
            "explicit reload never publishes a partial dependency set",
        ),
        ("CHANGELOG.md", "cache-identity collision"),
        ("docs/release-notes/v1.8.0.md", "cache-identity collision"),
    ] {
        require_contains(root, findings, RULE, path, &[needle]);
    }
}
