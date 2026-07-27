//! Primitive meshes, generated helper geometry, technical lines, arrows, grids, and labels.

use crate::material::Color;
use crate::scene::Vec3;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

mod bounds;
mod deformation;
mod helpers;
mod morph;
#[cfg(feature = "scene-host")]
mod photographic;
mod primitive;
mod primitive_meshes;
mod skinning;
mod spatial;
mod static_batch;
mod tangents;
pub use morph::GeometryMorphTarget;
pub use skinning::{GeometrySkin, SkinningMatrix};
pub(crate) use spatial::TriangleBvh;
pub use static_batch::StaticBatchReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryTopology {
    Triangles,
    Lines,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    EmptyVertices,
    PolylineTooShort {
        point_count: usize,
    },
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
    MissingSkinMatrices,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryVertex {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug)]
struct OptionalVertexAttribute<T> {
    authored: Option<Vec<T>>,
    default: T,
    len: usize,
    materialized_default: OnceLock<Vec<T>>,
}

impl<T: Clone> Clone for OptionalVertexAttribute<T> {
    fn clone(&self) -> Self {
        Self {
            authored: self.authored.clone(),
            default: self.default.clone(),
            len: self.len,
            materialized_default: self.materialized_default.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for OptionalVertexAttribute<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && (0..self.len).all(|index| self.value(index) == other.value(index))
    }
}

impl<T> OptionalVertexAttribute<T> {
    fn authored(values: Vec<T>, default: T) -> Self {
        let len = values.len();
        Self {
            authored: Some(values),
            default,
            len,
            materialized_default: OnceLock::new(),
        }
    }

    fn absent(len: usize, default: T) -> Self {
        Self {
            authored: None,
            default,
            len,
            materialized_default: OnceLock::new(),
        }
    }

    fn value(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        self.authored
            .as_ref()
            .and_then(|values| values.get(index))
            .or(Some(&self.default))
    }

    fn authored_slice(&self) -> Option<&[T]> {
        self.authored.as_deref()
    }

    #[cfg(test)]
    fn stored_bytes(&self) -> usize {
        self.authored
            .as_ref()
            .map_or(0, |values| values.len() * std::mem::size_of::<T>())
    }
}

impl<T: Clone> OptionalVertexAttribute<T> {
    fn as_compatibility_slice(&self) -> &[T] {
        self.authored.as_deref().unwrap_or_else(|| {
            self.materialized_default
                .get_or_init(|| vec![self.default.clone(); self.len])
        })
    }
}

#[derive(Debug, Clone, Default)]
struct GeneratedTangentCache(Arc<OnceLock<Arc<[[f32; 4]]>>>);

impl PartialEq for GeneratedTangentCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default)]
struct TriangleBvhCache(Arc<OnceLock<Arc<TriangleBvh>>>);

impl PartialEq for TriangleBvhCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl TriangleBvhCache {
    fn get_or_init(&self, geometry: &GeometryDesc) -> (Arc<TriangleBvh>, bool) {
        if let Some(cached) = self.0.get() {
            return (Arc::clone(cached), true);
        }
        let cached = self.0.get_or_init(|| {
            Arc::new(TriangleBvh::from_indexed(
                geometry.vertices(),
                geometry.indices(),
            ))
        });
        (Arc::clone(cached), false)
    }
}

