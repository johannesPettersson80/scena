use crate::diagnostics::LookupError;
use crate::geometry::SkinningMatrix;

use super::{NodeKey, Scene};

#[derive(Debug, Clone, PartialEq)]
pub struct SceneSkinBinding {
    joints: Vec<NodeKey>,
    inverse_bind_matrices: Vec<SkinningMatrix>,
}

impl Scene {
    pub fn skin_binding(&self, node: NodeKey) -> Option<&SceneSkinBinding> {
        self.skin_bindings.get(&node)
    }

    pub fn skin_matrices(&self, node: NodeKey) -> Option<Vec<SkinningMatrix>> {
        let binding = self.skin_bindings.get(&node)?;
        let mesh_inverse = SkinningMatrix::inverse_from_transform(self.world_transform(node)?);
        binding
            .joints
            .iter()
            .zip(binding.inverse_bind_matrices.iter().copied())
            .map(|(joint, inverse_bind)| {
                let joint_world = SkinningMatrix::from_transform(self.world_transform(*joint)?);
                Some(mesh_inverse.then(joint_world).then(inverse_bind))
            })
            .collect()
    }

    pub(crate) fn set_initial_skin_binding(&mut self, node: NodeKey, binding: SceneSkinBinding) {
        self.skin_bindings.insert(node, binding);
    }

    pub fn set_skin_binding(
        &mut self,
        node: NodeKey,
        binding: SceneSkinBinding,
    ) -> Result<(), LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        if binding.joints.len() != binding.inverse_bind_matrices.len() {
            return Err(LookupError::InvalidSkinBinding {
                joint_count: binding.joints.len(),
                inverse_bind_count: binding.inverse_bind_matrices.len(),
            });
        }
        for joint in &binding.joints {
            if !self.nodes.contains_key(*joint) {
                return Err(LookupError::NodeNotFound(*joint));
            }
        }
        if self.skin_bindings.get(&node) == Some(&binding) {
            return Ok(());
        }
        self.skin_bindings.insert(node, binding);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(())
    }
}

impl SceneSkinBinding {
    pub fn new(joints: Vec<NodeKey>, inverse_bind_matrices: Vec<SkinningMatrix>) -> Self {
        Self {
            joints,
            inverse_bind_matrices,
        }
    }

    pub fn joints(&self) -> &[NodeKey] {
        &self.joints
    }

    pub fn inverse_bind_matrices(&self) -> &[SkinningMatrix] {
        &self.inverse_bind_matrices
    }
}
