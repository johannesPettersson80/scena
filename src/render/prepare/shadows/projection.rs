//! Focused orthographic fit for the directional shadow map.
//!
//! Fitting the map around every occluder spends nearly all of its resolution
//! on a large floor that cannot shadow anything. A shadow falls exactly inside
//! the light-space footprint of the object casting it, so the map only has to
//! cover the footprint of occluders that can shadow something:
//!
//! - every occluder with thickness, which can at least shadow itself, and
//! - a flat occluder only when another occluder lies behind its plane and
//!   inside its footprint (a ceiling over a floor).
//!
//! Receivers outside the fitted footprint are lit, which the shaders already
//! guarantee for samples that leave the map. Depth is fitted to the casters
//! plus the patch of each excluded plane under the footprint, so the floor
//! under a subject still receives its shadow.

use std::ops::Range;

use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::geometry::{Aabb, GeometryDesc, GeometryTopology};
use crate::scene::{NodeKey, Scene, Vec3, WorldTransform};

use super::super::identity_matrix4;
use super::super::transforms::{compose_transform, transform_position, transform_primitive};
use super::DeformationInputs;
use super::collection::shadow_vertices;
use super::math::bounds_corners;

/// World extents thinner than this fraction of their span are treated as a plane.
const FLAT_THICKNESS_FRACTION: f32 = 1.0e-3;
/// Scenes with more flat occluders than this skip the pairwise relevance test.
const MAX_FLAT_RELEVANCE_TESTS: usize = 64;
/// Footprint margin kept for penumbrae and the filter kernel.
const FOOTPRINT_PADDING_FRACTION: f32 = 0.08;
const DEPTH_PADDING_FRACTION: f32 = 0.05;

/// World-space points of every shadow occluder, grouped by the object they
/// belong to.
#[derive(Debug, Default)]
pub(in crate::render) struct ShadowProjectionGroups {
    points: Vec<Vec3>,
    groups: Vec<Range<usize>>,
}

impl ShadowProjectionGroups {
    fn push_group(&mut self, points: impl IntoIterator<Item = Vec3>) {
        let start = self.points.len();
        self.points.extend(points);
        if self.points.len() > start {
            self.groups.push(start..self.points.len());
        }
    }

    fn group_points(&self) -> impl Iterator<Item = &[Vec3]> {
        self.groups.iter().map(|range| &self.points[range.clone()])
    }
}

pub(in crate::render) fn collect_shadow_projection_groups<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<ShadowProjectionGroups, PrepareError> {
    let mut groups = ShadowProjectionGroups::default();
    for (renderable, transform) in scene.renderables() {
        groups.push_group(renderable.primitives().iter().flat_map(|primitive| {
            transform_primitive(primitive, transform, origin_shift)
                .vertices()
                .map(|vertex| vertex.position)
        }));
    }
    let Some(assets) = assets else {
        return Ok(groups);
    };
    for (node, mesh, transform) in scene.mesh_nodes() {
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        push_geometry_group(
            &mut groups,
            node,
            &geometry,
            DeformationInputs {
                morph_weights: scene.morph_weights(node),
                skin_matrices: scene.skin_matrices(node).as_deref(),
            },
            transform,
            origin_shift,
        )?;
    }
    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        for instance in instance_set.instances() {
            push_geometry_group(
                &mut groups,
                node,
                &geometry,
                DeformationInputs::default(),
                compose_transform(node_transform, instance.transform()),
                origin_shift,
            )?;
        }
    }
    Ok(groups)
}

fn push_geometry_group(
    groups: &mut ShadowProjectionGroups,
    node: NodeKey,
    geometry: &GeometryDesc,
    deformation: DeformationInputs<'_>,
    transform: WorldTransform,
    origin_shift: Vec3,
) -> Result<(), PrepareError> {
    if geometry.topology() != GeometryTopology::Triangles {
        return Ok(());
    }
    if deformation.morph_weights.is_none()
        && deformation.skin_matrices.is_none()
        && geometry.skin().is_none()
    {
        groups.push_group(
            bounds_corners(geometry.bounds())
                .into_iter()
                .map(|corner| transform_position(corner, transform, origin_shift)),
        );
        return Ok(());
    }
    let vertices = shadow_vertices(node, geometry, deformation)?;
    groups.push_group(
        vertices
            .iter()
            .map(|vertex| transform_position(vertex.position, transform, origin_shift)),
    );
    Ok(())
}