impl GeneratedTangentCache {
    fn get_or_init(&self, generate: impl FnOnce() -> Vec<[f32; 4]>) -> (Arc<[[f32; 4]]>, bool) {
        if let Some(cached) = self.0.get() {
            return (Arc::clone(cached), true);
        }
        let cached = self.0.get_or_init(|| Arc::<[[f32; 4]]>::from(generate()));
        (Arc::clone(cached), false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryDesc {
    topology: GeometryTopology,
    vertices: Vec<GeometryVertex>,
    indices: Vec<u32>,
    vertex_colors: OptionalVertexAttribute<Color>,
    tex_coords0: OptionalVertexAttribute<[f32; 2]>,
    tangents: Option<Vec<[f32; 4]>>,
    generated_tangent_cache: GeneratedTangentCache,
    triangle_bvh_cache: TriangleBvhCache,
    morph_targets: Vec<GeometryMorphTarget>,
    skin: Option<GeometrySkin>,
    bounds: Aabb,
}

#[cfg(test)]
mod pf07_tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::Arc;

    #[test]
    fn pf07_generated_model_tangent_cache_is_shared_and_geometry_equality_ignores_warmth() {
        let geometry = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                GeometryVertex {
                    position: Vec3::ZERO,
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::X,
                    normal: Vec3::Z,
                },
                GeometryVertex {
                    position: Vec3::Y,
                    normal: Vec3::Z,
                },
            ],
            vec![0, 1, 2],
        )
        .expect("triangle");
        let cold_clone = geometry.clone();
        let calls = Cell::new(0);
        let generate = || {
            calls.set(calls.get() + 1);
            vec![[1.0, 0.0, 0.0, 1.0]]
        };

        let (first, first_hit) = geometry.cached_generated_tangents(generate);
        let (second, second_hit) = cold_clone.cached_generated_tangents(generate);

        assert!(!first_hit);
        assert!(second_hit);
        assert_eq!(calls.get(), 1);
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            geometry, cold_clone,
            "cache warmth is not geometry identity"
        );
    }
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
}

impl GeometryDesc {
    pub fn try_new(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        Self::try_new_with_optional_vertex_attributes(topology, vertices, indices, None, None)
    }

    pub fn try_new_with_vertex_colors(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
        vertex_colors: Vec<Color>,
    ) -> Result<Self, GeometryError> {
        Self::try_new_with_optional_vertex_attributes(
            topology,
            vertices,
            indices,
            Some(vertex_colors),
            None,
        )
    }

    pub fn try_new_with_vertex_colors_and_tex_coords(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
        vertex_colors: Vec<Color>,
        tex_coords0: Vec<[f32; 2]>,
    ) -> Result<Self, GeometryError> {
        Self::try_new_with_optional_vertex_attributes(
            topology,
            vertices,
            indices,
            Some(vertex_colors),
            Some(tex_coords0),
        )
    }

