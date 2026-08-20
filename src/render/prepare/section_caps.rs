use earcut::Earcut;

use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::scene::{ClippingPlane, SectionBox, Transform, Vec3};

use super::transforms::transform_position;

#[derive(Clone, Copy)]
struct CutPoint {
    local: Vec3,
    rendered: Vec3,
}

#[derive(Clone, Copy)]
struct CutSegment {
    start: CutPoint,
    end: CutPoint,
}

struct CapLoop {
    points: Vec<CutPoint>,
    projected: Vec<[f64; 2]>,
    area: f64,
    parent: Option<usize>,
}

pub(super) fn build_section_cap_geometry(
    geometry: &GeometryDesc,
    transform: Transform,
    origin_shift: Vec3,
    section: SectionBox,
) -> Option<GeometryDesc> {
    if section.inverted() || geometry.topology() != GeometryTopology::Triangles {
        return None;
    }

    let rendered_positions = geometry
        .vertices()
        .iter()
        .map(|vertex| transform_position(vertex.position, transform, origin_shift))
        .collect::<Vec<_>>();
    let epsilon = position_epsilon(&rendered_positions);
    let mut cap_vertices = Vec::new();
    let mut cap_indices = Vec::new();

    for plane in section.planes() {
        let segments = intersected_segments(geometry, &rendered_positions, plane, epsilon);
        let loops = closed_loops(segments, plane, epsilon);
        append_triangulated_loops(
            &loops,
            plane,
            transform,
            &mut cap_vertices,
            &mut cap_indices,
        );
    }

    (!cap_indices.is_empty()).then(|| {
        GeometryDesc::try_new(GeometryTopology::Triangles, cap_vertices, cap_indices)
            .expect("section cap triangulation emits valid indexed triangles")
    })
}

fn position_epsilon(positions: &[Vec3]) -> f32 {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for point in positions {
        min = min.min(*point);
        max = max.max(*point);
    }
    (max - min).max_element().max(1.0) * 1.0e-5
}

fn intersected_segments(
    geometry: &GeometryDesc,
    rendered_positions: &[Vec3],
    plane: ClippingPlane,
    epsilon: f32,
) -> Vec<CutSegment> {
    let mut segments = Vec::new();
    for triangle in geometry.indices().chunks_exact(3) {
        let triangle = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let local = triangle.map(|index| geometry.vertices()[index].position);
        let rendered = triangle.map(|index| rendered_positions[index]);
        let distances = rendered.map(|point| signed_distance(plane, point));
        let mut intersections = Vec::with_capacity(3);
        for (start, end) in [(0, 1), (1, 2), (2, 0)] {
            append_edge_intersections(
                CutPoint {
                    local: local[start],
                    rendered: rendered[start],
                },
                distances[start],
                CutPoint {
                    local: local[end],
                    rendered: rendered[end],
                },
                distances[end],
                epsilon,
                &mut intersections,
            );
        }
        deduplicate_points(&mut intersections, epsilon);
        if intersections.len() != 2
            || intersections[0]
                .rendered
                .distance_squared(intersections[1].rendered)
                <= epsilon * epsilon
        {
            continue;
        }
        let segment = CutSegment {
            start: intersections[0],
            end: intersections[1],
        };
        if !segments
            .iter()
            .any(|existing| same_segment(*existing, segment, epsilon))
        {
            segments.push(segment);
        }
    }
    segments
}

fn signed_distance(plane: ClippingPlane, point: Vec3) -> f32 {
    plane.normal().dot(point) + plane.distance()
}

fn append_edge_intersections(
    start: CutPoint,
    start_distance: f32,
    end: CutPoint,
    end_distance: f32,
    epsilon: f32,
    intersections: &mut Vec<CutPoint>,
) {
    let start_on_plane = start_distance.abs() <= epsilon;
    let end_on_plane = end_distance.abs() <= epsilon;
    if start_on_plane {
        intersections.push(start);
    }
    if end_on_plane {
        intersections.push(end);
    }
    if !start_on_plane && !end_on_plane && start_distance.signum() != end_distance.signum() {
        let t = start_distance / (start_distance - end_distance);
        intersections.push(CutPoint {
            local: start.local.lerp(end.local, t),
            rendered: start.rendered.lerp(end.rendered, t),
        });
    }
}

fn deduplicate_points(points: &mut Vec<CutPoint>, epsilon: f32) {
    let mut unique = Vec::with_capacity(points.len());
    for point in points.drain(..) {
        if !unique.iter().any(|candidate: &CutPoint| {
            candidate.rendered.distance_squared(point.rendered) <= epsilon * epsilon
        }) {
            unique.push(point);
        }
    }
    *points = unique;
}

fn same_segment(left: CutSegment, right: CutSegment, epsilon: f32) -> bool {
    let near = |a: Vec3, b: Vec3| a.distance_squared(b) <= epsilon * epsilon;
    (near(left.start.rendered, right.start.rendered) && near(left.end.rendered, right.end.rendered))
        || (near(left.start.rendered, right.end.rendered)
            && near(left.end.rendered, right.start.rendered))
}

