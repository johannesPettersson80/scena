use crate::assets::MaterialHandle;
use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{NodeKey, NodeKind, Scene};

impl Scene {
    pub fn set_mesh_material(
        &mut self,
        node: NodeKey,
        material: MaterialHandle,
    ) -> Result<(), LookupError> {
        let node_data = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        let NodeKind::Mesh(mesh) = &mut node_data.kind else {
            return Err(LookupError::NodeIsNotMesh { node });
        };
        if mesh.material != material {
            mesh.material = material;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn set_subtree_mesh_material(
        &mut self,
        root: NodeKey,
        material: MaterialHandle,
    ) -> Result<usize, LookupError> {
        let nodes = self.subtree_nodes(root)?;
        let mut changed = 0usize;
        for node in nodes {
            let Some(node_data) = self.nodes.get_mut(node) else {
                return Err(LookupError::NodeNotFound(node));
            };
            let NodeKind::Mesh(mesh) = &mut node_data.kind else {
                continue;
            };
            if mesh.material != material {
                mesh.material = material;
                changed = changed.saturating_add(1);
            }
        }
        if changed > 0 {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(changed)
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn add_subtree_edge_overlays(
        &mut self,
        root: NodeKey,
        material: MaterialHandle,
    ) -> Result<Vec<NodeKey>, LookupError> {
        let nodes = self.subtree_nodes(root)?;
        let mut overlays = Vec::new();
        for node in nodes {
            let Some(node_data) = self.nodes.get(node) else {
                return Err(LookupError::NodeNotFound(node));
            };
            let NodeKind::Mesh(mesh) = &node_data.kind else {
                continue;
            };
            overlays.push((
                node_data.parent.unwrap_or(self.root),
                node_data.transform,
                mesh.geometry,
                node_data.visible,
                node_data.layer_mask,
                node_data.render_group.saturating_add(1),
                self.node_bounds.get(&node).copied(),
            ));
        }

        let mut added = Vec::new();
        for (parent, transform, geometry, visible, layer_mask, render_group, bounds) in overlays {
            let overlay = self.insert_node(
                parent,
                NodeKind::Mesh(super::MeshNode { geometry, material }),
                transform,
            )?;
            if let Some(bounds) = bounds {
                self.node_bounds.insert(overlay, bounds);
            }
            if let Some(node_data) = self.nodes.get_mut(overlay) {
                node_data.visible = visible;
                node_data.layer_mask = layer_mask;
                node_data.render_group = render_group;
                node_data.helper_on_top = true;
            }
            added.push(overlay);
        }
        Ok(added)
    }

    pub fn set_node_tint(&mut self, node: NodeKey, tint: Option<Color>) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.tint != tint {
            let structural = tint_requires_structure_revision(node.tint)
                || tint_requires_structure_revision(tint);
            node.tint = tint;
            if structural {
                self.structure_revision = self.structure_revision.saturating_add(1);
            } else {
                self.appearance_revision = self.appearance_revision.saturating_add(1);
            }
        }
        Ok(())
    }

    pub fn node_tint(&self, node: NodeKey) -> Result<Option<Color>, LookupError> {
        self.nodes
            .get(node)
            .map(|node| node.tint)
            .ok_or(LookupError::NodeNotFound(node))
    }
}

fn tint_requires_structure_revision(tint: Option<Color>) -> bool {
    tint.is_some_and(|tint| tint.a < 1.0)
}
