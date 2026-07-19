use crate::app::prelude::*;

pub(crate) fn check_asset_load_test_evidence(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_native_fetcher_cache_dedup_reload_retain_and_external_buffers_are_explicit",
            "AssetLoadProgress::ExternalBufferFetched",
            "tests/assets/gltf/khronos/TextureTransformTest/TextureTransformTest.bin",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &["external.fetched_bytes() > first.fetched_bytes()"],
    );
}
