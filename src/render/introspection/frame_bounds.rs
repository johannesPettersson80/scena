use crate::geometry::Aabb;
use crate::scene::{SceneInspectionReportV1, Transform, Vec3};

use super::{RenderIntrospectionFixV1, push_fix, round3};

pub(in crate::render) fn push_frame_bounds_fix(
    fixes: &mut Vec<RenderIntrospectionFixV1>,
    help: &str,
    inspection: &SceneInspectionReportV1,
) {
    let Some(patch) = frame_bounds_patch(inspection) else {
        return;
    };
    push_fix(
        fixes,
        "frame_bounds",
        inspection.active_camera,
        Some(patch),
        help,
    );
}

pub(in crate::render) fn frame_bounds_patch(
    inspection: &SceneInspectionReportV1,
) -> Option<serde_json::Value> {
    let bounds = combined_draw_world_bounds(inspection)?;
    let center = bounds.center();
    let radius = bounds.bounding_sphere_radius().max(0.25);
    let distance = round3((radius * 2.8).max(1.0));
    Some(serde_json::json!({
        "camera": {
            "target": [round3(center.x), round3(center.y), round3(center.z)],
            "distance": distance,
            "yaw_radians": 0.785,
            "pitch_radians": 0.524
        }
    }))
}

fn combined_draw_world_bounds(inspection: &SceneInspectionReportV1) -> Option<Aabb> {
    inspection
        .draw_list
        .iter()
        .filter_map(|draw| transform_aabb(draw.local_bounds, draw.world_transform))
        .reduce(Aabb::union)
}

fn transform_aabb(bounds: Aabb, transform: Transform) -> Option<Aabb> {
    let mut corners = [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ];
    for corner in &mut corners {
        *corner = transform_point(*corner, transform);
        if !corner.is_finite() {
            return None;
        }
    }
    let mut min = corners[0];
    let mut max = corners[0];
    for corner in &corners[1..] {
        min = min.min(*corner);
        max = max.max(*corner);
    }
    Some(Aabb::new(min, max))
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
}
