//! Bounding volume hierarchy over the traced scene's triangles.

use glam::Vec3;

const LEAF_TRIANGLES: usize = 4;
const BINS: usize = 12;
/// Deeper subtrees become leaves, which bounds the traversal stack.
const MAX_DEPTH: usize = 56;
const STACK: usize = MAX_DEPTH + 8;

#[derive(Clone, Copy)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// Leaf: first slot in `order`. Interior: index of the right child (the
    /// left child is the next node).
    first_or_right: u32,
    /// Triangle count for a leaf, zero for an interior node.
    count: u32,
}

/// Closest (or any) intersection of a ray with a triangle.
#[derive(Clone, Copy, Debug)]
pub(super) struct Hit {
    pub(super) distance: f32,
    /// Barycentric weights of the second and third vertex.
    pub(super) u: f32,
    pub(super) v: f32,
    pub(super) triangle: u32,
}

pub(super) struct Bvh {
    nodes: Vec<Node>,
    order: Vec<u32>,
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    fn grow(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    fn merge(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn half_area(self) -> f32 {
        let extent = (self.max - self.min).max(Vec3::ZERO);
        extent.x * extent.y + extent.y * extent.z + extent.z * extent.x
    }
}

impl Bvh {
    pub(super) fn build(triangles: &[[Vec3; 3]]) -> Self {
        let centroids: Vec<Vec3> = triangles
            .iter()
            .map(|t| (t[0] + t[1] + t[2]) / 3.0)
            .collect();
        let mut order: Vec<u32> = (0..triangles.len() as u32).collect();
        let mut nodes = Vec::with_capacity(triangles.len().max(1) * 2);
        if triangles.is_empty() {
            nodes.push(Node {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
                first_or_right: 0,
                count: 0,
            });
        } else {
            build_node(triangles, &centroids, &mut order, 0, 0, &mut nodes);
        }
        Self { nodes, order }
    }

    /// Finds the closest hit within `(0, max_distance)`, or the first hit
    /// found when `any_hit` is set. `accept` can reject a candidate, which is
    /// how cut-out texels and culled back faces let a ray through.
    pub(super) fn intersect(
        &self,
        triangles: &[[Vec3; 3]],
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        any_hit: bool,
        mut accept: impl FnMut(&Hit) -> bool,
    ) -> Option<Hit> {
        if self.order.is_empty() {
            return None;
        }
        let inverse = direction.recip();
        let mut best: Option<Hit> = None;
        let mut limit = max_distance;
        // Each entry is a node and the distance at which the ray enters it.
        let mut stack = [(0_u32, 0.0_f32); STACK];
        let mut depth = 0;
        if let Some(entry) = self.entry(0, origin, inverse, limit) {
            stack[0] = (0, entry);
            depth = 1;
        }
        while depth > 0 {
            depth -= 1;
            let (index, entry) = stack[depth];
            // A hit found since this node was pushed may already be nearer.
            if entry > limit {
                continue;
            }
            let node = &self.nodes[index as usize];
            if node.count == 0 {
                if depth + 2 > STACK {
                    continue;
                }
                // Visit the nearer child first: a hit there usually rules
                // the farther one out before it is opened.
                let children = [index + 1, node.first_or_right];
                let entries =
                    children.map(|child| self.entry(child as usize, origin, inverse, limit));
                let order =
                    if entries[0].unwrap_or(f32::INFINITY) <= entries[1].unwrap_or(f32::INFINITY) {
                        [1, 0]
                    } else {
                        [0, 1]
                    };
                for child in order {
                    if let Some(entry) = entries[child] {
                        stack[depth] = (children[child], entry);
                        depth += 1;
                    }
                }
                continue;
            }
            let first = node.first_or_right as usize;
            for &triangle in &self.order[first..first + node.count as usize] {
                let Some(hit) =
                    intersect_triangle(&triangles[triangle as usize], origin, direction, triangle)
                else {
                    continue;
                };
                if hit.distance < limit && accept(&hit) {
                    limit = hit.distance;
                    best = Some(hit);
                    if any_hit {
                        return best;
                    }
                }
            }
        }
        best
    }

    /// Distance at which a ray enters a node's box, if it does before `limit`.
    fn entry(&self, node: usize, origin: Vec3, inverse: Vec3, limit: f32) -> Option<f32> {
        let node = &self.nodes[node];
        let t0 = (node.min - origin) * inverse;
        let t1 = (node.max - origin) * inverse;
        let near = t0.min(t1).max_element().max(0.0);
        let far = t0.max(t1).min_element().min(limit);
        // The comparison also rejects the NaN a ray parallel to a slab
        // produces when its origin lies on the slab plane.
        (near <= far).then_some(near)
    }
}

fn build_node(
    triangles: &[[Vec3; 3]],
    centroids: &[Vec3],
    order: &mut [u32],
    first: usize,
    depth: usize,
    nodes: &mut Vec<Node>,
) {
    let mut bounds = Bounds::EMPTY;
    let mut centroid_bounds = Bounds::EMPTY;
    for &triangle in order.iter() {
        for vertex in triangles[triangle as usize] {
            bounds.grow(vertex);
        }
        centroid_bounds.grow(centroids[triangle as usize]);
    }
    let index = nodes.len();
    nodes.push(Node {
        min: bounds.min,
        max: bounds.max,
        first_or_right: first as u32,
        count: order.len() as u32,
    });
    if order.len() <= LEAF_TRIANGLES || depth >= MAX_DEPTH {
        return;
    }
    let extent = centroid_bounds.max - centroid_bounds.min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    if extent[axis] <= 0.0 || extent[axis].is_nan() {
        return;
    }
    let Some(split) = sah_split(triangles, centroids, order, axis, centroid_bounds) else {
        return;
    };
    let (left, right) = order.split_at_mut(split);
    build_node(triangles, centroids, left, first, depth + 1, nodes);
    let right_index = nodes.len();
    build_node(triangles, centroids, right, first + split, depth + 1, nodes);
    nodes[index].first_or_right = right_index as u32;
    nodes[index].count = 0;
}

/// Partitions `order` at the binned surface-area-heuristic split along
/// `axis` and returns the size of the left part.
fn sah_split(
    triangles: &[[Vec3; 3]],
    centroids: &[Vec3],
    order: &mut [u32],
    axis: usize,
    centroid_bounds: Bounds,
) -> Option<usize> {
    let low = centroid_bounds.min[axis];
    let scale = BINS as f32 / (centroid_bounds.max[axis] - low);
    let bin_of = |triangle: u32| {
        (((centroids[triangle as usize][axis] - low) * scale) as usize).min(BINS - 1)
    };
    let mut bins = [(Bounds::EMPTY, 0_usize); BINS];
    for &triangle in order.iter() {
        let bin = &mut bins[bin_of(triangle)];
        for vertex in triangles[triangle as usize] {
            bin.0.grow(vertex);
        }
        bin.1 += 1;
    }
    let mut right_area = [0.0_f32; BINS];
    let mut accumulated = Bounds::EMPTY;
    let mut count = 0;
    for bin in (1..BINS).rev() {
        accumulated.merge(bins[bin].0);
        count += bins[bin].1;
        right_area[bin] = accumulated.half_area() * count as f32;
    }
    let mut best: Option<(f32, usize)> = None;
    let mut accumulated = Bounds::EMPTY;
    let mut count = 0;
    for bin in 0..BINS - 1 {
        accumulated.merge(bins[bin].0);
        count += bins[bin].1;
        if count == 0 || count == order.len() {
            continue;
        }
        let cost = accumulated.half_area() * count as f32 + right_area[bin + 1];
        if best.is_none_or(|(best_cost, _)| cost < best_cost) {
            best = Some((cost, bin));
        }
    }
    let (_, split_bin) = best?;
    let mut left = 0;
    for index in 0..order.len() {
        if bin_of(order[index]) <= split_bin {
            order.swap(left, index);
            left += 1;
        }
    }
    (left > 0 && left < order.len()).then_some(left)
}

/// Moller-Trumbore intersection, two-sided.
fn intersect_triangle(
    triangle: &[Vec3; 3],
    origin: Vec3,
    direction: Vec3,
    index: u32,
) -> Option<Hit> {
    let edge1 = triangle[1] - triangle[0];
    let edge2 = triangle[2] - triangle[0];
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() < 1.0e-12 {
        return None;
    }
    let inverse = determinant.recip();
    let to_origin = origin - triangle[0];
    let u = to_origin.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = to_origin.cross(edge1);
    let v = direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(q) * inverse;
    (distance > 0.0).then_some(Hit {
        distance,
        u,
        v,
        triangle: index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(z: f32, half: f32) -> [[Vec3; 3]; 2] {
        let p = |x: f32, y: f32| Vec3::new(x * half, y * half, z);
        [
            [p(-1.0, -1.0), p(1.0, -1.0), p(1.0, 1.0)],
            [p(-1.0, -1.0), p(1.0, 1.0), p(-1.0, 1.0)],
        ]
    }

    #[test]
    fn closest_hit_matches_a_brute_force_search() {
        // A grid of small quads at varying depths, more than one leaf deep.
        let mut triangles = Vec::new();
        for i in 0..40 {
            let offset = Vec3::new((i % 8) as f32 - 3.5, (i / 8) as f32 - 2.0, 0.0);
            for triangle in quad(-(i as f32) * 0.25 - 1.0, 0.45) {
                triangles.push(triangle.map(|vertex| vertex + offset));
            }
        }
        let bvh = Bvh::build(&triangles);
        let mut rng = super::super::sampling::Rng::new(7, 1);
        let mut hits = 0;
        for _ in 0..2_000 {
            let origin = Vec3::new(rng.next() * 8.0 - 4.0, rng.next() * 5.0 - 2.5, 2.0);
            let direction =
                Vec3::new(rng.next() * 0.4 - 0.2, rng.next() * 0.4 - 0.2, -1.0).normalize();
            let expected = triangles
                .iter()
                .enumerate()
                .filter_map(|(index, triangle)| {
                    intersect_triangle(triangle, origin, direction, index as u32)
                })
                .min_by(|a, b| a.distance.total_cmp(&b.distance));
            let actual = bvh.intersect(&triangles, origin, direction, f32::INFINITY, false, |_| {
                true
            });
            assert_eq!(
                actual.map(|hit| hit.triangle),
                expected.map(|hit| hit.triangle)
            );
            hits += usize::from(actual.is_some());
        }
        assert!(
            hits > 200,
            "the probe rays must actually exercise the tree: {hits}"
        );
    }

    #[test]
    fn rejected_candidates_let_the_ray_reach_what_is_behind_them() {
        let mut triangles = quad(-1.0, 1.0).to_vec();
        triangles.extend(quad(-3.0, 1.0));
        let bvh = Bvh::build(&triangles);
        let (origin, direction) = (Vec3::new(0.2, 0.1, 0.0), Vec3::NEG_Z);
        let front = bvh.intersect(&triangles, origin, direction, f32::INFINITY, false, |_| {
            true
        });
        assert!((front.expect("front quad is hit").distance - 1.0).abs() < 1.0e-5);
        let behind = bvh.intersect(&triangles, origin, direction, f32::INFINITY, false, |hit| {
            hit.triangle >= 2
        });
        assert!((behind.expect("back quad is hit").distance - 3.0).abs() < 1.0e-5);
        assert!(
            bvh.intersect(&triangles, origin, direction, 0.5, true, |_| true)
                .is_none()
        );
    }

    #[test]
    fn empty_scene_never_reports_a_hit() {
        let bvh = Bvh::build(&[]);
        assert!(
            bvh.intersect(&[], Vec3::ZERO, Vec3::Z, f32::INFINITY, false, |_| true)
                .is_none()
        );
    }
}
