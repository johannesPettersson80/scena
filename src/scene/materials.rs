use crate::assets::MaterialHandle;
use crate::diagnostics::LookupError;

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
}
