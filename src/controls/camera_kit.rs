use crate::diagnostics::LookupError;
use crate::scene::{CameraKey, NodeKey, Scene, Transform, Vec3};

use super::{
    MAX_PITCH_RADIANS, MIN_DISTANCE, ORBIT_RADIANS_PER_PIXEL, OrbitControlAction,
    sanitize_distance_limit, sanitize_finite, sanitize_vec3,
};

const DEFAULT_FLY_MOVE_SPEED: f32 = 1.0;
const DEFAULT_FLY_LOOK_RADIANS_PER_PIXEL: f32 = ORBIT_RADIANS_PER_PIXEL;

/// Keeps a camera at a named world-space offset from a scene node.
///
/// `FollowControls` is intentionally platform-neutral: applications decide
/// when to call [`Self::apply_to_scene`], while the controls own the
/// reusable follow-camera math.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FollowControls {
    offset: Vec3,
    target_offset: Vec3,
}

impl FollowControls {
    pub fn new(offset: Vec3) -> Self {
        Self {
            offset: sanitize_vec3(offset, Vec3::new(0.0, 1.0, 3.0)),
            target_offset: Vec3::ZERO,
        }
    }

    /// Places the camera behind the followed target along +Z and above it on +Y.
    pub fn behind_and_above(distance: f32, height: f32) -> Self {
        let distance = sanitize_distance_limit(distance, 3.0);
        let height = sanitize_finite(height, 1.0).max(0.0);
        Self::new(Vec3::new(0.0, height, distance))
    }

    pub fn with_target_offset(mut self, offset: Vec3) -> Self {
        self.target_offset = sanitize_vec3(offset, Vec3::ZERO);
        self
    }

    pub const fn offset(&self) -> Vec3 {
        self.offset
    }

    pub const fn target_offset(&self) -> Vec3 {
        self.target_offset
    }

    pub fn apply_to_scene(
        &self,
        scene: &mut Scene,
        camera: CameraKey,
        target: NodeKey,
    ) -> Result<(), LookupError> {
        let camera_node = scene
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let target_world = scene
            .world_transform(target)
            .ok_or(LookupError::NodeNotFound(target))?;
        let look_at = target_world.translation + self.target_offset;
        let camera_position = look_at + self.offset;

        scene.align_to(camera_node, Transform::at(camera_position))?;
        scene.ensure_camera_depth_reaches(camera, self.offset.length().max(MIN_DISTANCE))?;
        scene.look_at_point(camera, look_at)
    }
}

/// Free-flight camera controls with explicit host-driven movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlyControls {
    position: Vec3,
    yaw_radians: f32,
    pitch_radians: f32,
    move_speed: f32,
    look_speed_radians_per_pixel: f32,
}

impl FlyControls {
    pub fn new(position: Vec3) -> Self {
        Self {
            position: sanitize_vec3(position, Vec3::ZERO),
            yaw_radians: 0.0,
            pitch_radians: 0.0,
            move_speed: DEFAULT_FLY_MOVE_SPEED,
            look_speed_radians_per_pixel: DEFAULT_FLY_LOOK_RADIANS_PER_PIXEL,
        }
    }

    pub fn with_yaw_pitch_degrees(self, yaw_degrees: f32, pitch_degrees: f32) -> Self {
        self.with_yaw_pitch_radians(yaw_degrees.to_radians(), pitch_degrees.to_radians())
    }

    pub fn with_yaw_pitch_radians(mut self, yaw_radians: f32, pitch_radians: f32) -> Self {
        if yaw_radians.is_finite() {
            self.yaw_radians = yaw_radians;
        }
        if pitch_radians.is_finite() {
            self.pitch_radians = pitch_radians.clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
        }
        self
    }

    pub fn with_move_speed(mut self, units_per_second: f32) -> Self {
        self.move_speed = sanitize_distance_limit(units_per_second, DEFAULT_FLY_MOVE_SPEED);
        self
    }

    pub fn with_look_speed(mut self, radians_per_pixel: f32) -> Self {
        self.look_speed_radians_per_pixel =
            sanitize_distance_limit(radians_per_pixel, DEFAULT_FLY_LOOK_RADIANS_PER_PIXEL);
        self
    }

    pub fn move_local(
        &mut self,
        forward: f32,
        right: f32,
        up: f32,
        delta_seconds: f32,
    ) -> OrbitControlAction {
        if !forward.is_finite()
            || !right.is_finite()
            || !up.is_finite()
            || !delta_seconds.is_finite()
            || delta_seconds <= 0.0
        {
            return OrbitControlAction::None;
        }

        let movement = self.forward_vector() * forward + self.right_vector() * right + Vec3::Y * up;
        if movement.length_squared() <= f32::EPSILON {
            return OrbitControlAction::None;
        }

        self.position += movement * self.move_speed * delta_seconds;
        OrbitControlAction::Pan
    }

    pub fn look_delta(&mut self, delta_x: f32, delta_y: f32) -> OrbitControlAction {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return OrbitControlAction::None;
        }

        self.yaw_radians += delta_x * self.look_speed_radians_per_pixel;
        self.pitch_radians = (self.pitch_radians + delta_y * self.look_speed_radians_per_pixel)
            .clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
        OrbitControlAction::Orbit
    }

    pub fn apply_to_scene(&self, scene: &mut Scene, camera: CameraKey) -> Result<(), LookupError> {
        let camera_node = scene
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        scene.align_to(camera_node, Transform::at(self.position))?;
        scene.ensure_camera_depth_reaches(camera, MIN_DISTANCE)?;
        scene.look_at_point(camera, self.position + self.forward_vector())
    }

    pub const fn position(&self) -> Vec3 {
        self.position
    }

    pub const fn yaw_radians(&self) -> f32 {
        self.yaw_radians
    }

    pub const fn pitch_radians(&self) -> f32 {
        self.pitch_radians
    }

    pub const fn move_speed(&self) -> f32 {
        self.move_speed
    }

    pub const fn look_speed_radians_per_pixel(&self) -> f32 {
        self.look_speed_radians_per_pixel
    }

    fn forward_vector(&self) -> Vec3 {
        let pitch_cos = self.pitch_radians.cos();
        Vec3::new(
            self.yaw_radians.sin() * pitch_cos,
            self.pitch_radians.sin(),
            self.yaw_radians.cos() * pitch_cos,
        )
    }

    fn right_vector(&self) -> Vec3 {
        Vec3::new(self.yaw_radians.cos(), 0.0, -self.yaw_radians.sin())
    }
}
