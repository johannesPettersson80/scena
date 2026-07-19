use std::collections::{BTreeSet, VecDeque};

use crate::assets::SceneAsset;
use crate::diagnostics::InstantiateError;

pub(super) fn validate_scene_asset_for_instantiation(
    scene_asset: &SceneAsset,
) -> Result<(), InstantiateError> {
    let nodes = scene_asset.nodes();
    let mut parents = vec![None; nodes.len()];
    let mut indegree = vec![0_usize; nodes.len()];

    for (parent, node) in nodes.iter().enumerate() {
        validate_node_metadata(scene_asset, parent)?;
        for &child in node.children() {
            if child >= nodes.len() {
                return Err(InstantiateError::InvalidChildIndex { parent, child });
            }
            if let Some(first_parent) = parents[child] {
                return Err(InstantiateError::MultipleNodeParents {
                    node: child,
                    first_parent,
                    second_parent: parent,
                });
            }
            parents[child] = Some(parent);
            indegree[child] = 1;
        }
    }

    let mut remaining_indegree = indegree;
    let mut queue = remaining_indegree
        .iter()
        .enumerate()
        .filter_map(|(node, &degree)| (degree == 0).then_some(node))
        .collect::<VecDeque<_>>();
    let mut visited = 0_usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        for &child in nodes[node].children() {
            remaining_indegree[child] -= 1;
            if remaining_indegree[child] == 0 {
                queue.push_back(child);
            }
        }
    }
    if visited != nodes.len() {
        let node = remaining_indegree
            .iter()
            .position(|&degree| degree != 0)
            .unwrap_or(0);
        return Err(InstantiateError::CyclicNodeGraph { node });
    }

    Ok(())
}

fn validate_node_metadata(
    scene_asset: &SceneAsset,
    node_index: usize,
) -> Result<(), InstantiateError> {
    let node = &scene_asset.nodes()[node_index];
    let node_name = node.name().unwrap_or("<unnamed>");

    let mut anchor_names = BTreeSet::new();
    for anchor in node.anchors() {
        if let Some(reason) = anchor.invalid_reason() {
            return Err(InstantiateError::InvalidAnchorExtras {
                node: node_name.to_owned(),
                reason: reason.to_owned(),
            });
        }
        if !anchor_names.insert(anchor.name()) {
            return Err(InstantiateError::InvalidAnchorExtras {
                node: node_name.to_owned(),
                reason: format!("duplicate anchor '{}'", anchor.name()),
            });
        }
    }

    let mut connector_names = BTreeSet::new();
    for connector in node.connectors() {
        if let Some(reason) = connector.invalid_reason() {
            return Err(InstantiateError::InvalidConnectorExtras {
                node: node_name.to_owned(),
                reason: reason.to_owned(),
            });
        }
        if !connector_names.insert(connector.name()) {
            return Err(InstantiateError::InvalidConnectorExtras {
                node: node_name.to_owned(),
                reason: format!("duplicate connector '{}'", connector.name()),
            });
        }
    }

    if let Some(skin_index) = node.skin() {
        let skin =
            scene_asset
                .skins()
                .get(skin_index)
                .ok_or(InstantiateError::InvalidSkinIndex {
                    node: node_index,
                    skin: skin_index,
                })?;
        for &joint in skin.joints() {
            if joint >= scene_asset.nodes().len() {
                return Err(InstantiateError::InvalidSkinJointIndex {
                    skin: skin_index,
                    joint,
                });
            }
        }
    }

    Ok(())
}
