use crate::geometry::Aabb;

use super::{Quat, Transform, Vec3};

pub(super) fn look_rotation(forward: Vec3, up: Vec3) -> Quat {
    let right = normalize_or(cross_vec3(forward, up), Vec3::new(1.0, 0.0, 0.0));
    let up = cross_vec3(right, forward);
    quat_from_basis(right, up, scale_vec3(forward, -1.0))
}

fn quat_from_basis(right: Vec3, up: Vec3, back: Vec3) -> Quat {
    let trace = right.x + up.y + back.z;
    let quat = if trace > 0.0 {
        let scale = (trace + 1.0).sqrt() * 2.0;
        Quat {
            w: 0.25 * scale,
            x: (up.z - back.y) / scale,
            y: (back.x - right.z) / scale,
            z: (right.y - up.x) / scale,
        }
    } else if right.x > up.y && right.x > back.z {
        let scale = (1.0 + right.x - up.y - back.z).sqrt() * 2.0;
        Quat {
            w: (up.z - back.y) / scale,
            x: 0.25 * scale,
            y: (up.x + right.y) / scale,
            z: (back.x + right.z) / scale,
        }
    } else if up.y > back.z {
        let scale = (1.0 + up.y - right.x - back.z).sqrt() * 2.0;
        Quat {
            w: (back.x - right.z) / scale,
            x: (up.x + right.y) / scale,
            y: 0.25 * scale,
            z: (back.y + up.z) / scale,
        }
    } else {
        let scale = (1.0 + back.z - right.x - up.y).sqrt() * 2.0;
        Quat {
            w: (right.y - up.x) / scale,
            x: (back.x + right.z) / scale,
            y: (back.y + up.z) / scale,
            z: 0.25 * scale,
        }
    };
    normalize_quat(quat)
}

fn normalize_quat(quat: Quat) -> Quat {
    let length = (quat.x * quat.x + quat.y * quat.y + quat.z * quat.z + quat.w * quat.w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Quat::IDENTITY
    } else {
        Quat::from_xyzw(
            quat.x / length,
            quat.y / length,
            quat.z / length,
            quat.w / length,
        )
    }
}

pub(super) fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        scale_vec3(value, 1.0 / length)
    }
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

pub(super) fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

pub(crate) fn world_to_view(world_position: Vec3, world_from_camera: Transform) -> Option<Vec3> {
    if !world_from_camera.translation.is_finite()
        || !world_from_camera.rotation.is_finite()
        || !is_finite_nonzero_scale(world_from_camera.scale)
    {
        return None;
    }
    let translated = subtract_vec3(world_position, world_from_camera.translation);
    let rotated = rotate_vec3(inverse_unit_quat(world_from_camera.rotation), translated);
    Some(Vec3::new(
        rotated.x / world_from_camera.scale.x,
        rotated.y / world_from_camera.scale.y,
        rotated.z / world_from_camera.scale.z,
    ))
}

fn is_finite_nonzero_scale(scale: Vec3) -> bool {
    scale.is_finite()
        && scale.x.abs() > f32::EPSILON
        && scale.y.abs() > f32::EPSILON
        && scale.z.abs() > f32::EPSILON
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

pub(super) fn positive_min(values: [f32; 3]) -> f32 {
    values
        .into_iter()
        .filter(|value| value.is_finite() && *value > 0.0)
        .min_by(f32::total_cmp)
        .unwrap_or(1.0)
}

pub(super) fn transform_aabb(bounds: Aabb, transform: Transform) -> Aabb {
    aabb_corners(bounds)
        .into_iter()
        .map(|corner| transform_point(corner, transform))
        .map(|point| Aabb::new(point, point))
        .reduce(union_aabb)
        .expect("AABB has corners")
}

pub(super) fn merge_optional_bounds(bounds: Option<Aabb>, next: Aabb) -> Aabb {
    bounds.map_or(next, |bounds| union_aabb(bounds, next))
}

fn aabb_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    let scaled = Vec3::new(
        point.x * transform.scale.x,
        point.y * transform.scale.y,
        point.z * transform.scale.z,
    );
    add_vec3(
        rotate_vec3(transform.rotation, scaled),
        transform.translation,
    )
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

pub(super) fn multiply_quat(left: Quat, right: Quat) -> Quat {
    normalize_quat(Quat::from_xyzw(
        left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    ))
}

pub(super) fn inverse_unit_quat(rotation: Quat) -> Quat {
    let normalized = normalize_quat(rotation);
    Quat::from_xyzw(-normalized.x, -normalized.y, -normalized.z, normalized.w)
}

pub(super) fn union_aabb(left: Aabb, right: Aabb) -> Aabb {
    Aabb::new(
        Vec3::new(
            left.min.x.min(right.min.x),
            left.min.y.min(right.min.y),
            left.min.z.min(right.min.z),
        ),
        Vec3::new(
            left.max.x.max(right.max.x),
            left.max.y.max(right.max.y),
            left.max.z.max(right.max.z),
        ),
    )
}

const fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

pub(super) const fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}
