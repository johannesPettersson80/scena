use serde::{Deserialize, Serialize};

use crate::assets::{AssetLoadReportV1, AssetMaterialFallbackV1, SceneAssetGeometrySummary};
use crate::scene::Vec3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCatalogV1 {
    pub schema: String,
    #[serde(default)]
    pub assets: Vec<AssetCatalogAssetV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCatalogAssetV1 {
    pub id: String,
    pub display_name: String,
    pub source: String,
    #[serde(default)]
    pub required_files: Vec<String>,
    #[serde(default)]
    pub preview: Option<AssetCatalogPreviewV1>,
    #[serde(default)]
    pub declared_units: Option<String>,
    #[serde(default)]
    pub source_coordinate_system: Option<String>,
    #[serde(default)]
    pub expected_bounds: Option<AssetCatalogExpectedBoundsV1>,
    #[serde(default)]
    pub required_anchors: Vec<AssetCatalogFeatureRequirementV1>,
    #[serde(default)]
    pub required_connectors: Vec<AssetCatalogFeatureRequirementV1>,
    #[serde(default)]
    pub required_tags: Vec<String>,
    #[serde(default)]
    pub material_requirements: AssetCatalogMaterialRequirementsV1,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub provenance: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCatalogPreviewV1 {
    pub kind: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub background: Option<[f32; 4]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCatalogExpectedBoundsV1 {
    #[serde(default)]
    pub min: Option<Vec3>,
    #[serde(default)]
    pub max: Option<Vec3>,
    #[serde(default)]
    pub min_extent: Option<f32>,
    #[serde(default)]
    pub max_extent: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetCatalogFeatureRequirementV1 {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AssetCatalogMaterialRequirementsV1 {
    #[serde(default)]
    pub required_variants: Vec<String>,
    #[serde(default)]
    pub require_base_color_textures: bool,
    #[serde(default)]
    pub allow_material_fallbacks: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetReadinessReportV1 {
    pub schema: String,
    pub ok: bool,
    pub summary: AssetReadinessSummaryV1,
    pub assets: Vec<AssetReadinessAssetReportV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReadinessSummaryV1 {
    pub total_assets: usize,
    pub ready_assets: usize,
    pub failed_assets: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetReadinessAssetReportV1 {
    pub id: String,
    pub display_name: String,
    pub source: String,
    pub ok: bool,
    #[serde(default)]
    pub declared_units: Option<String>,
    #[serde(default)]
    pub source_coordinate_system: Option<String>,
    #[serde(default)]
    pub preview: Option<AssetReadinessPreviewV1>,
    #[serde(default)]
    pub geometry: Option<SceneAssetGeometrySummary>,
    #[serde(default)]
    pub asset_load_report: Option<AssetLoadReportV1>,
    #[serde(default)]
    pub material_fallbacks: Vec<AssetMaterialFallbackV1>,
    #[serde(default)]
    pub findings: Vec<AssetReadinessFindingV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReadinessPreviewV1 {
    pub kind: String,
    pub status: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReadinessFindingV1 {
    pub severity: AssetReadinessSeverityV1,
    pub code: String,
    pub message: String,
    pub help: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetReadinessSeverityV1 {
    Error,
    Warning,
    Info,
}

impl AssetReadinessReportV1 {
    pub fn asset(&self, id: &str) -> Option<&AssetReadinessAssetReportV1> {
        self.assets.iter().find(|asset| asset.id == id)
    }
}
