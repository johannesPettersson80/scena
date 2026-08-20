use super::*;

pub(super) fn face_altitude_to_edge(face: Face, edge: &MeshEdge, positions: &[Vec3]) -> f32 {
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

pub(super) fn validate_options(options: EdgeRoundingOptions) -> Result<(), EdgeRoundingError> {
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

pub(super) fn weld_positions(geometry: &GeometryDesc) -> (Vec<Vec3>, Vec<usize>) {
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

pub(super) fn build_faces(
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

pub(super) fn minimum_triangle_area_squared(geometry: &GeometryDesc) -> f32 {
    let bounds_scale = geometry.bounds().bounding_sphere_radius().max(1.0e-6);
    bounds_scale.powi(4) * 1.0e-16
}

pub(super) fn build_edges(faces: &[Face]) -> BTreeMap<(usize, usize), MeshEdge> {
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

pub(super) fn edge_has_continuous_authored_normals(
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

pub(super) fn suppress_faceted_curved_rim_cycles(
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

pub(super) fn validate_topology(
    edges: &BTreeMap<(usize, usize), MeshEdge>,
) -> Result<(), EdgeRoundingError> {
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

pub(super) fn cluster_displacements(
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

pub(super) fn recomputed_corner_normals(
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

pub(super) fn geometry_with_smooth_region_normals(
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
