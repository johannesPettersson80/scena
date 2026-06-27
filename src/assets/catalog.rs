mod checks;
mod loaded;
mod types;
mod validation;

pub use types::{
    AssetCatalogAssetV1, AssetCatalogExpectedBoundsV1, AssetCatalogFeatureRequirementV1,
    AssetCatalogMaterialRequirementsV1, AssetCatalogPreviewV1, AssetCatalogV1,
    AssetReadinessAssetReportV1, AssetReadinessFindingV1, AssetReadinessPreviewV1,
    AssetReadinessReportV1, AssetReadinessSeverityV1, AssetReadinessSummaryV1,
};

pub const ASSET_CATALOG_SCHEMA_V1: &str = "scena.asset_catalog.v1";
pub const ASSET_READINESS_REPORT_SCHEMA_V1: &str = "scena.asset_readiness_report.v1";
