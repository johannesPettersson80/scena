use crate::diagnostics::LookupError;

use super::{CameraKey, NodeKey, Scene};

impl Scene {
    pub fn set_visible(&mut self, node: NodeKey, visible: bool) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.visible != visible {
            node.visible = visible;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn visible(&self, node: NodeKey) -> Option<bool> {
        self.nodes.get(node).map(|node| node.visible)
    }

    pub fn add_tag(&mut self, node: NodeKey, tag: impl Into<String>) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.tags.insert(tag.into()) {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn has_tag(&self, node: NodeKey, tag: &str) -> bool {
        self.nodes
            .get(node)
            .is_some_and(|node| node.tags.contains(tag))
    }

    pub fn tagged<'scene>(
        &'scene self,
        tag: &'scene str,
    ) -> impl Iterator<Item = NodeKey> + 'scene {
        self.nodes
            .iter()
            .filter(move |(_, node)| node.tags.contains(tag))
            .map(|(node, _)| node)
    }

    pub fn set_layer_mask(&mut self, node: NodeKey, mask: u64) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.layer_mask != mask {
            node.layer_mask = mask;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn layer_mask(&self, node: NodeKey) -> Option<u64> {
        self.nodes.get(node).map(|node| node.layer_mask)
    }

    pub fn set_camera_layer_mask(
        &mut self,
        camera: CameraKey,
        mask: u64,
    ) -> Result<(), LookupError> {
        if !self.cameras.contains_key(camera) {
            return Err(LookupError::CameraNotFound(camera));
        }
        let current = self
            .camera_layer_masks
            .get(&camera)
            .copied()
            .unwrap_or(u64::MAX);
        if current != mask {
            self.camera_layer_masks.insert(camera, mask);
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn camera_layer_mask(&self, camera: CameraKey) -> Option<u64> {
        self.cameras.contains_key(camera).then(|| {
            self.camera_layer_masks
                .get(&camera)
                .copied()
                .unwrap_or(u64::MAX)
        })
    }

    pub fn set_render_group(&mut self, node: NodeKey, group: i16) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.render_group != group {
            node.render_group = group;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn render_group(&self, node: NodeKey) -> Option<i16> {
        self.nodes.get(node).map(|node| node.render_group)
    }

    pub fn set_helper_on_top(&mut self, node: NodeKey, on_top: bool) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.helper_on_top != on_top {
            node.helper_on_top = on_top;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn helper_on_top(&self, node: NodeKey) -> Option<bool> {
        self.nodes.get(node).map(|node| node.helper_on_top)
    }

    pub(crate) fn visible_in_hierarchy(&self, mut node: NodeKey) -> bool {
        loop {
            let Some(data) = self.nodes.get(node) else {
                return false;
            };
            if !data.visible {
                return false;
            }
            let Some(parent) = data.parent else {
                return true;
            };
            node = parent;
        }
    }

    pub(crate) fn visible_for_active_camera(&self, node: NodeKey) -> bool {
        let Some(node_data) = self.nodes.get(node) else {
            return false;
        };
        self.visible_in_hierarchy(node)
            && self.node_matches_active_camera_mask(node_data.layer_mask)
    }

    fn node_matches_active_camera_mask(&self, layer_mask: u64) -> bool {
        let Some(camera) = self.active_camera else {
            return true;
        };
        let camera_mask = self
            .camera_layer_masks
            .get(&camera)
            .copied()
            .unwrap_or(u64::MAX);
        camera_mask & layer_mask != 0
    }
}
