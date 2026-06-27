use crate::app::prelude::*;

pub(super) fn check_asset_load_report_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/scene_loading.rs",
        &["load_scene_with_progress"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/load.rs",
        &[
            "pub struct AssetLoadControl",
            "pub struct AssetLoadOptions",
            "pub struct AssetLoadReport",
            "pub enum AssetLoadProgress",
            "strict_textures",
            "with_strict_textures",
            "progress_events",
            "emit_progress",
            "AssetError::Cancelled",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/load/fallback.rs",
        &[
            "pub struct AssetMaterialFallback",
            "pub enum AssetMaterialFallbackKind",
            "pub struct AssetMaterialFallbackV1",
            "material_index",
            "texture_basisu_fallback",
            "missing_texture_fallback",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/scene_loading.rs",
        &[
            "load_scene_with_report",
            "load_scene_with_options",
            "load_scene_with_report_options",
            "load_scene_controlled",
        ],
    );
}
