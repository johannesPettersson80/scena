use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn append_bevel_strip(
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
pub(super) fn derived_edge_vertex(
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

pub(super) fn append_corner_patches(
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

pub(super) fn deduplicate_boundary(boundary: &[DerivedVertex]) -> Vec<DerivedVertex> {
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

pub(super) fn source_corner_for_welded(face: &Face, welded: usize) -> usize {
    face.welded
        .iter()
        .position(|candidate| *candidate == welded)
        .map(|corner| face.source[corner])
        .expect("edge endpoint belongs to adjacent face")
}

pub(super) fn push_vertex(vertices: &mut Vec<DerivedVertex>, vertex: DerivedVertex) -> u32 {
    let index = u32::try_from(vertices.len()).expect("derived vertex count fits u32");
    vertices.push(vertex);
    index
}

pub(super) fn push_oriented_triangle(
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

pub(super) fn nlerp(left: Vec3, right: Vec3, t: f32) -> Vec3 {
    left.lerp(right, t).normalize_or_zero()
}

pub(super) fn lerp_color(left: Color, right: Color, t: f32) -> Color {
    Color::from_linear_rgba(
        left.r + (right.r - left.r) * t,
        left.g + (right.g - left.g) * t,
        left.b + (right.b - left.b) * t,
        left.a + (right.a - left.a) * t,
    )
}

pub(super) fn lerp_uv(left: [f32; 2], right: [f32; 2], t: f32) -> [f32; 2] {
    [
        left[0] + (right[0] - left[0]) * t,
        left[1] + (right[1] - left[1]) * t,
    ]
}

pub(super) fn average_color(vertices: &[DerivedVertex]) -> Color {
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

pub(super) fn average_uv(vertices: &[DerivedVertex]) -> [f32; 2] {
    let scale = 1.0 / vertices.len() as f32;
    let sum = vertices.iter().fold([0.0; 2], |mut sum, vertex| {
        sum[0] += vertex.uv[0];
        sum[1] += vertex.uv[1];
        sum
    });
    [sum[0] * scale, sum[1] * scale]
}

pub(super) fn perpendicular(axis: Vec3) -> Vec3 {
    let candidate = if axis.x.abs() < 0.8 { Vec3::X } else { Vec3::Y };
    axis.cross(candidate).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
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
