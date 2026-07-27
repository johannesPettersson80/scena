use super::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::Vec3;

#[derive(Debug)]
pub(crate) struct PhotographicGeometryRepair {
    pub(crate) geometry: Option<GeometryDesc>,
    pub(crate) removed_degenerate_triangles: usize,
    pub(crate) repaired_normals: bool,
    pub(crate) reversed_winding: bool,
    pub(crate) disconnected_components: usize,
    pub(crate) boundary_edges: usize,
    pub(crate) nonmanifold_edges: usize,
    pub(crate) folded_edges: usize,
    pub(crate) self_intersections: usize,
    pub(crate) duplicate_vertices_removed: usize,
    pub(crate) rejected_reason: Option<&'static str>,
}

impl GeometryDesc {
    pub(crate) fn repair_for_photography(&self) -> PhotographicGeometryRepair {
        if self.topology != GeometryTopology::Triangles {
            return PhotographicGeometryRepair {
                geometry: None,
                removed_degenerate_triangles: 0,
                repaired_normals: false,
                reversed_winding: false,
                disconnected_components: 0,
                boundary_edges: 0,
                nonmanifold_edges: 0,
                folded_edges: 0,
                self_intersections: 0,
                duplicate_vertices_removed: 0,
                rejected_reason: None,
            };
        }
        if self
            .vertices
            .iter()
            .any(|vertex| !vertex.position.is_finite())
        {
            return PhotographicGeometryRepair {
                geometry: None,
                removed_degenerate_triangles: 0,
                repaired_normals: false,
                reversed_winding: false,
                disconnected_components: 0,
                boundary_edges: 0,
                nonmanifold_edges: 0,
                folded_edges: 0,
                self_intersections: 0,
                duplicate_vertices_removed: 0,
                rejected_reason: Some("non_finite_position"),
            };
        }

        let (source_vertices, source_indices, duplicate_vertices_removed) =
            deduplicate_safe_vertices(self);
        let bounds_scale = self.bounds.bounding_sphere_radius().max(1.0e-6);
        let minimum_area_squared = bounds_scale.powi(4) * 1.0e-16;
        let mut indices = Vec::with_capacity(source_indices.len());
        let mut removed_degenerate_triangles = 0;
        let mut aligned = 0usize;
        let mut inverted = 0usize;
        for triangle in source_indices.chunks_exact(3) {
            let a = source_vertices[triangle[0] as usize].position;
            let b = source_vertices[triangle[1] as usize].position;
            let c = source_vertices[triangle[2] as usize].position;
            let cross = (b - a).cross(c - a);
            if !cross.is_finite() || cross.length_squared() <= minimum_area_squared {
                removed_degenerate_triangles += 1;
                continue;
            }
            let face = cross.normalize();
            let authored = (source_vertices[triangle[0] as usize].normal
                + source_vertices[triangle[1] as usize].normal
                + source_vertices[triangle[2] as usize].normal)
                .normalize_or_zero();
            if authored.length_squared() > 0.5 {
                if face.dot(authored) < -0.2 {
                    inverted += 1;
                } else {
                    aligned += 1;
                }
            }
            indices.extend_from_slice(triangle);
        }
        if indices.is_empty() {
            return PhotographicGeometryRepair {
                geometry: None,
                removed_degenerate_triangles,
                repaired_normals: false,
                reversed_winding: false,
                disconnected_components: 0,
                boundary_edges: 0,
                nonmanifold_edges: 0,
                folded_edges: 0,
                self_intersections: 0,
                duplicate_vertices_removed,
                rejected_reason: Some("no_nondegenerate_triangles"),
            };
        }

        let disconnected_components = triangle_component_count(&indices);
        let (boundary_edges, nonmanifold_edges, folded_edges) =
            mesh_edge_health(&source_vertices, &indices);
        let self_intersections = triangle_self_intersection_count(&source_vertices, &indices);
        let reversed_winding = inverted > aligned && inverted > 0;
        if reversed_winding {
            for triangle in indices.chunks_exact_mut(3) {
                triangle.swap(1, 2);
            }
        }
        let invalid_normals = source_vertices.iter().any(|vertex| {
            !vertex.normal.is_finite() || !(0.5..=1.5).contains(&vertex.normal.length_squared())
        });
        let uneven_normals =
            !invalid_normals && normals_require_weighted_reconstruction(&source_vertices, &indices);
        let repaired_normals = invalid_normals || uneven_normals || reversed_winding;
        let mut vertices = source_vertices;
        if repaired_normals {
            reconstruct_area_weighted_normals(&mut vertices, &indices);
        }
        let changed =
            removed_degenerate_triangles > 0 || repaired_normals || duplicate_vertices_removed > 0;
        let geometry = changed.then(|| {
            let mut repaired = GeometryDesc::try_new_with_optional_vertex_attributes(
                GeometryTopology::Triangles,
                vertices,
                indices,
                self.authored_vertex_colors().map(|values| values.to_vec()),
                self.authored_tex_coords0().map(|values| values.to_vec()),
            )
            .expect("validated photographic repair streams remain valid");
            repaired.morph_targets = self.morph_targets.clone();
            repaired.skin = self.skin.clone();
            repaired.tangents = if repaired_normals {
                None
            } else {
                self.tangents.clone()
            };
            repaired
        });
        PhotographicGeometryRepair {
            geometry,
            removed_degenerate_triangles,
            repaired_normals,
            reversed_winding,
            disconnected_components,
            boundary_edges,
            nonmanifold_edges,
            folded_edges,
            self_intersections,
            duplicate_vertices_removed,
            rejected_reason: None,
        }
    }

