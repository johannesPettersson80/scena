use serde::{Deserialize, Serialize};

use super::PhotographicSurfaceKind;

pub const MATERIAL_LIBRARY_CATALOG_SCHEMA_V1: &str = "scena.material_library_catalog.v1";
pub const MATERIAL_LIBRARY_CATALOG_SCHEMA_V2: &str = "scena.material_library_catalog.v2";
pub const PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1: &str = "scena.photographic_material_pack.v1";
pub const PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2: &str = "scena.photographic_material_pack.v2";
pub const PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES: usize = 256 * 1024 * 1024;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
pub enum PhotographicMaterialResolutionV1 {
    #[serde(rename = "1k")]
    OneK,
    #[serde(rename = "2k")]
    TwoK,
    #[serde(rename = "4k")]
    FourK,
}

impl PhotographicMaterialResolutionV1 {
    pub const ALL: [Self; 3] = [Self::OneK, Self::TwoK, Self::FourK];

    pub const fn dimension_px(self) -> u32 {
        match self {
            Self::OneK => 1_024,
            Self::TwoK => 2_048,
            Self::FourK => 4_096,
        }
    }

    pub const fn scale_from_one_k(self) -> f64 {
        match self {
            Self::OneK => 1.0,
            Self::TwoK => 2.0,
            Self::FourK => 4.0,
        }
    }

    pub const fn ambientcg_token(self) -> &'static str {
        match self {
            Self::OneK => "1K",
            Self::TwoK => "2K",
            Self::FourK => "4K",
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OneK => "1k",
            Self::TwoK => "2k",
            Self::FourK => "4k",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "1k" => Some(Self::OneK),
            "2k" => Some(Self::TwoK),
            "4k" => Some(Self::FourK),
            _ => None,
        }
    }
}

