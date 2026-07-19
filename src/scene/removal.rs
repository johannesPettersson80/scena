use std::collections::BTreeSet;

use crate::diagnostics::LookupError;

use super::{NodeKey, NodeKind, Scene, SceneImport, slot_index};
use crate::scene::transaction::SceneTransaction;

impl Scene {
    /// Removes `node` and its descendants from the scene graph.
    ///
    /// The root node is permanent and cannot be removed. Removing a subtree
    /// also drops node-owned cameras, lights, labels, instance sets, bounds,
    /// morph weights, skin bindings, anchors, measurements, and connectors that
    /// point at the removed nodes.
    pub fn remove_node(&mut self, node: NodeKey) -> Result<(), LookupError> {
        let removed = self.node_removal_closure(node)?;
        let mut transaction = SceneTransaction::new(self);
        transaction.scene().remove_nodes_unchecked(&removed);
        transaction.scene().assert_overlay_ownership_invariant();
        transaction.commit();
        Ok(())
    }

    /// Removes every root owned by an import and marks the import stale.
    pub fn remove_import(&mut self, import: &SceneImport) -> Result<(), LookupError> {
        import.ensure_live()?;
        if !import.belongs_to(self) {
            return Err(LookupError::ImportFromDifferentScene);
        }
        let roots = import.roots().to_vec();
        for root in &roots {
            if *root == self.root {
                return Err(LookupError::CannotRemoveRootNode(*root));
            }
            if !self.nodes.contains_key(*root) {
                return Err(LookupError::NodeNotFound(*root));
            }
        }

        let removed = roots
            .into_iter()
            .map(|root| self.node_removal_closure(root))
            .collect::<Result<Vec<_>, _>>()?;
        let mut transaction = SceneTransaction::new(self);
        for nodes in removed {
            transaction.scene().remove_nodes_unchecked(&nodes);
        }
        transaction.scene().assert_overlay_ownership_invariant();
        transaction.commit();
        import.mark_stale();
        Ok(())
    }

    pub(crate) fn subtree_nodes(&self, node: NodeKey) -> Result<Vec<NodeKey>, LookupError> {
        let mut nodes = Vec::new();
        self.append_subtree_nodes(node, &mut nodes)?;
        Ok(nodes)
    }

    pub(crate) fn node_removal_closure(&self, node: NodeKey) -> Result<Vec<NodeKey>, LookupError> {
        if node == self.root {
            return Err(LookupError::CannotRemoveRootNode(node));
        }
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        let mut removed = self.subtree_nodes(node)?;
        self.expand_overlay_removal_closure(&mut removed);
        Ok(removed)
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

    pub(in crate::scene) fn remove_nodes_unchecked(&mut self, removed: &[NodeKey]) {
        if removed.is_empty() {
            return;
        }
        let removed_set = removed.iter().copied().collect::<BTreeSet<_>>();

        for node in self.nodes.values_mut() {
            node.children.retain(|child| !removed_set.contains(child));
        }

        for node in removed.iter().rev().copied() {
            self.remove_single_node_storage(node);
        }
        self.remove_cross_node_metadata(&removed_set);
        self.structure_revision = self.structure_revision.saturating_add(1);
        self.transform_revision = self.transform_revision.saturating_add(1);
        self.assert_overlay_ownership_invariant();
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
        self.mesh_lods.remove(&node);
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
            if let Some(frame) = self.anchors.remove(anchor)
                && let Some(name) = frame.import_retirement_name()
            {
                self.retired_anchors
                    .insert(slot_index(anchor), (anchor, name));
            }
        }

        self.annotations.retain(|_, annotation| {
            !annotation
                .target_node()
                .is_some_and(|node| removed.contains(&node))
        });
        let removed_callouts = self
            .callouts
            .iter()
            .filter_map(|(id, callout)| callout.touches_removed_node(removed).then_some(id.clone()))
            .collect::<Vec<_>>();
        for id in removed_callouts {
            self.callouts.remove(&id);
            self.annotations.remove(&id);
        }
        self.measurements_mut()
            .retain(|_, measurement| !measurement.touches_removed_node(removed));
        self.overlay_owners
            .retain(|node, _| !removed.contains(node));

        let connectors = self
            .connectors
            .iter()
            .filter_map(|(key, connector)| removed.contains(&connector.node()).then_some(key))
            .collect::<Vec<_>>();
        for connector in connectors {
            if let Some(frame) = self.connectors.remove(connector)
                && let Some(name) = frame.import_retirement_name()
            {
                self.retired_connectors
                    .insert(slot_index(connector), (connector, name));
            }
        }

        self.skin_bindings.retain(|node, binding| {
            !removed.contains(node) && !binding.joints().iter().any(|joint| removed.contains(joint))
        });
    }
}
