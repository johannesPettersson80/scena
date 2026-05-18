use crate::geometry::{Primitive, PrimitiveVertexAttributes, Vertex};
use crate::scene::{Quat, Transform, Vec3};

pub(super) fn transform_primitive(
    primitive: &Primitive,
    transform: Transform,
    origin_shift: Vec3,
) -> Primitive {
    let [a, b, c] = primitive.vertices();
    let [attributes_a, attributes_b, attributes_c] = primitive.vertex_attributes();
    let transformed = Primitive::triangle_with_attributes(
        [
            transform_vertex(*a, transform, origin_shift),
            transform_vertex(*b, transform, origin_shift),
            transform_vertex(*c, transform, origin_shift),
        ],
        [
            transform_vertex_attributes(*attributes_a, transform),
            transform_vertex_attributes(*attributes_b, transform),
            transform_vertex_attributes(*attributes_c, transform),
        ],
    )
    .with_render_material_slot(primitive.render_material_slot());
    if primitive.depth_prepass_eligible() {
        transformed
    } else {
        transformed.without_depth_prepass()
    }
}

pub(super) fn prepared_primitive(
    primitive: &Primitive,
    transform: Transform,
    origin_shift: Vec3,
) -> Primitive {
    let world_from_model = world_from_model_matrix(transform, origin_shift);
    let normal_from_model = normal_from_model_matrix(transform);
    transform_primitive(primitive, transform, origin_shift)
        .with_world_from_model(world_from_model, normal_from_model)
}

/// Returns the model-space position of a world-baked vertex by applying the
/// inverse of the matrix that produced the bake. Used by the GPU vertex
/// upload path to recover model-space data so the GPU vertex shader can
/// apply the per-draw `world_from_model` matrix without double-transforming.
/// CPU consumers (picking, culling, CPU rasterization, shadow occluders)
/// continue to read world-baked vertices unchanged. Closes
/// scena-wgpu-architect Phase 6 finding F2 for the GPU path.
pub(crate) fn unbake_position_to_model_space(
    world_baked: Vec3,
    world_from_model_inverse: &[f32; 16],
) -> Vec3 {
    apply_matrix4_to_vec3(world_from_model_inverse, world_baked, 1.0)
}

pub(crate) fn unbake_normal_to_model_space(
    world_baked_normal: Vec3,
    normal_from_model_inverse: &[f32; 16],
) -> Vec3 {
    apply_matrix4_to_vec3(normal_from_model_inverse, world_baked_normal, 0.0)
}

pub(crate) fn invert_matrix4(matrix: &[f32; 16]) -> Option<[f32; 16]> {
    // Standard 4x4 cofactor inverse. Returns None if the matrix is singular.
    let m = matrix;
    let mut inv = [0.0_f32; 16];
    inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
        + m[9] * m[7] * m[14]
        + m[13] * m[6] * m[11]
        - m[13] * m[7] * m[10];
    inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
        - m[8] * m[7] * m[14]
        - m[12] * m[6] * m[11]
        + m[12] * m[7] * m[10];
    inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
        + m[8] * m[7] * m[13]
        + m[12] * m[5] * m[11]
        - m[12] * m[7] * m[9];
    inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
        - m[8] * m[6] * m[13]
        - m[12] * m[5] * m[10]
        + m[12] * m[6] * m[9];
    inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
        - m[9] * m[3] * m[14]
        - m[13] * m[2] * m[11]
        + m[13] * m[3] * m[10];
    inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
        + m[8] * m[3] * m[14]
        + m[12] * m[2] * m[11]
        - m[12] * m[3] * m[10];
    inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
        - m[8] * m[3] * m[13]
        - m[12] * m[1] * m[11]
        + m[12] * m[3] * m[9];
    inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
        + m[8] * m[2] * m[13]
        + m[12] * m[1] * m[10]
        - m[12] * m[2] * m[9];
    inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
        + m[5] * m[3] * m[14]
        + m[13] * m[2] * m[7]
        - m[13] * m[3] * m[6];
    inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
        - m[4] * m[3] * m[14]
        - m[12] * m[2] * m[7]
        + m[12] * m[3] * m[6];
    inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
        + m[4] * m[3] * m[13]
        + m[12] * m[1] * m[7]
        - m[12] * m[3] * m[5];
    inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
        - m[4] * m[2] * m[13]
        - m[12] * m[1] * m[6]
        + m[12] * m[2] * m[5];
    inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
        - m[5] * m[3] * m[10]
        - m[9] * m[2] * m[7]
        + m[9] * m[3] * m[6];
    inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
        + m[4] * m[3] * m[10]
        + m[8] * m[2] * m[7]
        - m[8] * m[3] * m[6];
    inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
        - m[4] * m[3] * m[9]
        - m[8] * m[1] * m[7]
        + m[8] * m[3] * m[5];
    inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
        + m[4] * m[2] * m[9]
        + m[8] * m[1] * m[6]
        - m[8] * m[2] * m[5];

    let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
    if det.abs() <= f32::EPSILON || !det.is_finite() {
        return None;
    }
    let inv_det = 1.0 / det;
    for value in inv.iter_mut() {
        *value *= inv_det;
    }
    Some(inv)
}

fn apply_matrix4_to_vec3(matrix: &[f32; 16], vector: Vec3, w: f32) -> Vec3 {
    Vec3::new(
        matrix[0] * vector.x + matrix[4] * vector.y + matrix[8] * vector.z + matrix[12] * w,
        matrix[1] * vector.x + matrix[5] * vector.y + matrix[9] * vector.z + matrix[13] * w,
        matrix[2] * vector.x + matrix[6] * vector.y + matrix[10] * vector.z + matrix[14] * w,
    )
}

