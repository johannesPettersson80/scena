use crate::material::Color;
use crate::scene::{Quat, Vec3, WorldTransform};

pub(super) fn multiply_color(left: Color, right: Color) -> Color {
    Color::from_linear_rgba(
        left.r * right.r,
        left.g * right.g,
        left.b * right.b,
        left.a * right.a,
    )
}

/// The world direction a light shines in: its local -Z axis carried through
/// the exact world matrix.
///
/// A placement that is exactly translation, rotation, and scale keeps the
/// rotation-only arithmetic, so its direction is unchanged bit for bit; only
/// a negative Z scale, which mirrors the light through its own forward axis,
/// turns it around. A sheared placement reads the matrix column directly.
pub(super) fn light_direction(world: WorldTransform) -> Vec3 {
    const FORWARD: Vec3 = Vec3::new(0.0, 0.0, -1.0);
    let rotated = normalize_or(rotate_vec3(world.trs().rotation, FORWARD), FORWARD);
    match world.exact_trs() {
        Some(trs) if trs.scale.z < 0.0 => negate_vec3(rotated),
        Some(_) => rotated,
        None => normalize_or(world.transform_vector(FORWARD), rotated),
    }
}

/// The world direction of a light's local `axis`, unit length.
///
/// Shear-free placements keep the rotation-only arithmetic. The sign a
/// mirrored scale would give the axis is dropped there: every area light
/// shape is symmetric about its own axes, so the emitter is the same.
pub(super) fn light_axis_direction(world: WorldTransform, axis: Vec3) -> Vec3 {
    let rotated = rotate_vec3(world.trs().rotation, axis);
    match world.exact_trs() {
        Some(_) => rotated,
        None => normalize_or(world.transform_vector(axis), rotated),
    }
}

pub(super) fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
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

pub(super) fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

pub(super) fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

pub(super) fn negate_vec3(vector: Vec3) -> Vec3 {
    Vec3::new(-vector.x, -vector.y, -vector.z)
}

pub(super) fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn length_vec3(vector: Vec3) -> f32 {
    dot_vec3(vector, vector).sqrt()
}

pub(super) fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = length_vec3(vector);
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

pub(super) fn scale_color(color: Color, scale: f32) -> Vec3 {
    Vec3::new(color.r * scale, color.g * scale, color.b * scale)
}

pub(super) fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
