use super::camera::controls_from_scene_camera;
use super::{SceneHostCore, SceneHostError};
use crate::AssetFetcher;
use crate::NodeKey;

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn show_only(&mut self, nodes: &[u64]) -> Result<(), SceneHostError> {
        let nodes = self.resolve_node_handles(nodes)?;
        self.scene.show_only(nodes)?;
        Ok(())
    }

    pub fn isolate(&mut self, nodes: &[u64]) -> Result<(), SceneHostError> {
        self.show_only(nodes)
    }

    pub fn ghost(&mut self, node: u64, alpha: f32) -> Result<(), SceneHostError> {
        let node = self.resolve_node(node)?;
        let touched = self.scene.subtree_nodes(node)?;
        self.scene.ghost(node, alpha)?;
        for node in touched {
            if let Some(handle) = self.node_handle_map.get(&node).copied() {
                self.cancel_tint_transition(handle);
            }
        }
        Ok(())
    }

    pub fn fit_selection(&mut self, nodes: &[u64]) -> Result<(), SceneHostError> {
        let nodes = self.resolve_node_handles(nodes)?;
        let bounds =
            self.scene
                .fit_selection_with_assets(self.active_camera, nodes, &self.assets)?;
        self.cancel_camera_transition();
        self.camera_controls =
            controls_from_scene_camera(&self.scene, self.active_camera, bounds.center())?;
        Ok(())
    }

    fn resolve_node_handles(&self, nodes: &[u64]) -> Result<Vec<NodeKey>, SceneHostError> {
        nodes
            .iter()
            .copied()
            .map(|handle| self.resolve_node(handle))
            .collect()
    }
}
