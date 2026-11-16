//! Camera rays for the scene's perspective or orthographic camera, with a
//! thin lens when depth of field is configured.

use glam::Vec3;

use crate::scene::{Camera, CameraKey, Scene, WorldTransform};

use super::super::output::DepthOfFieldConfig;
use super::PathTraceError;
use super::sampling::uniform_disc;

#[derive(Clone, Copy)]
enum Projection {
    /// Half extents of the image plane one unit in front of the camera.
    Perspective { half_width: f32, half_height: f32 },
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
    },
}

pub(super) struct TraceCamera {
    projection: Projection,
    /// Camera axes in world space; the traced scene is measured from the
    /// camera, so its position is the origin.
    axes: WorldTransform,
    near: f32,
    far: f32,
    lens_radius: f32,
    focus_distance: f32,
}

pub(super) struct CameraRay {
    pub(super) origin: Vec3,
    pub(super) direction: Vec3,
    pub(super) max_distance: f32,
}

fn scena_camera(scene: &Scene, camera: CameraKey) -> Option<Camera> {
    scene.camera(camera).cloned()
}

impl TraceCamera {
    /// Returns the camera and its world position.
    pub(super) fn from_scene(
        scene: &Scene,
        camera: CameraKey,
        (width, height): (u32, u32),
        depth_of_field: Option<DepthOfFieldConfig>,
    ) -> Result<(Self, Vec3), PathTraceError> {
        let not_found = PathTraceError::CameraNotFound(camera);
        let desc = scena_camera(scene, camera).ok_or(not_found.clone())?;
        let node = scene.camera_node(camera).ok_or(not_found.clone())?;
        let world = scene.resolved_world(node).ok_or(not_found)?;
        let invalid = |reason: &'static str| PathTraceError::InvalidCamera { camera, reason };
        let (projection, near, far) = match desc {
            Camera::Perspective(perspective) => {
                let target_aspect = width.max(1) as f32 / height.max(1) as f32;
                let aspect = if perspective.aspect.is_finite() && perspective.aspect > 0.0 {
                    perspective.aspect
                } else {
                    target_aspect
                };
                let half_height = (perspective.vertical_fov.radians() * 0.5).tan();
                if !(half_height.is_finite() && half_height > 0.0) {
                    return Err(invalid(
                        "the vertical field of view must lie between 0 and 180 degrees",
                    ));
                }
                (
                    Projection::Perspective {
                        half_width: half_height * aspect,
                        half_height,
                    },
                    perspective.near,
                    perspective.far,
                )
            }
            Camera::Orthographic(orthographic) => {
                let (width, height) = (
                    orthographic.right - orthographic.left,
                    orthographic.top - orthographic.bottom,
                );
                if !(width.abs() > f32::EPSILON && height.abs() > f32::EPSILON) {
                    return Err(invalid("the orthographic volume has no width or height"));
                }
                (
                    Projection::Orthographic {
                        left: orthographic.left,
                        right: orthographic.right,
                        bottom: orthographic.bottom,
                        top: orthographic.top,
                    },
                    orthographic.near,
                    orthographic.far,
                )
            }
        };
        if !(near.is_finite() && far.is_finite() && far > near) {
            return Err(invalid("the far plane must lie beyond the near plane"));
        }
        let axes = world.without_translation();
        let forward = axes.transform_vector(Vec3::NEG_Z).length();
        if forward <= 0.0 || !forward.is_finite() {
            return Err(invalid("the camera's world transform is singular"));
        }
        // A lens of focal length f at f-stop N has an aperture f / N wide.
        let (lens_radius, focus_distance) = depth_of_field.map_or((0.0, 1.0), |config| {
            (
                config.focal_length_mm() * 0.001 / (2.0 * config.aperture_f_stop()),
                config.focus_distance().max(1.0e-3),
            )
        });
        let lens_radius = match projection {
            Projection::Perspective { .. } => lens_radius,
            Projection::Orthographic { .. } => 0.0,
        };
        Ok((
            Self {
                projection,
                axes,
                near: near.max(0.0),
                far,
                lens_radius,
                focus_distance,
            },
            world.translation(),
        ))
    }