    pub(crate) fn try_new_with_optional_vertex_attributes(
        topology: GeometryTopology,
        vertices: Vec<GeometryVertex>,
        indices: Vec<u32>,
        vertex_colors: Option<Vec<Color>>,
        tex_coords0: Option<Vec<[f32; 2]>>,
    ) -> Result<Self, GeometryError> {
        let Some(bounds) = Aabb::from_vertices(&vertices) else {
            return Err(GeometryError::EmptyVertices);
        };
        if let Some(vertex_colors) = vertex_colors.as_ref()
            && vertex_colors.len() != vertices.len()
        {
            return Err(GeometryError::InvalidVertexColorCount {
                vertex_count: vertices.len(),
                color_count: vertex_colors.len(),
            });
        }
        if let Some(tex_coords0) = tex_coords0.as_ref()
            && tex_coords0.len() != vertices.len()
        {
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
        let vertex_count = vertices.len();
        Ok(Self {
            topology,
            vertices,
            indices,
            vertex_colors: vertex_colors.map_or_else(
                || OptionalVertexAttribute::absent(vertex_count, Color::WHITE),
                |values| OptionalVertexAttribute::authored(values, Color::WHITE),
            ),
            tex_coords0: tex_coords0.map_or_else(
                || OptionalVertexAttribute::absent(vertex_count, [0.0, 0.0]),
                |values| OptionalVertexAttribute::authored(values, [0.0, 0.0]),
            ),
            tangents: None,
            generated_tangent_cache: GeneratedTangentCache::default(),
            triangle_bvh_cache: TriangleBvhCache::default(),
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

    /// Legacy infallible wrapper for fixed, trusted point lists.
    ///
    /// Runtime or untrusted input must use [`Self::try_polyline`] so short
    /// point lists return [`GeometryError::PolylineTooShort`] instead of
    /// unwinding.
    #[deprecated(note = "use GeometryDesc::try_polyline for untrusted or runtime input")]
    pub fn polyline(points: &[Vec3]) -> Self {
        Self::try_polyline(points).expect("polyline requires at least two points")
    }

    /// Builds connected line segments from two or more points.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{GeometryDesc, GeometryError, Vec3};
    ///
    /// let geometry = GeometryDesc::try_polyline(&[
    ///     Vec3::ZERO,
    ///     Vec3::new(1.0, 0.0, 0.0),
    /// ]).expect("two points form one line segment");
    /// assert_eq!(geometry.indices(), &[0, 1]);
    ///
    /// assert_eq!(
    ///     GeometryDesc::try_polyline(&[Vec3::ZERO]),
    ///     Err(GeometryError::PolylineTooShort { point_count: 1 }),
    /// );
    /// ```
    pub fn try_polyline(points: &[Vec3]) -> Result<Self, GeometryError> {
        if points.len() < 2 {
            return Err(GeometryError::PolylineTooShort {
                point_count: points.len(),
            });
        }
        let mut indices = Vec::with_capacity((points.len() - 1) * 2);
        for index in 0..points.len() as u32 - 1 {
            indices.extend_from_slice(&[index, index + 1]);
        }
        let vertices = points
            .iter()
            .copied()
            .map(|position| GeometryVertex {
                position,
                normal: Vec3::ZERO,
            })
            .collect();
        Self::try_new(GeometryTopology::Lines, vertices, indices)
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
        self.vertex_colors.as_compatibility_slice()
    }

    pub fn tex_coords0(&self) -> &[[f32; 2]] {
        self.tex_coords0.as_compatibility_slice()
    }

    pub(crate) fn authored_vertex_colors(&self) -> Option<&[Color]> {
        self.vertex_colors.authored_slice()
    }

    pub(crate) fn authored_tex_coords0(&self) -> Option<&[[f32; 2]]> {
        self.tex_coords0.authored_slice()
    }

    pub(crate) fn vertex_color_or_default(&self, index: usize) -> Color {
        self.vertex_colors
            .value(index)
            .copied()
            .unwrap_or(Color::WHITE)
    }

    pub(crate) fn tex_coord0_or_default(&self, index: usize) -> [f32; 2] {
        self.tex_coords0.value(index).copied().unwrap_or([0.0, 0.0])
    }

    pub(crate) fn cached_generated_tangents(
        &self,
        generate: impl FnOnce() -> Vec<[f32; 4]>,
    ) -> (Arc<[[f32; 4]]>, bool) {
        self.generated_tangent_cache.get_or_init(generate)
    }

    pub(crate) fn cached_triangle_bvh(&self) -> (Arc<TriangleBvh>, bool) {
        self.triangle_bvh_cache.get_or_init(self)
    }

    #[cfg(test)]
    pub(crate) fn optional_attribute_storage_bytes(&self) -> usize {
        self.vertex_colors.stored_bytes() + self.tex_coords0.stored_bytes()
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

#[cfg(test)]
mod optional_attribute_tests {
    use super::*;

    #[test]
    fn pf10_absent_optional_vertex_attributes_are_lazy_and_default_identically() {
        let vertices = vec![
            GeometryVertex {
                position: Vec3::ZERO,
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ];
        let geometry = GeometryDesc::try_new_with_optional_vertex_attributes(
            GeometryTopology::Triangles,
            vertices,
            vec![0, 1, 2],
            None,
            None,
        )
        .expect("valid geometry");

        assert_eq!(geometry.optional_attribute_storage_bytes(), 0);
        assert_eq!(geometry.vertex_color_or_default(2), Color::WHITE);
        assert_eq!(geometry.tex_coord0_or_default(2), [0.0, 0.0]);
        assert_eq!(geometry.vertex_colors(), [Color::WHITE; 3]);
        assert_eq!(geometry.tex_coords0(), [[0.0, 0.0]; 3]);
    }
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