    pub(crate) fn micro_beveled_box(&self, bevel: f32) -> Option<GeometryDesc> {
        if self.topology != GeometryTopology::Triangles || self.indices.len() != 36 {
            return None;
        }
        let bounds = self.bounds;
        let size = bounds.half_extent() * 2.0;
        let epsilon = size.max_element().max(1.0e-6) * 1.0e-5;
        let corners_only = self.vertices.iter().all(|vertex| {
            [vertex.position.x, vertex.position.y, vertex.position.z]
                .into_iter()
                .zip([
                    (bounds.min.x, bounds.max.x),
                    (bounds.min.y, bounds.max.y),
                    (bounds.min.z, bounds.max.z),
                ])
                .all(|(value, (minimum, maximum))| {
                    (value - minimum).abs() <= epsilon || (value - maximum).abs() <= epsilon
                })
        });
        if !corners_only {
            return None;
        }
        let mut geometry = GeometryDesc::box_xyz_with_bevel(size.x, size.y, size.z, bevel);
        let center = bounds.center();
        for vertex in &mut geometry.vertices {
            vertex.position += center;
        }
        geometry.bounds = crate::Aabb::new(bounds.min, bounds.max);
        Some(geometry)
    }
}

fn deduplicate_safe_vertices(geometry: &GeometryDesc) -> (Vec<GeometryVertex>, Vec<u32>, usize) {
    if geometry.authored_vertex_colors().is_some()
        || geometry.authored_tex_coords0().is_some()
        || geometry.tangents().is_some()
        || geometry.skin().is_some()
        || !geometry.morph_targets().is_empty()
    {
        return (geometry.vertices.clone(), geometry.indices.clone(), 0);
    }
    let mut remap = Vec::with_capacity(geometry.vertices.len());
    let mut unique = Vec::with_capacity(geometry.vertices.len());
    let mut by_vertex = std::collections::HashMap::<[u32; 6], u32>::new();
    for vertex in &geometry.vertices {
        let mut key = [0_u32; 6];
        key[..3].copy_from_slice(&position_key(vertex.position));
        key[3..].copy_from_slice(&position_key(vertex.normal));
        let next = unique.len() as u32;
        let index = *by_vertex.entry(key).or_insert_with(|| {
            unique.push(*vertex);
            next
        });
        remap.push(index);
    }
    let indices = geometry
        .indices
        .iter()
        .map(|index| remap[*index as usize])
        .collect();
    let removed = geometry.vertices.len().saturating_sub(unique.len());
    (unique, indices, removed)
}

fn mesh_edge_health(vertices: &[GeometryVertex], indices: &[u32]) -> (usize, usize, usize) {
    let mut uses = std::collections::HashMap::<([u32; 3], [u32; 3]), Vec<(bool, Vec3)>>::new();
    for triangle in indices.chunks_exact(3) {
        let a = vertices[triangle[0] as usize].position;
        let b = vertices[triangle[1] as usize].position;
        let c = vertices[triangle[2] as usize].position;
        let normal = (b - a).cross(c - a).normalize_or_zero();
        for [a, b] in [
            [triangle[0], triangle[1]],
            [triangle[1], triangle[2]],
            [triangle[2], triangle[0]],
        ] {
            let a = position_key(vertices[a as usize].position);
            let b = position_key(vertices[b as usize].position);
            uses.entry(if a <= b { (a, b) } else { (b, a) })
                .or_default()
                .push((a <= b, normal));
        }
    }
    (
        uses.values().filter(|uses| uses.len() == 1).count(),
        uses.values().filter(|uses| uses.len() > 2).count(),
        uses.values()
            .filter(|uses| {
                uses.len() == 2 && (uses[0].0 == uses[1].0 || uses[0].1.dot(uses[1].1) < -0.95)
            })
            .count(),
    )
}

