use std::collections::BTreeSet;

use crate::diagnostics::LookupError;

use super::{NodeKey, NodeKind, Scene, SceneImport};

impl Scene {
    /// Removes `node` and its descendants from the scene graph.
    ///
    /// The root node is permanent and cannot be removed. Removing a subtree
    /// also drops node-owned cameras, lights, labels, instance sets, bounds,
    /// morph weights, skin bindings, anchors, and connectors that point at the
    /// removed nodes.
    pub fn remove_node(&mut self, node: NodeKey) -> Result<(), LookupError> {
        if node == self.root {
            return Err(LookupError::CannotRemoveRootNode(node));
        }
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        let removed = self.subtree_nodes(node)?;
        self.remove_nodes_unchecked(&removed);
        Ok(())
    }

    /// Removes every root owned by an import and marks the import stale.
    pub fn remove_import(&mut self, import: &SceneImport) -> Result<(), LookupError> {
        import.ensure_live()?;
        let roots = import.roots().to_vec();
        for root in &roots {
            if *root == self.root {
                return Err(LookupError::CannotRemoveRootNode(*root));
            }
            if !self.nodes.contains_key(*root) {
                return Err(LookupError::NodeNotFound(*root));
            }
        }

        for root in roots {
            let removed = self.subtree_nodes(root)?;
            self.remove_nodes_unchecked(&removed);
        }
        import.mark_stale();
        Ok(())
    }

    pub(crate) fn subtree_nodes(&self, node: NodeKey) -> Result<Vec<NodeKey>, LookupError> {
        let mut nodes = Vec::new();
        self.append_subtree_nodes(node, &mut nodes)?;
        Ok(nodes)
    }

    fn append_subtree_nodes(
        &self,
        node: NodeKey,
        nodes: &mut Vec<NodeKey>,
    ) -> Result<(), LookupError> {
        let node_ref = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        nodes.push(node);
        let children = node_ref.children.clone();
        for child in children {
            self.append_subtree_nodes(child, nodes)?;
        }
        Ok(())
    }

    fn remove_nodes_unchecked(&mut self, removed: &[NodeKey]) {
        if removed.is_empty() {
            return;
        }
        let removed_set = removed.iter().copied().collect::<BTreeSet<_>>();

        if let Some(parent) = self.nodes.get(removed[0]).and_then(|node| node.parent)
            && let Some(parent_node) = self.nodes.get_mut(parent)
        {
            parent_node.children.retain(|child| *child != removed[0]);
        }

        for node in removed.iter().rev().copied() {
            self.remove_single_node_storage(node);
        }
        self.remove_cross_node_metadata(&removed_set);
        self.structure_revision = self.structure_revision.saturating_add(1);
        self.transform_revision = self.transform_revision.saturating_add(1);
    }

    fn remove_single_node_storage(&mut self, node: NodeKey) {
        let Some(removed) = self.nodes.remove(node) else {
            return;
        };
        match removed.kind {
            NodeKind::Camera(camera) => {
                self.cameras.remove(camera);
                self.camera_layer_masks.remove(&camera);
                if self.active_camera == Some(camera) {
                    self.active_camera = None;
                }
            }
            NodeKind::Light(light) => {
                self.lights.remove(light);
            }
            NodeKind::InstanceSet(instance_set) => {
                self.instance_sets.remove(instance_set);
            }
            NodeKind::ParticleSet(particle_set) => {
                self.particle_sets.remove(particle_set);
            }
            NodeKind::Label(label) => {
                self.labels.remove(label);
            }
            NodeKind::Empty | NodeKind::Renderable(_) | NodeKind::Mesh(_) | NodeKind::Model(_) => {}
        }
        self.node_bounds.remove(&node);
        self.morph_weights.remove(&node);
        self.skin_bindings.remove(&node);
        self.connection_locked_nodes.remove(&node);
    }

    fn remove_cross_node_metadata(&mut self, removed: &BTreeSet<NodeKey>) {
        let anchors = self
            .anchors
            .iter()
            .filter_map(|(key, anchor)| removed.contains(&anchor.node()).then_some(key))
            .collect::<Vec<_>>();
        for anchor in anchors {
            self.anchors.remove(anchor);
        }

        self.annotations.retain(|_, annotation| {
            !annotation
                .target_node()
                .is_some_and(|node| removed.contains(&node))
        });
        self.callouts_mut()
            .retain(|_, callout| !callout.touches_removed_node(removed));

        let connectors = self
            .connectors
            .iter()
            .filter_map(|(key, connector)| removed.contains(&connector.node()).then_some(key))
            .collect::<Vec<_>>();
        for connector in connectors {
            self.connectors.remove(connector);
        }

        self.skin_bindings.retain(|node, binding| {
            !removed.contains(node) && !binding.joints().iter().any(|joint| removed.contains(joint))
        });
    }
}
