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

    pub fn set_node_tint(&mut self, node: NodeKey, tint: Option<Color>) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.tint != tint {
            node.tint = tint;
            self.structure_revision = self.structure_revision.saturating_add(1);
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
