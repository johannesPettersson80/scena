use std::collections::BTreeSet;

use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{NodeKey, Scene};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SceneVisibilitySnapshot {
    entries: Vec<SceneVisibilitySnapshotEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneVisibilitySnapshotEntry {
    pub node: NodeKey,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneTintSnapshot {
    entries: Vec<SceneTintSnapshotEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTintSnapshotEntry {
    pub node: NodeKey,
    pub tint: Option<Color>,
}

impl SceneVisibilitySnapshot {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[SceneVisibilitySnapshotEntry] {
        &self.entries
    }
}

impl SceneTintSnapshot {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[SceneTintSnapshotEntry] {
        &self.entries
    }
}

impl Scene {
    pub fn hide(&mut self, node: NodeKey) -> Result<(), LookupError> {
        self.set_visible(node, false)
    }

    pub fn show(&mut self, node: NodeKey) -> Result<(), LookupError> {
        self.set_visible(node, true)
    }

    pub fn toggle_visibility(&mut self, node: NodeKey) -> Result<bool, LookupError> {
        let visible = self.visible(node).ok_or(LookupError::NodeNotFound(node))?;
        let next = !visible;
        self.set_visible(node, next)?;
        Ok(next)
    }

    pub fn show_only(
        &mut self,
        nodes: impl IntoIterator<Item = NodeKey>,
    ) -> Result<SceneVisibilitySnapshot, LookupError> {
        let mut keep = BTreeSet::from([self.root()]);
        for node in nodes {
            self.collect_isolate_keep_set(node, &mut keep)?;
        }
        self.apply_visibility_keep_set(&keep)
    }

    pub fn isolate(
        &mut self,
        nodes: impl IntoIterator<Item = NodeKey>,
    ) -> Result<SceneVisibilitySnapshot, LookupError> {
        self.show_only(nodes)
    }

    pub fn restore_visibility(
        &mut self,
        snapshot: &SceneVisibilitySnapshot,
    ) -> Result<(), LookupError> {
        for entry in &snapshot.entries {
            self.set_visible(entry.node, entry.visible)?;
        }
        Ok(())
    }

    pub fn ghost(&mut self, node: NodeKey, alpha: f32) -> Result<SceneTintSnapshot, LookupError> {
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(LookupError::InvalidFramingOption {
                field: "alpha",
                reason: "ghost alpha must be finite and between 0 and 1",
            });
        }
        self.ghost_with_tint(node, Color::from_linear_rgba(1.0, 1.0, 1.0, alpha))
    }

    pub fn ghost_with_tint(
        &mut self,
        node: NodeKey,
        tint: Color,
    ) -> Result<SceneTintSnapshot, LookupError> {
        if !color_is_finite(tint) {
            return Err(LookupError::InvalidFramingOption {
                field: "tint",
                reason: "ghost tint must contain finite color channels",
            });
        }
        let mut nodes = Vec::new();
        self.collect_subtree(node, &mut nodes)?;
        let mut entries = Vec::new();
        for node in nodes {
            let previous = self.node_tint(node)?;
            if previous != Some(tint) {
                entries.push(SceneTintSnapshotEntry {
                    node,
                    tint: previous,
                });
                self.set_node_tint(node, Some(tint))?;
            }
        }
        Ok(SceneTintSnapshot { entries })
    }

    pub fn restore_tints(&mut self, snapshot: &SceneTintSnapshot) -> Result<(), LookupError> {
        for entry in &snapshot.entries {
            self.set_node_tint(entry.node, entry.tint)?;
        }
        Ok(())
    }

    fn collect_isolate_keep_set(
        &self,
        node: NodeKey,
        keep: &mut BTreeSet<NodeKey>,
    ) -> Result<(), LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        let mut current = Some(node);
        while let Some(node) = current {
            keep.insert(node);
            current = self.nodes.get(node).and_then(|node| node.parent);
        }
        self.collect_subtree_into_set(node, keep)
    }

    fn collect_subtree_into_set(
        &self,
        node: NodeKey,
        keep: &mut BTreeSet<NodeKey>,
    ) -> Result<(), LookupError> {
        let node_ref = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        keep.insert(node);
        for child in &node_ref.children {
            self.collect_subtree_into_set(*child, keep)?;
        }
        Ok(())
    }

    fn collect_subtree(&self, node: NodeKey, nodes: &mut Vec<NodeKey>) -> Result<(), LookupError> {
        let node_ref = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        nodes.push(node);
        for child in &node_ref.children {
            self.collect_subtree(*child, nodes)?;
        }
        Ok(())
    }

    fn apply_visibility_keep_set(
        &mut self,
        keep: &BTreeSet<NodeKey>,
    ) -> Result<SceneVisibilitySnapshot, LookupError> {
        let updates = self
            .nodes
            .iter()
            .map(|(node, node_ref)| {
                let desired = keep.contains(&node);
                (node, node_ref.visible, desired)
            })
            .filter(|(_, current, desired)| current != desired)
            .collect::<Vec<_>>();
        let mut entries = Vec::with_capacity(updates.len());
        for (node, current, desired) in updates {
            entries.push(SceneVisibilitySnapshotEntry {
                node,
                visible: current,
            });
            self.set_visible(node, desired)?;
        }
        Ok(SceneVisibilitySnapshot { entries })
    }
}

fn color_is_finite(color: Color) -> bool {
    color.r.is_finite() && color.g.is_finite() && color.b.is_finite() && color.a.is_finite()
}
