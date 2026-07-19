use crate::assets::GeometryHandle;
use crate::geometry::Aabb;
use crate::scene::{NodeKey, Scene, Transform, Vec3};

use crate::render::camera::CameraProjection;

pub(super) fn select_mesh_lod_geometry(
    scene: &Scene,
    node: NodeKey,
    base_geometry: GeometryHandle,
    local_bounds: Aabb,
    transform: Transform,
    camera_projection: Option<&CameraProjection>,
) -> GeometryHandle {
    let Some(levels) = scene.mesh_lods(node) else {
        return base_geometry;
    };
    let Some(fraction) =
        projected_bounds_screen_fraction(local_bounds, transform, camera_projection)
    else {
        return base_geometry;
    };
    levels
        .iter()
        .find(|level| fraction <= level.max_screen_fraction())
        .map_or(base_geometry, |level| level.geometry())
}

fn projected_bounds_screen_fraction(
    local_bounds: Aabb,
    transform: Transform,
    camera_projection: Option<&CameraProjection>,
) -> Option<f32> {
    let projection = camera_projection?;
    let bounds = crate::scene::view_math::transform_aabb(local_bounds, transform);
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut projected_any = false;
    for corner in aabb_corners(bounds) {
        let Some(projected) = projection.project(corner) else {
            continue;
        };
        min_x = min_x.min(projected.ndc_x);
        min_y = min_y.min(projected.ndc_y);
        max_x = max_x.max(projected.ndc_x);
        max_y = max_y.max(projected.ndc_y);
        projected_any = true;
    }
    if !projected_any {
        return None;
    }
    let width_fraction = ((max_x - min_x).abs() * 0.5).clamp(0.0, 1.0);
    let height_fraction = ((max_y - min_y).abs() * 0.5).clamp(0.0, 1.0);
    Some(width_fraction.max(height_fraction))
}

fn aabb_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}