fn triangle_self_intersection_count(vertices: &[GeometryVertex], indices: &[u32]) -> usize {
    let triangles = indices
        .chunks_exact(3)
        .map(|triangle| {
            [
                vertices[triangle[0] as usize].position,
                vertices[triangle[1] as usize].position,
                vertices[triangle[2] as usize].position,
            ]
        })
        .collect::<Vec<_>>();
    let mut intersections = 0;
    for left in 0..triangles.len() {
        for right in left + 1..triangles.len() {
            if triangles_share_vertex(triangles[left], triangles[right]) {
                continue;
            }
            if triangle_bounds_overlap(triangles[left], triangles[right])
                && triangles_intersect(triangles[left], triangles[right])
            {
                intersections += 1;
            }
        }
    }
    intersections
}

fn triangles_share_vertex(left: [Vec3; 3], right: [Vec3; 3]) -> bool {
    left.into_iter().any(|left| {
        let left = position_key(left);
        right.into_iter().any(|right| left == position_key(right))
    })
}

fn triangle_bounds_overlap(left: [Vec3; 3], right: [Vec3; 3]) -> bool {
    let bounds = |triangle: [Vec3; 3]| {
        (
            triangle
                .into_iter()
                .fold(Vec3::splat(f32::INFINITY), Vec3::min),
            triangle
                .into_iter()
                .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
        )
    };
    let (left_min, left_max) = bounds(left);
    let (right_min, right_max) = bounds(right);
    left_min.x <= right_max.x
        && left_max.x >= right_min.x
        && left_min.y <= right_max.y
        && left_max.y >= right_min.y
        && left_min.z <= right_max.z
        && left_max.z >= right_min.z
}

fn triangles_intersect(left: [Vec3; 3], right: [Vec3; 3]) -> bool {
    triangle_edges(left)
        .into_iter()
        .any(|[start, end]| segment_hits_triangle(start, end, right))
        || triangle_edges(right)
            .into_iter()
            .any(|[start, end]| segment_hits_triangle(start, end, left))
}

fn triangle_edges(triangle: [Vec3; 3]) -> [[Vec3; 2]; 3] {
    [
        [triangle[0], triangle[1]],
        [triangle[1], triangle[2]],
        [triangle[2], triangle[0]],
    ]
}

fn segment_hits_triangle(start: Vec3, end: Vec3, triangle: [Vec3; 3]) -> bool {
    let direction = end - start;
    let edge_1 = triangle[1] - triangle[0];
    let edge_2 = triangle[2] - triangle[0];
    let cross = direction.cross(edge_2);
    let determinant = edge_1.dot(cross);
    let epsilon = 1.0e-7;
    if determinant.abs() <= epsilon {
        return false;
    }
    let inverse = determinant.recip();
    let offset = start - triangle[0];
    let u = offset.dot(cross) * inverse;
    if !(-epsilon..=1.0 + epsilon).contains(&u) {
        return false;
    }
    let q = offset.cross(edge_1);
    let v = direction.dot(q) * inverse;
    if v < -epsilon || u + v > 1.0 + epsilon {
        return false;
    }
    let distance = edge_2.dot(q) * inverse;
    (epsilon..=1.0 - epsilon).contains(&distance)
}

fn position_key(position: Vec3) -> [u32; 3] {
    position
        .to_array()
        .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
}

fn triangle_component_count(indices: &[u32]) -> usize {
    let triangle_count = indices.len() / 3;
    if triangle_count == 0 {
        return 0;
    }
    let mut parents: Vec<usize> = (0..triangle_count).collect();
    let mut owner_by_vertex = std::collections::HashMap::<u32, usize>::new();
    for (triangle_index, triangle) in indices.chunks_exact(3).enumerate() {
        for vertex in triangle {
            if let Some(owner) = owner_by_vertex.insert(*vertex, triangle_index) {
                union(&mut parents, owner, triangle_index);
            }
        }
    }
    for index in 0..triangle_count {
        parents[index] = find(&mut parents, index);
    }
    parents.sort_unstable();
    parents.dedup();
    parents.len()
}

