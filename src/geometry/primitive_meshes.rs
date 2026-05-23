use crate::material::Color;
use crate::scene::Vec3;

use super::{GeometryDesc, GeometryTopology, GeometryVertex};

impl GeometryDesc {
    pub fn box_xyz(width: f32, height: f32, depth: f32) -> Self {
        let half = Vec3::new(width.abs() * 0.5, height.abs() * 0.5, depth.abs() * 0.5);
        let faces = [
            (
                Vec3::new(0.0, 0.0, -1.0),
                [
                    Vec3::new(-half.x, -half.y, -half.z),
                    Vec3::new(half.x, -half.y, -half.z),
                    Vec3::new(half.x, half.y, -half.z),
                    Vec3::new(-half.x, half.y, -half.z),
                ],
            ),
            (
                Vec3::new(0.0, 0.0, 1.0),
                [
                    Vec3::new(-half.x, -half.y, half.z),
                    Vec3::new(-half.x, half.y, half.z),
                    Vec3::new(half.x, half.y, half.z),
                    Vec3::new(half.x, -half.y, half.z),
                ],
            ),
            (
                Vec3::new(-1.0, 0.0, 0.0),
                [
                    Vec3::new(-half.x, -half.y, -half.z),
                    Vec3::new(-half.x, half.y, -half.z),
                    Vec3::new(-half.x, half.y, half.z),
                    Vec3::new(-half.x, -half.y, half.z),
                ],
            ),
            (
                Vec3::new(1.0, 0.0, 0.0),
                [
                    Vec3::new(half.x, -half.y, -half.z),
                    Vec3::new(half.x, -half.y, half.z),
                    Vec3::new(half.x, half.y, half.z),
                    Vec3::new(half.x, half.y, -half.z),
                ],
            ),
            (
                Vec3::new(0.0, 1.0, 0.0),
                [
                    Vec3::new(-half.x, half.y, -half.z),
                    Vec3::new(half.x, half.y, -half.z),
                    Vec3::new(half.x, half.y, half.z),
                    Vec3::new(-half.x, half.y, half.z),
                ],
            ),
            (
                Vec3::new(0.0, -1.0, 0.0),
                [
                    Vec3::new(-half.x, -half.y, -half.z),
                    Vec3::new(-half.x, -half.y, half.z),
                    Vec3::new(half.x, -half.y, half.z),
                    Vec3::new(half.x, -half.y, -half.z),
                ],
            ),
        ];
        let face_uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mut vertices = Vec::with_capacity(24);
        let mut tex_coords0 = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (face_index, (normal, positions)) in faces.into_iter().enumerate() {
            let base = (face_index * 4) as u32;
            vertices.extend(
                positions
                    .into_iter()
                    .map(|position| GeometryVertex { position, normal }),
            );
            tex_coords0.extend(face_uvs);
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn sphere(radius: f32, segments: u32, rings: u32) -> Self {
        let radius = radius.abs();
        let segments = segments.max(3);
        let rings = rings.max(2);
        let mut vertices = Vec::new();
        let mut tex_coords0 = Vec::new();
        let mut indices = Vec::new();

        for ring in 0..=rings {
            let v = ring as f32 / rings as f32;
            let phi = v * std::f32::consts::PI;
            for segment in 0..=segments {
                let u = segment as f32 / segments as f32;
                let theta = u * std::f32::consts::TAU;
                let normal = Vec3::new(theta.cos() * phi.sin(), phi.cos(), theta.sin() * phi.sin());
                vertices.push(GeometryVertex {
                    position: scale(normal, radius),
                    normal,
                });
                tex_coords0.push([u, v]);
            }
        }

        let row = segments + 1;
        for ring in 0..rings {
            for segment in 0..segments {
                let a = ring * row + segment;
                let b = a + 1;
                let c = a + row;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        let radius = radius.abs();
        let half_height = height.abs() * 0.5;
        let segments = segments.max(3);
        let mut vertices = Vec::new();
        let mut tex_coords0 = Vec::new();
        let mut indices = Vec::new();

        for (ring, y) in [-half_height, half_height].into_iter().enumerate() {
            for segment in 0..segments {
                let u = segment as f32 / segments as f32;
                let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
                let normal = Vec3::new(theta.cos(), 0.0, theta.sin());
                vertices.push(GeometryVertex {
                    position: Vec3::new(normal.x * radius, y, normal.z * radius),
                    normal,
                });
                tex_coords0.push([u, ring as f32]);
            }
        }
        let bottom_cap_base = vertices.len() as u32;
        for segment in 0..segments {
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(GeometryVertex {
                position: Vec3::new(theta.cos() * radius, -half_height, theta.sin() * radius),
                normal: Vec3::new(0.0, -1.0, 0.0),
            });
            tex_coords0.push([theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5]);
        }
        let top_cap_base = vertices.len() as u32;
        for segment in 0..segments {
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(GeometryVertex {
                position: Vec3::new(theta.cos() * radius, half_height, theta.sin() * radius),
                normal: Vec3::new(0.0, 1.0, 0.0),
            });
            tex_coords0.push([theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5]);
        }
        let bottom_center = vertices.len() as u32;
        vertices.push(GeometryVertex {
            position: Vec3::new(0.0, -half_height, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
        });
        tex_coords0.push([0.5, 0.5]);
        let top_center = vertices.len() as u32;
        vertices.push(GeometryVertex {
            position: Vec3::new(0.0, half_height, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
        });
        tex_coords0.push([0.5, 0.5]);

        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let bottom = segment;
            let bottom_next = next;
            let top = segment + segments;
            let top_next = next + segments;
            indices.extend_from_slice(&[bottom, top, bottom_next, bottom_next, top, top_next]);
            indices.extend_from_slice(&[
                bottom_center,
                bottom_cap_base + segment,
                bottom_cap_base + next,
            ]);
            indices.extend_from_slice(&[top_center, top_cap_base + next, top_cap_base + segment]);
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn plane(width: f32, depth: f32) -> Self {
        let half_width = width.abs() * 0.5;
        let half_depth = depth.abs() * 0.5;
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let vertices = vec![
            GeometryVertex {
                position: Vec3::new(-half_width, 0.0, -half_depth),
                normal,
            },
            GeometryVertex {
                position: Vec3::new(half_width, 0.0, -half_depth),
                normal,
            },
            GeometryVertex {
                position: Vec3::new(half_width, 0.0, half_depth),
                normal,
            },
            GeometryVertex {
                position: Vec3::new(-half_width, 0.0, half_depth),
                normal,
            },
        ];
        new_with_tex_coords(
            GeometryTopology::Triangles,
            vertices,
            vec![0, 1, 2, 0, 2, 3],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        )
    }
}

fn new_with_tex_coords(
    topology: GeometryTopology,
    vertices: Vec<GeometryVertex>,
    indices: Vec<u32>,
    tex_coords0: Vec<[f32; 2]>,
) -> GeometryDesc {
    let vertex_colors = vec![Color::WHITE; vertices.len()];
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        topology,
        vertices,
        indices,
        vertex_colors,
        tex_coords0,
    )
    .expect("built-in geometry must be valid")
}

fn scale(value: Vec3, factor: f32) -> Vec3 {
    Vec3::new(value.x * factor, value.y * factor, value.z * factor)
}