fn closed_loops(mut segments: Vec<CutSegment>, plane: ClippingPlane, epsilon: f32) -> Vec<CapLoop> {
    let mut loops = Vec::new();
    while let Some(segment) = segments.pop() {
        let mut points = vec![segment.start, segment.end];
        let mut closed = false;
        let max_points = segments.len().saturating_add(2);
        while points.len() <= max_points {
            let last = points.last().expect("loop has a last point").rendered;
            if points.len() >= 3 && last.distance_squared(points[0].rendered) <= epsilon * epsilon {
                points.pop();
                closed = true;
                break;
            }
            let Some((index, next)) = segments.iter().enumerate().find_map(|(index, candidate)| {
                if last.distance_squared(candidate.start.rendered) <= epsilon * epsilon {
                    Some((index, candidate.end))
                } else if last.distance_squared(candidate.end.rendered) <= epsilon * epsilon {
                    Some((index, candidate.start))
                } else {
                    None
                }
            }) else {
                break;
            };
            segments.swap_remove(index);
            points.push(next);
        }
        if !closed || points.len() < 3 {
            continue;
        }
        remove_collinear_points(&mut points, plane.normal(), epsilon);
        if points.len() < 3 {
            continue;
        }
        let projected = points
            .iter()
            .map(|point| project(point.rendered, plane.normal()))
            .collect::<Vec<_>>();
        let area = signed_area(&projected).abs();
        if area > f64::from(epsilon * epsilon) {
            loops.push(CapLoop {
                points,
                projected,
                area,
                parent: None,
            });
        }
    }
    assign_parents(&mut loops);
    loops
}

fn remove_collinear_points(points: &mut Vec<CutPoint>, normal: Vec3, epsilon: f32) {
    loop {
        let mut removed = false;
        for index in 0..points.len() {
            let previous = points[(index + points.len() - 1) % points.len()].rendered;
            let current = points[index].rendered;
            let next = points[(index + 1) % points.len()].rendered;
            let turn = (current - previous).cross(next - current).dot(normal).abs();
            if turn <= epsilon * (current.distance(previous) + next.distance(current)) {
                points.remove(index);
                removed = true;
                break;
            }
        }
        if !removed || points.len() < 3 {
            break;
        }
    }
}

fn project(point: Vec3, normal: Vec3) -> [f64; 2] {
    if normal.x.abs() > 0.5 {
        [f64::from(point.y), f64::from(point.z)]
    } else if normal.y.abs() > 0.5 {
        [f64::from(point.x), f64::from(point.z)]
    } else {
        [f64::from(point.x), f64::from(point.y)]
    }
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left[0] * right[1] - right[0] * left[1])
        .sum::<f64>()
        * 0.5
}

fn assign_parents(loops: &mut [CapLoop]) {
    for child in 0..loops.len() {
        let sample = loops[child].projected[0];
        loops[child].parent = (0..loops.len())
            .filter(|candidate| {
                *candidate != child
                    && loops[*candidate].area > loops[child].area
                    && point_in_polygon(sample, &loops[*candidate].projected)
            })
            .min_by(|left, right| loops[*left].area.total_cmp(&loops[*right].area));
    }
}

fn point_in_polygon(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for (start, end) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (start[1] > point[1]) != (end[1] > point[1])
            && point[0]
                < (end[0] - start[0]) * (point[1] - start[1]) / (end[1] - start[1]) + start[0]
        {
            inside = !inside;
        }
    }
    inside
}

fn loop_depth(loops: &[CapLoop], mut index: usize) -> usize {
    let mut depth = 0;
    while let Some(parent) = loops[index].parent {
        depth += 1;
        index = parent;
        if depth > loops.len() {
            break;
        }
    }
    depth
}

fn append_triangulated_loops(
    loops: &[CapLoop],
    plane: ClippingPlane,
    transform: Transform,
    vertices: &mut Vec<GeometryVertex>,
    indices: &mut Vec<u32>,
) {
    let desired_world_normal = -plane.normal();
    let local_normal = inverse_rotate_normal(desired_world_normal, transform);
    for outer in 0..loops.len() {
        if !loop_depth(loops, outer).is_multiple_of(2) {
            continue;
        }
        let mut group_points = loops[outer].points.clone();
        let mut group_projected = loops[outer].projected.clone();
        let mut hole_indices = Vec::new();
        for hole in loops {
            if hole.parent == Some(outer) {
                hole_indices.push(group_points.len() as u32);
                group_points.extend_from_slice(&hole.points);
                group_projected.extend_from_slice(&hole.projected);
            }
        }
        let mut triangles = Vec::new();
        Earcut::<f64>::new().earcut::<u32>(
            group_projected.iter().copied(),
            &hole_indices,
            &mut triangles,
        );
        for triangle in triangles.chunks_exact(3) {
            let mut corners = [
                group_points[triangle[0] as usize].local,
                group_points[triangle[1] as usize].local,
                group_points[triangle[2] as usize].local,
            ];
            let world_a = transform_position(corners[0], transform, Vec3::ZERO);
            let world_b = transform_position(corners[1], transform, Vec3::ZERO);
            let world_c = transform_position(corners[2], transform, Vec3::ZERO);
            if (world_b - world_a)
                .cross(world_c - world_a)
                .dot(desired_world_normal)
                < 0.0
            {
                corners.swap(1, 2);
            }
            let base = vertices.len() as u32;
            vertices.extend(corners.map(|position| GeometryVertex {
                position,
                normal: local_normal,
            }));
            indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
    }
}

fn inverse_rotate_normal(normal: Vec3, transform: Transform) -> Vec3 {
    let rotation = transform.rotation;
    if !rotation.is_finite() || rotation.length_squared() <= f32::EPSILON {
        normal
    } else {
        (rotation.normalize().conjugate() * normal).normalize_or_zero()
    }
}
