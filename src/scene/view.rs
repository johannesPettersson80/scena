use crate::diagnostics::LookupError;
use crate::geometry::Aabb;

use super::{Camera, CameraKey, NodeKey, NodeKind, Quat, Scene, Transform, Vec3};

impl Scene {
    /// Returns the scene node that owns a camera descriptor.
    pub fn camera_node(&self, camera: CameraKey) -> Option<NodeKey> {
        self.nodes.iter().find_map(|(node_key, node)| {
            if node.kind == NodeKind::Camera(camera) {
                Some(node_key)
            } else {
                None
            }
        })
    }

    /// Frames bounds with the selected camera and tightens the camera depth range.
    pub fn frame(&mut self, camera: CameraKey, bounds: Aabb) -> Result<(), LookupError> {
        let camera_node = self
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let center = bounds.center();
        let radius = bounds.bounding_sphere_radius().max(MIN_FRAME_RADIUS);
        let camera_descriptor = self
            .cameras
            .get_mut(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;

        let transform = match camera_descriptor {
            Camera::Perspective(camera) => {
                let half_vertical_fov = camera.vertical_fov.radians() * 0.5;
                let half_horizontal_fov =
                    (half_vertical_fov.tan() * camera.aspect.max(0.001)).atan();
                let limiting_half_fov = half_vertical_fov.min(half_horizontal_fov).max(0.001);
                let distance = radius / limiting_half_fov.tan() * FRAME_PADDING;
                let depth_radius = radius * FRAME_PADDING;
                let depth = super::DepthRange::fit_sphere(distance, depth_radius);
                camera.near = depth.near();
                camera.far = depth.far();
                Transform {
                    translation: Vec3::new(center.x, center.y, center.z + distance),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                }
            }
            Camera::Orthographic(camera) => {
                let half = bounds.half_extent();
                let half_width = half.x.max(radius) * FRAME_PADDING;
                let half_height = half.y.max(radius) * FRAME_PADDING;
                let distance = (radius * FRAME_PADDING).max(1.0);
                let depth = super::DepthRange::fit_sphere(distance, radius * FRAME_PADDING);
                camera.left = -half_width;
                camera.right = half_width;
                camera.bottom = -half_height;
                camera.top = half_height;
                camera.near = depth.near();
                camera.far = depth.far();
                Transform {
                    translation: Vec3::new(center.x, center.y, center.z + distance),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                }
            }
        };

        self.set_node_transform_and_mark_changed(camera_node, transform)
    }

    /// Rotates the selected camera node so its local -Z axis points at `target`.
    pub fn look_at(&mut self, camera: CameraKey, target: NodeKey) -> Result<(), LookupError> {
        let camera_node = self
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        if !self.cameras.contains_key(camera) {
            return Err(LookupError::CameraNotFound(camera));
        }
        let target_position = self
            .nodes
            .get(target)
            .ok_or(LookupError::NodeNotFound(target))?
            .transform
            .translation;
        let mut camera_transform = self
            .nodes
            .get(camera_node)
            .ok_or(LookupError::CameraNotFound(camera))?
            .transform;
        let forward = normalize_or(
            subtract_vec3(target_position, camera_transform.translation),
            Vec3::new(0.0, 0.0, -1.0),
        );

        camera_transform.rotation = look_rotation(forward, Vec3::new(0.0, 1.0, 0.0));
        self.set_node_transform_and_mark_changed(camera_node, camera_transform)
    }

    fn set_node_transform_and_mark_changed(
        &mut self,
        node: NodeKey,
        transform: Transform,
    ) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.transform != transform {
            node.transform = transform;
            self.structure_revision = self.structure_revision.saturating_add(1);
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }
}

const FRAME_PADDING: f32 = 1.15;
const MIN_FRAME_RADIUS: f32 = 0.5;

fn look_rotation(forward: Vec3, up: Vec3) -> Quat {
    let right = normalize_or(cross_vec3(forward, up), Vec3::new(1.0, 0.0, 0.0));
    let up = cross_vec3(right, forward);
    quat_from_basis(right, up, scale_vec3(forward, -1.0))
}

fn quat_from_basis(right: Vec3, up: Vec3, back: Vec3) -> Quat {
    let trace = right.x + up.y + back.z;
    let quat = if trace > 0.0 {
        let scale = (trace + 1.0).sqrt() * 2.0;
        Quat {
            w: 0.25 * scale,
            x: (up.z - back.y) / scale,
            y: (back.x - right.z) / scale,
            z: (right.y - up.x) / scale,
        }
    } else if right.x > up.y && right.x > back.z {
        let scale = (1.0 + right.x - up.y - back.z).sqrt() * 2.0;
        Quat {
            w: (up.z - back.y) / scale,
            x: 0.25 * scale,
            y: (up.x + right.y) / scale,
            z: (back.x + right.z) / scale,
        }
    } else if up.y > back.z {
        let scale = (1.0 + up.y - right.x - back.z).sqrt() * 2.0;
        Quat {
            w: (back.x - right.z) / scale,
            x: (up.x + right.y) / scale,
            y: 0.25 * scale,
            z: (back.y + up.z) / scale,
        }
    } else {
        let scale = (1.0 + back.z - right.x - up.y).sqrt() * 2.0;
        Quat {
            w: (right.y - up.x) / scale,
            x: (back.x + right.z) / scale,
            y: (back.y + up.z) / scale,
            z: 0.25 * scale,
        }
    };
    normalize_quat(quat)
}

fn normalize_quat(quat: Quat) -> Quat {
    let length = (quat.x * quat.x + quat.y * quat.y + quat.z * quat.z + quat.w * quat.w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Quat::IDENTITY
    } else {
        Quat {
            x: quat.x / length,
            y: quat.y / length,
            z: quat.z / length,
            w: quat.w / length,
        }
    }
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        scale_vec3(value, 1.0 / length)
    }
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}
