use crate::diagnostics::LookupError;

use super::{FramingOptions, aabb_corners, validate_bounds};
use crate::scene::view_math::look_rotation;
use crate::scene::{PerspectiveCamera, Quat, Transform, Vec3};

#[derive(Debug, Clone, Copy)]
pub(super) struct ValidFramingOptions {
    pub(super) view_direction: Vec3,
    pub(super) up: Vec3,
    pub(super) fill: f32,
    pub(super) margin_px: f32,
    pub(super) viewport_width: u32,
    pub(super) viewport_height: u32,
    pub(super) tighten_depth_range: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PerspectiveFit {
    pub(super) camera_transform: Transform,
    pub(super) target: Vec3,
    pub(super) distance: f32,
    pub(super) yaw_radians: f32,
    pub(super) pitch_radians: f32,
    pub(super) depth_radius: f32,
}

impl ValidFramingOptions {
    pub(super) fn new(options: FramingOptions) -> Result<Self, LookupError> {
        if options.viewport_width == 0 || options.viewport_height == 0 {
            return Err(LookupError::InvalidViewport {
                width: options.viewport_width,
                height: options.viewport_height,
            });
        }
        if !options.fill.is_finite() || options.fill <= 0.0 || options.fill > 1.0 {
            return Err(LookupError::InvalidFramingOption {
                field: "fill",
                reason: "fill must be finite and within 0 < fill <= 1",
            });
        }
        if !options.margin_px.is_finite() || options.margin_px < 0.0 {
            return Err(LookupError::InvalidFramingOption {
                field: "margin_px",
                reason: "margin_px must be finite and non-negative",
            });
        }
        let min_dimension = options.viewport_width.min(options.viewport_height) as f32;
        if options.margin_px * 2.0 >= min_dimension {
            return Err(LookupError::InvalidFramingOption {
                field: "margin_px",
                reason: "margin_px leaves no usable viewport",
            });
        }
        let view_direction =
            normalize(options.view_direction).ok_or(LookupError::InvalidFramingOption {
                field: "view_direction",
                reason: "view_direction must be finite and non-zero",
            })?;
        let up = normalize(options.up).ok_or(LookupError::InvalidFramingOption {
            field: "up",
            reason: "up vector must be finite and non-zero",
        })?;
        Ok(Self {
            view_direction,
            up,
            fill: options.fill,
            margin_px: options.margin_px,
            viewport_width: options.viewport_width,
            viewport_height: options.viewport_height,
            tighten_depth_range: options.tighten_depth_range,
        })
    }

    pub(super) fn aspect(self) -> f32 {
        self.viewport_width as f32 / self.viewport_height as f32
    }

    fn allowed_ndc_x(self) -> f32 {
        let usable = self.viewport_width as f32 - self.margin_px * 2.0;
        (usable / self.viewport_width as f32 * self.fill).max(0.001)
    }