    /// The ray through `film`, a position on the image from its top-left
    /// corner in `[0, 1]`, leaving the lens at the point `lens` selects.
    /// Width of one of the image's `rows` of pixels on a surface the camera
    /// faces: per unit of distance from the camera, plus a constant.
    pub(super) fn pixel_footprint(&self, rows: u32) -> (f32, f32) {
        let rows = rows.max(1) as f32;
        match self.projection {
            Projection::Perspective { half_height, .. } => (2.0 * half_height / rows, 0.0),
            Projection::Orthographic { bottom, top, .. } => (0.0, (top - bottom).abs() / rows),
        }
    }

    pub(super) fn ray(&self, film: (f32, f32), lens: (f32, f32)) -> CameraRay {
        let (x, y) = (film.0 * 2.0 - 1.0, 1.0 - film.1 * 2.0);
        // In camera space the ray advances one unit of depth per unit of
        // its parameter, so the near and far planes are parameter values.
        let (origin, direction) = match self.projection {
            Projection::Perspective {
                half_width,
                half_height,
            } => {
                let through = Vec3::new(x * half_width, y * half_height, -1.0);
                if self.lens_radius > 0.0 {
                    let (lens_x, lens_y) = uniform_disc(lens);
                    let origin =
                        Vec3::new(lens_x * self.lens_radius, lens_y * self.lens_radius, 0.0);
                    (
                        origin,
                        (through * self.focus_distance - origin) / self.focus_distance,
                    )
                } else {
                    (Vec3::ZERO, through)
                }
            }
            Projection::Orthographic {
                left,
                right,
                bottom,
                top,
            } => (
                Vec3::new(
                    left + (right - left) * (x * 0.5 + 0.5),
                    bottom + (top - bottom) * (y * 0.5 + 0.5),
                    0.0,
                ),
                Vec3::NEG_Z,
            ),
        };
        let world_direction = self.axes.transform_vector(direction);
        let scale = world_direction.length();
        CameraRay {
            origin: self.axes.transform_point(origin + direction * self.near),
            direction: world_direction / scale,
            max_distance: (self.far - self.near) * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Angle, PerspectiveCamera, Transform};

    fn camera(depth_of_field: Option<DepthOfFieldConfig>) -> (TraceCamera, Vec3) {
        let mut scene = Scene::new();
        let key = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera {
                    vertical_fov: Angle::from_degrees(90.0),
                    aspect: 0.0,
                    near: 0.5,
                    far: 50.0,
                },
                Transform::at(Vec3::new(1.0, 2.0, 3.0)),
            )
            .expect("camera inserts");
        TraceCamera::from_scene(&scene, key, (200, 100), depth_of_field).expect("camera resolves")
    }

    #[test]
    fn pinhole_rays_cover_the_field_of_view_and_start_at_the_near_plane() {
        let (camera, position) = camera(None);
        assert_eq!(position, Vec3::new(1.0, 2.0, 3.0));
        let center = camera.ray((0.5, 0.5), (0.5, 0.5));
        assert!((center.direction - Vec3::NEG_Z).length() < 1.0e-6);
        assert!((center.origin - Vec3::new(0.0, 0.0, -0.5)).length() < 1.0e-6);
        assert!((center.max_distance - 49.5).abs() < 1.0e-4);
        // A 90 degree vertical view on a 2:1 target reaches 45 degrees up
        // and twice that tangent sideways.
        let top = camera.ray((0.5, 0.0), (0.5, 0.5)).direction;
        assert!((top.y / -top.z - 1.0).abs() < 1.0e-5 && top.x.abs() < 1.0e-6);
        let right = camera.ray((1.0, 0.5), (0.5, 0.5)).direction;
        assert!((right.x / -right.z - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn lens_rays_converge_on_the_focus_plane() {
        let config = DepthOfFieldConfig::physical(4.0, 50.0, 24.0, 2.0, 9, 8);
        let (camera, _) = camera(Some(config));
        let film = (0.7, 0.3);
        let reference = camera.ray(film, (0.0, 0.0));
        let focus_depth = 4.0 - 0.5;
        let at_focus =
            |ray: &CameraRay| ray.origin + ray.direction * (focus_depth / -ray.direction.z);
        let mut spread = 0.0_f32;
        for lens in [(0.9, 0.1), (0.9, 0.6), (0.4, 0.85)] {
            let ray = camera.ray(film, lens);
            assert!((at_focus(&ray) - at_focus(&reference)).length() < 1.0e-4);
            spread = spread.max((ray.origin - reference.origin).length());
        }
        // f/2 at 50 mm is a 25 mm aperture.
        assert!(spread > 0.005 && spread < 0.03, "lens spread {spread}");
    }
}
