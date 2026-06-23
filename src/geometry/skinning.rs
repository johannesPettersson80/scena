use crate::scene::{Quat, Transform, Vec3};

use super::{GeometryDesc, GeometryError, GeometryVertex};

#[derive(Debug, Clone, PartialEq)]
pub struct GeometrySkin {
    joints: Vec<[usize; 4]>,
    weights: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinningMatrix {
    rows: [[f32; 4]; 4],
}

impl GeometryDesc {
    pub fn with_skin(mut self, skin: GeometrySkin) -> Result<Self, GeometryError> {
        if skin.joints.len() != self.vertices.len() {
            return Err(GeometryError::InvalidSkinJointVertexCount {
                vertex_count: self.vertices.len(),
                joint_count: skin.joints.len(),
            });
        }
        if skin.weights.len() != self.vertices.len() {
            return Err(GeometryError::InvalidSkinWeightVertexCount {
                vertex_count: self.vertices.len(),
                weight_count: skin.weights.len(),
            });
        }
        self.skin = Some(skin);
        Ok(self)
    }

    pub fn skin(&self) -> Option<&GeometrySkin> {
        self.skin.as_ref()
    }

    pub fn skinned_vertices(
        &self,
        source_vertices: &[GeometryVertex],
        joint_matrices: &[SkinningMatrix],
    ) -> Result<Option<Vec<GeometryVertex>>, GeometryError> {
        let Some(skin) = &self.skin else {
            return Ok(None);
        };
        if source_vertices.len() != self.vertices.len() {
            return Err(GeometryError::InvalidSkinSourceVertexCount {
                vertex_count: self.vertices.len(),
                source_count: source_vertices.len(),
            });
        }
        let mut vertices = Vec::with_capacity(source_vertices.len());
        for (vertex_index, source) in source_vertices.iter().enumerate() {
            let mut position = Vec3::ZERO;
            let mut normal = Vec3::ZERO;
            for influence in 0..4 {
                let weight = skin.weights[vertex_index][influence];
                if weight == 0.0 {
                    continue;
                }
                let joint = skin.joints[vertex_index][influence];
                let matrix =
                    joint_matrices
                        .get(joint)
                        .ok_or(GeometryError::InvalidSkinJointIndex {
                            vertex_index,
                            joint,
                            joint_count: joint_matrices.len(),
                        })?;
                position = add_vec3(
                    position,
                    scale_vec3(matrix.transform_position(source.position), weight),
                );
                normal = add_vec3(
                    normal,
                    scale_vec3(matrix.transform_normal(source.normal), weight),
                );
            }
            vertices.push(GeometryVertex {
                position,
                normal: normalize_or(normal, source.normal),
            });
        }
        Ok(Some(vertices))
    }
}

impl GeometrySkin {
    pub fn new(joints: Vec<[usize; 4]>, weights: Vec<[f32; 4]>) -> Self {
        Self { joints, weights }
    }

    pub fn joints(&self) -> &[[usize; 4]] {
        &self.joints
    }

    pub fn influence_indices(&self) -> &[[usize; 4]] {
        &self.joints
    }

