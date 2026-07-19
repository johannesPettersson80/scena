use std::collections::HashMap;

use super::types::ImportedNode;
use super::{ImportClip, ImportOptions};
use crate::animation::{AnimationClipKey, AnimationTarget};
use crate::assets::SceneAsset;
use crate::diagnostics::InstantiateError;

pub(super) fn rebind_import_clips(
    scene_asset: &SceneAsset,
    records: &[ImportedNode],
    options: ImportOptions,
) -> Result<Vec<ImportClip>, InstantiateError> {
    let source_nodes = SourceNodeIndex::new(records);
    scene_asset
        .clips()
        .iter()
        .map(|clip| {
            let rebased = clip
                .clip()
                .rebind_imported_many(
                    AnimationClipKey::fresh(),
                    |source_index, target| {
                        let mut ignored = SourceNodeLookupMetrics::default();
                        map_target_nodes_profiled(&source_nodes, source_index, target, &mut ignored)
                    },
                    |target, value| options.convert_animation_vec3(target, value),
                )
                .map_err(|error| InstantiateError::InvalidAnimationClip {
                    name: clip.name().map(str::to_string),
                    reason: error.to_string(),
                })?;
            Ok(ImportClip { clip: rebased })
        })
        .collect()
}

struct SourceNodeIndex<'a> {
    records: &'a [ImportedNode],
    offsets: HashMap<usize, usize>,
}

impl<'a> SourceNodeIndex<'a> {
    fn new(records: &'a [ImportedNode]) -> Self {
        Self {
            records,
            offsets: records
                .iter()
                .enumerate()
                .map(|(offset, record)| (record.source_index, offset))
                .collect(),
        }
    }

    fn get(&self, source_index: usize) -> Option<&ImportedNode> {
        self.offsets
            .get(&source_index)
            .and_then(|offset| self.records.get(*offset))
    }
}

#[derive(Debug, Default)]
struct SourceNodeLookupMetrics {
    record_probes: u64,
}

fn map_target_nodes_profiled(
    index: &SourceNodeIndex<'_>,
    source_index: usize,
    target: AnimationTarget,
    metrics: &mut SourceNodeLookupMetrics,
) -> Vec<crate::scene::NodeKey> {
    metrics.record_probes = metrics.record_probes.saturating_add(1);
    let Some(record) = index.get(source_index) else {
        return Vec::new();
    };
    if target == AnimationTarget::Weights {
        record.morph_nodes.clone()
    } else {
        vec![record.node]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::NodeKey;
    use slotmap::Key;

    #[test]
    fn pf10_import_animation_rebind_uses_one_source_node_index() {
        const NODE_COUNT: usize = 16_384;
        let records = (0..NODE_COUNT)
            .rev()
            .map(|source_index| ImportedNode {
                source_index,
                node: NodeKey::null(),
                morph_nodes: vec![NodeKey::null(), NodeKey::null()],
                parent: None,
                name: None,
                bounds: None,
            })
            .collect::<Vec<_>>();
        let index = SourceNodeIndex::new(&records);
        let mut metrics = SourceNodeLookupMetrics::default();

        for source_index in 0..NODE_COUNT {
            assert_eq!(
                map_target_nodes_profiled(
                    &index,
                    source_index,
                    AnimationTarget::Translation,
                    &mut metrics,
                ),
                vec![NodeKey::null()]
            );
        }
        assert!(
            metrics.record_probes <= NODE_COUNT as u64,
            "indexed rebind must be O(channels + nodes), got {} probes",
            metrics.record_probes
        );
    }
}