pub fn select_photographic_material_resolution(
    texels_per_output_pixel_at_one_k: f64,
) -> Option<PhotographicMaterialResolutionV1> {
    if !texels_per_output_pixel_at_one_k.is_finite() || texels_per_output_pixel_at_one_k <= 0.0 {
        return None;
    }
    PhotographicMaterialResolutionV1::ALL
        .into_iter()
        .find(|resolution| texels_per_output_pixel_at_one_k * resolution.scale_from_one_k() >= 1.0)
        .or(Some(PhotographicMaterialResolutionV1::FourK))
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicMaterialCategoryV1 {
    Metal,
    Plastic,
    Fabric,
    Leather,
    Rubber,
}

impl PhotographicMaterialCategoryV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Metal => "metal",
            Self::Plastic => "plastic",
            Self::Fabric => "fabric",
            Self::Leather => "leather",
            Self::Rubber => "rubber",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "metal" => Some(Self::Metal),
            "plastic" => Some(Self::Plastic),
            "fabric" => Some(Self::Fabric),
            "leather" => Some(Self::Leather),
            "rubber" => Some(Self::Rubber),
            _ => None,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicMaterialMapKindV1 {
    BaseColor,
    NormalGl,
    Roughness,
    Metalness,
    Occlusion,
    Displacement,
}

impl PhotographicMaterialMapKindV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BaseColor => "base_color",
            Self::NormalGl => "normal_gl",
            Self::Roughness => "roughness",
            Self::Metalness => "metalness",
            Self::Occlusion => "occlusion",
            Self::Displacement => "displacement",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialCatalogEntryV1 {
    pub id: String,
    pub label: String,
    pub category: PhotographicMaterialCategoryV1,
    pub surface_kind: PhotographicSurfaceKind,
    pub provider: String,
    pub provider_asset_id: String,
    pub creation_method: String,
    pub source_page: String,
    pub archive_uri: String,
    pub license: String,
    pub recommended_tile_size_m: f32,
    pub maps: Vec<PhotographicMaterialMapKindV1>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialCatalogV1 {
    pub schema: String,
    pub scope: String,
    pub entries: Vec<PhotographicMaterialCatalogEntryV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialArchiveVariantV2 {
    pub resolution: PhotographicMaterialResolutionV1,
    pub archive_uri: String,
    pub source_texture_dimension_px: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialCatalogEntryV2 {
    pub id: String,
    pub label: String,
    pub category: PhotographicMaterialCategoryV1,
    pub surface_kind: PhotographicSurfaceKind,
    pub provider: String,
    pub provider_asset_id: String,
    pub creation_method: String,
    pub source_page: String,
    pub archive_variants: Vec<PhotographicMaterialArchiveVariantV2>,
    pub license: String,
    pub recommended_tile_size_m: f32,
    pub maps: Vec<PhotographicMaterialMapKindV1>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialCatalogV2 {
    pub schema: String,
    pub scope: String,
    pub entries: Vec<PhotographicMaterialCatalogEntryV2>,
}

impl PhotographicMaterialCatalogEntryV2 {
    pub fn archive_variant(
        &self,
        resolution: PhotographicMaterialResolutionV1,
    ) -> Option<&PhotographicMaterialArchiveVariantV2> {
        self.archive_variants
            .iter()
            .find(|variant| variant.resolution == resolution)
    }

    pub fn for_resolution(
        &self,
        resolution: PhotographicMaterialResolutionV1,
    ) -> Option<PhotographicMaterialCatalogEntryV1> {
        Some(PhotographicMaterialCatalogEntryV1 {
            id: self.id.clone(),
            label: self.label.clone(),
            category: self.category,
            surface_kind: self.surface_kind,
            provider: self.provider.clone(),
            provider_asset_id: self.provider_asset_id.clone(),
            creation_method: self.creation_method.clone(),
            source_page: self.source_page.clone(),
            archive_uri: self.archive_variant(resolution)?.archive_uri.clone(),
            license: self.license.clone(),
            recommended_tile_size_m: self.recommended_tile_size_m,
            maps: self.maps.clone(),
            tags: self.tags.clone(),
        })
    }
}

mod pack;
#[cfg(feature = "scene-host")]
pub(crate) use pack::PhotographicMaterialPackBinding;
pub use pack::{
    PhotographicMaterialPackAssets, PhotographicMaterialPackMapRoleV1,
    PhotographicMaterialPackMapV1, PhotographicMaterialPackSourceV1, PhotographicMaterialPackV1,
    PhotographicMaterialPackV2,
};

pub fn photographic_material_catalog_v1() -> PhotographicMaterialCatalogV1 {
    let mut entries = provider_snapshot::entries();
    for seed in catalog_entries::CATALOG {
        let curated = seed.to_catalog_entry();
        let entry = entries
            .iter_mut()
            .find(|entry| entry.provider_asset_id == curated.provider_asset_id)
            .unwrap_or_else(|| {
                panic!(
                    "curated material {} is absent from the audited provider snapshot",
                    curated.provider_asset_id
                )
            });
        entry.label = curated.label;
        entry.category = curated.category;
        entry.surface_kind = curated.surface_kind;
        entry.recommended_tile_size_m = curated.recommended_tile_size_m;
        entry.tags = curated.tags;
    }
    PhotographicMaterialCatalogV1 {
        schema: MATERIAL_LIBRARY_CATALOG_SCHEMA_V1.to_string(),
        scope: "audited_product_and_industrial_surfaces".to_string(),
        entries,
    }
}

pub fn photographic_material_catalog_v2() -> PhotographicMaterialCatalogV2 {
    let catalog = photographic_material_catalog_v1();
    PhotographicMaterialCatalogV2 {
        schema: MATERIAL_LIBRARY_CATALOG_SCHEMA_V2.to_owned(),
        scope: catalog.scope,
        entries: catalog
            .entries
            .into_iter()
            .map(|entry| {
                let archive_variants = PhotographicMaterialResolutionV1::ALL
                    .into_iter()
                    .map(|resolution| PhotographicMaterialArchiveVariantV2 {
                        resolution,
                        archive_uri: format!(
                            "https://ambientcg.com/get?file={}_{}-JPG.zip",
                            entry.provider_asset_id,
                            resolution.ambientcg_token()
                        ),
                        source_texture_dimension_px: resolution.dimension_px(),
                    })
                    .collect();
                PhotographicMaterialCatalogEntryV2 {
                    id: entry.id,
                    label: entry.label,
                    category: entry.category,
                    surface_kind: entry.surface_kind,
                    provider: entry.provider,
                    provider_asset_id: entry.provider_asset_id,
                    creation_method: entry.creation_method,
                    source_page: entry.source_page,
                    archive_variants,
                    license: entry.license,
                    recommended_tile_size_m: entry.recommended_tile_size_m,
                    maps: entry.maps,
                    tags: entry.tags,
                }
            })
            .collect(),
    }
}

mod catalog_entries;
mod loader;
mod provider_snapshot;

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
mod compiler;

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
pub use compiler::compile_photographic_material_archive_at_resolution;
#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
pub use compiler::{PhotographicMaterialPackError, compile_photographic_material_archive};

#[derive(Debug, Clone, Copy)]
struct CatalogSeed {
    provider_asset_id: &'static str,
    label: &'static str,
    category: PhotographicMaterialCategoryV1,
    surface_kind: PhotographicSurfaceKind,
    creation_method: &'static str,
    recommended_tile_size_m: f32,
    tags: &'static [&'static str],
}

impl CatalogSeed {
    fn to_catalog_entry(self) -> PhotographicMaterialCatalogEntryV1 {
        let provider_slug = self.provider_asset_id.to_ascii_lowercase();
        PhotographicMaterialCatalogEntryV1 {
            id: format!("ambientcg-{provider_slug}"),
            label: self.label.to_string(),
            category: self.category,
            surface_kind: self.surface_kind,
            provider: "ambientcg".to_string(),
            provider_asset_id: self.provider_asset_id.to_string(),
            creation_method: self.creation_method.to_string(),
            source_page: format!("https://ambientcg.com/a/{}", self.provider_asset_id),
            archive_uri: format!(
                "https://ambientcg.com/get?file={}_1K-JPG.zip",
                self.provider_asset_id
            ),
            license: "CC0-1.0".to_string(),
            recommended_tile_size_m: self.recommended_tile_size_m,
            maps: maps_for_category(self.category).to_vec(),
            tags: self.tags.iter().map(|tag| (*tag).to_string()).collect(),
        }
    }
}

const COMMON_MAPS: &[PhotographicMaterialMapKindV1] = &[
    PhotographicMaterialMapKindV1::BaseColor,
    PhotographicMaterialMapKindV1::NormalGl,
    PhotographicMaterialMapKindV1::Roughness,
    PhotographicMaterialMapKindV1::Displacement,
];

const METAL_MAPS: &[PhotographicMaterialMapKindV1] = &[
    PhotographicMaterialMapKindV1::BaseColor,
    PhotographicMaterialMapKindV1::NormalGl,
    PhotographicMaterialMapKindV1::Roughness,
    PhotographicMaterialMapKindV1::Metalness,
    PhotographicMaterialMapKindV1::Displacement,
];

const fn maps_for_category(
    category: PhotographicMaterialCategoryV1,
) -> &'static [PhotographicMaterialMapKindV1] {
    match category {
        PhotographicMaterialCategoryV1::Metal => METAL_MAPS,
        PhotographicMaterialCategoryV1::Plastic
        | PhotographicMaterialCategoryV1::Fabric
        | PhotographicMaterialCategoryV1::Leather
        | PhotographicMaterialCategoryV1::Rubber => COMMON_MAPS,
    }
}

const fn metal(
    provider_asset_id: &'static str,
    label: &'static str,
    surface_kind: PhotographicSurfaceKind,
    tags: &'static [&'static str],
) -> CatalogSeed {
    CatalogSeed {
        provider_asset_id,
        label,
        category: PhotographicMaterialCategoryV1::Metal,
        surface_kind,
        creation_method: "surface-fully-procedural",
        recommended_tile_size_m: 0.25,
        tags,
    }
}

const fn plastic(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    CatalogSeed {
        provider_asset_id,
        label,
        category: PhotographicMaterialCategoryV1::Plastic,
        surface_kind: PhotographicSurfaceKind::MoldedPlastic,
        creation_method: "surface-fully-procedural",
        recommended_tile_size_m: 0.20,
        tags,
    }
}

const fn fabric(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    fabric_with_method(provider_asset_id, label, "surface-fully-procedural", tags)
}

const fn fabric_scanned(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    fabric_with_method(provider_asset_id, label, "surface-photometric-stereo", tags)
}

const fn fabric_with_method(
    provider_asset_id: &'static str,
    label: &'static str,
    creation_method: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    CatalogSeed {
        provider_asset_id,
        label,
        category: PhotographicMaterialCategoryV1::Fabric,
        surface_kind: PhotographicSurfaceKind::Fabric,
        creation_method,
        recommended_tile_size_m: 0.40,
        tags,
    }
}

const fn leather(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    leather_with_method(provider_asset_id, label, "surface-fully-procedural", tags)
}

const fn leather_scanned(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    leather_with_method(provider_asset_id, label, "surface-photometric-stereo", tags)
}

const fn leather_approximated(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    leather_with_method(provider_asset_id, label, "surface-approximated", tags)
}

const fn leather_with_method(
    provider_asset_id: &'static str,
    label: &'static str,
    creation_method: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    CatalogSeed {
        provider_asset_id,
        label,
        category: PhotographicMaterialCategoryV1::Leather,
        surface_kind: PhotographicSurfaceKind::Fabric,
        creation_method,
        recommended_tile_size_m: 0.45,
        tags,
    }
}

const fn rubber(
    provider_asset_id: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
) -> CatalogSeed {
    CatalogSeed {
        provider_asset_id,
        label,
        category: PhotographicMaterialCategoryV1::Rubber,
        surface_kind: PhotographicSurfaceKind::Rubber,
        creation_method: "surface-fully-procedural",
        recommended_tile_size_m: 0.75,
        tags,
    }
}
