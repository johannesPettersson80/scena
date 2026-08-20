use super::*;

pub(super) fn finish_with_curve_refinement(
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

pub(super) fn weld_continuous_render_vertices(
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
