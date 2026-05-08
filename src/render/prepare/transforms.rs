use crate::geometry::{Primitive, PrimitiveVertexAttributes, Vertex};
use crate::scene::{Quat, Transform, Vec3};

pub(super) fn transform_primitive(
    primitive: &Primitive,
    transform: Transform,
    origin_shift: Vec3,
) -> Primitive {
    let [a, b, c] = primitive.vertices();
    let [attributes_a, attributes_b, attributes_c] = primitive.vertex_attributes();
    Primitive::triangle_with_attributes(
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
    .with_render_material_slot(primitive.render_material_slot())
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

pub(super) fn transform_position(position: Vec3, transform: Transform, origin_shift: Vec3) -> Vec3 {
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
    normalize_quat(Quat {
        x: left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        y: left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        z: left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        w: left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    })
}

fn normalize_quat(value: Quat) -> Quat {
    let length_squared =
        value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return Quat::IDENTITY;
    }
    let inverse_length = length_squared.sqrt().recip();
    Quat {
        x: value.x * inverse_length,
        y: value.y * inverse_length,
        z: value.z * inverse_length,
        w: value.w * inverse_length,
    }
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
