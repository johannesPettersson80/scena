use crate::scene::Vec3;

use super::{Aabb, GeometryDesc};

impl GeometryDesc {
    pub fn bounding_box(bounds: Aabb) -> Self {
        let min = bounds.min;
        let max = bounds.max;
        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ];
        Self::lines_from_positions(
            corners.to_vec(),
            vec![
                0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
            ],
        )
    }

    pub fn camera_frustum(near: f32, far: f32, aspect: f32, vertical_fov_degrees: f32) -> Self {
        let near = positive_or(near, 0.1);
        let far = positive_or(far, 1.0).max(near + 0.001);
        let aspect = positive_or(aspect, 1.0);
        let half_fov = vertical_fov_degrees.clamp(1.0, 179.0).to_radians() * 0.5;
        let near_half_y = half_fov.tan() * near;
        let near_half_x = near_half_y * aspect;
        let far_half_y = half_fov.tan() * far;
        let far_half_x = far_half_y * aspect;
        Self::lines_from_positions(
            vec![
                Vec3::new(-near_half_x, -near_half_y, -near),
                Vec3::new(near_half_x, -near_half_y, -near),
                Vec3::new(near_half_x, near_half_y, -near),
                Vec3::new(-near_half_x, near_half_y, -near),
                Vec3::new(-far_half_x, -far_half_y, -far),
                Vec3::new(far_half_x, -far_half_y, -far),
                Vec3::new(far_half_x, far_half_y, -far),
                Vec3::new(-far_half_x, far_half_y, -far),
            ],
            vec![
                0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
            ],
        )
    }

    pub fn light_helper(size: f32) -> Self {
        let size = positive_or(size, 1.0);
        Self::lines_from_positions(
            vec![
                Vec3::new(-size, 0.0, 0.0),
                Vec3::new(size, 0.0, 0.0),
                Vec3::new(0.0, -size, 0.0),
                Vec3::new(0.0, size, 0.0),
                Vec3::new(0.0, 0.0, -size),
                Vec3::new(0.0, 0.0, size),
                Vec3::new(-size * 0.5, -size * 0.5, 0.0),
                Vec3::new(size * 0.5, size * 0.5, 0.0),
                Vec3::new(-size * 0.5, size * 0.5, 0.0),
                Vec3::new(size * 0.5, -size * 0.5, 0.0),
            ],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        )
    }

    pub fn origin_marker(size: f32) -> Self {
        Self::axes(size)
    }

    pub fn pivot_marker(size: f32) -> Self {
        marker_cross(size)
    }

    pub fn anchor_marker(size: f32) -> Self {
        let size = positive_or(size, 1.0);
        Self::lines_from_positions(
            vec![
                Vec3::new(0.0, size, 0.0),
                Vec3::new(size, 0.0, 0.0),
                Vec3::new(size, 0.0, 0.0),
                Vec3::new(0.0, -size, 0.0),
                Vec3::new(0.0, -size, 0.0),
                Vec3::new(-size, 0.0, 0.0),
                Vec3::new(-size, 0.0, 0.0),
                Vec3::new(0.0, size, 0.0),
                Vec3::new(0.0, 0.0, -size),
                Vec3::new(0.0, 0.0, size),
            ],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        )
    }

    pub fn normal_lines(source: &GeometryDesc, length: f32) -> Self {
        let length = positive_or(length, 1.0);
        let mut positions = Vec::with_capacity(source.vertices().len() * 2);
        let mut indices = Vec::with_capacity(source.vertices().len() * 2);
        for vertex in source.vertices() {
            let base = positions.len() as u32;
            positions.push(vertex.position);
            positions.push(add_vec3(vertex.position, scale_vec3(vertex.normal, length)));
            indices.extend_from_slice(&[base, base + 1]);
        }
        Self::lines_from_positions(positions, indices)
    }
}

fn marker_cross(size: f32) -> GeometryDesc {
    let size = positive_or(size, 1.0);
    GeometryDesc::lines_from_positions(
        vec![
            Vec3::new(-size, 0.0, 0.0),
            Vec3::new(size, 0.0, 0.0),
            Vec3::new(0.0, -size, 0.0),
            Vec3::new(0.0, size, 0.0),
            Vec3::new(0.0, 0.0, -size),
            Vec3::new(0.0, 0.0, size),
        ],
        vec![0, 1, 2, 3, 4, 5],
    )
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn scale_vec3(value: Vec3, factor: f32) -> Vec3 {
    Vec3::new(value.x * factor, value.y * factor, value.z * factor)
}
