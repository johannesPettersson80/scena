use crate::assets::SceneAsset;
use crate::diagnostics::{InstantiateError, LookupError};
use crate::scene::transaction::SceneTransaction;
use crate::scene::{NodeKey, Scene};

use super::handedness::reject_unproven_left_handed_mesh_import;
use super::prevalidation::validate_scene_asset_for_instantiation;
use super::{ImportOptions, SceneImport};

impl Scene {
    pub(super) fn instantiate_with_parent(
        &mut self,
        parent: NodeKey,
        scene_asset: &SceneAsset,
        options: ImportOptions,
    ) -> Result<SceneImport, InstantiateError> {
        validate_scene_asset_for_instantiation(scene_asset)?;
        reject_unproven_left_handed_mesh_import(scene_asset, options)?;
        let mut transaction = SceneTransaction::new(self);
        let import =
            transaction
                .scene()
                .instantiate_with_parent_validated(parent, scene_asset, options)?;
        transaction.commit();
        Ok(import)
    }

    pub fn instantiate(
        &mut self,
        scene_asset: &SceneAsset,
    ) -> Result<SceneImport, InstantiateError> {
        self.instantiate_with(scene_asset, ImportOptions::gltf_default())
    }

    pub fn instantiate_with(
        &mut self,
        scene_asset: &SceneAsset,
        options: ImportOptions,
    ) -> Result<SceneImport, InstantiateError> {
        self.instantiate_with_parent(self.root(), scene_asset, options)
    }

    pub fn instantiate_under(
        &mut self,
        parent: NodeKey,
        scene_asset: &SceneAsset,
        options: ImportOptions,
    ) -> crate::Result<SceneImport> {
        if self.node(parent).is_none() {
            return Err(LookupError::NodeNotFound(parent).into());
        }
        self.instantiate_with_parent(parent, scene_asset, options)
            .map_err(Into::into)
    }
}
