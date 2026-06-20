use crate::{Aabb, CaptureProjection, CaptureRgba8, CaptureScreenRect, Transform, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureProjectedPoint {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub ndc_x: f32,
    pub ndc_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureScreenRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn project_aabb_from_capture(
    capture: &CaptureRgba8,
    local_bounds: Aabb,
    world_transform: Transform,
) -> Option<CaptureScreenRect> {
    let mut points = Vec::with_capacity(8);
    for corner in aabb_corners(local_bounds) {
        let world = transform_point_for_projection(world_transform, corner);
        if let Some(point) = project_world_point_from_capture(capture, world) {
            points.push(point);
        }
    }
    screen_rect_from_points(points.as_slice())
}

pub fn project_world_point_from_capture(
    capture: &CaptureRgba8,
    world: Vec3,
) -> Option<CaptureProjectedPoint> {
    let world_from_camera = capture.descriptor.camera.world_transform?;
    let projection = capture.descriptor.camera.projection?;
    let view = world_to_view(world, world_from_camera)?;
    let (ndc_x, ndc_y, depth) = match projection {
        CaptureProjection::Perspective {
            vertical_fov_radians,
            aspect,
            near,
            far,
        } => {
            let depth = -view.z;
            if !depth.is_finite() || depth < near || depth > far {
                return None;
            }
            let aspect = if aspect.is_finite() && aspect > 0.0 {
                aspect
            } else if capture.descriptor.height > 0 {
                capture.descriptor.width as f32 / capture.descriptor.height as f32
            } else {
                return None;
            };
            let focal = (vertical_fov_radians * 0.5).tan().recip();
            if !focal.is_finite() {
                return None;
            }
            (
                view.x * focal / (aspect * depth),
                view.y * focal / depth,
                depth,
            )
        }
        CaptureProjection::Orthographic {
            left,
            right,
            bottom,
            top,
            near,
            far,
        } => {
            let depth = -view.z;
            if !depth.is_finite() || depth < near || depth > far {
                return None;
            }
            let width = right - left;
            let height = top - bottom;
            if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
                return None;
            }
            (
                (view.x - left) / width * 2.0 - 1.0,
                (view.y - bottom) / height * 2.0 - 1.0,
                depth,
            )
        }
    };
    if !ndc_x.is_finite()
        || !ndc_y.is_finite()
        || capture.descriptor.width == 0
        || capture.descriptor.height == 0
    {
        return None;
    }
    Some(CaptureProjectedPoint {
        x: (ndc_x * 0.5 + 0.5) * capture.descriptor.width as f32,
        y: (1.0 - (ndc_y * 0.5 + 0.5)) * capture.descriptor.height as f32,
        depth,
        ndc_x,
        ndc_y,
    })
}

pub fn screen_region_from_center_size(
    center_x: f32,
    center_y: f32,
    width_px: f32,
    height_px: f32,
    padding_px: f32,
    viewport_width: u32,
    viewport_height: u32,
) -> Option<CaptureScreenRegion> {
    if !center_x.is_finite()
        || !center_y.is_finite()
        || !width_px.is_finite()
        || !height_px.is_finite()
        || !padding_px.is_finite()
        || viewport_width == 0
        || viewport_height == 0
    {
        return None;
    }
    let half_width = (width_px * 0.5).max(0.0);
    let half_height = (height_px * 0.5).max(0.0);
    screen_region_from_bounds(
        center_x - half_width - padding_px,
        center_y - half_height - padding_px,
        center_x + half_width + padding_px,
        center_y + half_height + padding_px,
        viewport_width,
        viewport_height,
    )
}

pub fn screen_region_from_points(
    points: &[(f32, f32)],
    padding_px: f32,
    viewport_width: u32,
    viewport_height: u32,
) -> Option<CaptureScreenRegion> {
    if points.is_empty() || !padding_px.is_finite() || viewport_width == 0 || viewport_height == 0 {
        return None;
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (x, y) in points {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }
    screen_region_from_bounds(
        min_x - padding_px,
        min_y - padding_px,
        max_x + padding_px,
        max_y + padding_px,
        viewport_width,
        viewport_height,
    )
}

pub fn screen_region_from_rect(
    rect: CaptureScreenRect,
    viewport_width: u32,
    viewport_height: u32,
) -> Option<CaptureScreenRegion> {
    screen_region_from_bounds(
        rect.min_x,
        rect.min_y,
        rect.max_x,
        rect.max_y,
        viewport_width,
        viewport_height,
    )
}

pub fn transform_point_for_projection(transform: Transform, point: Vec3) -> Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
}

fn screen_rect_from_points(points: &[CaptureProjectedPoint]) -> Option<CaptureScreenRect> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    Some(CaptureScreenRect {
        min_x: round3_f32(min_x),
        min_y: round3_f32(min_y),
        max_x: round3_f32(max_x),
        max_y: round3_f32(max_y),
        width: round3_f32(width),
        height: round3_f32(height),
        center_x: round3_f32((min_x + max_x) * 0.5),
        center_y: round3_f32((min_y + max_y) * 0.5),
    })
}

fn screen_region_from_bounds(
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    viewport_width: u32,
    viewport_height: u32,
) -> Option<CaptureScreenRegion> {
    if !min_x.is_finite()
        || !min_y.is_finite()
        || !max_x.is_finite()
        || !max_y.is_finite()
        || viewport_width == 0
        || viewport_height == 0
    {
        return None;
    }
    let x0 = min_x.floor().max(0.0);
    let y0 = min_y.floor().max(0.0);
    let x1 = max_x.ceil().min(viewport_width as f32);
    let y1 = max_y.ceil().min(viewport_height as f32);
    let x = x0 as u32;
    let y = y0 as u32;
    let max_x = x1.max(x0) as u32;
    let max_y = y1.max(y0) as u32;
    Some(CaptureScreenRegion {
        x: x.min(viewport_width),
        y: y.min(viewport_height),
        width: max_x.saturating_sub(x).max(1),
        height: max_y.saturating_sub(y).max(1),
    })
}

fn world_to_view(world: Vec3, world_from_camera: Transform) -> Option<Vec3> {
    if !world_from_camera.translation.is_finite()
        || !world_from_camera.rotation.is_finite()
        || !world_from_camera.scale.is_finite()
        || world_from_camera.scale.x.abs() <= f32::EPSILON
        || world_from_camera.scale.y.abs() <= f32::EPSILON
        || world_from_camera.scale.z.abs() <= f32::EPSILON
    {
        return None;
    }
    let translated = world - world_from_camera.translation;
    let rotated = world_from_camera.rotation.inverse() * translated;
    Some(Vec3::new(
        rotated.x / world_from_camera.scale.x,
        rotated.y / world_from_camera.scale.y,
        rotated.z / world_from_camera.scale.z,
    ))
}

fn aabb_corners(bounds: Aabb) -> [Vec3; 8] {
    let min = bounds.min;
    let max = bounds.max;
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
}

fn round3_f32(value: f32) -> f32 {
    if value.is_finite() {
        ((value as f64) * 1000.0).round() as f32 / 1000.0
    } else {
        value
    }
}
