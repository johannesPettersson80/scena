//! Primitive meshes, generated helper geometry, technical lines, arrows, grids, and labels.

use crate::material::Color;
use crate::scene::Vec3;

mod bounds;
mod helpers;
mod morph;
mod primitive;
mod primitive_meshes;
mod skinning;
mod static_batch;
mod tangents;
pub use morph::GeometryMorphTarget;
pub use skinning::{GeometrySkin, SkinningMatrix};
pub use static_batch::StaticBatchReport;

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
    InvalidVertexColorCount {
        vertex_count: usize,
        color_count: usize,
    },
    InvalidTextureCoordinateCount {
        vertex_count: usize,
        tex_coord_count: usize,
    },
    InvalidTangentCount {
        vertex_count: usize,
        tangent_count: usize,
    },
    InvalidMorphTargetVertexCount {
        vertex_count: usize,
        target_index: usize,
        target_count: usize,
    },
    InvalidSkinJointVertexCount {
        vertex_count: usize,
        joint_count: usize,
    },
    InvalidSkinWeightVertexCount {
        vertex_count: usize,
        weight_count: usize,
    },
    InvalidSkinSourceVertexCount {
        vertex_count: usize,
        source_count: usize,
    },
    InvalidSkinJointIndex {
        vertex_index: usize,
        joint: usize,
        joint_count: usize,
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
    vertex_colors: Vec<Color>,
    tex_coords0: Vec<[f32; 2]>,
    tangents: Option<Vec<[f32; 4]>>,
    morph_targets: Vec<GeometryMorphTarget>,
    skin: Option<GeometrySkin>,
    bounds: Aabb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PrimitiveVertexAttributes {
    pub(crate) normal: Vec3,
    pub(crate) tex_coord0: [f32; 2],
    pub(crate) tangent: Vec3,
    pub(crate) tangent_handedness: f32,
    pub(crate) shadow_visibility: f32,
}

impl Default for PrimitiveVertexAttributes {
    fn default() -> Self {
        Self {
            normal: Vec3::new(0.0, 0.0, 1.0),
            tex_coord0: [0.0, 0.0],
            tangent: Vec3::new(1.0, 0.0, 0.0),
            tangent_handedness: 1.0,
            shadow_visibility: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Primitive {
    vertices: [Vertex; 3],
    attributes: [PrimitiveVertexAttributes; 3],
    render_material_slot: u32,
    depth_prepass_eligible: bool,
    world_from_model: [f32; 16],
    normal_from_model: [f32; 16],
}

impl GeometryDesc {
    pub fn try_new(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        let vertex_colors = vec![Color::WHITE; vertices.len()];
        Self::try_new_with_vertex_colors(topology, vertices, indices, vertex_colors)
    }

    pub fn try_new_with_vertex_colors(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
        vertex_colors: Vec<Color>,
    ) -> Result<Self, GeometryError> {
        let tex_coords0 = vec![[0.0, 0.0]; vertices.len()];
        Self::try_new_with_vertex_colors_and_tex_coords(
            topology,
            vertices,
            indices,
            vertex_colors,
            tex_coords0,
        )
    }

    pub fn try_new_with_vertex_colors_and_tex_coords(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
        vertex_colors: Vec<Color>,
        tex_coords0: Vec<[f32; 2]>,
    ) -> Result<Self, GeometryError> {
        let Some(bounds) = Aabb::from_vertices(&vertices) else {
            return Err(GeometryError::EmptyVertices);
        };
        if vertex_colors.len() != vertices.len() {
            return Err(GeometryError::InvalidVertexColorCount {
                vertex_count: vertices.len(),
                color_count: vertex_colors.len(),
            });
        }
        if tex_coords0.len() != vertices.len() {
            return Err(GeometryError::InvalidTextureCoordinateCount {
                vertex_count: vertices.len(),
                tex_coord_count: tex_coords0.len(),
            });
        }
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
            vertex_colors,
            tex_coords0,
            tangents: None,
            morph_targets: Vec::new(),
            skin: None,
            bounds,
        })
    }

    fn new(topology: GeometryTopology, vertices: Vec<GeometryVertex>, indices: Vec<u32>) -> Self {
        Self::try_new(topology, vertices, indices).expect("built-in geometry must be valid")
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

    pub fn vertex_colors(&self) -> &[Color] {
        &self.vertex_colors
    }

    pub fn tex_coords0(&self) -> &[[f32; 2]] {
        &self.tex_coords0
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
