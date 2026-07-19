use crate::assets::{AssetFetcher, AssetPath, Assets, SceneAsset};
use crate::diagnostics::{ImportError, InstantiateError};
use crate::scene::Scene;
use crate::scene::transaction::SceneTransaction;

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

    /// Atomically replaces an import-owned graph.
    ///
    /// Source units and coordinate-system policy carry forward. User-authored
    /// runtime overrides (visibility, tint, active material variant, animation
    /// state, and similar per-instance state) intentionally start fresh on the
    /// replacement graph; callers may reapply selected overrides after success.
    pub fn replace_import(
        &mut self,
        import: &SceneImport,
        scene_asset: &SceneAsset,
    ) -> Result<SceneImport, InstantiateError> {
        if !import.is_live() {
            return Err(InstantiateError::StaleReplacementImport);
        }
        if !import.belongs_to(self) {
            return Err(InstantiateError::ForeignReplacementImport);
        }
        let removed = import
            .roots()
            .iter()
            .copied()
            .map(|root| {
                if root == self.root() || self.node(root).is_none() {
                    return Err(InstantiateError::MissingReplacementRoot { root });
                }
                self.subtree_nodes(root)
                    .map_err(|_| InstantiateError::MissingReplacementRoot { root })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let options = ImportOptions::gltf_default()
            .with_source_units(import.source_units)
            .with_source_coordinate_system(import.source_coordinate_system);
        let mut transaction = SceneTransaction::new(self);
        let replacement = transaction.scene().instantiate_with(scene_asset, options)?;
        for nodes in removed {
            transaction.scene().remove_nodes_unchecked(&nodes);
        }
        transaction.commit();
        import.mark_stale();
        Ok(replacement)
    }
}
