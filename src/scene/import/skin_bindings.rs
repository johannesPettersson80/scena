use crate::assets::SceneAsset;
use crate::diagnostics::InstantiateError;
use crate::scene::{Scene, SceneSkinBinding};

use super::source_node_index::SourceNodeIndex;
#[cfg(test)]
use super::types::ImportedNode;
use super::types::PendingSkinBinding;

#[derive(Debug, Default)]
struct SkinJointLookupMetrics {
    record_probes: u64,
}

impl Scene {
    pub(super) fn resolve_import_skin_bindings(
        &mut self,
        scene_asset: &SceneAsset,
        source_nodes: &SourceNodeIndex<'_>,
        pending: &[PendingSkinBinding],
    ) -> Result<(), InstantiateError> {
        for pending in pending {
            let skin = scene_asset.skins().get(pending.skin).ok_or(
                InstantiateError::InvalidSkinIndex {
                    node: pending.source_node,
                    skin: pending.skin,
                },
            )?;
            let mut ignored = SkinJointLookupMetrics::default();
            let joints = resolve_skin_joint_nodes_profiled(
                source_nodes,
                pending.skin,
                skin.joints(),
                &mut ignored,
            )?;
            self.set_initial_skin_binding(
                pending.node,
                SceneSkinBinding::new(joints, skin.inverse_bind_matrices().to_vec()),
            );
        }
        Ok(())
    }
}

fn resolve_skin_joint_nodes_profiled(
    source_nodes: &SourceNodeIndex<'_>,
    skin: usize,
    source_joints: &[usize],
    metrics: &mut SkinJointLookupMetrics,
) -> Result<Vec<crate::scene::NodeKey>, InstantiateError> {
    source_joints
        .iter()
        .map(|source_joint| {
            metrics.record_probes = metrics.record_probes.saturating_add(1);
            source_nodes
                .get(*source_joint)
                .map(|record| record.node)
                .ok_or(InstantiateError::InvalidSkinJointIndex {
                    skin,
                    joint: *source_joint,
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::NodeKey;
    use slotmap::Key;

    #[test]
    fn imported_skin_joint_resolution_uses_the_shared_source_index() {
        const NODE_COUNT: usize = 16_384;
        let records = (0..NODE_COUNT)
            .rev()
            .map(|source_index| ImportedNode {
                source_index,
                node: NodeKey::null(),
                morph_nodes: Vec::new(),
                parent: None,
                name: None,
                bounds: None,
            })
            .collect::<Vec<_>>();
        let index = SourceNodeIndex::new(&records);
        let mut metrics = SkinJointLookupMetrics::default();

        let joints = resolve_skin_joint_nodes_profiled(
            &index,
            0,
            &(0..NODE_COUNT).collect::<Vec<_>>(),
            &mut metrics,
        )
        .expect("every source joint resolves");

        assert_eq!(joints.len(), NODE_COUNT);
        assert_eq!(metrics.record_probes, NODE_COUNT as u64);
    }
}
