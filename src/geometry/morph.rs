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

    /// Applies the weighted morph targets to the vertex positions and normals.
    ///
    /// A morphed normal is the authored normal plus every weighted normal
    /// delta, normalized once. Normalizing after each target would rescale the
    /// partial sum and make the result depend on the order of the targets. A
    /// sum too short to normalize keeps the authored normal.
    pub fn morphed_vertices(&self, weights: &[f32]) -> Option<Vec<GeometryVertex>> {
        if self.morph_targets.is_empty() || !self.morph_weight_width_matches(weights) {
            return None;
        }
        let mut vertices = self.vertices.clone();
        let mut normals_morphed = false;
        for (target, weight) in self.morph_targets.iter().zip(weights.iter().copied()) {
            for (vertex, delta) in vertices.iter_mut().zip(target.position_deltas()) {
                vertex.position = Vec3::new(
                    vertex.position.x + delta.x * weight,
                    vertex.position.y + delta.y * weight,
                    vertex.position.z + delta.z * weight,
                );
            }
            if let Some(normal_deltas) = target.normal_deltas() {
                normals_morphed = true;
                for (vertex, delta) in vertices.iter_mut().zip(normal_deltas) {
                    vertex.normal = Vec3::new(
                        vertex.normal.x + delta.x * weight,
                        vertex.normal.y + delta.y * weight,
                        vertex.normal.z + delta.z * weight,
                    );
                }
            }
        }
        if normals_morphed {
            for (vertex, authored) in vertices.iter_mut().zip(&self.vertices) {
                vertex.normal = normalize_or(vertex.normal, authored.normal);
            }
        }
        Some(vertices)
    }

    /// Whether a weight vector matches this geometry's morph-target count.
    ///
    /// glTF requires the two to be equal. Zipping a mismatched pair would
    /// apply only the leading targets and report success, so a declared
    /// deformation would silently vanish from the render.
    pub fn morph_weight_width_matches(&self, weights: &[f32]) -> bool {
        weights.len() == self.morph_targets.len()
    }

    /// Applies glTF morph-target tangent deltas while retaining the authored
    /// tangent handedness. Render preparation subsequently orthogonalizes and
    /// normalizes the XYZ direction against the morphed normal.
    pub fn morphed_tangents(&self, weights: &[f32]) -> Option<Vec<[f32; 4]>> {
        if !self.morph_weight_width_matches(weights) {
            return None;
        }
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

#[cfg(test)]
mod r04_tests {
    use super::super::{GeometryDesc, GeometryMorphTarget, GeometryTopology, GeometryVertex};
    use crate::scene::Vec3;

    fn vertex(x: f32) -> GeometryVertex {
        GeometryVertex {
            position: Vec3::new(x, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
        }
    }

    fn two_target_geometry() -> GeometryDesc {
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![vertex(0.0), vertex(1.0), vertex(2.0)],
            vec![0, 1, 2],
        )
        .expect("triangle geometry builds")
        .with_morph_targets(vec![
            GeometryMorphTarget::new(vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
            ]),
            GeometryMorphTarget::new(vec![
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ]),
        ])
        .expect("two morph targets attach")
    }

    /// R04: a weight vector whose width does not match the geometry's morph
    /// target count must not be silently zipped. Truncation applies only the
    /// leading targets and reports success, so the render silently omits a
    /// declared deformation.
    #[test]
    fn morph_weight_width_mismatch_is_rejected_instead_of_truncated() {
        let geometry = two_target_geometry();

        assert!(
            geometry.morphed_vertices(&[1.0]).is_none(),
            "a short weight vector must be rejected, not zipped against the \
             first target only"
        );
        assert!(
            geometry.morphed_vertices(&[1.0, 0.0, 0.5]).is_none(),
            "an over-wide weight vector must be rejected rather than ignoring \
             the surplus"
        );

        let exact = geometry
            .morphed_vertices(&[1.0, 0.0])
            .expect("an exactly-sized weight vector still applies");
        assert_eq!(exact[0].position.x, 1.0);
    }

    fn normal_morph_geometry(targets: [Vec3; 2]) -> GeometryDesc {
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![vertex(0.0), vertex(1.0), vertex(2.0)],
            vec![0, 1, 2],
        )
        .expect("triangle geometry builds")
        .with_morph_targets(
            targets
                .into_iter()
                .map(|delta| {
                    GeometryMorphTarget::new_with_normals(vec![Vec3::ZERO; 3], vec![delta; 3])
                })
                .collect(),
        )
        .expect("two normal morph targets attach")
    }

    fn assert_normal_near(actual: Vec3, expected: Vec3) {
        assert!(
            (actual - expected).length() < 1.0e-6,
            "expected normal {expected:?}, got {actual:?}",
        );
    }

    /// glTF defines the morphed normal as the base normal plus every weighted
    /// delta. Only that sum is normalized: normalizing after each target
    /// rescales the partial sum, which changes the direction and makes the
    /// result depend on the order of the targets.
    #[test]
    fn morph_normals_sum_every_weighted_delta_before_normalizing_once() {
        let toward_x = Vec3::new(1.0, 0.0, 0.0);
        let toward_z = Vec3::new(0.0, 0.0, 1.0);
        // Base normal +Y, plus both deltas at full weight: (1, 1, 1).
        let expected = Vec3::new(1.0, 1.0, 1.0).normalize();

        let forward = normal_morph_geometry([toward_x, toward_z])
            .morphed_vertices(&[1.0, 1.0])
            .expect("weights match the target count");
        assert_normal_near(forward[0].normal, expected);

        let reversed = normal_morph_geometry([toward_z, toward_x])
            .morphed_vertices(&[1.0, 1.0])
            .expect("weights match the target count");
        assert_normal_near(reversed[0].normal, forward[0].normal);

        // Base (0, 1, 0) + 0.5 * (1, 0, 0) + 2.0 * (0, 0, 1).
        let weighted = normal_morph_geometry([toward_x, toward_z])
            .morphed_vertices(&[0.5, 2.0])
            .expect("weights match the target count");
        assert_normal_near(weighted[0].normal, Vec3::new(0.5, 1.0, 2.0).normalize());
    }

    /// Deltas that cancel the base normal leave nothing to normalize; the
    /// vertex keeps its authored normal instead of a zero or NaN vector.
    #[test]
    fn morph_normals_that_cancel_out_keep_the_authored_normal() {
        let away = Vec3::new(0.0, -0.5, 0.0);
        let morphed = normal_morph_geometry([away, away])
            .morphed_vertices(&[1.0, 1.0])
            .expect("weights match the target count");
        assert_normal_near(morphed[0].normal, Vec3::new(0.0, 1.0, 0.0));
    }
}
