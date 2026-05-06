//! Primitive meshes, generated helper geometry, technical lines, arrows, grids, and labels.

use crate::material::Color;
use crate::scene::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryTopology {
    Triangles,
    Lines,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    EmptyVertices,
    InvalidIndexCount {
        topology: GeometryTopology,
        index_count: usize,
    },
    InvalidIndex {
        index: u32,
        vertex_count: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryVertex {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryDesc {
    topology: GeometryTopology,
    vertices: Vec<GeometryVertex>,
    indices: Vec<u32>,
    bounds: Aabb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Primitive {
    vertices: [Vertex; 3],
}

impl GeometryDesc {
    pub fn try_new(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        let Some(bounds) = Aabb::from_vertices(&vertices) else {
            return Err(GeometryError::EmptyVertices);
        };
        let valid_arity = match topology {
            GeometryTopology::Triangles => indices.len().is_multiple_of(3),
            GeometryTopology::Lines => indices.len().is_multiple_of(2),
        };
        if !valid_arity {
            return Err(GeometryError::InvalidIndexCount {
                topology,
                index_count: indices.len(),
            });
        }
        for index in &indices {
            if (*index as usize) >= vertices.len() {
                return Err(GeometryError::InvalidIndex {
                    index: *index,
                    vertex_count: vertices.len(),
                });
            }
        }
        Ok(Self {
            topology,
            vertices,
            indices,
            bounds,
        })
    }

    fn new(topology: GeometryTopology, vertices: Vec<GeometryVertex>, indices: Vec<u32>) -> Self {
        Self::try_new(topology, vertices, indices).expect("built-in geometry must be valid")
    }

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
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (face_index, (normal, positions)) in faces.into_iter().enumerate() {
            let base = (face_index * 4) as u32;
            vertices.extend(
                positions
                    .into_iter()
                    .map(|position| GeometryVertex { position, normal }),
            );
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::new(GeometryTopology::Triangles, vertices, indices)
    }

    pub fn sphere(radius: f32, segments: u32, rings: u32) -> Self {
        let radius = radius.abs();
        let segments = segments.max(3);
        let rings = rings.max(2);
        let mut vertices = Vec::new();
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

        Self::new(GeometryTopology::Triangles, vertices, indices)
    }

    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        let radius = radius.abs();
        let half_height = height.abs() * 0.5;
        let segments = segments.max(3);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for y in [-half_height, half_height] {
            for segment in 0..segments {
                let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
                let normal = Vec3::new(theta.cos(), 0.0, theta.sin());
                vertices.push(GeometryVertex {
                    position: Vec3::new(normal.x * radius, y, normal.z * radius),
                    normal,
                });
            }
        }
        let bottom_cap_base = vertices.len() as u32;
        for segment in 0..segments {
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(GeometryVertex {
                position: Vec3::new(theta.cos() * radius, -half_height, theta.sin() * radius),
                normal: Vec3::new(0.0, -1.0, 0.0),
            });
        }
        let top_cap_base = vertices.len() as u32;
        for segment in 0..segments {
            let theta = segment as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(GeometryVertex {
                position: Vec3::new(theta.cos() * radius, half_height, theta.sin() * radius),
                normal: Vec3::new(0.0, 1.0, 0.0),
            });
        }
        let bottom_center = vertices.len() as u32;
        vertices.push(GeometryVertex {
            position: Vec3::new(0.0, -half_height, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
        });
        let top_center = vertices.len() as u32;
        vertices.push(GeometryVertex {
            position: Vec3::new(0.0, half_height, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
        });

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

        Self::new(GeometryTopology::Triangles, vertices, indices)
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
        Self::new(
            GeometryTopology::Triangles,
            vertices,
            vec![0, 1, 2, 0, 2, 3],
        )
    }

    pub fn line(start: Vec3, end: Vec3) -> Self {
        Self::lines_from_positions(vec![start, end], vec![0, 1])
    }

    pub fn polyline(points: &[Vec3]) -> Self {
        assert!(points.len() >= 2, "polyline requires at least two points");
        let mut indices = Vec::with_capacity((points.len() - 1) * 2);
        for index in 0..points.len() as u32 - 1 {
            indices.extend_from_slice(&[index, index + 1]);
        }
        Self::lines_from_positions(points.to_vec(), indices)
    }

    pub fn arrow(start: Vec3, end: Vec3) -> Self {
        let length = distance(start, end);
        if length <= f32::EPSILON {
            return Self::line(start, end);
        }
        let direction = normalize(sub(end, start));
        let head_length = length * 0.15;
        let head_width = length * 0.06;
        let side_axis = if direction.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let base = sub(end, scale(direction, head_length));
        let side = scale(side_axis, head_width);
        Self::lines_from_positions(
            vec![start, end, end, add(base, side), end, sub(base, side)],
            vec![0, 1, 2, 3, 4, 5],
        )
    }

    pub fn grid(size: f32, divisions: u32) -> Self {
        let divisions = divisions.max(1);
        let half = size.abs() * 0.5;
        let step = size.abs() / divisions as f32;
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for index in 0..=divisions {
            let offset = -half + index as f32 * step;
            let base = positions.len() as u32;
            positions.extend_from_slice(&[
                Vec3::new(-half, 0.0, offset),
                Vec3::new(half, 0.0, offset),
                Vec3::new(offset, 0.0, -half),
                Vec3::new(offset, 0.0, half),
            ]);
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 3]);
        }
        Self::lines_from_positions(positions, indices)
    }

    pub fn axes(length: f32) -> Self {
        let length = length.abs();
        Self::lines_from_positions(
            vec![
                Vec3::ZERO,
                Vec3::new(length, 0.0, 0.0),
                Vec3::ZERO,
                Vec3::new(0.0, length, 0.0),
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, length),
            ],
            vec![0, 1, 2, 3, 4, 5],
        )
    }

    pub fn topology(&self) -> GeometryTopology {
        self.topology
    }

    pub fn vertices(&self) -> &[GeometryVertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn bounds(&self) -> Aabb {
        self.bounds
    }

    fn lines_from_positions(positions: Vec<Vec3>, indices: Vec<u32>) -> Self {
        let vertices = positions
            .into_iter()
            .map(|position| GeometryVertex {
                position,
                normal: Vec3::ZERO,
            })
            .collect();
        Self::new(GeometryTopology::Lines, vertices, indices)
    }
}

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_vertices(vertices: &[GeometryVertex]) -> Option<Self> {
        let first = vertices.first()?;
        let mut min = first.position;
        let mut max = first.position;
        for vertex in &vertices[1..] {
            min.x = min.x.min(vertex.position.x);
            min.y = min.y.min(vertex.position.y);
            min.z = min.z.min(vertex.position.z);
            max.x = max.x.max(vertex.position.x);
            max.y = max.y.max(vertex.position.y);
            max.z = max.z.max(vertex.position.z);
        }
        Some(Self { min, max })
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }
}

impl Primitive {
    pub fn triangle(vertices: [Vertex; 3]) -> Self {
        Self { vertices }
    }

    pub fn unlit_triangle() -> Self {
        Self::triangle([
            Vertex {
                position: Vec3::new(-0.6, -0.5, 0.0),
                color: Color::from_linear_rgb(1.0, 0.2, 0.1),
            },
            Vertex {
                position: Vec3::new(0.6, -0.5, 0.0),
                color: Color::from_linear_rgb(0.1, 0.8, 0.2),
            },
            Vertex {
                position: Vec3::new(0.0, 0.6, 0.0),
                color: Color::from_linear_rgb(0.1, 0.3, 1.0),
            },
        ])
    }

    pub fn vertices(&self) -> &[Vertex; 3] {
        &self.vertices
    }
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale(value: Vec3, factor: f32) -> Vec3 {
    Vec3::new(value.x * factor, value.y * factor, value.z * factor)
}

fn distance(a: Vec3, b: Vec3) -> f32 {
    let delta = sub(a, b);
    (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt()
}

fn normalize(value: Vec3) -> Vec3 {
    let length = distance(value, Vec3::ZERO);
    if length <= f32::EPSILON {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        scale(value, 1.0 / length)
    }
}
