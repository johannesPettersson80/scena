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
                    scale_vec3(matrix.transform_direction(source.normal), weight),
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
