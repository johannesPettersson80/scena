use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{Color, GeometryDesc, GeometryTopology, GeometryVertex};
use crate::Vec3;

mod bevel_mesh;
mod curve_refinement;
mod topology;

use bevel_mesh::*;
use curve_refinement::*;
use topology::*;

const DEFAULT_EDGE_ANGLE_DEGREES: f32 = 30.0;
const AUTHORED_NORMAL_CONTINUITY_DEGREES: f32 = 1.0;
const DEFAULT_SEGMENTS: u8 = 3;
const DEFAULT_MAX_DERIVED_TRIANGLES: usize = 250_000;
const DEFAULT_MAX_CURVE_SUBDIVISIONS: u8 = 4;
const MAX_INSET_EDGE_FRACTION: f32 = 0.20;
const MAX_CURVE_DISPLACEMENT_EDGE_FRACTION: f32 = 0.05;
const DEFAULT_CURVE_TARGET_DIVISOR: f32 = 4096.0;
const PHONG_CURVE_BLEND: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EdgeRoundingOptions {
    radius: f32,
    edge_angle_degrees: f32,
    segments: u8,
    max_derived_triangles: usize,
    max_curve_subdivisions: u8,
    curve_target_error: Option<f32>,
}

impl EdgeRoundingOptions {
    pub(crate) fn new(radius: f32) -> Self {
        Self {
            radius,
            edge_angle_degrees: DEFAULT_EDGE_ANGLE_DEGREES,
            segments: DEFAULT_SEGMENTS,
            max_derived_triangles: DEFAULT_MAX_DERIVED_TRIANGLES,
            max_curve_subdivisions: DEFAULT_MAX_CURVE_SUBDIVISIONS,
            curve_target_error: None,
        }
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn with_edge_angle_degrees(mut self, degrees: f32) -> Self {
        self.edge_angle_degrees = degrees;
        self
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn with_segments(mut self, segments: u8) -> Self {
        self.segments = segments;
        self
    }

    pub(crate) fn with_max_derived_triangles(mut self, limit: usize) -> Self {
        self.max_derived_triangles = limit;
        self
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn with_curve_target_error(mut self, error: f32) -> Self {
        self.curve_target_error = Some(error);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EdgeRoundingReport {
    pub(crate) eligible_edges: usize,
    pub(crate) rounded_edges: usize,
    pub(crate) skipped_edges: usize,
    pub(crate) rejected_edges: usize,
    pub(crate) removed_degenerate_triangles: usize,
    pub(crate) source_triangles: usize,
    pub(crate) derived_triangles: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EdgeRoundingError {
    UnsupportedTopology,
    DeformingMesh,
    InvalidOptions,
    NonFiniteGeometry,
    DegenerateTriangles { count: usize },
    OpenMesh { boundary_edges: usize },
    NonManifoldMesh { nonmanifold_edges: usize },
    InconsistentWinding { edges: usize },
    DerivedTriangleBudgetExceeded { required: usize, limit: usize },
}

#[derive(Clone, Copy)]
struct Face {
    source: [usize; 3],
    welded: [usize; 3],
    normal: Vec3,
}

#[derive(Clone, Copy)]
struct EdgeUse {
    face: usize,
    from: usize,
    to: usize,
}

#[derive(Clone)]
struct MeshEdge {
    a: usize,
    b: usize,
    uses: Vec<EdgeUse>,
    hard: bool,
    round: bool,
    radius: f32,
    inset: f32,
}

#[derive(Clone, Copy)]
struct DerivedVertex {
    position: Vec3,
    normal: Vec3,
    color: Color,
    uv: [f32; 2],
}

#[derive(Clone, Copy)]
struct CurveEdge {
    start: GeometryVertex,
    end: GeometryVertex,
    midpoint_deviation: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RenderVertexKey {
    position: [i64; 3],
    color: [i64; 4],
    uv: [i64; 2],
}

#[derive(Clone)]
struct DisjointSet {
    parents: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parents: (0..len).collect(),
        }
    }

    fn root(&mut self, index: usize) -> usize {
        let parent = self.parents[index];
        if parent == index {
            index
        } else {
            let root = self.root(parent);
            self.parents[index] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.root(left);
        let right = self.root(right);
        if left != right {
            let (minimum, maximum) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            self.parents[maximum] = minimum;
        }
    }
}

pub(crate) fn round_hard_edges(
    geometry: &GeometryDesc,
    options: EdgeRoundingOptions,
) -> Result<(GeometryDesc, EdgeRoundingReport), EdgeRoundingError> {
    validate_options(options)?;
    if geometry.topology() != GeometryTopology::Triangles {
        return Err(EdgeRoundingError::UnsupportedTopology);
    }
    if geometry.skin().is_some() || !geometry.morph_targets().is_empty() {
        return Err(EdgeRoundingError::DeformingMesh);
    }
    if geometry
        .vertices()
        .iter()
        .any(|vertex| !vertex.position.is_finite())
    {
        return Err(EdgeRoundingError::NonFiniteGeometry);
    }

    let source_triangles = geometry.indices().len() / 3;
    let (welded_positions, source_to_welded) = weld_positions(geometry);
    let minimum_area_squared = minimum_triangle_area_squared(geometry);
    let (faces, removed_degenerate_triangles) =
        build_faces(geometry, &source_to_welded, minimum_area_squared)?;
    let mut edges = build_edges(&faces);
    validate_topology(&edges)?;

    let threshold_cos = options.edge_angle_degrees.to_radians().cos();
    for edge in edges.values_mut() {
        let [left, right] = [edge.uses[0], edge.uses[1]];
        let dot = faces[left.face]
            .normal
            .dot(faces[right.face].normal)
            .clamp(-1.0, 1.0);
        let angle = dot.acos();
        edge.hard =
            dot < threshold_cos && !edge_has_continuous_authored_normals(geometry, &faces, edge);
        edge.round = edge.hard;
        if edge.hard {
            let edge_length = welded_positions[edge.a].distance(welded_positions[edge.b]);
            let tangent = (angle * 0.5).tan().abs().max(1.0e-4);
            let adjacent_altitude = [left, right]
                .into_iter()
                .map(|edge_use| {
                    face_altitude_to_edge(faces[edge_use.face], edge, &welded_positions)
                })
                .fold(f32::INFINITY, f32::min);
            let maximum_inset = (edge_length * MAX_INSET_EDGE_FRACTION)
                .min(adjacent_altitude * MAX_INSET_EDGE_FRACTION);
            edge.inset = (options.radius / tangent).min(maximum_inset);
            edge.radius = edge.inset * tangent;
        }
    }
    suppress_faceted_curved_rim_cycles(
        &mut edges,
        &welded_positions,
        options.edge_angle_degrees.to_radians(),
    );

    let mut smooth_regions = DisjointSet::new(faces.len());
    for edge in edges.values().filter(|edge| !edge.hard) {
        let [left, right] = [edge.uses[0], edge.uses[1]];
        smooth_regions.union(left.face, right.face);
    }
    let region_roots = (0..faces.len())
        .map(|face| smooth_regions.root(face))
        .collect::<Vec<_>>();

    let hard_edge_count = edges.values().filter(|edge| edge.round).count();
    if hard_edge_count == 0 {
        let smooth = geometry_with_smooth_region_normals(
            geometry,
            &faces,
            &welded_positions,
            &region_roots,
            minimum_area_squared,
        )?;
        return finish_with_curve_refinement(
            geometry,
            smooth,
            EdgeRoundingReport {
                eligible_edges: 0,
                rounded_edges: 0,
                skipped_edges: edges.len(),
                rejected_edges: 0,
                removed_degenerate_triangles,
                source_triangles,
                derived_triangles: faces.len(),
            },
            options,
        );
    }

    let displacements = cluster_displacements(&faces, &edges, &welded_positions, &region_roots);
    let corner_normals = recomputed_corner_normals(
        &faces,
        &welded_positions,
        &region_roots,
        &displacements,
        minimum_area_squared,
    )?;

    let minimum_required_triangles = source_triangles
        .saturating_add(
            hard_edge_count
                .saturating_mul(usize::from(options.segments))
                .saturating_mul(2),
        )
        .saturating_add(hard_edge_count.saturating_mul(2));
    if minimum_required_triangles > options.max_derived_triangles {
        return Err(EdgeRoundingError::DerivedTriangleBudgetExceeded {
            required: minimum_required_triangles,
            limit: options.max_derived_triangles,
        });
    }

    let mut derived = Vec::<DerivedVertex>::new();
    let mut indices = Vec::<u32>::new();
    for (face_index, face) in faces.iter().enumerate() {
        let mut triangle = [0_u32; 3];
        for (corner, triangle_vertex) in triangle.iter_mut().enumerate() {
            let welded = face.welded[corner];
            let region = region_roots[face_index];
            let position = welded_positions[welded]
                + displacements
                    .get(&(welded, region))
                    .copied()
                    .unwrap_or(Vec3::ZERO);
            *triangle_vertex = push_vertex(
                &mut derived,
                DerivedVertex {
                    position,
                    normal: corner_normals[&(welded, region)],
                    color: geometry.vertex_color_or_default(face.source[corner]),
                    uv: geometry.tex_coord0_or_default(face.source[corner]),
                },
            );
        }
        push_oriented_triangle(&mut indices, triangle, face.normal, &derived);
    }

    let mut corner_boundaries = BTreeMap::<usize, Vec<DerivedVertex>>::new();
    for edge in edges.values().filter(|edge| edge.round) {
        append_bevel_strip(
            geometry,
            &faces,
            edge,
            &region_roots,
            &welded_positions,
            &displacements,
            &corner_normals,
            options.segments,
            &mut derived,
            &mut indices,
            &mut corner_boundaries,
        );
    }
    append_corner_patches(
        geometry,
        &faces,
        &welded_positions,
        &corner_boundaries,
        &mut derived,
        &mut indices,
    );

    let derived_triangles = indices.len() / 3;
    if derived_triangles > options.max_derived_triangles {
        return Err(EdgeRoundingError::DerivedTriangleBudgetExceeded {
            required: derived_triangles,
            limit: options.max_derived_triangles,
        });
    }

    let vertices = derived
        .iter()
        .map(|vertex| GeometryVertex {
            position: vertex.position,
            normal: vertex.normal.normalize_or_zero(),
        })
        .collect();
    let colors = geometry
        .authored_vertex_colors()
        .is_some()
        .then(|| derived.iter().map(|vertex| vertex.color).collect());
    let uvs = geometry
        .authored_tex_coords0()
        .is_some()
        .then(|| derived.iter().map(|vertex| vertex.uv).collect());
    let rounded = GeometryDesc::try_new_with_optional_vertex_attributes(
        GeometryTopology::Triangles,
        vertices,
        indices,
        colors,
        uvs,
    )
    .expect("validated edge-rounding streams remain internally consistent");

    finish_with_curve_refinement(
        geometry,
        rounded,
        EdgeRoundingReport {
            eligible_edges: hard_edge_count,
            rounded_edges: hard_edge_count,
            skipped_edges: edges.len().saturating_sub(hard_edge_count),
            rejected_edges: 0,
            removed_degenerate_triangles,
            source_triangles,
            derived_triangles,
        },
        options,
    )
}
