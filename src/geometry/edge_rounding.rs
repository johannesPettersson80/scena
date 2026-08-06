use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{Color, GeometryDesc, GeometryTopology, GeometryVertex};
use crate::Vec3;

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

fn finish_with_curve_refinement(
    source: &GeometryDesc,
    rounded: GeometryDesc,
    mut report: EdgeRoundingReport,
    options: EdgeRoundingOptions,
) -> Result<(GeometryDesc, EdgeRoundingReport), EdgeRoundingError> {
    let refined = refine_smooth_curves(&rounded, options).unwrap_or_else(|()| source.clone());
    report.derived_triangles = refined.indices().len() / 3;
    if report.derived_triangles > options.max_derived_triangles {
        return Err(EdgeRoundingError::DerivedTriangleBudgetExceeded {
            required: report.derived_triangles,
            limit: options.max_derived_triangles,
        });
    }
    if refined == *source && report.rounded_edges == 0 {
        report.derived_triangles = report.source_triangles;
    }
    Ok((refined, report))
}

fn refine_smooth_curves(
    geometry: &GeometryDesc,
    options: EdgeRoundingOptions,
) -> Result<GeometryDesc, ()> {
    let source_triangles = geometry.indices().len() / 3;
    if source_triangles == 0 || source_triangles >= options.max_derived_triangles {
        return Ok(geometry.clone());
    }

    let (_, source_to_welded) = weld_positions(geometry);
    let curve_edges = build_curve_edges(geometry, &source_to_welded);
    let target_error = options.curve_target_error.unwrap_or_else(|| {
        (geometry.bounds().half_extent().max_element() * 2.0).max(1.0e-6)
            / DEFAULT_CURVE_TARGET_DIVISOR
    });
    let subdivisions = curve_refinement_subdivisions(
        source_triangles,
        options.max_derived_triangles,
        options.max_curve_subdivisions,
        target_error,
        &curve_edges,
    );
    if subdivisions <= 1 {
        return Ok(geometry.clone());
    }

    let subdivisions = usize::from(subdivisions);
    let derived_triangles = source_triangles.saturating_mul(subdivisions * subdivisions);
    let derived_vertices_per_triangle = (subdivisions + 1) * (subdivisions + 2) / 2;
    let mut derived =
        Vec::<DerivedVertex>::with_capacity(source_triangles * derived_vertices_per_triangle);
    let mut indices = Vec::<u32>::with_capacity(derived_triangles * 3);
    for triangle in geometry.indices().chunks_exact(3) {
        let source = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let welded = source.map(|index| source_to_welded[index]);
        let corners = source.map(|index| geometry.vertices()[index]);
        let colors = source.map(|index| geometry.vertex_color_or_default(index));
        let uvs = source.map(|index| geometry.tex_coord0_or_default(index));
        let desired_normal = (corners[1].position - corners[0].position)
            .cross(corners[2].position - corners[0].position)
            .normalize_or_zero();
        let mut grid = Vec::<Vec<u32>>::with_capacity(subdivisions + 1);
        for row in 0..=subdivisions {
            let mut grid_row = Vec::with_capacity(subdivisions - row + 1);
            for column in 0..=subdivisions - row {
                let weights = [
                    (subdivisions - row - column) as f32 / subdivisions as f32,
                    column as f32 / subdivisions as f32,
                    row as f32 / subdivisions as f32,
                ];
                let linear = corners[0].position * weights[0]
                    + corners[1].position * weights[1]
                    + corners[2].position * weights[2];
                let position = refined_triangle_position(
                    corners,
                    welded,
                    &curve_edges,
                    row,
                    column,
                    subdivisions,
                    weights,
                );
                let local_edge = refinement_local_edge_length(corners, row, column, subdivisions);
                let envelope = local_edge * MAX_CURVE_DISPLACEMENT_EDGE_FRACTION;
                let epsilon = local_edge.max(1.0e-6) * 1.0e-5;
                if !position.is_finite()
                    || position.distance(linear) > envelope + epsilon
                    || expands_unaffected_triangle_axis(corners, linear, position, epsilon)
                {
                    return Err(());
                }
                let normal = weighted_normal(corners.map(|corner| corner.normal), weights)
                    .unwrap_or(desired_normal);
                grid_row.push(push_vertex(
                    &mut derived,
                    DerivedVertex {
                        position,
                        normal,
                        color: weighted_color(colors, weights),
                        uv: weighted_uv(uvs, weights),
                    },
                ));
            }
            grid.push(grid_row);
        }
        for row in 0..subdivisions {
            for column in 0..subdivisions - row {
                push_oriented_triangle(
                    &mut indices,
                    [
                        grid[row][column],
                        grid[row][column + 1],
                        grid[row + 1][column],
                    ],
                    desired_normal,
                    &derived,
                );
                if column + 1 < subdivisions - row {
                    push_oriented_triangle(
                        &mut indices,
                        [
                            grid[row][column + 1],
                            grid[row + 1][column + 1],
                            grid[row + 1][column],
                        ],
                        desired_normal,
                        &derived,
                    );
                }
            }
        }
    }

    let (derived, indices) = weld_continuous_render_vertices(derived, indices, geometry);
    let vertices = derived
        .iter()
        .map(|vertex| GeometryVertex {
            position: vertex.position,
            normal: vertex.normal,
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
    Ok(GeometryDesc::try_new_with_optional_vertex_attributes(
        GeometryTopology::Triangles,
        vertices,
        indices,
        colors,
        uvs,
    )
    .expect("bounded curve refinement preserves valid geometry streams"))
}

fn refinement_local_edge_length(
    corners: [GeometryVertex; 3],
    row: usize,
    column: usize,
    subdivisions: usize,
) -> f32 {
    let edge_lengths = [
        corners[0].position.distance(corners[1].position),
        corners[0].position.distance(corners[2].position),
        corners[1].position.distance(corners[2].position),
    ];
    if row == 0 {
        edge_lengths[0]
    } else if column == 0 {
        edge_lengths[1]
    } else if row + column == subdivisions {
        edge_lengths[2]
    } else {
        edge_lengths.into_iter().fold(f32::INFINITY, f32::min)
    }
}

fn expands_unaffected_triangle_axis(
    corners: [GeometryVertex; 3],
    linear: Vec3,
    refined: Vec3,
    epsilon: f32,
) -> bool {
    (0..3).any(|axis| {
        let source_min = corners
            .iter()
            .map(|corner| corner.position[axis])
            .fold(f32::INFINITY, f32::min);
        let source_max = corners
            .iter()
            .map(|corner| corner.position[axis])
            .fold(f32::NEG_INFINITY, f32::max);
        source_max - source_min <= epsilon && (refined[axis] - linear[axis]).abs() > epsilon
    })
}

fn weld_continuous_render_vertices(
    vertices: Vec<DerivedVertex>,
    indices: Vec<u32>,
    source: &GeometryDesc,
) -> (Vec<DerivedVertex>, Vec<u32>) {
    let position_quantum = (source.bounds().half_extent().max_element() * 2.0).max(1.0e-6) * 1.0e-6;
    let normal_cosine = 0.1_f32.to_radians().cos();
    let mut welded = Vec::<DerivedVertex>::with_capacity(vertices.len());
    let mut buckets = HashMap::<RenderVertexKey, Vec<u32>>::new();
    let mut remap = Vec::<u32>::with_capacity(vertices.len());
    for vertex in vertices {
        let key = render_vertex_key(vertex, position_quantum);
        let candidates = buckets.entry(key).or_default();
        let existing = candidates.iter().copied().find(|candidate| {
            let canonical = welded[*candidate as usize];
            canonical.position.distance(vertex.position) <= position_quantum
                && canonical
                    .normal
                    .normalize_or_zero()
                    .dot(vertex.normal.normalize_or_zero())
                    >= normal_cosine
        });
        let index = existing.unwrap_or_else(|| {
            let index = u32::try_from(welded.len()).expect("derived vertex count fits u32");
            welded.push(vertex);
            candidates.push(index);
            index
        });
        remap.push(index);
    }
    let indices = indices
        .into_iter()
        .map(|index| remap[index as usize])
        .collect();
    (welded, indices)
}

fn render_vertex_key(vertex: DerivedVertex, position_quantum: f32) -> RenderVertexKey {
    RenderVertexKey {
        position: [
            quantize(vertex.position.x, position_quantum),
            quantize(vertex.position.y, position_quantum),
            quantize(vertex.position.z, position_quantum),
        ],
        color: [
            quantize(vertex.color.r, 1.0e-6),
            quantize(vertex.color.g, 1.0e-6),
            quantize(vertex.color.b, 1.0e-6),
            quantize(vertex.color.a, 1.0e-6),
        ],
        uv: [
            quantize(vertex.uv[0], 1.0e-6),
            quantize(vertex.uv[1], 1.0e-6),
        ],
    }
}

fn quantize(value: f32, quantum: f32) -> i64 {
    (value / quantum).round() as i64
}

fn build_curve_edges(
    geometry: &GeometryDesc,
    source_to_welded: &[usize],
) -> BTreeMap<(usize, usize), CurveEdge> {
    let mut edges = BTreeMap::<(usize, usize), CurveEdge>::new();
    for triangle in geometry.indices().chunks_exact(3) {
        let source = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        for [from_corner, to_corner] in [[0, 1], [1, 2], [2, 0]] {
            let from = source[from_corner];
            let to = source[to_corner];
            let from_welded = source_to_welded[from];
            let to_welded = source_to_welded[to];
            if from_welded == to_welded {
                continue;
            }
            let (key, start, end) = if from_welded < to_welded {
                (
                    (from_welded, to_welded),
                    geometry.vertices()[from],
                    geometry.vertices()[to],
                )
            } else {
                (
                    (to_welded, from_welded),
                    geometry.vertices()[to],
                    geometry.vertices()[from],
                )
            };
            let midpoint = curve_edge_point(start, end, 0.5);
            let linear_midpoint = start.position.lerp(end.position, 0.5);
            let midpoint_deviation = midpoint.distance(linear_midpoint);
            if midpoint_deviation <= 0.0 || !midpoint_deviation.is_finite() {
                continue;
            }
            let candidate = CurveEdge {
                start,
                end,
                midpoint_deviation,
            };
            if edges
                .get(&key)
                .is_none_or(|current| candidate.midpoint_deviation > current.midpoint_deviation)
            {
                edges.insert(key, candidate);
            }
        }
    }
    edges
}

fn curve_refinement_subdivisions(
    source_triangles: usize,
    max_derived_triangles: usize,
    max_subdivisions: u8,
    target_error: f32,
    curve_edges: &BTreeMap<(usize, usize), CurveEdge>,
) -> u8 {
    let maximum_deviation = curve_edges
        .values()
        .map(|edge| edge.midpoint_deviation)
        .fold(0.0_f32, f32::max);
    if maximum_deviation <= target_error {
        return 1;
    }
    let requested = (maximum_deviation / target_error)
        .sqrt()
        .ceil()
        .clamp(2.0, f32::from(max_subdivisions)) as u8;
    (2..=requested)
        .rev()
        .find(|subdivisions| {
            source_triangles.saturating_mul(usize::from(*subdivisions).pow(2))
                <= max_derived_triangles
        })
        .unwrap_or(1)
}

fn refined_triangle_position(
    corners: [GeometryVertex; 3],
    welded: [usize; 3],
    curve_edges: &BTreeMap<(usize, usize), CurveEdge>,
    row: usize,
    column: usize,
    subdivisions: usize,
    weights: [f32; 3],
) -> Vec3 {
    if row == 0 {
        return shared_curve_edge_point(
            welded[0],
            welded[1],
            column as f32 / subdivisions as f32,
            corners[0].position.lerp(corners[1].position, weights[1]),
            curve_edges,
        );
    }
    if column == 0 {
        return shared_curve_edge_point(
            welded[0],
            welded[2],
            row as f32 / subdivisions as f32,
            corners[0].position.lerp(corners[2].position, weights[2]),
            curve_edges,
        );
    }
    if row + column == subdivisions {
        return shared_curve_edge_point(
            welded[1],
            welded[2],
            row as f32 / subdivisions as f32,
            corners[1].position.lerp(corners[2].position, weights[2]),
            curve_edges,
        );
    }
    phong_triangle_point(corners, weights)
}

fn shared_curve_edge_point(
    from: usize,
    to: usize,
    t: f32,
    linear: Vec3,
    curve_edges: &BTreeMap<(usize, usize), CurveEdge>,
) -> Vec3 {
    let (key, t) = if from < to {
        ((from, to), t)
    } else {
        ((to, from), 1.0 - t)
    };
    curve_edges
        .get(&key)
        .map_or(linear, |edge| curve_edge_point(edge.start, edge.end, t))
}

fn curve_edge_point(start: GeometryVertex, end: GeometryVertex, t: f32) -> Vec3 {
    let linear = start.position.lerp(end.position, t);
    let start_normal = start.normal.normalize_or_zero();
    let end_normal = end.normal.normalize_or_zero();
    if start_normal.length_squared() < 0.5 || end_normal.length_squared() < 0.5 {
        return linear;
    }
    let start_projection = linear - start_normal * (linear - start.position).dot(start_normal);
    let end_projection = linear - end_normal * (linear - end.position).dot(end_normal);
    bounded_curve_point(
        linear,
        start_projection.lerp(end_projection, t),
        start.position.distance(end.position),
    )
}

fn phong_triangle_point(corners: [GeometryVertex; 3], weights: [f32; 3]) -> Vec3 {
    let linear = corners[0].position * weights[0]
        + corners[1].position * weights[1]
        + corners[2].position * weights[2];
    let mut curved = Vec3::ZERO;
    for (corner, weight) in corners.iter().copied().zip(weights) {
        let normal = corner.normal.normalize_or_zero();
        let projected = if normal.length_squared() >= 0.5 {
            linear - normal * (linear - corner.position).dot(normal)
        } else {
            linear
        };
        curved += projected * weight;
    }
    let maximum_edge_length = corners[0]
        .position
        .distance(corners[1].position)
        .max(corners[1].position.distance(corners[2].position))
        .max(corners[2].position.distance(corners[0].position));
    bounded_curve_point(linear, curved, maximum_edge_length)
}

fn bounded_curve_point(linear: Vec3, curved: Vec3, edge_length: f32) -> Vec3 {
    let curved = linear.lerp(curved, PHONG_CURVE_BLEND);
    let displacement = curved - linear;
    let maximum_displacement = edge_length * MAX_CURVE_DISPLACEMENT_EDGE_FRACTION;
    if !curved.is_finite() || !maximum_displacement.is_finite() {
        return linear;
    }
    if displacement.length() > maximum_displacement {
        linear + displacement.normalize_or_zero() * maximum_displacement
    } else {
        curved
    }
}

fn weighted_normal(normals: [Vec3; 3], weights: [f32; 3]) -> Option<Vec3> {
    let normal = normals[0] * weights[0] + normals[1] * weights[1] + normals[2] * weights[2];
    let normal = normal.normalize_or_zero();
    (normal.length_squared() >= 0.5 && normal.is_finite()).then_some(normal)
}

fn weighted_color(colors: [Color; 3], weights: [f32; 3]) -> Color {
    Color::from_linear_rgba(
        colors[0].r * weights[0] + colors[1].r * weights[1] + colors[2].r * weights[2],
        colors[0].g * weights[0] + colors[1].g * weights[1] + colors[2].g * weights[2],
        colors[0].b * weights[0] + colors[1].b * weights[1] + colors[2].b * weights[2],
        colors[0].a * weights[0] + colors[1].a * weights[1] + colors[2].a * weights[2],
    )
}

fn weighted_uv(uvs: [[f32; 2]; 3], weights: [f32; 3]) -> [f32; 2] {
    [
        uvs[0][0] * weights[0] + uvs[1][0] * weights[1] + uvs[2][0] * weights[2],
        uvs[0][1] * weights[0] + uvs[1][1] * weights[1] + uvs[2][1] * weights[2],
    ]
}

fn face_altitude_to_edge(face: Face, edge: &MeshEdge, positions: &[Vec3]) -> f32 {
    let opposite = face
        .welded
        .into_iter()
        .find(|vertex| *vertex != edge.a && *vertex != edge.b)
        .expect("triangle adjacent to edge has one opposite vertex");
    let a = positions[edge.a];
    let b = positions[edge.b];
    let point = positions[opposite];
    let direction = (b - a).normalize_or_zero();
    let closest = a + direction * (point - a).dot(direction);
    point.distance(closest)
}

fn validate_options(options: EdgeRoundingOptions) -> Result<(), EdgeRoundingError> {
    if !options.radius.is_finite()
        || options.radius <= 0.0
        || !options.edge_angle_degrees.is_finite()
        || !(0.0..180.0).contains(&options.edge_angle_degrees)
        || options.segments == 0
        || options.max_derived_triangles == 0
        || options.max_curve_subdivisions < 2
        || options
            .curve_target_error
            .is_some_and(|error| !error.is_finite() || error <= 0.0)
    {
        return Err(EdgeRoundingError::InvalidOptions);
    }
    Ok(())
}

fn weld_positions(geometry: &GeometryDesc) -> (Vec<Vec3>, Vec<usize>) {
    let scale = geometry.bounds().half_extent().max_element().max(1.0e-6) * 2.0;
    let epsilon = scale * 1.0e-6;
    let mut positions = Vec::<Vec3>::new();
    let mut buckets = HashMap::<[i64; 3], Vec<usize>>::new();
    let mut remap = Vec::with_capacity(geometry.vertices().len());
    for vertex in geometry.vertices() {
        let position = vertex.position;
        let key = [
            (position.x / epsilon).round() as i64,
            (position.y / epsilon).round() as i64,
            (position.z / epsilon).round() as i64,
        ];
        let welded = buckets
            .get(&key)
            .and_then(|candidates| {
                candidates.iter().copied().find(|candidate| {
                    positions[*candidate].distance_squared(position) <= epsilon * epsilon
                })
            })
            .unwrap_or_else(|| {
                let index = positions.len();
                positions.push(position);
                buckets.entry(key).or_default().push(index);
                index
            });
        remap.push(welded);
    }
    (positions, remap)
}

fn build_faces(
    geometry: &GeometryDesc,
    source_to_welded: &[usize],
    minimum_area_squared: f32,
) -> Result<(Vec<Face>, usize), EdgeRoundingError> {
    let mut faces = Vec::with_capacity(geometry.indices().len() / 3);
    let mut degenerate = 0;
    for triangle in geometry.indices().chunks_exact(3) {
        let source = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let welded = [
            source_to_welded[source[0]],
            source_to_welded[source[1]],
            source_to_welded[source[2]],
        ];
        let a = geometry.vertices()[source[0]].position;
        let b = geometry.vertices()[source[1]].position;
        let c = geometry.vertices()[source[2]].position;
        let cross = (b - a).cross(c - a);
        if welded[0] == welded[1]
            || welded[1] == welded[2]
            || welded[2] == welded[0]
            || !cross.is_finite()
            || cross.length_squared() <= minimum_area_squared
        {
            degenerate += 1;
            continue;
        }
        faces.push(Face {
            source,
            welded,
            normal: cross.normalize(),
        });
    }
    if faces.is_empty() && degenerate > 0 {
        return Err(EdgeRoundingError::DegenerateTriangles { count: degenerate });
    }
    Ok((faces, degenerate))
}

fn minimum_triangle_area_squared(geometry: &GeometryDesc) -> f32 {
    let bounds_scale = geometry.bounds().bounding_sphere_radius().max(1.0e-6);
    bounds_scale.powi(4) * 1.0e-16
}

fn build_edges(faces: &[Face]) -> BTreeMap<(usize, usize), MeshEdge> {
    let mut edges = BTreeMap::<(usize, usize), MeshEdge>::new();
    for (face_index, face) in faces.iter().enumerate() {
        for corner in 0..3 {
            let from = face.welded[corner];
            let to = face.welded[(corner + 1) % 3];
            let key = if from < to { (from, to) } else { (to, from) };
            edges
                .entry(key)
                .or_insert_with(|| MeshEdge {
                    a: key.0,
                    b: key.1,
                    uses: Vec::with_capacity(2),
                    hard: false,
                    round: false,
                    radius: 0.0,
                    inset: 0.0,
                })
                .uses
                .push(EdgeUse {
                    face: face_index,
                    from,
                    to,
                });
        }
    }
    edges
}

fn edge_has_continuous_authored_normals(
    geometry: &GeometryDesc,
    faces: &[Face],
    edge: &MeshEdge,
) -> bool {
    let [left, right] = [edge.uses[0], edge.uses[1]];
    let minimum_dot = AUTHORED_NORMAL_CONTINUITY_DEGREES.to_radians().cos();
    [edge.a, edge.b].into_iter().all(|welded| {
        let left_normal =
            geometry.vertices()[source_corner_for_welded(&faces[left.face], welded)].normal;
        let right_normal =
            geometry.vertices()[source_corner_for_welded(&faces[right.face], welded)].normal;
        left_normal.is_finite()
            && right_normal.is_finite()
            && left_normal.length_squared() >= 0.5
            && right_normal.length_squared() >= 0.5
            && left_normal
                .normalize_or_zero()
                .dot(right_normal.normalize_or_zero())
                >= minimum_dot
    })
}

fn suppress_faceted_curved_rim_cycles(
    edges: &mut BTreeMap<(usize, usize), MeshEdge>,
    positions: &[Vec3],
    maximum_turn: f32,
) {
    let mut adjacency = BTreeMap::<usize, Vec<(usize, usize)>>::new();
    for (key, edge) in edges.iter().filter(|(_, edge)| edge.hard) {
        adjacency.entry(edge.a).or_default().push(*key);
        adjacency.entry(edge.b).or_default().push(*key);
    }

    let mut visited = BTreeSet::<(usize, usize)>::new();
    let seeds = edges
        .iter()
        .filter_map(|(key, edge)| edge.hard.then_some(*key))
        .collect::<Vec<_>>();
    for seed in seeds {
        if visited.contains(&seed) {
            continue;
        }
        let mut pending = vec![seed];
        let mut component_edges = Vec::new();
        let mut component_vertices = BTreeSet::new();
        while let Some(key) = pending.pop() {
            if !visited.insert(key) {
                continue;
            }
            component_edges.push(key);
            let edge = &edges[&key];
            for vertex in [edge.a, edge.b] {
                component_vertices.insert(vertex);
                if let Some(connected) = adjacency.get(&vertex) {
                    pending.extend(
                        connected
                            .iter()
                            .copied()
                            .filter(|connected| !visited.contains(connected)),
                    );
                }
            }
        }

        let closed_shallow_cycle = component_vertices.iter().all(|vertex| {
            let Some([left, right]) = adjacency.get(vertex).map(Vec::as_slice) else {
                return false;
            };
            let other = |key: (usize, usize)| {
                let edge = &edges[&key];
                if edge.a == *vertex { edge.b } else { edge.a }
            };
            let left_direction = (positions[other(*left)] - positions[*vertex]).normalize_or_zero();
            let right_direction =
                (positions[other(*right)] - positions[*vertex]).normalize_or_zero();
            let interior = left_direction.dot(right_direction).clamp(-1.0, 1.0).acos();
            (std::f32::consts::PI - interior).abs() <= maximum_turn
        });
        if closed_shallow_cycle {
            for key in component_edges {
                edges
                    .get_mut(&key)
                    .expect("hard-edge component key remains present")
                    .round = false;
            }
        }
    }
}

fn validate_topology(edges: &BTreeMap<(usize, usize), MeshEdge>) -> Result<(), EdgeRoundingError> {
    let boundary_edges = edges.values().filter(|edge| edge.uses.len() == 1).count();
    if boundary_edges > 0 {
        return Err(EdgeRoundingError::OpenMesh { boundary_edges });
    }
    let nonmanifold_edges = edges.values().filter(|edge| edge.uses.len() > 2).count();
    if nonmanifold_edges > 0 {
        return Err(EdgeRoundingError::NonManifoldMesh { nonmanifold_edges });
    }
    let inconsistent = edges
        .values()
        .filter(|edge| {
            edge.uses.len() == 2
                && edge.uses[0].from == edge.uses[1].from
                && edge.uses[0].to == edge.uses[1].to
        })
        .count();
    if inconsistent > 0 {
        return Err(EdgeRoundingError::InconsistentWinding {
            edges: inconsistent,
        });
    }
    Ok(())
}

fn cluster_displacements(
    faces: &[Face],
    edges: &BTreeMap<(usize, usize), MeshEdge>,
    welded_positions: &[Vec3],
    region_roots: &[usize],
) -> BTreeMap<(usize, usize), Vec3> {
    let mut constraints = BTreeMap::<(usize, usize), Vec<(Vec3, f32)>>::new();
    for edge in edges.values().filter(|edge| edge.hard) {
        for edge_use in &edge.uses {
            let face = faces[edge_use.face];
            let edge_direction = (welded_positions[edge_use.to] - welded_positions[edge_use.from])
                .normalize_or_zero();
            let inward = face.normal.cross(edge_direction).normalize_or_zero();
            for welded in [edge.a, edge.b] {
                constraints
                    .entry((welded, region_roots[edge_use.face]))
                    .or_default()
                    .push((inward, edge.inset));
            }
        }
    }
    constraints
        .into_iter()
        .map(|(key, constraints)| {
            let mut displacement = Vec3::ZERO;
            for _ in 0..16 {
                for (inward, inset) in &constraints {
                    displacement += *inward * (*inset - displacement.dot(*inward));
                }
            }
            let max_inset = constraints
                .iter()
                .map(|(_, inset)| *inset)
                .fold(0.0_f32, f32::max);
            if displacement.length() > max_inset * 4.0 {
                displacement = displacement.normalize_or_zero() * max_inset * 4.0;
            }
            (key, displacement)
        })
        .collect()
}

fn recomputed_corner_normals(
    faces: &[Face],
    welded_positions: &[Vec3],
    region_roots: &[usize],
    displacements: &BTreeMap<(usize, usize), Vec3>,
    minimum_area_squared: f32,
) -> Result<BTreeMap<(usize, usize), Vec3>, EdgeRoundingError> {
    let mut normals = BTreeMap::<(usize, usize), Vec3>::new();
    for (face_index, face) in faces.iter().enumerate() {
        let region = region_roots[face_index];
        let positions = face.welded.map(|welded| {
            welded_positions[welded]
                + displacements
                    .get(&(welded, region))
                    .copied()
                    .unwrap_or(Vec3::ZERO)
        });
        let cross = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
        if !cross.is_finite() || cross.length_squared() <= minimum_area_squared {
            return Err(EdgeRoundingError::DegenerateTriangles { count: 1 });
        }
        for welded in face.welded {
            *normals.entry((welded, region)).or_default() += cross;
        }
    }
    for normal in normals.values_mut() {
        *normal = normal.normalize_or_zero();
    }
    Ok(normals)
}

fn geometry_with_smooth_region_normals(
    geometry: &GeometryDesc,
    faces: &[Face],
    welded_positions: &[Vec3],
    region_roots: &[usize],
    minimum_area_squared: f32,
) -> Result<GeometryDesc, EdgeRoundingError> {
    let geometric_normals = recomputed_corner_normals(
        faces,
        welded_positions,
        region_roots,
        &BTreeMap::new(),
        minimum_area_squared,
    )?;
    let mut authored_normals = BTreeMap::<(usize, usize), Vec<Vec3>>::new();
    for (face_index, face) in faces.iter().enumerate() {
        let region = region_roots[face_index];
        for corner in 0..3 {
            authored_normals
                .entry((face.welded[corner], region))
                .or_default()
                .push(geometry.vertices()[face.source[corner]].normal);
        }
    }
    let minimum_dot = AUTHORED_NORMAL_CONTINUITY_DEGREES.to_radians().cos();
    let normals = authored_normals
        .into_iter()
        .map(|(key, authored)| {
            let reference = authored[0].normalize_or_zero();
            let continuous = reference.length_squared() >= 0.5
                && authored.iter().all(|normal| {
                    normal.is_finite()
                        && normal.length_squared() >= 0.5
                        && reference.dot(normal.normalize_or_zero()) >= minimum_dot
                });
            let normal = if continuous {
                authored
                    .into_iter()
                    .fold(Vec3::ZERO, |sum, normal| sum + normal)
                    .normalize_or_zero()
            } else {
                geometric_normals[&key]
            };
            (key, normal)
        })
        .collect::<BTreeMap<_, _>>();

    let mut derived = Vec::with_capacity(faces.len() * 3);
    let mut indices = Vec::with_capacity(faces.len() * 3);
    for (face_index, face) in faces.iter().enumerate() {
        let region = region_roots[face_index];
        for corner in 0..3 {
            let source = face.source[corner];
            indices.push(push_vertex(
                &mut derived,
                DerivedVertex {
                    position: geometry.vertices()[source].position,
                    normal: normals[&(face.welded[corner], region)],
                    color: geometry.vertex_color_or_default(source),
                    uv: geometry.tex_coord0_or_default(source),
                },
            ));
        }
    }
    let (derived, indices) = weld_continuous_render_vertices(derived, indices, geometry);
    let vertices = derived
        .iter()
        .map(|vertex| GeometryVertex {
            position: vertex.position,
            normal: vertex.normal,
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
    Ok(GeometryDesc::try_new_with_optional_vertex_attributes(
        GeometryTopology::Triangles,
        vertices,
        indices,
        colors,
        uvs,
    )
    .expect("smooth-region normal reconstruction preserves valid geometry streams"))
}

#[allow(clippy::too_many_arguments)]
fn append_bevel_strip(
    geometry: &GeometryDesc,
    faces: &[Face],
    edge: &MeshEdge,
    region_roots: &[usize],
    welded_positions: &[Vec3],
    displacements: &BTreeMap<(usize, usize), Vec3>,
    corner_normals: &BTreeMap<(usize, usize), Vec3>,
    segments: u8,
    derived: &mut Vec<DerivedVertex>,
    indices: &mut Vec<u32>,
    corner_boundaries: &mut BTreeMap<usize, Vec<DerivedVertex>>,
) {
    let left = edge.uses[0];
    let right = edge.uses[1];
    let left_region = region_roots[left.face];
    let right_region = region_roots[right.face];
    let mut rings = Vec::<[u32; 2]>::new();
    let mut ring_vertices = Vec::<[DerivedVertex; 2]>::new();
    for segment in 0..=segments {
        let t = f32::from(segment) / f32::from(segments);
        let pair = [
            derived_edge_vertex(
                geometry,
                faces,
                edge.a,
                left,
                right,
                left_region,
                right_region,
                welded_positions,
                displacements,
                corner_normals,
                edge.radius,
                t,
            ),
            derived_edge_vertex(
                geometry,
                faces,
                edge.b,
                left,
                right,
                left_region,
                right_region,
                welded_positions,
                displacements,
                corner_normals,
                edge.radius,
                t,
            ),
        ];
        for (endpoint, welded) in [edge.a, edge.b].into_iter().enumerate() {
            corner_boundaries
                .entry(welded)
                .or_default()
                .push(pair[endpoint]);
        }
        rings.push([push_vertex(derived, pair[0]), push_vertex(derived, pair[1])]);
        ring_vertices.push(pair);
    }
    for segment in 0..usize::from(segments) {
        let a = rings[segment][0];
        let b = rings[segment][1];
        let c = rings[segment + 1][1];
        let d = rings[segment + 1][0];
        let desired = (ring_vertices[segment][0].normal + ring_vertices[segment + 1][0].normal)
            .normalize_or_zero();
        push_oriented_triangle(indices, [a, b, c], desired, derived);
        push_oriented_triangle(indices, [a, c, d], desired, derived);
    }
}

#[allow(clippy::too_many_arguments)]
fn derived_edge_vertex(
    geometry: &GeometryDesc,
    faces: &[Face],
    welded: usize,
    left: EdgeUse,
    right: EdgeUse,
    left_region: usize,
    right_region: usize,
    welded_positions: &[Vec3],
    displacements: &BTreeMap<(usize, usize), Vec3>,
    corner_normals: &BTreeMap<(usize, usize), Vec3>,
    radius: f32,
    t: f32,
) -> DerivedVertex {
    let left_position = welded_positions[welded] + displacements[&(welded, left_region)];
    let right_position = welded_positions[welded] + displacements[&(welded, right_region)];
    let left_normal = corner_normals[&(welded, left_region)];
    let right_normal = corner_normals[&(welded, right_region)];
    let normal = nlerp(left_normal, right_normal, t);
    let center =
        ((left_position - left_normal * radius) + (right_position - right_normal * radius)) * 0.5;
    let raw_left = center + left_normal * radius;
    let raw_right = center + right_normal * radius;
    let correction = (left_position - raw_left).lerp(right_position - raw_right, t);
    let source_left = source_corner_for_welded(&faces[left.face], welded);
    let source_right = source_corner_for_welded(&faces[right.face], welded);
    DerivedVertex {
        position: center + normal * radius + correction,
        normal,
        color: lerp_color(
            geometry.vertex_color_or_default(source_left),
            geometry.vertex_color_or_default(source_right),
            t,
        ),
        uv: lerp_uv(
            geometry.tex_coord0_or_default(source_left),
            geometry.tex_coord0_or_default(source_right),
            t,
        ),
    }
}

fn append_corner_patches(
    geometry: &GeometryDesc,
    faces: &[Face],
    welded_positions: &[Vec3],
    corner_boundaries: &BTreeMap<usize, Vec<DerivedVertex>>,
    derived: &mut Vec<DerivedVertex>,
    indices: &mut Vec<u32>,
) {
    for (welded, boundary) in corner_boundaries {
        let mut boundary = deduplicate_boundary(boundary);
        if boundary.len() < 3 {
            continue;
        }
        let mut axis = boundary
            .iter()
            .map(|vertex| vertex.normal)
            .fold(Vec3::ZERO, |sum, normal| sum + normal)
            .normalize_or_zero();
        if axis.length_squared() < 0.5 {
            axis = (welded_positions[*welded] - geometry.bounds().center()).normalize_or_zero();
        }
        let tangent = perpendicular(axis);
        let bitangent = axis.cross(tangent).normalize_or_zero();
        let center_position = boundary
            .iter()
            .map(|vertex| vertex.position)
            .fold(Vec3::ZERO, |sum, position| sum + position)
            / boundary.len() as f32;
        boundary.sort_by(|left, right| {
            let left_delta = left.position - center_position;
            let right_delta = right.position - center_position;
            let left_angle = left_delta.dot(bitangent).atan2(left_delta.dot(tangent));
            let right_angle = right_delta.dot(bitangent).atan2(right_delta.dot(tangent));
            left_angle.total_cmp(&right_angle)
        });
        let center_color = average_color(&boundary);
        let center_uv = average_uv(&boundary);
        let center = push_vertex(
            derived,
            DerivedVertex {
                position: center_position,
                normal: axis,
                color: center_color,
                uv: center_uv,
            },
        );
        let ring = boundary
            .iter()
            .copied()
            .map(|vertex| push_vertex(derived, vertex))
            .collect::<Vec<_>>();
        for index in 0..ring.len() {
            push_oriented_triangle(
                indices,
                [center, ring[index], ring[(index + 1) % ring.len()]],
                axis,
                derived,
            );
        }

        // Keep the source lookup exercised for vertices whose imported UV/color
        // seams create multiple render vertices at one topological corner.
        let _ = faces
            .iter()
            .find_map(|face| face.welded.contains(welded).then_some(face.source));
    }
}

fn deduplicate_boundary(boundary: &[DerivedVertex]) -> Vec<DerivedVertex> {
    let mut unique = Vec::<DerivedVertex>::new();
    for vertex in boundary {
        if !unique
            .iter()
            .any(|candidate| candidate.position.distance_squared(vertex.position) <= 1.0e-14)
        {
            unique.push(*vertex);
        }
    }
    unique
}

fn source_corner_for_welded(face: &Face, welded: usize) -> usize {
    face.welded
        .iter()
        .position(|candidate| *candidate == welded)
        .map(|corner| face.source[corner])
        .expect("edge endpoint belongs to adjacent face")
}

fn push_vertex(vertices: &mut Vec<DerivedVertex>, vertex: DerivedVertex) -> u32 {
    let index = u32::try_from(vertices.len()).expect("derived vertex count fits u32");
    vertices.push(vertex);
    index
}

fn push_oriented_triangle(
    indices: &mut Vec<u32>,
    triangle: [u32; 3],
    desired_normal: Vec3,
    vertices: &[DerivedVertex],
) {
    let a = vertices[triangle[0] as usize].position;
    let b = vertices[triangle[1] as usize].position;
    let c = vertices[triangle[2] as usize].position;
    if (b - a).cross(c - a).dot(desired_normal) >= 0.0 {
        indices.extend_from_slice(&triangle);
    } else {
        indices.extend_from_slice(&[triangle[0], triangle[2], triangle[1]]);
    }
}

fn nlerp(left: Vec3, right: Vec3, t: f32) -> Vec3 {
    left.lerp(right, t).normalize_or_zero()
}

fn lerp_color(left: Color, right: Color, t: f32) -> Color {
    Color::from_linear_rgba(
        left.r + (right.r - left.r) * t,
        left.g + (right.g - left.g) * t,
        left.b + (right.b - left.b) * t,
        left.a + (right.a - left.a) * t,
    )
}

fn lerp_uv(left: [f32; 2], right: [f32; 2], t: f32) -> [f32; 2] {
    [
        left[0] + (right[0] - left[0]) * t,
        left[1] + (right[1] - left[1]) * t,
    ]
}

fn average_color(vertices: &[DerivedVertex]) -> Color {
    let scale = 1.0 / vertices.len() as f32;
    let sum = vertices.iter().fold([0.0; 4], |mut sum, vertex| {
        sum[0] += vertex.color.r;
        sum[1] += vertex.color.g;
        sum[2] += vertex.color.b;
        sum[3] += vertex.color.a;
        sum
    });
    Color::from_linear_rgba(
        sum[0] * scale,
        sum[1] * scale,
        sum[2] * scale,
        sum[3] * scale,
    )
}

fn average_uv(vertices: &[DerivedVertex]) -> [f32; 2] {
    let scale = 1.0 / vertices.len() as f32;
    let sum = vertices.iter().fold([0.0; 2], |mut sum, vertex| {
        sum[0] += vertex.uv[0];
        sum[1] += vertex.uv[1];
        sum
    });
    [sum[0] * scale, sum[1] * scale]
}

fn perpendicular(axis: Vec3) -> Vec3 {
    let candidate = if axis.x.abs() < 0.8 { Vec3::X } else { Vec3::Y };
    axis.cross(candidate).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_aligned_faceted_torus_does_not_receive_partial_edge_rounding() {
        const SEGMENTS: usize = 96;
        const RINGS: usize = 12;

        let source = GeometryDesc::torus(1.0, 0.2, SEGMENTS as u32, RINGS as u32);
        let mut vertices = source.vertices().to_vec();
        for segment in 0..=SEGMENTS {
            let theta = if segment == SEGMENTS {
                0.0
            } else {
                segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU
            };
            let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
            let phase = if segment % 3 == 2 {
                -0.04_f32.to_radians()
            } else {
                0.04_f32.to_radians()
            };
            let ring = 1;
            let phi = ring as f32 / RINGS as f32 * std::f32::consts::TAU + phase;
            let normal = Vec3::new(radial.x * phi.cos(), phi.sin(), radial.z * phi.cos());
            vertices[segment * (RINGS + 1) + ring].position = radial + normal * 0.2;
        }
        let torus = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vertices,
            source.indices().to_vec(),
        )
        .expect("the threshold fixture remains a closed triangle mesh");
        let (_, report) = round_hard_edges(&torus, EdgeRoundingOptions::new(0.01))
            .expect("a closed threshold-aligned torus supports curve refinement");

        assert_eq!(
            report.rounded_edges, 0,
            "floating-point noise around the hard-edge threshold must not turn a \
             continuous curved ring into alternating bevel wedges"
        );
    }

    #[test]
    fn flat_shaded_low_tessellation_curve_gains_silhouette_triangles() {
        let source = GeometryDesc::cylinder(1.0, 2.0, 16);
        let flat = flat_shaded_copy(&source);
        let source_angles = side_angles(&flat);
        let (refined, report) = round_hard_edges(&flat, EdgeRoundingOptions::new(0.04))
            .expect("a closed flat-shaded cylinder supports curve refinement");

        assert_eq!(report.rounded_edges, 0);
        assert!(
            refined.indices().len() > flat.indices().len(),
            "geometrically smooth regions must be refined even when authored normals \
             are flat"
        );
        assert!(
            side_angles(&refined).len() >= source_angles.len() * 2,
            "flat shading must not prevent real silhouette refinement"
        );
    }

    fn flat_shaded_copy(source: &GeometryDesc) -> GeometryDesc {
        let mut vertices = Vec::with_capacity(source.indices().len());
        let mut indices = Vec::with_capacity(source.indices().len());
        for triangle in source.indices().chunks_exact(3) {
            let positions = [
                source.vertices()[triangle[0] as usize].position,
                source.vertices()[triangle[1] as usize].position,
                source.vertices()[triangle[2] as usize].position,
            ];
            let normal = (positions[1] - positions[0])
                .cross(positions[2] - positions[0])
                .normalize_or_zero();
            for position in positions {
                indices.push(vertices.len() as u32);
                vertices.push(GeometryVertex { position, normal });
            }
        }
        GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
            .expect("flat-shaded fixture remains a valid triangle mesh")
    }

    fn side_angles(geometry: &GeometryDesc) -> BTreeSet<i32> {
        geometry
            .vertices()
            .iter()
            .filter(|vertex| vertex.normal.y.abs() < 0.1)
            .map(|vertex| {
                (vertex.position.z.atan2(vertex.position.x).to_degrees() * 100.0).round() as i32
            })
            .collect()
    }
}