/// Orthonormal light-view basis: `forward` is the direction the light travels.
struct LightBasis {
    right: Vec3,
    up: Vec3,
    forward: Vec3,
}

impl LightBasis {
    fn new(light_direction: Vec3) -> Self {
        let forward = light_direction.try_normalize().unwrap_or(Vec3::NEG_Z);
        let world_up = if forward.y.abs() > 0.99 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let right = world_up.cross(forward).try_normalize().unwrap_or(Vec3::X);
        let up = forward.cross(right);
        Self { right, up, forward }
    }

    fn to_light(&self, world: Vec3) -> Vec3 {
        Vec3::new(
            self.right.dot(world),
            self.up.dot(world),
            self.forward.dot(world),
        )
    }
}

/// One occluder object in world and light space.
struct Occluder {
    world: Aabb,
    light: Aabb,
    /// Axis index and world coordinate of the plane, for flat occluders.
    plane: Option<(usize, f32)>,
}

impl Occluder {
    fn new(points: &[Vec3], basis: &LightBasis) -> Self {
        let mut world = empty_bounds();
        let mut light = empty_bounds();
        for point in points {
            grow(&mut world, *point);
            grow(&mut light, basis.to_light(*point));
        }
        let extent = world.max - world.min;
        let span = extent.max_element();
        let axis = (0..3)
            .min_by(|left, right| extent[*left].total_cmp(&extent[*right]))
            .unwrap_or(1);
        let plane = (span > 0.0 && extent[axis] <= span * FLAT_THICKNESS_FRACTION)
            .then(|| (axis, 0.5 * (world.min[axis] + world.max[axis])));
        Self {
            world,
            light,
            plane,
        }
    }

    fn footprint_overlaps(&self, other: &Self) -> bool {
        self.light.min.x <= other.light.max.x
            && other.light.min.x <= self.light.max.x
            && self.light.min.y <= other.light.max.y
            && other.light.min.y <= self.light.max.y
    }

    /// Whether `other` reaches behind this plane as seen from the light.
    fn plane_can_shadow(&self, other: &Self, basis: &LightBasis) -> bool {
        let Some((axis, coordinate)) = self.plane else {
            return true;
        };
        let toward_light = -basis.forward[axis];
        if toward_light.abs() <= 1.0e-4 {
            // Edge-on to the light: the plane casts no shadow at all.
            return false;
        }
        let tolerance = (self.world.max - self.world.min).max_element() * FLAT_THICKNESS_FRACTION;
        let behind = if toward_light > 0.0 {
            other.world.min[axis] < coordinate - tolerance
        } else {
            other.world.max[axis] > coordinate + tolerance
        };
        behind && self.footprint_overlaps(other)
    }

    /// Light-space depth range of this plane over a light-space rectangle.
    fn plane_depth_over(&self, min: Vec3, max: Vec3, basis: &LightBasis) -> Option<(f32, f32)> {
        let (axis, coordinate) = self.plane?;
        let denominator = basis.forward[axis];
        if denominator.abs() <= 1.0e-4 {
            return None;
        }
        let x0 = min.x.max(self.light.min.x);
        let x1 = max.x.min(self.light.max.x);
        let y0 = min.y.max(self.light.min.y);
        let y1 = max.y.min(self.light.max.y);
        if x0 > x1 || y0 > y1 {
            return None;
        }
        // world = right * lx + up * ly + forward * lz, and world[axis] is the
        // plane coordinate, so lz is linear in (lx, ly).
        let depth = |lx: f32, ly: f32| {
            (coordinate - basis.right[axis] * lx - basis.up[axis] * ly) / denominator
        };
        let corners = [depth(x0, y0), depth(x1, y0), depth(x0, y1), depth(x1, y1)];
        Some((
            corners.into_iter().fold(f32::INFINITY, f32::min),
            corners.into_iter().fold(f32::NEG_INFINITY, f32::max),
        ))
    }
}

