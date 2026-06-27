use crate::capture::{CapturePixelBounds, CaptureProjection, CaptureRgba8};
use crate::geometry::Aabb;
use crate::scene::{Quat, Transform, Vec3};

pub(super) fn projected_aabb_bounds(
    capture: &CaptureRgba8,
    local_bounds: Aabb,
    world_transform: Transform,
) -> Option<CapturePixelBounds> {
    let mut points = Vec::with_capacity(8);
    for corner in aabb_corners(local_bounds) {
        let world = transform_point(corner, world_transform);
        let point = project_world_point(
            capture.descriptor.camera.world_transform?,
            capture.descriptor.camera.projection?,
            capture.descriptor.width,
            capture.descriptor.height,
            world,
        )?;
        points.push(point);
    }
    pixel_bounds_from_points(&points, capture.descriptor.width, capture.descriptor.height)
}

fn project_world_point(
    world_from_camera: Transform,
    projection: CaptureProjection,
    viewport_width: u32,
    viewport_height: u32,
    world_position: Vec3,
) -> Option<[f32; 2]> {
    if viewport_width == 0 || viewport_height == 0 {
        return None;
    }
    let view = world_to_view(world_position, world_from_camera)?;
    let (ndc_x, ndc_y) = match projection {
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
            } else {
                viewport_width as f32 / viewport_height as f32
            };
            let focal = (vertical_fov_radians * 0.5).tan().recip();
            if !focal.is_finite() {
                return None;
            }
            (view.x * focal / (aspect * depth), view.y * focal / depth)
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
            )
        }
    };
    if !ndc_x.is_finite() || !ndc_y.is_finite() {
        return None;
    }
    Some([
        (ndc_x * 0.5 + 0.5) * viewport_width as f32,
        (1.0 - (ndc_y * 0.5 + 0.5)) * viewport_height as f32,
    ])
}

fn pixel_bounds_from_points(
    points: &[[f32; 2]],
    width: u32,
    height: u32,
) -> Option<CapturePixelBounds> {
    if points.is_empty() || width == 0 || height == 0 {
        return None;
    }
    let min_x = points.iter().map(|point| point[0]).min_by(f32::total_cmp)?;
    let min_y = points.iter().map(|point| point[1]).min_by(f32::total_cmp)?;
    let max_x = points.iter().map(|point| point[0]).max_by(f32::total_cmp)?;
    let max_y = points.iter().map(|point| point[1]).max_by(f32::total_cmp)?;
    if max_x < 0.0 || max_y < 0.0 || min_x >= width as f32 || min_y >= height as f32 {
        return None;
    }

    let last_x = width.saturating_sub(1) as f32;
    let last_y = height.saturating_sub(1) as f32;
    let min_x = min_x.floor().clamp(0.0, last_x) as u32;
    let min_y = min_y.floor().clamp(0.0, last_y) as u32;
    let max_x = max_x.ceil().clamp(0.0, last_x) as u32;
    let max_y = max_y.ceil().clamp(0.0, last_y) as u32;
    if max_x < min_x || max_y < min_y {
        return None;
    }
    Some(CapturePixelBounds {
        min_x,
        min_y,
        max_x,
        max_y,
        width: max_x.saturating_sub(min_x).saturating_add(1),
        height: max_y.saturating_sub(min_y).saturating_add(1),
    })
}

fn world_to_view(world_position: Vec3, world_from_camera: Transform) -> Option<Vec3> {
    if !world_from_camera.translation.is_finite()
        || !world_from_camera.rotation.is_finite()
        || !is_finite_nonzero_scale(world_from_camera.scale)
    {
        return None;
    }
    let translated = world_position - world_from_camera.translation;
    let rotated = inverse_unit_quat(world_from_camera.rotation) * translated;
    Some(Vec3::new(
        rotated.x / world_from_camera.scale.x,
        rotated.y / world_from_camera.scale.y,
        rotated.z / world_from_camera.scale.z,
    ))
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    transform.translation + normalize_quat(transform.rotation) * (point * transform.scale)
}

fn inverse_unit_quat(rotation: Quat) -> Quat {
    normalize_quat(rotation).inverse()
}

fn normalize_quat(rotation: Quat) -> Quat {
    let length_squared = rotation.length_squared();
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        Quat::IDENTITY
    } else {
        rotation.normalize()
    }
}

fn is_finite_nonzero_scale(scale: Vec3) -> bool {
    scale.is_finite()
        && scale.x.abs() > f32::EPSILON
        && scale.y.abs() > f32::EPSILON
        && scale.z.abs() > f32::EPSILON
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
