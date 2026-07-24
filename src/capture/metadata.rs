use crate::geometry::Aabb;
use crate::scene::{Camera, CameraKey, Scene, SceneDirtyState, Vec3};

use super::{
    CaptureAutoFrame, CaptureAutoFrameViewport, CaptureCamera, CaptureError, CaptureOptions,
    CapturePoint2, CaptureProjection, CaptureRevisions, CaptureScreenRect, CaptureViewport,
};

pub fn auto_frame_metadata(
    scene: &Scene,
    camera: CameraKey,
    bounds: Aabb,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<CaptureAutoFrame, CaptureError> {
    let mut points = Vec::with_capacity(8);
    for corner in aabb_corners(bounds) {
        let point = scene
            .project_world_point(camera, corner, viewport_width, viewport_height)
            .map_err(CaptureError::from)?
            .ok_or_else(|| CaptureError::AutoFrameProjection {
                reason: "auto-frame projected a bounds corner outside the active camera".to_owned(),
            })?;
        points.push(point);
    }

    let rect = crate::ScreenRect::from_points(&points).ok_or_else(|| {
        CaptureError::AutoFrameProjection {
            reason: "auto-frame projection produced no points".to_owned(),
        }
    })?;
    let projected_rect = CaptureScreenRect::from_rect(rect);
    let viewport_width_f = viewport_width as f32;
    let viewport_height_f = viewport_height as f32;
    let center_error_x = (projected_rect.center_x - viewport_width_f * 0.5).abs();
    let center_error_y = (projected_rect.center_y - viewport_height_f * 0.5).abs();
    let fill_fraction = rect.fill_fraction(viewport_width, viewport_height);
    let inside_viewport = projected_rect.min_x >= -0.5
        && projected_rect.min_y >= -0.5
        && projected_rect.max_x <= viewport_width_f + 0.5
        && projected_rect.max_y <= viewport_height_f + 0.5;
    let centered =
        center_error_x <= viewport_width_f * 0.05 && center_error_y <= viewport_height_f * 0.05;
    let passed = inside_viewport && centered && fill_fraction > 0.2 && fill_fraction <= 0.75;

    Ok(CaptureAutoFrame {
        status: if passed { "passed" } else { "failed" }.to_owned(),
        proof_class: "viewer-level-auto-framing".to_owned(),
        viewport: CaptureAutoFrameViewport {
            width: viewport_width,
            height: viewport_height,
        },
        projected_rect,
        center_error_px: CapturePoint2 {
            x: center_error_x,
            y: center_error_y,
        },
        fill_fraction,
        inside_viewport,
        centered,
    })
}

pub(super) fn capture_auto_frame(
    scene: &Scene,
    camera: CameraKey,
    bounds: Option<Aabb>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<Option<CaptureAutoFrame>, CaptureError> {
    let Some(bounds) = bounds else {
        return Ok(None);
    };
    if scene.camera(camera).is_none() {
        return Err(CaptureError::NoActiveCameraForAutoFrame);
    }
    Ok(Some(auto_frame_metadata(
        scene,
        camera,
        bounds,
        viewport_width,
        viewport_height,
    )?))
}

pub(super) fn capture_camera(scene: &Scene, camera: CameraKey) -> CaptureCamera {
    let world_transform = scene
        .camera_node(camera)
        .and_then(|node| scene.world_transform(node));
    let projection = scene.camera(camera).map(projection_from_camera);
    CaptureCamera {
        active: projection.is_some(),
        world_transform,
        projection,
    }
}

pub(super) fn revisions_from_dirty(dirty: SceneDirtyState) -> CaptureRevisions {
    CaptureRevisions {
        structure: dirty.structure_revision,
        transform: dirty.transform_revision,
        camera: dirty.camera_revision,
        appearance: dirty
            .appearance_revision
            .saturating_add(dirty.visibility_revision),
        interaction: dirty.interaction_revision,
    }
}

pub(super) fn capture_viewport(
    width: u32,
    height: u32,
    options: CaptureOptions,
) -> Result<CaptureViewport, CaptureError> {
    let dpr = options.device_pixel_ratio;
    if !dpr.is_finite() || dpr <= 0.0 {
        return Err(CaptureError::InvalidDevicePixelRatio { value: dpr });
    }
    let (logical_width, logical_height) = options
        .logical_size
        .unwrap_or((width as f32 / dpr, height as f32 / dpr));
    Ok(CaptureViewport {
        width,
        height,
        logical_width,
        logical_height,
        device_pixel_ratio: dpr,
    })
}

fn projection_from_camera(camera: &Camera) -> CaptureProjection {
    match camera {
        Camera::Perspective(camera) => CaptureProjection::Perspective {
            vertical_fov_radians: camera.vertical_fov.radians(),
            aspect: camera.aspect,
            near: camera.near,
            far: camera.far,
        },
        Camera::Orthographic(camera) => CaptureProjection::Orthographic {
            left: camera.left,
            right: camera.right,
            bottom: camera.bottom,
            top: camera.top,
            near: camera.near,
            far: camera.far,
        },
    }
}

fn aabb_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}

impl CaptureScreenRect {
    fn from_rect(rect: crate::ScreenRect) -> Self {
        Self {
            min_x: rect.min_x,
            min_y: rect.min_y,
            max_x: rect.max_x,
            max_y: rect.max_y,
            width: rect.width(),
            height: rect.height(),
            center_x: rect.center_x(),
            center_y: rect.center_y(),
        }
    }
}