    fn allowed_ndc_y(self) -> f32 {
        let usable = self.viewport_height as f32 - self.margin_px * 2.0;
        (usable / self.viewport_height as f32 * self.fill).max(0.001)
    }
}

pub(super) fn perspective_fit(
    bounds: crate::Aabb,
    camera: PerspectiveCamera,
    options: ValidFramingOptions,
) -> Result<PerspectiveFit, LookupError> {
    validate_bounds(bounds)?;
    let base_target = bounds.center();
    let rotation = look_rotation(-options.view_direction, options.up);
    let inverse_rotation = rotation.inverse();

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for corner in aabb_corners(bounds) {
        let view = inverse_rotation * (corner - base_target);
        min = min.min(view);
        max = max.max(view);
    }

    let half_fov = camera.vertical_fov.radians() * 0.5;
    if !half_fov.is_finite() || half_fov <= 0.0 {
        return Err(LookupError::InvalidFramingOption {
            field: "vertical_fov",
            reason: "perspective camera vertical_fov must be positive",
        });
    }
    let focal = half_fov.tan().recip();

    let view_center = (min + max) * 0.5;
    let mut target = base_target + rotation * Vec3::new(view_center.x, view_center.y, 0.0);
    let mut solve = solve_perspective_distance(bounds, target, inverse_rotation, focal, options);
    for _ in 0..4 {
        let shift = Vec3::new(
            solve.center_ndc_x * solve.distance * options.aspect() / focal,
            solve.center_ndc_y * solve.distance / focal,
            0.0,
        );
        if shift.length_squared() <= 1e-8 {
            break;
        }
        target += rotation * shift;
        solve = solve_perspective_distance(bounds, target, inverse_rotation, focal, options);
    }
    let distance = solve.distance;
    let camera_transform = Transform {
        translation: target + options.view_direction * distance,
        rotation,
        scale: Vec3::ONE,
    };
    let (yaw_radians, pitch_radians) = orbit_angles_from_direction(options.view_direction);
    let depth_radius = (distance - solve.min_z)
        .max(distance + solve.max_z)
        .max(0.01);

    Ok(PerspectiveFit {
        camera_transform,
        target,
        distance,
        yaw_radians,
        pitch_radians,
        depth_radius,
    })
}

#[derive(Debug, Clone, Copy)]
struct PerspectiveDistanceSolve {
    distance: f32,
    min_z: f32,
    max_z: f32,
    center_ndc_x: f32,
    center_ndc_y: f32,
}

fn solve_perspective_distance(
    bounds: crate::Aabb,
    target: Vec3,
    inverse_rotation: Quat,
    focal: f32,
    options: ValidFramingOptions,
) -> PerspectiveDistanceSolve {
    let mut distance: f32 = 0.01;
    let mut min_z: f32 = 0.0;
    let mut max_z: f32 = 0.0;
    let mut views = Vec::with_capacity(8);
    for corner in aabb_corners(bounds) {
        let view = inverse_rotation * (corner - target);
        min_z = min_z.min(view.z);
        max_z = max_z.max(view.z);
        let x_distance =
            view.z + view.x.abs() * focal / (options.aspect() * options.allowed_ndc_x());
        let y_distance = view.z + view.y.abs() * focal / options.allowed_ndc_y();
        distance = distance.max(x_distance).max(y_distance);
        views.push(view);
    }
    distance = distance.max(0.01) * 1.001;

    let mut min_ndc_x = f32::INFINITY;
    let mut max_ndc_x = f32::NEG_INFINITY;
    let mut min_ndc_y = f32::INFINITY;
    let mut max_ndc_y = f32::NEG_INFINITY;
    for view in views {
        let depth = (distance - view.z).max(0.001);
        let ndc_x = view.x * focal / (options.aspect() * depth);
        let ndc_y = view.y * focal / depth;
        min_ndc_x = min_ndc_x.min(ndc_x);
        max_ndc_x = max_ndc_x.max(ndc_x);
        min_ndc_y = min_ndc_y.min(ndc_y);
        max_ndc_y = max_ndc_y.max(ndc_y);
    }

    PerspectiveDistanceSolve {
        distance,
        min_z,
        max_z,
        center_ndc_x: (min_ndc_x + max_ndc_x) * 0.5,
        center_ndc_y: (min_ndc_y + max_ndc_y) * 0.5,
    }
}

fn normalize(value: Vec3) -> Option<Vec3> {
    if !value.is_finite() {
        return None;
    }
    let length = value.length();
    (length > f32::EPSILON && length.is_finite()).then_some(value / length)
}

fn orbit_angles_from_direction(direction: Vec3) -> (f32, f32) {
    let direction = normalize(direction).unwrap_or(Vec3::new(0.0, 0.0, 1.0));
    let yaw = direction.x.atan2(direction.z);
    let pitch = direction.y.clamp(-1.0, 1.0).asin();
    (yaw, pitch)
}
