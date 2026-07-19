use crate::scene::Vec3;

use super::{Aabb, GeometryVertex};

const LEAF_TRIANGLES: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TriangleBvh {
    nodes: Vec<BvhNode>,
    triangle_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
struct BvhNode {
    bounds: Aabb,
    kind: BvhNodeKind,
}

#[derive(Debug, Clone, PartialEq)]
enum BvhNodeKind {
    Branch { left: usize, right: usize },
    Leaf { first: usize, count: usize },
}

#[derive(Debug, Clone, Copy)]
struct BuildTriangle {
    index: usize,
    bounds: Aabb,
    centroid: Vec3,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RayCandidates {
    pub(crate) triangles: Vec<usize>,
    pub(crate) node_bounds_tests: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RayVisitResult {
    pub(crate) hit: bool,
    pub(crate) node_bounds_tests: u64,
}

impl TriangleBvh {
    pub(crate) fn from_indexed(vertices: &[GeometryVertex], indices: &[u32]) -> Self {
        let positions = vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>();
        Self::from_positions_indexed(&positions, indices)
    }

    pub(crate) fn from_positions_indexed(vertices: &[Vec3], indices: &[u32]) -> Self {
        let triangles = indices
            .chunks_exact(3)
            .enumerate()
            .filter_map(|(index, triangle)| {
                let a = *vertices.get(triangle[0] as usize)?;
                let b = *vertices.get(triangle[1] as usize)?;
                let c = *vertices.get(triangle[2] as usize)?;
                Some(build_triangle(index, a, b, c))
            })
            .collect();
        Self::build(triangles)
    }

    pub(crate) fn from_triangles(triangles: &[[Vec3; 3]]) -> Self {
        Self::build(
            triangles
                .iter()
                .enumerate()
                .map(|(index, triangle)| {
                    build_triangle(index, triangle[0], triangle[1], triangle[2])
                })
                .collect(),
        )
    }

    fn build(mut triangles: Vec<BuildTriangle>) -> Self {
        let mut bvh = Self {
            nodes: Vec::new(),
            triangle_indices: Vec::with_capacity(triangles.len()),
        };
        if !triangles.is_empty() {
            bvh.build_node(&mut triangles);
        }
        bvh
    }

    fn build_node(&mut self, triangles: &mut [BuildTriangle]) -> usize {
        let node_index = self.nodes.len();
        let bounds = triangles
            .iter()
            .skip(1)
            .fold(triangles[0].bounds, |bounds, triangle| {
                bounds.union(triangle.bounds)
            });
        self.nodes.push(BvhNode {
            bounds,
            kind: BvhNodeKind::Leaf { first: 0, count: 0 },
        });

        if triangles.len() <= LEAF_TRIANGLES {
            triangles.sort_by_key(|triangle| triangle.index);
            let first = self.triangle_indices.len();
            self.triangle_indices
                .extend(triangles.iter().map(|triangle| triangle.index));
            self.nodes[node_index].kind = BvhNodeKind::Leaf {
                first,
                count: triangles.len(),
            };
            return node_index;
        }

        let extent = bounds.half_extent();
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        triangles.sort_by(|left, right| {
            component(left.centroid, axis)
                .total_cmp(&component(right.centroid, axis))
                .then_with(|| left.index.cmp(&right.index))
        });
        let middle = triangles.len() / 2;
        let (left_triangles, right_triangles) = triangles.split_at_mut(middle);
        let left = self.build_node(left_triangles);
        let right = self.build_node(right_triangles);
        self.nodes[node_index].kind = BvhNodeKind::Branch { left, right };
        node_index
    }

    pub(crate) fn ray_candidates(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> RayCandidates {
        let mut result = RayCandidates::default();
        if self.nodes.is_empty() {
            return result;
        }
        let mut stack = vec![0usize];
        while let Some(node_index) = stack.pop() {
            let node = &self.nodes[node_index];
            result.node_bounds_tests = result.node_bounds_tests.saturating_add(1);
            if !ray_hits_aabb(origin, direction, node.bounds, max_distance) {
                continue;
            }
            match node.kind {
                BvhNodeKind::Branch { left, right } => {
                    stack.push(right);
                    stack.push(left);
                }
                BvhNodeKind::Leaf { first, count } => result
                    .triangles
                    .extend_from_slice(&self.triangle_indices[first..first + count]),
            }
        }
        result.triangles.sort_unstable();
        result
    }

    pub(crate) fn bounds(&self) -> Option<Aabb> {
        self.nodes.first().map(|node| node.bounds)
    }

    pub(crate) fn any_ray_candidate(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        mut visitor: impl FnMut(usize) -> bool,
    ) -> RayVisitResult {
        let mut result = RayVisitResult::default();
        if self.nodes.is_empty() {
            return result;
        }
        let mut stack = [0usize; 64];
        let mut stack_len = 1usize;
        while stack_len > 0 {
            stack_len -= 1;
            let node = &self.nodes[stack[stack_len]];
            result.node_bounds_tests = result.node_bounds_tests.saturating_add(1);
            if !ray_hits_aabb(origin, direction, node.bounds, max_distance) {
                continue;
            }
            match node.kind {
                BvhNodeKind::Branch { left, right } => {
                    debug_assert!(stack_len + 2 <= stack.len());
                    stack[stack_len] = right;
                    stack[stack_len + 1] = left;
                    stack_len += 2;
                }
                BvhNodeKind::Leaf { first, count } => {
                    for triangle in &self.triangle_indices[first..first + count] {
                        if visitor(*triangle) {
                            result.hit = true;
                            return result;
                        }
                    }
                }
            }
        }
        result
    }
}

fn build_triangle(index: usize, a: Vec3, b: Vec3, c: Vec3) -> BuildTriangle {
    let min = Vec3::new(
        a.x.min(b.x).min(c.x),
        a.y.min(b.y).min(c.y),
        a.z.min(b.z).min(c.z),
    );
    let max = Vec3::new(
        a.x.max(b.x).max(c.x),
        a.y.max(b.y).max(c.y),
        a.z.max(b.z).max(c.z),
    );
    BuildTriangle {
        index,
        bounds: Aabb::new(min, max),
        centroid: Vec3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        ),
    }
}

fn component(value: Vec3, axis: usize) -> f32 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

fn ray_hits_aabb(origin: Vec3, direction: Vec3, bounds: Aabb, max_distance: f32) -> bool {
    let Some((x_min, x_max)) = axis_interval(origin.x, direction.x, bounds.min.x, bounds.max.x)
    else {
        return false;
    };
    let Some((y_min, y_max)) = axis_interval(origin.y, direction.y, bounds.min.y, bounds.max.y)
    else {
        return false;
    };
    let Some((z_min, z_max)) = axis_interval(origin.z, direction.z, bounds.min.z, bounds.max.z)
    else {
        return false;
    };
    let near = x_min.max(y_min).max(z_min).max(0.0);
    let far = x_max.min(y_max).min(z_max).min(max_distance);
    far >= near
}

fn axis_interval(origin: f32, direction: f32, min: f32, max: f32) -> Option<(f32, f32)> {
    const EPSILON: f32 = 1.0e-8;
    if direction.abs() <= EPSILON {
        return (origin >= min && origin <= max).then_some((f32::NEG_INFINITY, f32::INFINITY));
    }
    let first = (min - origin) / direction;
    let second = (max - origin) / direction;
    Some((first.min(second), first.max(second)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_bvh_reduces_candidates_for_spread_triangles() {
        let triangles = (0..4096)
            .map(|index| {
                let x = index as f32 * 2.0;
                [
                    Vec3::new(x, -0.25, -2.0),
                    Vec3::new(x + 0.5, -0.25, -2.0),
                    Vec3::new(x, 0.25, -2.0),
                ]
            })
            .collect::<Vec<_>>();
        let first = TriangleBvh::from_triangles(&triangles);
        let second = TriangleBvh::from_triangles(&triangles);
        assert_eq!(first, second);

        let candidates = first.ray_candidates(
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            f32::INFINITY,
        );
        assert!(candidates.triangles.len() < 32, "{candidates:?}");
        assert!(candidates.node_bounds_tests < 128, "{candidates:?}");
    }
}