/// Orthographic `light_from_world` focused on the occluders that can cast a
/// visible shadow. Column-major, depth in `[0, 1]`.
pub(in crate::render) fn focused_light_view_projection(
    light_direction: Vec3,
    groups: &ShadowProjectionGroups,
) -> [f32; 16] {
    let basis = LightBasis::new(light_direction);
    let occluders: Vec<Occluder> = groups
        .group_points()
        .map(|points| Occluder::new(points, &basis))
        .collect();
    if occluders.is_empty() {
        return identity_matrix4();
    }
    let flat_count = occluders
        .iter()
        .filter(|occluder| occluder.plane.is_some())
        .count();
    let relevant: Vec<bool> = occluders
        .iter()
        .enumerate()
        .map(|(index, occluder)| {
            occluder.plane.is_none()
                || flat_count > MAX_FLAT_RELEVANCE_TESTS
                || occluders.iter().enumerate().any(|(other_index, other)| {
                    other_index != index && occluder.plane_can_shadow(other, &basis)
                })
        })
        .collect();
    let any_relevant = relevant.iter().any(|relevant| *relevant);

    let mut bounds = empty_bounds();
    for (occluder, relevant) in occluders.iter().zip(&relevant) {
        if *relevant || !any_relevant {
            grow(&mut bounds, occluder.light.min);
            grow(&mut bounds, occluder.light.max);
        }
    }
    if any_relevant {
        // Excluded planes still receive shadows inside the footprint.
        let (min, max) = (bounds.min, bounds.max);
        for (occluder, relevant) in occluders.iter().zip(&relevant) {
            if !*relevant && let Some((near, far)) = occluder.plane_depth_over(min, max, &basis) {
                bounds.min.z = bounds.min.z.min(near);
                bounds.max.z = bounds.max.z.max(far);
            }
        }
    }

    let extent = bounds.max - bounds.min;
    let footprint_pad = extent.x.max(extent.y) * FOOTPRINT_PADDING_FRACTION;
    let depth_pad = (extent.x + extent.y + extent.z) * DEPTH_PADDING_FRACTION;
    let min = bounds.min - Vec3::new(footprint_pad, footprint_pad, depth_pad);
    let max = bounds.max + Vec3::new(footprint_pad, footprint_pad, depth_pad);
    orthographic_light_from_world(&basis, min, max)
}

fn orthographic_light_from_world(basis: &LightBasis, min: Vec3, max: Vec3) -> [f32; 16] {
    let inv_x = 1.0 / (max.x - min.x).max(f32::EPSILON);
    let inv_y = 1.0 / (max.y - min.y).max(f32::EPSILON);
    let inv_z = 1.0 / (max.z - min.z).max(f32::EPSILON);
    let (right, up, forward) = (basis.right, basis.up, basis.forward);
    [
        2.0 * right.x * inv_x,
        2.0 * up.x * inv_y,
        forward.x * inv_z,
        0.0,
        2.0 * right.y * inv_x,
        2.0 * up.y * inv_y,
        forward.y * inv_z,
        0.0,
        2.0 * right.z * inv_x,
        2.0 * up.z * inv_y,
        forward.z * inv_z,
        0.0,
        -(max.x + min.x) * inv_x,
        -(max.y + min.y) * inv_y,
        -min.z * inv_z,
        1.0,
    ]
}

fn empty_bounds() -> Aabb {
    Aabb {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    }
}

