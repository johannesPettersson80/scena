use crate::assets::SceneAsset;
use crate::diagnostics::InstantiateError;
use crate::scene::{Scene, SceneSkinBinding};

use super::types::{ImportedNode, PendingSkinBinding};

impl Scene {
    pub(super) fn resolve_import_skin_bindings(
        &mut self,
        scene_asset: &SceneAsset,
        records: &[ImportedNode],
        pending: &[PendingSkinBinding],
    ) -> Result<(), InstantiateError> {
        for pending in pending {
            let skin = scene_asset.skins().get(pending.skin).ok_or(
                InstantiateError::InvalidSkinIndex {
                    node: pending.source_node,
                    skin: pending.skin,
                },
            )?;
            let joints = skin
                .joints()
                .iter()
                .map(|source_joint| {
                    records
                        .iter()
                        .find(|record| record.source_index == *source_joint)
                        .map(|record| record.node)
                        .ok_or(InstantiateError::InvalidSkinJointIndex {
                            skin: pending.skin,
                            joint: *source_joint,
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            self.set_initial_skin_binding(
                pending.node,
                SceneSkinBinding::new(joints, skin.inverse_bind_matrices().to_vec()),
            );
        }
        Ok(())
    }
}
