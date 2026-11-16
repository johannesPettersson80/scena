use crate::geometry::{Primitive, PrimitiveVertexAttributes, Vertex};
use crate::scene::{Vec3, WorldTransform};

pub(super) const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

pub(super) fn transform_primitive(
    primitive: &Primitive,
    transform: WorldTransform,
    origin_shift: Vec3,
) -> Primitive {
    let [a, b, c] = primitive.vertices();
    let [attributes_a, attributes_b, attributes_c] = primitive.vertex_attributes();
    let mut vertices = [
        transform_vertex(*a, transform, origin_shift),
        transform_vertex(*b, transform, origin_shift),
        transform_vertex(*c, transform, origin_shift),
    ];
    let mut attributes = [
        transform_vertex_attributes(*attributes_a, transform),
        transform_vertex_attributes(*attributes_b, transform),
        transform_vertex_attributes(*attributes_c, transform),
    ];
    restore_front_face_winding(transform, &mut vertices);
    restore_front_face_winding(transform, &mut attributes);
    let transformed = Primitive::triangle_with_attributes(vertices, attributes)
        .with_render_material_slot(primitive.render_material_slot());
    if primitive.depth_prepass_eligible() {
        transformed
    } else {
        transformed.without_depth_prepass()
    }
}

/// Keeps counter-clockwise triangles front-facing under a mirrored transform.
///
/// A world matrix with a negative determinant reverses triangle winding, which
/// would make both rasterizers cull the front faces of single-sided geometry
/// and draw its inside. Swapping two corners of every baked triangle undoes
/// the reversal once, for the CPU rasterizer and for the vertex data the GPU
/// path derives from the same bake.
pub(super) fn restore_front_face_winding<T>(transform: WorldTransform, corners: &mut [T; 3]) {
    if transform.is_mirrored() {
        corners.swap(1, 2);
    }
}

pub(super) fn prepared_primitive(
    primitive: &Primitive,
    transform: WorldTransform,
    origin_shift: Vec3,
) -> Primitive {
    transform_primitive(primitive, transform, origin_shift)
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

/// The per-draw model matrix: the node's exact world matrix, rebased onto the
/// scene origin shift.
pub(in crate::render) fn world_from_model_matrix(
    transform: impl Into<WorldTransform>,
    origin_shift: Vec3,
) -> [f32; 16] {
    let matrix = transform.into().matrix();
    let x = matrix.matrix3.x_axis;
    let y = matrix.matrix3.y_axis;
    let z = matrix.matrix3.z_axis;
    let tx = matrix.translation.x - origin_shift.x;
    let ty = matrix.translation.y - origin_shift.y;
    let tz = matrix.translation.z - origin_shift.z;
    [
        x.x, x.y, x.z, 0.0, y.x, y.y, y.z, 0.0, z.x, z.y, z.z, 0.0, tx, ty, tz, 1.0,
    ]
}

/// The per-draw normal matrix: the inverse transpose of the world matrix's
/// linear part, up to a positive factor the shaders normalize away.
pub(in crate::render) fn normal_from_model_matrix(
    transform: impl Into<WorldTransform>,
) -> [f32; 16] {
    let normal = transform.into().normal_matrix();
    let x = normal.x_axis;
    let y = normal.y_axis;
    let z = normal.z_axis;
    [
        x.x, x.y, x.z, 0.0, y.x, y.y, y.z, 0.0, z.x, z.y, z.z, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

pub(in crate::render) fn transform_position(
    position: Vec3,
    transform: WorldTransform,
    origin_shift: Vec3,
) -> Vec3 {
    subtract_vec3(transform.transform_point(position), origin_shift)
}

/// Carries a surface normal into world space with the inverse transpose of
/// the world matrix, so non-uniform scale and mirroring keep it perpendicular
/// to the surface and pointing outward.
pub(super) fn transform_normal(normal: Vec3, transform: WorldTransform) -> Vec3 {
    normalize_or(transform.transform_normal(normal), Vec3::new(0.0, 0.0, 1.0))
}

/// Carries a surface tangent into world space. Tangents lie in the surface,
/// so they follow the world matrix itself rather than its inverse transpose.
pub(super) fn transform_tangent(tangent: Vec3, transform: WorldTransform) -> Vec3 {
    match transform.uniform_rotation() {
        Some(rotation) => normalize_or(
            crate::scene::rotate_vec3(rotation, tangent),
            Vec3::new(0.0, 0.0, 1.0),
        ),
        None => normalize_or(
            transform.transform_vector(tangent),
            Vec3::new(0.0, 0.0, 1.0),
        ),
    }
}

pub(super) fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn transform_vertex(vertex: Vertex, transform: WorldTransform, origin_shift: Vec3) -> Vertex {
    Vertex {
        position: transform_position(vertex.position, transform, origin_shift),
        color: vertex.color,
    }
}

fn transform_vertex_attributes(
    attributes: PrimitiveVertexAttributes,
    transform: WorldTransform,
) -> PrimitiveVertexAttributes {
    PrimitiveVertexAttributes {
        normal: transform_normal(attributes.normal, transform),
        tex_coord0: attributes.tex_coord0,
        tangent: transform_tangent(attributes.tangent, transform),
        tangent_handedness: attributes.tangent_handedness,
        shadow_visibility: attributes.shadow_visibility,
        ambient_visibility: attributes.ambient_visibility,
    }
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