    pub fn weights(&self) -> &[[f32; 4]] {
        &self.weights
    }
}

impl SkinningMatrix {
    pub const IDENTITY: Self = Self {
        rows: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub fn from_gltf_column_major(values: [f32; 16]) -> Self {
        Self {
            rows: [
                [values[0], values[4], values[8], values[12]],
                [values[1], values[5], values[9], values[13]],
                [values[2], values[6], values[10], values[14]],
                [values[3], values[7], values[11], values[15]],
            ],
        }
    }

    pub fn from_transform(transform: Transform) -> Self {
        let translation = Self::translation(transform.translation);
        let rotation = Self::rotation(transform.rotation);
        let scale = Self::scale(transform.scale);
        translation.then(rotation).then(scale)
    }

    pub fn inverse_from_transform(transform: Transform) -> Self {
        let scale = Self::scale(Vec3::new(
            reciprocal_or_zero(transform.scale.x),
            reciprocal_or_zero(transform.scale.y),
            reciprocal_or_zero(transform.scale.z),
        ));
        let rotation = Self::rotation(inverse_quat(transform.rotation));
        let translation = Self::translation(Vec3::new(
            -transform.translation.x,
            -transform.translation.y,
            -transform.translation.z,
        ));
        scale.then(rotation).then(translation)
    }

    pub fn then(self, other: Self) -> Self {
        let mut rows = [[0.0; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value = self.rows[row_index][0] * other.rows[0][column_index]
                    + self.rows[row_index][1] * other.rows[1][column_index]
                    + self.rows[row_index][2] * other.rows[2][column_index]
                    + self.rows[row_index][3] * other.rows[3][column_index];
            }
        }
        Self { rows }
    }

    pub fn transform_position(self, position: Vec3) -> Vec3 {
        Vec3::new(
            self.rows[0][0] * position.x
                + self.rows[0][1] * position.y
                + self.rows[0][2] * position.z
                + self.rows[0][3],
            self.rows[1][0] * position.x
                + self.rows[1][1] * position.y
                + self.rows[1][2] * position.z
                + self.rows[1][3],
            self.rows[2][0] * position.x
                + self.rows[2][1] * position.y
                + self.rows[2][2] * position.z
                + self.rows[2][3],
        )
    }

    pub fn transform_direction(self, direction: Vec3) -> Vec3 {
        Vec3::new(
            self.rows[0][0] * direction.x
                + self.rows[0][1] * direction.y
                + self.rows[0][2] * direction.z,
            self.rows[1][0] * direction.x
                + self.rows[1][1] * direction.y
                + self.rows[1][2] * direction.z,
            self.rows[2][0] * direction.x
                + self.rows[2][1] * direction.y
                + self.rows[2][2] * direction.z,
        )
    }

    pub fn transform_normal(self, normal: Vec3) -> Vec3 {
        let m00 = self.rows[0][0];
        let m01 = self.rows[0][1];
        let m02 = self.rows[0][2];
        let m10 = self.rows[1][0];
        let m11 = self.rows[1][1];
        let m12 = self.rows[1][2];
        let m20 = self.rows[2][0];
        let m21 = self.rows[2][1];
        let m22 = self.rows[2][2];

        let c00 = m11 * m22 - m12 * m21;
        let c01 = m12 * m20 - m10 * m22;
        let c02 = m10 * m21 - m11 * m20;
        let c10 = m02 * m21 - m01 * m22;
        let c11 = m00 * m22 - m02 * m20;
        let c12 = m01 * m20 - m00 * m21;
        let c20 = m01 * m12 - m02 * m11;
        let c21 = m02 * m10 - m00 * m12;
        let c22 = m00 * m11 - m01 * m10;
        let determinant = m00 * c00 + m01 * c01 + m02 * c02;
        if determinant.abs() <= f32::EPSILON || !determinant.is_finite() {
            return self.transform_direction(normal);
        }
        let inverse_determinant = determinant.recip();
        Vec3::new(
            (c00 * normal.x + c01 * normal.y + c02 * normal.z) * inverse_determinant,
            (c10 * normal.x + c11 * normal.y + c12 * normal.z) * inverse_determinant,
            (c20 * normal.x + c21 * normal.y + c22 * normal.z) * inverse_determinant,
        )
    }

    fn translation(translation: Vec3) -> Self {
        let mut matrix = Self::IDENTITY;
        matrix.rows[0][3] = translation.x;
        matrix.rows[1][3] = translation.y;
        matrix.rows[2][3] = translation.z;
        matrix
    }

    fn scale(scale: Vec3) -> Self {
        Self {
            rows: [
                [scale.x, 0.0, 0.0, 0.0],
                [0.0, scale.y, 0.0, 0.0],
                [0.0, 0.0, scale.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn rotation(rotation: Quat) -> Self {
        let rotation = normalize_quat(rotation);
        let x2 = rotation.x + rotation.x;
        let y2 = rotation.y + rotation.y;
        let z2 = rotation.z + rotation.z;
        let xx = rotation.x * x2;
        let xy = rotation.x * y2;
        let xz = rotation.x * z2;
        let yy = rotation.y * y2;
        let yz = rotation.y * z2;
        let zz = rotation.z * z2;
        let wx = rotation.w * x2;
        let wy = rotation.w * y2;
        let wz = rotation.w * z2;
        Self {
            rows: [
                [1.0 - (yy + zz), xy - wz, xz + wy, 0.0],
                [xy + wz, 1.0 - (xx + zz), yz - wx, 0.0],
                [xz - wy, yz + wx, 1.0 - (xx + yy), 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

fn reciprocal_or_zero(value: f32) -> f32 {
    if value.abs() <= f32::EPSILON || !value.is_finite() {
        0.0
    } else {
        value.recip()
    }
}

fn inverse_quat(value: Quat) -> Quat {
    let normalized = normalize_quat(value);
    Quat::from_xyzw(-normalized.x, -normalized.y, -normalized.z, normalized.w)
}

fn normalize_quat(value: Quat) -> Quat {
    let length_squared =
        value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return Quat::IDENTITY;
    }
    let inverse_length = length_squared.sqrt().recip();
    Quat::from_xyzw(
        value.x * inverse_length,
        value.y * inverse_length,
        value.z * inverse_length,
        value.w * inverse_length,
    )
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn scale_vec3(vector: Vec3, scale: f32) -> Vec3 {
    Vec3::new(vector.x * scale, vector.y * scale, vector.z * scale)
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
mod tests {
    use super::*;
    use crate::geometry::GeometryTopology;

    #[test]
    fn skinned_normals_use_inverse_transpose_under_nonuniform_joint_scale() {
        let diagonal = normalize_or(Vec3::new(1.0, 1.0, 0.0), Vec3::Y);
        let geometry = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                GeometryVertex {
                    position: Vec3::ZERO,
                    normal: diagonal,
                },
                GeometryVertex {
                    position: Vec3::X,
                    normal: diagonal,
                },
                GeometryVertex {
                    position: Vec3::Y,
                    normal: diagonal,
                },
            ],
            vec![0, 1, 2],
        )
        .expect("geometry validates")
        .with_skin(GeometrySkin::new(
            vec![[0, 0, 0, 0]; 3],
            vec![[1.0, 0.0, 0.0, 0.0]; 3],
        ))
        .expect("skin validates");
        let joint = SkinningMatrix::from_transform(Transform {
            scale: Vec3::new(2.0, 1.0, 1.0),
            ..Transform::default()
        });
        let skinned = geometry
            .skinned_vertices(geometry.vertices(), &[joint])
            .expect("skinning succeeds")
            .expect("skinned vertices are produced");

        let expected = normalize_or(Vec3::new(0.5, 1.0, 0.0), Vec3::Y);
        assert_vec3_near(skinned[0].normal, expected);
    }

    fn assert_vec3_near(actual: Vec3, expected: Vec3) {
        const EPSILON: f32 = 0.0001;
        assert!(
            (actual.x - expected.x).abs() <= EPSILON
                && (actual.y - expected.y).abs() <= EPSILON
                && (actual.z - expected.z).abs() <= EPSILON,
            "expected {actual:?} to be within {EPSILON} of {expected:?}"
        );
    }
}
