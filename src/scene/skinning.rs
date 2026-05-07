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
