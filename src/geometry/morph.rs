use crate::scene::Vec3;

use super::{GeometryDesc, GeometryError, GeometryVertex};

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryMorphTarget {
    position_deltas: Vec<Vec3>,
    normal_deltas: Option<Vec<Vec3>>,
    tangent_deltas: Option<Vec<Vec3>>,
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
            if let Some(normal_deltas) = target.normal_deltas.as_ref()
                && normal_deltas.len() != self.vertices.len()
            {
                return Err(GeometryError::InvalidMorphTargetVertexCount {
                    vertex_count: self.vertices.len(),
                    target_index,
                    target_count: normal_deltas.len(),
                });
            }
            if let Some(tangent_deltas) = target.tangent_deltas.as_ref()
                && tangent_deltas.len() != self.vertices.len()
            {
                return Err(GeometryError::InvalidMorphTargetVertexCount {
                    vertex_count: self.vertices.len(),
                    target_index,
                    target_count: tangent_deltas.len(),
                });
            }
        }
        self.morph_targets = morph_targets;
        self.generated_tangent_cache = Default::default();
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
            if let Some(normal_deltas) = target.normal_deltas() {
                for (vertex, delta) in vertices.iter_mut().zip(normal_deltas) {
                    vertex.normal = normalize_or(
                        Vec3::new(
                            vertex.normal.x + delta.x * weight,
                            vertex.normal.y + delta.y * weight,
                            vertex.normal.z + delta.z * weight,
                        ),
                        vertex.normal,
                    );
                }
            }
        }
        Some(vertices)
    }

    /// Applies glTF morph-target tangent deltas while retaining the authored
    /// tangent handedness. Render preparation subsequently orthogonalizes and
    /// normalizes the XYZ direction against the morphed normal.
    pub fn morphed_tangents(&self, weights: &[f32]) -> Option<Vec<[f32; 4]>> {
        let mut tangents = self.tangents.clone()?;
        for (target, weight) in self.morph_targets.iter().zip(weights.iter().copied()) {
            let Some(tangent_deltas) = target.tangent_deltas() else {
                continue;
            };
            for (tangent, delta) in tangents.iter_mut().zip(tangent_deltas) {
                tangent[0] += delta.x * weight;
                tangent[1] += delta.y * weight;
                tangent[2] += delta.z * weight;
            }
        }
        Some(tangents)
    }
}

impl GeometryMorphTarget {
    pub fn new(position_deltas: Vec<Vec3>) -> Self {
        Self {
            position_deltas,
            normal_deltas: None,
            tangent_deltas: None,
        }
    }

    pub fn new_with_normals(position_deltas: Vec<Vec3>, normal_deltas: Vec<Vec3>) -> Self {
        Self {
            position_deltas,
            normal_deltas: Some(normal_deltas),
            tangent_deltas: None,
        }
    }

    pub fn new_with_semantics(
        position_deltas: Vec<Vec3>,
        normal_deltas: Option<Vec<Vec3>>,
        tangent_deltas: Option<Vec<Vec3>>,
    ) -> Self {
        Self {
            position_deltas,
            normal_deltas,
            tangent_deltas,
        }
    }

    pub fn position_deltas(&self) -> &[Vec3] {
        &self.position_deltas
    }

    pub fn normal_deltas(&self) -> Option<&[Vec3]> {
        self.normal_deltas.as_deref()
    }

    pub fn tangent_deltas(&self) -> Option<&[Vec3]> {
        self.tangent_deltas.as_deref()
    }
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}
