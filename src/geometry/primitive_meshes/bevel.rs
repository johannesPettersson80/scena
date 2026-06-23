use crate::scene::Vec3;

use super::{GeometryDesc, GeometryTopology, GeometryVertex};
use super::{add, new_with_tex_coords, normalize, triangle_normal};

impl GeometryDesc {
    pub fn box_xyz_with_bevel(width: f32, height: f32, depth: f32, bevel: f32) -> Self {
        let half = Vec3::new(width.abs() * 0.5, height.abs() * 0.5, depth.abs() * 0.5);
        let max_bevel = half.x.min(half.y).min(half.z) * 0.95;
        let bevel = bevel.abs().min(max_bevel);
        if bevel <= f32::EPSILON {
            return Self::box_xyz(width, height, depth);
        }

        let inner = Vec3::new(half.x - bevel, half.y - bevel, half.z - bevel);
        let mut vertices = Vec::with_capacity(96);
        let mut tex_coords0 = Vec::with_capacity(96);
        let mut indices = Vec::with_capacity(132);

        for sx in [-1.0_f32, 1.0] {
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                Vec3::new(sx, 0.0, 0.0),
                &[
                    Vec3::new(sx * half.x, -inner.y, -inner.z),
                    Vec3::new(sx * half.x, -inner.y, inner.z),
                    Vec3::new(sx * half.x, inner.y, inner.z),
                    Vec3::new(sx * half.x, inner.y, -inner.z),
                ],
            );
        }
        for sy in [-1.0_f32, 1.0] {
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                Vec3::new(0.0, sy, 0.0),
                &[
                    Vec3::new(-inner.x, sy * half.y, -inner.z),
                    Vec3::new(inner.x, sy * half.y, -inner.z),
                    Vec3::new(inner.x, sy * half.y, inner.z),
                    Vec3::new(-inner.x, sy * half.y, inner.z),
                ],
            );
        }
        for sz in [-1.0_f32, 1.0] {
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                Vec3::new(0.0, 0.0, sz),
                &[
                    Vec3::new(-inner.x, -inner.y, sz * half.z),
                    Vec3::new(-inner.x, inner.y, sz * half.z),
                    Vec3::new(inner.x, inner.y, sz * half.z),
                    Vec3::new(inner.x, -inner.y, sz * half.z),
                ],
            );
        }

        for sy in [-1.0_f32, 1.0] {
            for sz in [-1.0_f32, 1.0] {
                push_polygon(
                    &mut vertices,
                    &mut tex_coords0,
                    &mut indices,
                    normalize(Vec3::new(0.0, sy, sz)),
                    &[
                        Vec3::new(-inner.x, sy * half.y, sz * inner.z),
                        Vec3::new(inner.x, sy * half.y, sz * inner.z),
                        Vec3::new(inner.x, sy * inner.y, sz * half.z),
                        Vec3::new(-inner.x, sy * inner.y, sz * half.z),
                    ],
                );
            }
        }
        for sx in [-1.0_f32, 1.0] {
            for sz in [-1.0_f32, 1.0] {
                push_polygon(
                    &mut vertices,
                    &mut tex_coords0,
                    &mut indices,
                    normalize(Vec3::new(sx, 0.0, sz)),
                    &[
                        Vec3::new(sx * half.x, -inner.y, sz * inner.z),
                        Vec3::new(sx * half.x, inner.y, sz * inner.z),
                        Vec3::new(sx * inner.x, inner.y, sz * half.z),
                        Vec3::new(sx * inner.x, -inner.y, sz * half.z),
                    ],
                );
            }
        }
        for sx in [-1.0_f32, 1.0] {
            for sy in [-1.0_f32, 1.0] {
                push_polygon(
                    &mut vertices,
                    &mut tex_coords0,
                    &mut indices,
                    normalize(Vec3::new(sx, sy, 0.0)),
                    &[
                        Vec3::new(sx * half.x, sy * inner.y, -inner.z),
                        Vec3::new(sx * half.x, sy * inner.y, inner.z),
                        Vec3::new(sx * inner.x, sy * half.y, inner.z),
                        Vec3::new(sx * inner.x, sy * half.y, -inner.z),
                    ],
                );
            }
        }

        for sx in [-1.0_f32, 1.0] {
            for sy in [-1.0_f32, 1.0] {
                for sz in [-1.0_f32, 1.0] {
                    push_polygon(
                        &mut vertices,
                        &mut tex_coords0,
                        &mut indices,
                        normalize(Vec3::new(sx, sy, sz)),
                        &[
                            Vec3::new(sx * half.x, sy * inner.y, sz * inner.z),
                            Vec3::new(sx * inner.x, sy * half.y, sz * inner.z),
                            Vec3::new(sx * inner.x, sy * inner.y, sz * half.z),
                        ],
                    );
                }
            }
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn cylinder_with_bevel(radius: f32, height: f32, segments: u32, bevel: f32) -> Self {
        let radius = radius.abs();
        let half_height = height.abs() * 0.5;
        let segments = segments.max(3);
        let max_bevel = radius.min(half_height) * 0.95;
        let bevel = bevel.abs().min(max_bevel);
        if bevel <= f32::EPSILON {
            return Self::cylinder(radius, height, segments);
        }

        let side_bottom_y = -half_height + bevel;
        let side_top_y = half_height - bevel;
        let cap_radius = radius - bevel;
        let mut vertices = Vec::with_capacity(segments as usize * 14);
        let mut tex_coords0 = Vec::with_capacity(segments as usize * 14);
        let mut indices = Vec::with_capacity(segments as usize * 18);

        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let u0 = segment as f32 / segments as f32;
            let u1 = next as f32 / segments as f32;
            let theta0 = u0 * std::f32::consts::TAU;
            let theta1 = u1 * std::f32::consts::TAU;
            let radial0 = Vec3::new(theta0.cos(), 0.0, theta0.sin());
            let radial1 = Vec3::new(theta1.cos(), 0.0, theta1.sin());
            let radial_mid = normalize(add(radial0, radial1));

            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                radial_mid,
                &[
                    Vec3::new(radial0.x * radius, side_bottom_y, radial0.z * radius),
                    Vec3::new(radial1.x * radius, side_bottom_y, radial1.z * radius),
                    Vec3::new(radial1.x * radius, side_top_y, radial1.z * radius),
                    Vec3::new(radial0.x * radius, side_top_y, radial0.z * radius),
                ],
            );
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                normalize(add(radial_mid, Vec3::new(0.0, 1.0, 0.0))),
                &[
                    Vec3::new(radial0.x * radius, side_top_y, radial0.z * radius),
                    Vec3::new(radial1.x * radius, side_top_y, radial1.z * radius),
                    Vec3::new(radial1.x * cap_radius, half_height, radial1.z * cap_radius),
                    Vec3::new(radial0.x * cap_radius, half_height, radial0.z * cap_radius),
                ],
            );
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                normalize(add(radial_mid, Vec3::new(0.0, -1.0, 0.0))),
                &[
                    Vec3::new(radial0.x * cap_radius, -half_height, radial0.z * cap_radius),
                    Vec3::new(radial1.x * cap_radius, -half_height, radial1.z * cap_radius),
                    Vec3::new(radial1.x * radius, side_bottom_y, radial1.z * radius),
                    Vec3::new(radial0.x * radius, side_bottom_y, radial0.z * radius),
                ],
            );
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                Vec3::new(0.0, 1.0, 0.0),
                &[
                    Vec3::new(0.0, half_height, 0.0),
                    Vec3::new(radial1.x * cap_radius, half_height, radial1.z * cap_radius),
                    Vec3::new(radial0.x * cap_radius, half_height, radial0.z * cap_radius),
                ],
            );
            push_polygon(
                &mut vertices,
                &mut tex_coords0,
                &mut indices,
                Vec3::new(0.0, -1.0, 0.0),
                &[
                    Vec3::new(0.0, -half_height, 0.0),
                    Vec3::new(radial0.x * cap_radius, -half_height, radial0.z * cap_radius),
                    Vec3::new(radial1.x * cap_radius, -half_height, radial1.z * cap_radius),
                ],
            );
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }
}

