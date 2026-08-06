use serde::Deserialize;

use super::{
    PhotographicMaterialCatalogEntryV1, PhotographicMaterialCategoryV1, maps_for_category,
};
use crate::assets::PhotographicSurfaceKind;

#[derive(Debug, Deserialize)]
struct ProviderSnapshot {
    schema: String,
    provider: String,
    entries: Vec<ProviderSnapshotEntry>,
}

#[derive(Debug, Deserialize)]
struct ProviderSnapshotEntry {
    provider_asset_id: String,
    label: String,
    creation_method: String,
    tags: Vec<String>,
    archive_uri: String,
}

pub(super) fn entries() -> Vec<PhotographicMaterialCatalogEntryV1> {
    let snapshot: ProviderSnapshot = serde_json::from_str(include_str!("catalog_snapshot.json"))
        .expect("checked-in ambientCG material catalog snapshot must be valid JSON");
    assert_eq!(
        snapshot.schema, "scena.material_library_provider_snapshot.v1",
        "checked-in material catalog snapshot schema changed without loader support"
    );
    assert_eq!(
        snapshot.provider, "ambientcg",
        "checked-in material catalog snapshot uses an unsupported provider"
    );
    snapshot.entries.into_iter().map(snapshot_entry).collect()
}

fn snapshot_entry(source: ProviderSnapshotEntry) -> PhotographicMaterialCatalogEntryV1 {
    let category = provider_category(&source.provider_asset_id);
    let surface_kind = provider_surface_kind(category, &source.tags);
    let recommended_tile_size_m = match category {
        PhotographicMaterialCategoryV1::Metal
            if source.provider_asset_id.starts_with("MetalPlates")
                || source.provider_asset_id.starts_with("MetalWalkway") =>
        {
            1.0
        }
        PhotographicMaterialCategoryV1::Metal => 0.25,
        PhotographicMaterialCategoryV1::Plastic => 0.20,
        PhotographicMaterialCategoryV1::Fabric => 0.40,
        PhotographicMaterialCategoryV1::Leather => 0.45,
        PhotographicMaterialCategoryV1::Rubber => 0.75,
    };
    PhotographicMaterialCatalogEntryV1 {
        id: format!(
            "ambientcg-{}",
            source.provider_asset_id.to_ascii_lowercase()
        ),
        label: source.label,
        category,
        surface_kind,
        provider: "ambientcg".to_string(),
        source_page: format!("https://ambientcg.com/a/{}", source.provider_asset_id),
        provider_asset_id: source.provider_asset_id,
        creation_method: source.creation_method,
        archive_uri: source.archive_uri,
        license: "CC0-1.0".to_string(),
        recommended_tile_size_m,
        maps: maps_for_category(category).to_vec(),
        tags: source.tags,
    }
}

fn provider_category(provider_asset_id: &str) -> PhotographicMaterialCategoryV1 {
    if provider_asset_id.starts_with("Metal") {
        PhotographicMaterialCategoryV1::Metal
    } else if provider_asset_id.starts_with("Plastic") {
        PhotographicMaterialCategoryV1::Plastic
    } else if provider_asset_id.starts_with("Fabric") {
        PhotographicMaterialCategoryV1::Fabric
    } else if provider_asset_id.starts_with("Leather") {
        PhotographicMaterialCategoryV1::Leather
    } else if provider_asset_id.starts_with("Rubber") {
        PhotographicMaterialCategoryV1::Rubber
    } else {
        panic!("unsupported material family in provider snapshot: {provider_asset_id}")
    }
}

fn provider_surface_kind(
    category: PhotographicMaterialCategoryV1,
    tags: &[String],
) -> PhotographicSurfaceKind {
    match category {
        PhotographicMaterialCategoryV1::Metal if has_tag(tags, "brushed") => {
            PhotographicSurfaceKind::BrushedMetal
        }
        PhotographicMaterialCategoryV1::Metal
            if has_tag(tags, "polished") || has_tag(tags, "chrome") || has_tag(tags, "shiny") =>
        {
            PhotographicSurfaceKind::PolishedMetal
        }
        PhotographicMaterialCategoryV1::Metal
            if has_tag(tags, "cast")
                || has_tag(tags, "corroded")
                || has_tag(tags, "rust")
                || has_tag(tags, "rusted")
                || has_tag(tags, "rusty") =>
        {
            PhotographicSurfaceKind::CastMetal
        }
        PhotographicMaterialCategoryV1::Metal => PhotographicSurfaceKind::SatinMetal,
        PhotographicMaterialCategoryV1::Plastic => PhotographicSurfaceKind::MoldedPlastic,
        PhotographicMaterialCategoryV1::Fabric | PhotographicMaterialCategoryV1::Leather => {
            PhotographicSurfaceKind::Fabric
        }
        PhotographicMaterialCategoryV1::Rubber => PhotographicSurfaceKind::Rubber,
    }
}

fn has_tag(tags: &[String], expected: &str) -> bool {
    tags.iter().any(|tag| tag.eq_ignore_ascii_case(expected))
}
