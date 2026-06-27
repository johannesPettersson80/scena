use std::collections::{BTreeMap, BTreeSet};

use super::reporting::{SceneHostSubtreeNodeV1, SceneHostSubtreeReportV1};
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, Color, LookupError};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_visible(&mut self, node: u64, visible: bool) -> Result<(), SceneHostError> {
        if self.is_instance_root_handle(node) {
            return self.set_instance_root_visible(node, visible);
        }
        let node = self.resolve_node(node)?;
        self.scene.set_visible(node, visible)?;
        Ok(())
    }

    pub fn set_subtree_tint(
        &mut self,
        root: u64,
        tint: Option<Color>,
        exclude: &[u64],
    ) -> Result<(), SceneHostError> {
        let root = self.resolve_node(root)?;
        let mut excluded = BTreeSet::new();
        for handle in exclude {
            let node = self.resolve_node(*handle)?;
            excluded.extend(self.scene.subtree_nodes(node)?);
        }
        let nodes = self.scene.subtree_nodes(root)?;
        for node in nodes {
            if !excluded.contains(&node) {
                self.scene.set_node_tint(node, tint)?;
                if let Some(handle) = self.node_handle_map.get(&node).copied() {
                    self.cancel_tint_transition(handle);
                }
            }
        }
        Ok(())
    }

    pub fn subtree_nodes_json(&mut self, root: u64) -> Result<String, SceneHostError> {
        let root = self.resolve_node(root)?;
        let nodes = self.scene.subtree_nodes(root)?;
        let node_set = nodes.iter().copied().collect::<BTreeSet<_>>();
        let mut handles = BTreeMap::new();
        for node in &nodes {
            handles.insert(*node, self.register_node(*node));
        }
        let mut report_nodes = Vec::with_capacity(nodes.len());
        for node in nodes {
            let handle = handles[&node];
            let node = self
                .scene
                .node(node)
                .ok_or(LookupError::NodeNotFound(node))?;
            let parent = node
                .parent()
                .filter(|parent| node_set.contains(parent))
                .and_then(|parent| handles.get(&parent).copied());
            let children = node
                .children()
                .iter()
                .filter_map(|child| node_set.contains(child).then_some(handles[child]))
                .collect();
            report_nodes.push(SceneHostSubtreeNodeV1 {
                handle,
                parent,
                children,
                name: None,
                tags: node.tags().map(str::to_owned).collect(),
            });
        }
        let report = SceneHostSubtreeReportV1::new(report_nodes);
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("subtree serialization failed: {error}"),
            )
        })
    }
}
