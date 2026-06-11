use std::collections::BTreeSet;

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
            }
        }
        Ok(())
    }

    pub fn subtree_nodes_json(&mut self, root: u64) -> Result<String, SceneHostError> {
        let root = self.resolve_node(root)?;
        let nodes = self.scene.subtree_nodes(root)?;
        let mut report_nodes = Vec::with_capacity(nodes.len());
        for node in nodes {
            let handle = self.register_node(node);
            let node = self
                .scene
                .node(node)
                .ok_or(LookupError::NodeNotFound(node))?;
            report_nodes.push(SceneHostSubtreeNodeV1 {
                handle,
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
