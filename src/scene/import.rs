use std::collections::BTreeSet;

use crate::assets::SceneAsset;
use crate::diagnostics::{InstantiateError, LookupError};

use super::{NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneImport {
    roots: Vec<NodeKey>,
    records: Vec<ImportedNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportedNode {
    node: NodeKey,
    parent: Option<NodeKey>,
    name: Option<String>,
}

impl Scene {
    pub fn instantiate(
        &mut self,
        scene_asset: &SceneAsset,
    ) -> Result<SceneImport, InstantiateError> {
        let nodes = scene_asset.nodes();
        let mut child_indices = BTreeSet::new();
        for node in nodes {
            child_indices.extend(node.children().iter().copied());
        }

        let roots = (0..nodes.len())
            .filter(|index| !child_indices.contains(index))
            .collect::<Vec<_>>();
        let mut import = SceneImport {
            roots: Vec::new(),
            records: Vec::new(),
        };
        for source_index in roots {
            let node = self.instantiate_scene_asset_node(
                scene_asset,
                source_index,
                self.root,
                None,
                &mut import.records,
            )?;
            import.roots.push(node);
        }
        Ok(import)
    }

    fn instantiate_scene_asset_node(
        &mut self,
        scene_asset: &SceneAsset,
        source_index: usize,
        parent: NodeKey,
        imported_parent: Option<NodeKey>,
        records: &mut Vec<ImportedNode>,
    ) -> Result<NodeKey, InstantiateError> {
        let source_node =
            scene_asset
                .nodes()
                .get(source_index)
                .ok_or(InstantiateError::InvalidChildIndex {
                    parent: source_index,
                    child: source_index,
                })?;
        let node = self
            .insert_node(parent, NodeKind::Empty, Transform::default())
            .expect("import parent was inserted by this scene");
        records.push(ImportedNode {
            node,
            parent: imported_parent,
            name: source_node.name().map(str::to_string),
        });
        for child in source_node.children() {
            if scene_asset.nodes().get(*child).is_none() {
                return Err(InstantiateError::InvalidChildIndex {
                    parent: source_index,
                    child: *child,
                });
            }
            self.instantiate_scene_asset_node(scene_asset, *child, node, Some(node), records)?;
        }
        Ok(node)
    }
}

impl SceneImport {
    pub fn node(&self, name: &str) -> Result<NodeKey, LookupError> {
        let matches = self.nodes_named(name).collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(LookupError::NodeNameNotFound {
                name: name.to_string(),
            }),
            [node] => Ok(*node),
            _ => Err(LookupError::AmbiguousNodeName {
                name: name.to_string(),
                matches,
            }),
        }
    }

    pub fn first_node(&self, name: &str) -> Option<NodeKey> {
        self.nodes_named(name).next()
    }

    pub fn nodes_named<'import>(
        &'import self,
        name: &'import str,
    ) -> impl Iterator<Item = NodeKey> + 'import {
        self.records
            .iter()
            .filter(move |record| record.name.as_deref() == Some(name))
            .map(|record| record.node)
    }

    pub fn path(&self, path: &str) -> Result<NodeKey, LookupError> {
        let mut segments = path.split('/');
        let Some(first) = segments.next().filter(|segment| !segment.is_empty()) else {
            return Err(LookupError::PathNotFound {
                path: path.to_string(),
            });
        };
        let mut current = self
            .records
            .iter()
            .find(|record| record.parent.is_none() && record.name.as_deref() == Some(first))
            .map(|record| record.node)
            .ok_or_else(|| LookupError::PathNotFound {
                path: path.to_string(),
            })?;

        for segment in segments {
            current = self
                .records
                .iter()
                .find(|record| {
                    record.parent == Some(current) && record.name.as_deref() == Some(segment)
                })
                .map(|record| record.node)
                .ok_or_else(|| LookupError::PathNotFound {
                    path: path.to_string(),
                })?;
        }
        Ok(current)
    }

    pub fn roots(&self) -> &[NodeKey] {
        &self.roots
    }
}
