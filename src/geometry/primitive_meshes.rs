use crate::material::Color;
use crate::scene::Vec3;

use super::{GeometryDesc, GeometryTopology, GeometryVertex};

mod bevel;
#[cfg(test)]
mod tests;

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
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    /// A UV sphere of `radius` with `segments` longitudinal and `rings`
    /// latitudinal subdivisions.
    ///
    /// Tessellation and reflections: a near-mirror material (e.g. `chrome` or
    /// any low-roughness metal) reflects the environment sharply and therefore
    /// reveals the sphere's polygon facets. Reflective spheres need a high
    /// subdivision count — use `segments >= 256` and `rings >= 192` (the
    /// showcase chrome hero uses `384, 256`). A coarse sphere (~`96, 64`) looks
    /// smooth under matte/rough materials but blocky/faceted once the surface
    /// becomes a sharp mirror.
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
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    /// Builds a closed cylinder with a duplicated side seam at `u = 1`.
    ///
    /// The seam duplicates position and normal data so no side triangle
    /// interpolates backward from the final UV interval to `u = 0`.
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        let radius = radius.abs();
        let half_height = height.abs() * 0.5;
        let segments = segments.max(3);
        let mut vertices = Vec::new();
        let mut tex_coords0 = Vec::new();
        let mut indices = Vec::new();

        for (ring, y) in [-half_height, half_height].into_iter().enumerate() {
            for segment in 0..=segments {
                let u = segment as f32 / segments as f32;
                let theta = if segment == segments {
                    0.0
                } else {
                    u * std::f32::consts::TAU
                };
                let normal = Vec3::new(theta.cos(), 0.0, theta.sin());
                vertices.push(GeometryVertex {
                    position: Vec3::new(normal.x * radius, y, normal.z * radius),
                    normal,
                });
                tex_coords0.push([u, ring as f32]);
            }
        }
        let side_row = segments + 1;
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
            let bottom_next = segment + 1;
            let top = segment + side_row;
            let top_next = segment + 1 + side_row;
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

    /// Builds a closed cone with face-local side UVs and tip vertices.
    ///
    /// The final face ends at `u = 1`; its base edge never wraps backward to
    /// `u = 0` during interpolation.
    pub fn cone(radius: f32, height: f32, segments: u32) -> Self {
        let radius = radius.abs();
        let half_height = height.abs() * 0.5;
        let segments = segments.max(3);
        let mut vertices = Vec::with_capacity((segments as usize * 4) + 1);
        let mut tex_coords0 = Vec::with_capacity((segments as usize * 4) + 1);
        let mut indices = Vec::with_capacity(segments as usize * 6);

        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            let next_theta = next as f32 / segments as f32 * std::f32::consts::TAU;
            let p0 = Vec3::new(theta.cos() * radius, -half_height, theta.sin() * radius);
            let p1 = Vec3::new(
                next_theta.cos() * radius,
                -half_height,
                next_theta.sin() * radius,
            );
            let tip = Vec3::new(0.0, half_height, 0.0);
            let normal = triangle_normal(p0, tip, p1);
            let base = vertices.len() as u32;
            vertices.extend_from_slice(&[
                GeometryVertex {
                    position: p0,
                    normal,
                },
                GeometryVertex {
                    position: p1,
                    normal,
                },
                GeometryVertex {
                    position: tip,
                    normal,
                },
            ]);
            tex_coords0.extend_from_slice(&[
                [segment as f32 / segments as f32, 1.0],
                [(segment + 1) as f32 / segments as f32, 1.0],
                [(segment as f32 + 0.5) / segments as f32, 0.0],
            ]);
            indices.extend_from_slice(&[base, base + 2, base + 1]);
        }

        let bottom_center = vertices.len() as u32;
        vertices.push(GeometryVertex {
            position: Vec3::new(0.0, -half_height, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
        });
        tex_coords0.push([0.5, 0.5]);
        let bottom_base = vertices.len() as u32;
        for segment in 0..segments {
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(GeometryVertex {
                position: Vec3::new(theta.cos() * radius, -half_height, theta.sin() * radius),
                normal: Vec3::new(0.0, -1.0, 0.0),
            });
            tex_coords0.push([theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5]);
        }
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            indices.extend_from_slice(&[bottom_center, bottom_base + segment, bottom_base + next]);
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn disc(radius: f32, segments: u32) -> Self {
        let radius = radius.abs();
        let segments = segments.max(3);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let mut vertices = Vec::with_capacity(segments as usize + 1);
        let mut tex_coords0 = Vec::with_capacity(segments as usize + 1);
        let mut indices = Vec::with_capacity(segments as usize * 3);
        vertices.push(GeometryVertex {
            position: Vec3::ZERO,
            normal,
        });
        tex_coords0.push([0.5, 0.5]);
        for segment in 0..segments {
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(GeometryVertex {
                position: Vec3::new(theta.cos() * radius, 0.0, theta.sin() * radius),
                normal,
            });
            tex_coords0.push([theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5]);
        }
        for segment in 0..segments {
            let current = 1 + segment;
            let next = 1 + ((segment + 1) % segments);
            indices.extend_from_slice(&[0, next, current]);
        }
        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn torus(major_radius: f32, minor_radius: f32, segments: u32, rings: u32) -> Self {
        let major_radius = major_radius.abs();
        let minor_radius = minor_radius.abs();
        let segments = segments.max(3);
        let rings = rings.max(3);
        let vertex_capacity = (segments as usize + 1).saturating_mul(rings as usize + 1);
        let index_capacity = (segments as usize)
            .saturating_mul(rings as usize)
            .saturating_mul(6);
        let mut vertices = Vec::with_capacity(vertex_capacity);
        let mut tex_coords0 = Vec::with_capacity(vertex_capacity);
        let mut indices = Vec::with_capacity(index_capacity);

        for segment in 0..=segments {
            let u = segment as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;
            let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
            for ring in 0..=rings {
                let v = ring as f32 / rings as f32;
                let phi = v * std::f32::consts::TAU;
                let normal = Vec3::new(radial.x * phi.cos(), phi.sin(), radial.z * phi.cos());
                let center = scale(radial, major_radius);
                vertices.push(GeometryVertex {
                    position: add(center, scale(normal, minor_radius)),
                    normal,
                });
                tex_coords0.push([u, v]);
            }
        }

        let row = rings + 1;
        for segment in 0..segments {
            for ring in 0..rings {
                let a = segment * row + ring;
                let b = (segment + 1) * row + ring;
                let c = a + 1;
                let d = b + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }

        new_with_tex_coords(GeometryTopology::Triangles, vertices, indices, tex_coords0)
    }

    pub fn wedge(width: f32, height: f32, depth: f32) -> Self {
        let half_width = width.abs() * 0.5;
        let half_height = height.abs() * 0.5;
        let half_depth = depth.abs() * 0.5;
        let a = Vec3::new(-half_width, -half_height, -half_depth);
        let b = Vec3::new(half_width, -half_height, -half_depth);
        let c = Vec3::new(-half_width, -half_height, half_depth);
        let d = Vec3::new(half_width, -half_height, half_depth);
        let e = Vec3::new(-half_width, half_height, half_depth);
        let f = Vec3::new(half_width, half_height, half_depth);
        let faces: &[&[Vec3]] = &[
            &[a, c, d, b],
            &[c, e, f, d],
            &[a, b, f, e],
            &[a, e, c],
            &[b, d, f],
        ];
        let face_uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mut vertices = Vec::with_capacity(18);
        let mut tex_coords0 = Vec::with_capacity(18);
        let mut indices = Vec::with_capacity(24);
        for face in faces {
            let base = vertices.len() as u32;
            let normal = triangle_normal(face[0], face[2], face[1]);
            vertices.extend(
                face.iter()
                    .copied()
                    .map(|position| GeometryVertex { position, normal }),
            );
            tex_coords0.extend(face.iter().enumerate().map(|(index, _)| face_uvs[index]));
            if face.len() == 3 {
                indices.extend_from_slice(&[base, base + 2, base + 1]);
            } else {
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
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
            vec![0, 2, 1, 0, 3, 2],
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

fn add(lhs: Vec3, rhs: Vec3) -> Vec3 {
    Vec3::new(lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z)
}

fn sub(lhs: Vec3, rhs: Vec3) -> Vec3 {
    Vec3::new(lhs.x - rhs.x, lhs.y - rhs.y, lhs.z - rhs.z)
}

fn cross(lhs: Vec3, rhs: Vec3) -> Vec3 {
    Vec3::new(
        lhs.y * rhs.z - lhs.z * rhs.y,
        lhs.z * rhs.x - lhs.x * rhs.z,
        lhs.x * rhs.y - lhs.y * rhs.x,
    )
}

fn normalize(value: Vec3) -> Vec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length <= f32::EPSILON {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        scale(value, 1.0 / length)
    }
}

fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    normalize(cross(sub(b, a), sub(c, a)))
}
