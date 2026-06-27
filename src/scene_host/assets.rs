use super::reporting::SceneHostAssetImportReportV1;
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, AssetPath};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub async fn asset_doctor_json(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<String, SceneHostError> {
        let report = self.assets.doctor_asset_path(path).await;
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("asset doctor report serialization failed: {error}"),
            )
        })
    }

    pub async fn instantiate_url_with_report_json(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> Result<String, SceneHostError> {
        self.instantiate_url_under_with_report_json(self.root_handle(), path)
            .await
    }

    pub async fn instantiate_url_under_with_report_json(
        &mut self,
        parent: u64,
        path: impl Into<AssetPath>,
    ) -> Result<String, SceneHostError> {
        let parent = self.resolve_node(parent)?;
        let report = self.assets.load_scene_with_report(path).await?;
        let import = self.instantiate_scene_asset_under(parent, report.asset())?;
        let asset_report = report.to_schema_report();
        self.emit_asset_load_events(import, &asset_report);
        let host_report = SceneHostAssetImportReportV1::from_import(
            import,
            self.resolve_import(import)?,
            asset_report,
        );
        serde_json::to_string(&host_report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("asset load report serialization failed: {error}"),
            )
        })
    }
}
