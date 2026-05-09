use crate::assets::{AssetFetcher, AssetPath, Assets, SceneAsset};
use crate::diagnostics::{ImportError, InstantiateError};
use crate::scene::Scene;

use super::{ImportOptions, SceneImport};

impl Scene {
    pub async fn import<F: AssetFetcher>(
        &mut self,
        assets: &Assets<F>,
        path: impl Into<AssetPath>,
    ) -> Result<SceneImport, ImportError> {
        self.import_with(assets, path, ImportOptions::gltf_default())
            .await
    }

    pub async fn import_with<F: AssetFetcher>(
        &mut self,
        assets: &Assets<F>,
        path: impl Into<AssetPath>,
        options: ImportOptions,
    ) -> Result<SceneImport, ImportError> {
        let scene_asset = assets.load_scene(path).await?;
        self.instantiate_with(&scene_asset, options)
            .map_err(Into::into)
    }

    pub fn replace_import(
        &mut self,
        import: &SceneImport,
        scene_asset: &SceneAsset,
    ) -> Result<SceneImport, InstantiateError> {
        let options = ImportOptions::gltf_default()
            .with_source_units(import.source_units)
            .with_source_coordinate_system(import.source_coordinate_system);
        import.mark_stale();
        self.instantiate_with(scene_asset, options)
    }
}