pub(in crate::render) fn world_from_model_matrix(
    transform: Transform,
    origin_shift: Vec3,
) -> [f32; 16] {
    let s = transform.scale;
    let qx = transform.rotation.x;
    let qy = transform.rotation.y;
    let qz = transform.rotation.z;
    let qw = transform.rotation.w;
    let length_squared = qx * qx + qy * qy + qz * qz + qw * qw;
    let (rx, ry, rz, rw) = if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        (0.0, 0.0, 0.0, 1.0)
    } else {
        let inverse_length = length_squared.sqrt().recip();
        (
            qx * inverse_length,
            qy * inverse_length,
            qz * inverse_length,
            qw * inverse_length,
        )
    };
    let xx = rx * rx;
    let yy = ry * ry;
    let zz = rz * rz;
    let xy = rx * ry;
    let xz = rx * rz;
    let yz = ry * rz;
    let wx = rw * rx;
    let wy = rw * ry;
    let wz = rw * rz;
    let m00 = (1.0 - 2.0 * (yy + zz)) * s.x;
    let m01 = (2.0 * (xy + wz)) * s.x;
    let m02 = (2.0 * (xz - wy)) * s.x;
    let m10 = (2.0 * (xy - wz)) * s.y;
    let m11 = (1.0 - 2.0 * (xx + zz)) * s.y;
    let m12 = (2.0 * (yz + wx)) * s.y;
    let m20 = (2.0 * (xz + wy)) * s.z;
    let m21 = (2.0 * (yz - wx)) * s.z;
    let m22 = (1.0 - 2.0 * (xx + yy)) * s.z;
    let tx = transform.translation.x - origin_shift.x;
    let ty = transform.translation.y - origin_shift.y;
    let tz = transform.translation.z - origin_shift.z;
    [
        m00, m01, m02, 0.0, m10, m11, m12, 0.0, m20, m21, m22, 0.0, tx, ty, tz, 1.0,
    ]
}

pub(in crate::render) fn normal_from_model_matrix(transform: Transform) -> [f32; 16] {
    let qx = transform.rotation.x;
    let qy = transform.rotation.y;
    let qz = transform.rotation.z;
    let qw = transform.rotation.w;
    let length_squared = qx * qx + qy * qy + qz * qz + qw * qw;
    let (rx, ry, rz, rw) = if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        (0.0, 0.0, 0.0, 1.0)
    } else {
        let inverse_length = length_squared.sqrt().recip();
        (
            qx * inverse_length,
            qy * inverse_length,
            qz * inverse_length,
            qw * inverse_length,
        )
    };
    let xx = rx * rx;
    let yy = ry * ry;
    let zz = rz * rz;
    let xy = rx * ry;
    let xz = rx * rz;
    let yz = ry * rz;
    let wx = rw * rx;
    let wy = rw * ry;
    let wz = rw * rz;
    let m00 = 1.0 - 2.0 * (yy + zz);
    let m01 = 2.0 * (xy + wz);
    let m02 = 2.0 * (xz - wy);
    let m10 = 2.0 * (xy - wz);
    let m11 = 1.0 - 2.0 * (xx + zz);
    let m12 = 2.0 * (yz + wx);
    let m20 = 2.0 * (xz + wy);
    let m21 = 2.0 * (yz - wx);
    let m22 = 1.0 - 2.0 * (xx + yy);
    [
        m00, m01, m02, 0.0, m10, m11, m12, 0.0, m20, m21, m22, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

pub(super) fn compose_transform(parent: Transform, child: Transform) -> Transform {
    let scaled_child_translation = Vec3::new(
        child.translation.x * parent.scale.x,
        child.translation.y * parent.scale.y,
        child.translation.z * parent.scale.z,
    );
    Transform {
        translation: add_vec3(
            parent.translation,
            rotate_vec3(parent.rotation, scaled_child_translation),
        ),
        rotation: multiply_quat(parent.rotation, child.rotation),
        scale: Vec3::new(
            parent.scale.x * child.scale.x,
            parent.scale.y * child.scale.y,
            parent.scale.z * child.scale.z,
        ),
    }
}

pub(in crate::render) fn transform_position(
    position: Vec3,
    transform: Transform,
    origin_shift: Vec3,
) -> Vec3 {
    let scaled = Vec3::new(
        position.x * transform.scale.x,
        position.y * transform.scale.y,
        position.z * transform.scale.z,
    );
    let rotated = rotate_vec3(transform.rotation, scaled);
    subtract_vec3(add_vec3(rotated, transform.translation), origin_shift)
}

pub(super) fn transform_normal(normal: Vec3, transform: Transform) -> Vec3 {
    normalize_or(
        rotate_vec3(transform.rotation, normal),
        Vec3::new(0.0, 0.0, 1.0),
    )
}

pub(super) fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn transform_vertex(vertex: Vertex, transform: Transform, origin_shift: Vec3) -> Vertex {
    Vertex {
        position: transform_position(vertex.position, transform, origin_shift),
        color: vertex.color,
    }
}

fn transform_vertex_attributes(
    attributes: PrimitiveVertexAttributes,
    transform: Transform,
) -> PrimitiveVertexAttributes {
    PrimitiveVertexAttributes {
        normal: transform_normal(attributes.normal, transform),
        tex_coord0: attributes.tex_coord0,
        tangent: transform_normal(attributes.tangent, transform),
        tangent_handedness: attributes.tangent_handedness,
        shadow_visibility: attributes.shadow_visibility,
    }
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

fn multiply_quat(left: Quat, right: Quat) -> Quat {
    normalize_quat(Quat::from_xyzw(
        left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    ))
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

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn length_vec3(vector: Vec3) -> f32 {
    dot_vec3(vector, vector).sqrt()
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = length_vec3(vector);
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}
