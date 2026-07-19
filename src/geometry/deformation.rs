use std::borrow::Cow;

use super::{GeometryDesc, GeometryError, GeometryVertex, SkinningMatrix};

impl GeometryDesc {
    /// Evaluates the canonical renderer pose: morph targets first, then skinning.
    ///
    /// Render preparation, shadow preparation, and picking share this function
    /// so an interaction query cannot silently inspect a different vertex pose
    /// from the one submitted for rendering.
    pub(crate) fn deformed_vertices<'a>(
        &'a self,
        morph_weights: Option<&[f32]>,
        skin_matrices: Option<&[SkinningMatrix]>,
    ) -> Result<Cow<'a, [GeometryVertex]>, GeometryError> {
        let morphed = morph_weights.and_then(|weights| self.morphed_vertices(weights));
        let base_vertices = morphed.as_deref().unwrap_or_else(|| self.vertices());

        match skin_matrices {
            Some(matrices) => self
                .skinned_vertices(base_vertices, matrices)
                .map(|skinned| match skinned {
                    Some(vertices) => Cow::Owned(vertices),
                    None => morphed.map_or_else(|| Cow::Borrowed(self.vertices()), Cow::Owned),
                }),
            None if self.skin().is_some() => Err(GeometryError::MissingSkinMatrices),
            None => Ok(morphed.map_or_else(|| Cow::Borrowed(self.vertices()), Cow::Owned)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{GeometryMorphTarget, GeometrySkin, GeometryTopology};
    use crate::scene::Vec3;

    #[test]
    fn canonical_deformation_applies_morph_before_skin() {
        let geometry = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![vertex(-0.5, -0.5), vertex(0.5, -0.5), vertex(0.0, 0.5)],
            vec![0, 1, 2],
        )
        .unwrap()
        .with_morph_targets(vec![GeometryMorphTarget::new(vec![Vec3::X; 3])])
        .unwrap()
        .with_skin(GeometrySkin::new(
            vec![[0, 0, 0, 0]; 3],
            vec![[1.0, 0.0, 0.0, 0.0]; 3],
        ))
        .unwrap();
        let skin = SkinningMatrix::from_transform(crate::scene::Transform::at(Vec3::Y));

        let vertices = geometry
            .deformed_vertices(Some(&[1.0]), Some(&[skin]))
            .expect("canonical deformation evaluates");

        assert_eq!(vertices[2].position, Vec3::new(1.0, 1.5, 0.0));
    }

    fn vertex(x: f32, y: f32) -> GeometryVertex {
        GeometryVertex {
            position: Vec3::new(x, y, 0.0),
            normal: Vec3::Z,
        }
    }
}
