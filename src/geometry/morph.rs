use crate::scene::Vec3;

use super::{GeometryDesc, GeometryError, GeometryVertex};

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryMorphTarget {
    position_deltas: Vec<Vec3>,
}

impl GeometryDesc {
    pub fn with_morph_targets(
        mut self,
        morph_targets: Vec<GeometryMorphTarget>,
    ) -> Result<Self, GeometryError> {
        for (target_index, target) in morph_targets.iter().enumerate() {
            if target.position_deltas.len() != self.vertices.len() {
                return Err(GeometryError::InvalidMorphTargetVertexCount {
                    vertex_count: self.vertices.len(),
                    target_index,
                    target_count: target.position_deltas.len(),
                });
            }
        }
        self.morph_targets = morph_targets;
        Ok(self)
    }

    pub fn morph_targets(&self) -> &[GeometryMorphTarget] {
        &self.morph_targets
    }

    pub fn morphed_vertices(&self, weights: &[f32]) -> Option<Vec<GeometryVertex>> {
        if self.morph_targets.is_empty() {
            return None;
        }
        let mut vertices = self.vertices.clone();
        for (target, weight) in self.morph_targets.iter().zip(weights.iter().copied()) {
            for (vertex, delta) in vertices.iter_mut().zip(target.position_deltas()) {
                vertex.position = Vec3::new(
                    vertex.position.x + delta.x * weight,
                    vertex.position.y + delta.y * weight,
                    vertex.position.z + delta.z * weight,
                );
            }
        }
        Some(vertices)
    }
}

impl GeometryMorphTarget {
    pub fn new(position_deltas: Vec<Vec3>) -> Self {
        Self { position_deltas }
    }

    pub fn position_deltas(&self) -> &[Vec3] {
        &self.position_deltas
    }
}