fn push_polygon(
    vertices: &mut Vec<GeometryVertex>,
    tex_coords0: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    normal: Vec3,
    positions: &[Vec3],
) {
    debug_assert!(positions.len() >= 3);
    let normal = normalize(normal);
    let base = vertices.len() as u32;
    for (index, position) in positions.iter().copied().enumerate() {
        vertices.push(GeometryVertex { position, normal });
        tex_coords0.push(polygon_uv(index, positions.len()));
    }
    for index in 1..positions.len().saturating_sub(1) {
        let a = positions[0];
        let b = positions[index];
        let c = positions[index + 1];
        let face = triangle_normal(a, b, c);
        let aligns = face.x * normal.x + face.y * normal.y + face.z * normal.z >= 0.0;
        if aligns {
            indices.extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
        } else {
            indices.extend_from_slice(&[base, base + index as u32 + 1, base + index as u32]);
        }
    }
}

fn polygon_uv(index: usize, len: usize) -> [f32; 2] {
    match (len, index) {
        (3, 0) => [0.5, 1.0],
        (3, 1) => [0.0, 0.0],
        (3, _) => [1.0, 0.0],
        (_, 0) => [0.0, 0.0],
        (_, 1) => [1.0, 0.0],
        (_, 2) => [1.0, 1.0],
        _ => [0.0, 1.0],
    }
}
