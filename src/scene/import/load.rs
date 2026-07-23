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
    /// Source units and coordinate-system policy carry forward. Replacement
    /// roots are paired to prior roots by source-root ordinal and retain their
    /// host parent, local transform, direct visibility, and host-added tags.
    /// Other per-instance state, including child overrides, tint, material
    /// variants, and animation state, starts fresh on the replacement graph.
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
        let placements = import
            .roots()
            .iter()
            .copied()
            .map(|root| {
                let Some(node) = self.node(root) else {
                    return Err(InstantiateError::MissingReplacementRoot { root });
                };
                if root == self.root() {
                    return Err(InstantiateError::MissingReplacementRoot { root });
                }
                let parent = node
                    .parent()
                    .ok_or(InstantiateError::MissingReplacementRoot { root })?;
                let removed = self
                    .subtree_nodes(root)
                    .map_err(|_| InstantiateError::MissingReplacementRoot { root })?;
                Ok((
                    parent,
                    node.transform(),
                    node.visible,
                    node.tags.clone(),
                    removed,
                ))
            })
            .collect::<Result<Vec<_>, InstantiateError>>()?;
        let options = ImportOptions::gltf_default()
            .with_source_units(import.source_units)
            .with_source_coordinate_system(import.source_coordinate_system);
        let mut transaction = SceneTransaction::new(self);
        let replacement = transaction.scene().instantiate_with(scene_asset, options)?;
        let fallback_parent = placements.first().map(|(parent, _, _, _, _)| *parent);
        for (index, replacement_root) in replacement.roots().iter().copied().enumerate() {
            if let Some((parent, transform, visible, tags, _)) = placements.get(index) {
                reparent_replacement_root(transaction.scene(), replacement_root, *parent);
                transaction.scene().nodes[replacement_root].transform = *transform;
                transaction.scene().nodes[replacement_root].visible = *visible;
                transaction.scene().nodes[replacement_root].tags = tags.clone();
            } else if let Some(parent) = fallback_parent {
                reparent_replacement_root(transaction.scene(), replacement_root, parent);
            }
        }
        for (_, _, _, _, nodes) in placements {
            transaction.scene().remove_nodes_unchecked(&nodes);
        }
        transaction.commit();
        import.mark_stale();
        Ok(replacement)
    }
}

pub(super) fn reparent_replacement_root(
    scene: &mut Scene,
    node: crate::scene::NodeKey,
    parent: crate::scene::NodeKey,
) {
    let old_parent = scene.nodes[node].parent();
    if old_parent == Some(parent) {
        return;
    }
    if let Some(old_parent) = old_parent {
        scene.nodes[old_parent]
            .children
            .retain(|child| *child != node);
    }
    scene.nodes[node].parent = Some(parent);
    if !scene.nodes[parent].children.contains(&node) {
        scene.nodes[parent].children.push(node);
    }
    scene.structure_revision = scene.structure_revision.saturating_add(1);
    scene.transform_revision = scene.transform_revision.saturating_add(1);
}