fn grow(bounds: &mut Aabb, point: Vec3) {
    bounds.min = bounds.min.min(point);
    bounds.max = bounds.max.max(point);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
        bounds_corners(Aabb { min, max })
    }

    fn groups_of(objects: &[[Vec3; 8]]) -> ShadowProjectionGroups {
        let mut groups = ShadowProjectionGroups::default();
        for object in objects {
            groups.push_group(*object);
        }
        groups
    }

    /// World-space width of the fitted footprint along light-space X.
    fn footprint_width(matrix: &[f32; 16]) -> f32 {
        2.0 / Vec3::new(matrix[0], matrix[4], matrix[8]).length()
    }

    fn project(matrix: &[f32; 16], point: Vec3) -> Vec3 {
        glam::Mat4::from_cols_array(matrix).project_point3(point)
    }

    fn inside(ndc: Vec3) -> bool {
        ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0 && (0.0..=1.0).contains(&ndc.z)
    }

    const LIGHT: Vec3 = Vec3::new(-0.3, -0.8, -0.5);

    #[test]
    fn a_large_floor_no_longer_dilutes_the_map_around_a_small_subject() {
        let floor = box_corners(Vec3::new(-80.0, 0.0, -80.0), Vec3::new(80.0, 0.0, 80.0));
        let subject = box_corners(Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5));
        let focused = focused_light_view_projection(LIGHT, &groups_of(&[floor, subject]));
        let subject_only = focused_light_view_projection(LIGHT, &groups_of(&[subject]));
        assert!(
            footprint_width(&focused) < 4.0,
            "footprint must hug the 1 m subject, got {} m",
            footprint_width(&focused)
        );
        assert!((footprint_width(&focused) - footprint_width(&subject_only)).abs() < 1.0e-3);
    }

    #[test]
    fn the_floor_under_the_subject_stays_inside_the_fitted_depth_range() {
        let floor = box_corners(Vec3::new(-80.0, 0.0, -80.0), Vec3::new(80.0, 0.0, 80.0));
        let subject = box_corners(Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5));
        let matrix = focused_light_view_projection(LIGHT, &groups_of(&[floor, subject]));
        // The point on the floor that the top of the subject shadows.
        let light = LIGHT.normalize();
        let top = Vec3::new(0.5, 1.0, 0.5);
        let shadowed = top + light * (top.y / -light.y);
        assert!(shadowed.y.abs() < 1.0e-4);
        assert!(
            inside(project(&matrix, shadowed)),
            "shadow receiver must be sampled"
        );
        assert!(
            inside(project(&matrix, top)),
            "caster must be inside the map"
        );
        assert!(
            !inside(project(&matrix, Vec3::new(60.0, 0.0, 60.0))),
            "distant floor is outside the map and therefore lit"
        );
    }

    #[test]
    fn a_ceiling_over_a_floor_is_kept_because_it_shadows_the_floor() {
        let floor = box_corners(Vec3::new(-10.0, 0.0, -10.0), Vec3::new(10.0, 0.0, 10.0));
        let ceiling = box_corners(Vec3::new(-2.0, 3.0, -2.0), Vec3::new(2.0, 3.0, 2.0));
        let matrix = focused_light_view_projection(LIGHT, &groups_of(&[floor, ceiling]));
        let light = LIGHT.normalize();
        let edge = Vec3::new(2.0, 3.0, 2.0);
        let shadowed = edge + light * (edge.y / -light.y);
        assert!(inside(project(&matrix, edge)));
        assert!(inside(project(&matrix, shadowed)));
        assert!(footprint_width(&matrix) < 12.0);
    }

    #[test]
    fn a_lone_floor_falls_back_to_the_whole_scene() {
        let floor = box_corners(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 0.0, 5.0));
        let matrix = focused_light_view_projection(LIGHT, &groups_of(&[floor]));
        for corner in floor {
            assert!(inside(project(&matrix, corner)));
        }
    }

    #[test]
    fn empty_scenes_produce_the_identity_matrix() {
        let matrix = focused_light_view_projection(LIGHT, &ShadowProjectionGroups::default());
        assert_eq!(matrix, identity_matrix4());
    }
}