fn find(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }
    index
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left_root = find(parents, left);
    let right_root = find(parents, right);
    if left_root != right_root {
        parents[right_root] = left_root;
    }
}

fn reconstruct_area_weighted_normals(vertices: &mut [GeometryVertex], indices: &[u32]) {
    let mut sums = vec![Vec3::ZERO; vertices.len()];
    for triangle in indices.chunks_exact(3) {
        let a = vertices[triangle[0] as usize].position;
        let b = vertices[triangle[1] as usize].position;
        let c = vertices[triangle[2] as usize].position;
        let weighted = (b - a).cross(c - a);
        for index in triangle {
            sums[*index as usize] += weighted;
        }
    }
    for (vertex, sum) in vertices.iter_mut().zip(sums) {
        vertex.normal = sum.normalize_or_zero();
    }
}

fn normals_require_weighted_reconstruction(vertices: &[GeometryVertex], indices: &[u32]) -> bool {
    let mut weighted = vec![Vec3::ZERO; vertices.len()];
    let mut agreement = vec![true; vertices.len()];
    for triangle in indices.chunks_exact(3) {
        let a = vertices[triangle[0] as usize].position;
        let b = vertices[triangle[1] as usize].position;
        let c = vertices[triangle[2] as usize].position;
        let face = (b - a).cross(c - a);
        let face_normal = face.normalize_or_zero();
        for index in triangle {
            let slot = *index as usize;
            if weighted[slot].length_squared() > 0.0
                && weighted[slot].normalize().dot(face_normal) < 0.90
            {
                agreement[slot] = false;
            }
            weighted[slot] += face;
        }
    }
    vertices
        .iter()
        .zip(weighted)
        .zip(agreement)
        .any(|((vertex, weighted), smooth)| {
            smooth
                && weighted.length_squared() > 0.0
                && vertex.normal.normalize().dot(weighted.normalize()) < 0.75
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photographic_geometry_health_distinguishes_closed_and_open_surfaces() {
        let closed = GeometryDesc::box_xyz(1.0, 1.0, 1.0).repair_for_photography();
        assert_eq!(closed.boundary_edges, 0);
        assert_eq!(closed.nonmanifold_edges, 0);

        let open = GeometryDesc::plane(1.0, 1.0).repair_for_photography();
        assert!(open.boundary_edges > 0);
        assert_eq!(open.nonmanifold_edges, 0);
    }

    #[test]
    fn photographic_geometry_health_rejects_non_finite_positions() {
        let mut geometry = GeometryDesc::box_xyz(1.0, 1.0, 1.0);
        geometry.vertices[0].position.x = f32::NAN;
        let repair = geometry.repair_for_photography();
        assert_eq!(repair.rejected_reason, Some("non_finite_position"));
        assert!(repair.geometry.is_none());
    }

    #[test]
    fn photographic_geometry_health_detects_crossing_faces_and_safe_duplicates() {
        let vertex = |position| GeometryVertex {
            position,
            normal: Vec3::Z,
        };
        let crossing = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                vertex(Vec3::new(-1.0, 0.0, 0.0)),
                vertex(Vec3::new(1.0, 0.0, 0.0)),
                vertex(Vec3::new(0.0, 1.0, 0.0)),
                vertex(Vec3::new(0.0, -0.5, -1.0)),
                vertex(Vec3::new(0.0, -0.5, 1.0)),
                vertex(Vec3::new(0.0, 0.5, 0.0)),
            ],
            vec![0, 1, 2, 3, 4, 5],
        )
        .expect("crossing triangles are structurally valid");
        let repair = crossing.repair_for_photography();
        assert_eq!(repair.self_intersections, 1);

        let duplicate = GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                vertex(Vec3::ZERO),
                vertex(Vec3::X),
                vertex(Vec3::Y),
                vertex(Vec3::ZERO),
            ],
            vec![3, 1, 2],
        )
        .expect("duplicate vertex fixture is structurally valid");
        let repair = duplicate.repair_for_photography();
        assert_eq!(repair.duplicate_vertices_removed, 1);
        assert_eq!(
            repair
                .geometry
                .expect("duplicate is repaired")
                .vertices()
                .len(),
            3
        );
    }
}
