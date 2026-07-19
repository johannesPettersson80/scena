use crate::assets::{AssetFetcher, AssetPath, Assets};

use super::checks::{
    finding, validate_catalog_identity, validate_declared_coordinate_system,
    validate_declared_units, validate_load_report, validate_preview,
};
use super::loaded::validate_loaded_asset;
use super::{
    ASSET_READINESS_REPORT_SCHEMA_V1, AssetCatalogAssetV1, AssetCatalogV1,
    AssetReadinessAssetReportV1, AssetReadinessFindingV1, AssetReadinessReportV1,
    AssetReadinessSeverityV1, AssetReadinessSummaryV1,
};

impl<F: AssetFetcher> Assets<F> {
    pub async fn validate_asset_catalog(&self, catalog: &AssetCatalogV1) -> AssetReadinessReportV1 {
        let mut asset_reports = Vec::with_capacity(catalog.assets.len());
        for asset in &catalog.assets {
            asset_reports.push(self.validate_catalog_asset(asset).await);
        }

        let total_assets = asset_reports.len();
        let ready_assets = asset_reports.iter().filter(|asset| asset.ok).count();
        let failed_assets = total_assets - ready_assets;
        let error_count = asset_reports
            .iter()
            .flat_map(|asset| &asset.findings)
            .filter(|finding| finding.severity == AssetReadinessSeverityV1::Error)
            .count();
        let warning_count = asset_reports
            .iter()
            .flat_map(|asset| &asset.findings)
            .filter(|finding| finding.severity == AssetReadinessSeverityV1::Warning)
            .count();
        AssetReadinessReportV1 {
            schema: ASSET_READINESS_REPORT_SCHEMA_V1.to_owned(),
            ok: error_count == 0,
            summary: AssetReadinessSummaryV1 {
                total_assets,
                ready_assets,
                failed_assets,
                error_count,
                warning_count,
            },
            assets: asset_reports,
        }
    }

    async fn validate_catalog_asset(
        &self,
        catalog_asset: &AssetCatalogAssetV1,
    ) -> AssetReadinessAssetReportV1 {
        let mut findings = Vec::new();
        validate_catalog_identity(catalog_asset, &mut findings);
        validate_declared_units(catalog_asset, &mut findings);
        validate_declared_coordinate_system(catalog_asset, &mut findings);
        let mut preview = validate_preview(catalog_asset, &mut findings);
        self.validate_preview_fetch(&mut preview, &mut findings)
            .await;

        for required_file in &catalog_asset.required_files {
            self.validate_required_file(required_file, &mut findings)
                .await;
        }

        let mut geometry = None;
        let mut asset_load_report = None;
        let mut material_fallbacks = Vec::new();
        if !catalog_asset.source.trim().is_empty() {
            match self
                .load_scene_with_report(catalog_asset.source.as_str())
                .await
            {
                Ok(report) => {
                    let asset = report.asset();
                    validate_loaded_asset(self, catalog_asset, asset, &mut findings);
                    let schema_report = report.to_schema_report();
                    validate_load_report(&schema_report, &mut findings);
                    geometry = Some(schema_report.geometry.clone());
                    material_fallbacks = schema_report.material_fallbacks.clone();
                    asset_load_report = Some(schema_report);
                }
                Err(error) => findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "load_failed",
                    format!("asset '{}' did not load: {error}", catalog_asset.id),
                    "fix the source path or the underlying asset error before catalog approval",
                    Some(catalog_asset.source.clone()),
                    Some("source"),
                )),
            }
        }

        let ok = !findings
            .iter()
            .any(|finding| finding.severity == AssetReadinessSeverityV1::Error);
        AssetReadinessAssetReportV1 {
            id: catalog_asset.id.clone(),
            display_name: catalog_asset.display_name.clone(),
            source: catalog_asset.source.clone(),
            ok,
            declared_units: catalog_asset.declared_units.clone(),
            source_coordinate_system: catalog_asset.source_coordinate_system.clone(),
            preview,
            geometry,
            asset_load_report,
            material_fallbacks,
            findings,
        }
    }

    async fn validate_required_file(
        &self,
        required_file: &str,
        findings: &mut Vec<AssetReadinessFindingV1>,
    ) {
        if required_file.trim().is_empty() {
            findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "required_file_empty",
                "required file path must not be empty",
                "remove the empty entry or supply a fetchable path",
                None,
                Some("required_files"),
            ));
            return;
        }
        let path = AssetPath::from(required_file.to_owned());
        if let Err(error) = self.tracked_fetcher().fetch(&path).await {
            findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "required_file_missing",
                format!("required file '{required_file}' is unavailable: {error}"),
                "serve the required file from the same asset source root or remove it from the manifest",
                Some(required_file.to_owned()),
                Some("required_files"),
            ));
        }
    }

    async fn validate_preview_fetch(
        &self,
        preview: &mut Option<super::AssetReadinessPreviewV1>,
        findings: &mut Vec<AssetReadinessFindingV1>,
    ) {
        let Some(preview) = preview.as_mut() else {
            return;
        };
        if preview.kind != "image" {
            return;
        }
        let Some(path) = preview.path.as_deref() else {
            return;
        };
        let path = AssetPath::from(path.to_owned());
        match self.tracked_fetcher().fetch(&path).await {
            Ok(bytes) if !bytes.is_empty() => {
                preview.status = "fetched".to_owned();
            }
            Ok(_) => {
                preview.status = "empty".to_owned();
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "preview_empty",
                    format!("preview image '{}' is empty", path.as_str()),
                    "replace the preview image with non-empty PNG/JPEG/WebP bytes",
                    Some(path.as_str().to_owned()),
                    Some("preview.path"),
                ));
            }
            Err(error) => {
                preview.status = "missing".to_owned();
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "preview_fetch_failed",
                    format!("preview image '{}' is unavailable: {error}", path.as_str()),
                    "serve the preview image from the configured AssetFetcher or mark it generated",
                    Some(path.as_str().to_owned()),
                    Some("preview.path"),
                ));
            }
        }
    }
}
