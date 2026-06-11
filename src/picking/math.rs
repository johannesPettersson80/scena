use crate::scene::{Quat, Transform, Vec3};

use super::Ray;

pub(super) fn ray_triangle_intersection(
    ray: Ray,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<(f32, f32, f32)> {
    const EPSILON: f32 = 1.0e-6;
    let edge1 = subtract_vec3(b, a);
    let edge2 = subtract_vec3(c, a);
    let p = cross(ray.direction, edge2);
    let determinant = dot(edge1, p);
    if determinant.abs() <= EPSILON {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let t = subtract_vec3(ray.origin, a);
    let u = dot(t, p) * inverse_determinant;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(t, edge1);
    let v = dot(ray.direction, q) * inverse_determinant;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = dot(edge2, q) * inverse_determinant;
    (distance >= 0.0).then_some((distance, u, v))
}

pub(super) fn triangle_bounds(a: Vec3, b: Vec3, c: Vec3) -> (Vec3, Vec3) {
    (
        Vec3::new(
            a.x.min(b.x).min(c.x),
            a.y.min(b.y).min(c.y),
            a.z.min(b.z).min(c.z),
        ),
        Vec3::new(
            a.x.max(b.x).max(c.x),
            a.y.max(b.y).max(c.y),
            a.z.max(b.z).max(c.z),
        ),
    )
}

pub(super) fn ray_hits_bounds(ray: Ray, min: Vec3, max: Vec3) -> bool {
    let Some((x_min, x_max)) = axis_interval(ray.origin.x, ray.direction.x, min.x, max.x) else {
        return false;
    };
    let Some((y_min, y_max)) = axis_interval(ray.origin.y, ray.direction.y, min.y, max.y) else {
        return false;
    };
    let Some((z_min, z_max)) = axis_interval(ray.origin.z, ray.direction.z, min.z, max.z) else {
        return false;
    };
    let near = x_min.max(y_min).max(z_min);
    let far = x_max.min(y_max).min(z_max);
    far >= near.max(0.0)
}

fn axis_interval(origin: f32, direction: f32, min: f32, max: f32) -> Option<(f32, f32)> {
    const EPSILON: f32 = 1.0e-6;
    if direction.abs() <= EPSILON {
        return (origin >= min && origin <= max).then_some((f32::NEG_INFINITY, f32::INFINITY));
    }
    let first = (min - origin) / direction;
    let second = (max - origin) / direction;
    Some((first.min(second), first.max(second)))
}

pub(super) fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    let scaled = Vec3::new(
        point.x * transform.scale.x,
        point.y * transform.scale.y,
        point.z * transform.scale.z,
    );
    let rotated = rotate_vec3(transform.rotation, scaled);
    Vec3::new(
        rotated.x + transform.translation.x,
        rotated.y + transform.translation.y,
        rotated.z + transform.translation.z,
    )
}

pub(super) const fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

pub(super) const fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

pub(super) const fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn dot(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

pub(super) fn cross(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

pub(super) fn normalize(value: Vec3) -> Vec3 {
    normalize_optional(value).unwrap_or(Vec3::new(0.0, 0.0, -1.0))
}

pub(super) fn normalize_optional(value: Vec3) -> Option<Vec3> {
    let length_squared = dot(value, value);
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return None;
    }
    Some(scale_vec3(value, length_squared.sqrt().recip()))
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
