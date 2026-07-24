use crate::app::prelude::*;

pub(crate) fn check_c10_cache_policy_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C10-SEMANTIC-SCENE-CACHE-POLICY";

    require_contains(
        root,
        findings,
        RULE,
        "src/assets.rs",
        &[
            "mod scene_cache;",
            "BTreeMap<scene_cache::SceneCacheKey, SceneAsset>",
            "BTreeMap<scene_cache::SceneCacheKey, load::AssetLoadTelemetry>",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/load/options.rs",
        &[
            "strict_textures: bool",
            "strict_external_resources: bool",
            "fetch_byte_limit: Option<usize>",
            "PartialOrd, Ord, Hash",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/scene_cache.rs",
        &[
            "struct SceneCacheKey",
            "options: AssetLoadOptions",
            "fn satisfies(&self, requested: &AssetLoadOptions)",
            "if requested.strict_textures()",
            "AssetLoadWarning::ExternalImageMissing",
            "if requested.strict_external_resources()",
            "AssetLoadWarning::ExternalBufferMissing",
            "self.fetched_bytes <= limit",
            ".satisfies(&requested)",
            "fn replace_cached_scene",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/scene_loading.rs",
        &[
            "storage.cached_scene(&path, options.clone())",
            "storage.cache_scene(",
            "storage.replace_cached_scene(",
            "requested_options: options",
            "cache_entry_options",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/load.rs",
        &[
            "pub(super) requested_options: AssetLoadOptions",
            "pub(super) cache_entry_options: AssetLoadOptions",
            "pub fn options(&self) -> AssetLoadOptions",
            "pub fn cache_entry_options(&self) -> AssetLoadOptions",
            "pub requested_options: AssetLoadOptions",
            "pub cache_entry_options: AssetLoadOptions",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "scene_cache_lenient_then_strict_does_not_bypass_texture_policy",
            "scene_cache_lenient_then_strict_does_not_bypass_external_buffer_policy",
            "scene_cache_strict_then_lenient_keeps_policy_specific_evidence",
            "scene_cache_unlimited_then_bounded_does_not_bypass_fetch_limit",
        ],
    );
    for (path, needle) in [
        (
            "docs/assets.md",
            "The scene cache never keys on a path alone",
        ),
        (
            "docs/schema-contracts.md",
            "cache_entry_options` records the",
        ),
        ("docs/api.md", "AssetLoadReport::cache_entry_options"),
        (
            "CHANGELOG.md",
            "Key scene-cache reuse by semantic load policy",
        ),
        (
            "docs/release-notes/v1.8.0.md",
            "scene cache was keyed only by source path",
        ),
    ] {
        require_contains(root, findings, RULE, path, &[needle]);
    }
}
