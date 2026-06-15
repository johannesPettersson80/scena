//! Platform-neutral orbit, pan, fly, and focus controls.

use std::f32::consts::TAU;

mod camera_kit;
mod camera_transition;
mod gizmo;
mod url_state;
pub use camera_kit::{FlyControls, FollowControls};
#[cfg(feature = "scene-host")]
pub(crate) use camera_transition::eased_amount;
pub use camera_transition::{
    CameraBookmark, CameraFlyTo, CameraState, CameraTransitionError, TransitionEasing,
};
pub use gizmo::{
    GizmoAxis, GizmoConstraint, GizmoMode, GizmoRay, GizmoSpace, TransformGizmo,
    TransformGizmoHelpers,
};
pub use url_state::{CameraOrbitUrlState, CameraOrbitUrlStateError};

use crate::diagnostics::LookupError;
use crate::scene::FramingOutcome;
use crate::scene::Vec3;
use crate::scene::{CameraKey, Scene, Transform};

const CINEMATIC_DAMPING: f32 = 0.18;
const PRESENTATION_DAMPING: f32 = 0.12;
const SNAPPY_DAMPING: f32 = 0.04;
const PRESENTATION_RPM: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PointerEventKind {
    Pressed,
    Released,
    Moved,
    Wheel,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    pub position: (f32, f32),
    pub button: Option<PointerButton>,
    pub delta: (f32, f32),
    pub scroll_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TouchEventKind {
    Started,
    Moved,
    Pinched,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchEvent {
    pub kind: TouchEventKind,
    pub position: (f32, f32),
    pub delta: (f32, f32),
    pub pinch_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OrbitControlAction {
    None,
    BeginOrbit,
    Orbit,
    Pan,
    Zoom,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitControls {
    target: Vec3,
    distance: f32,
    yaw_radians: f32,
    pitch_radians: f32,
    damping_factor: f32,
    auto_rotate_rpm: f32,
    min_distance: f32,
    max_distance: f32,
    orbiting: bool,
    panning: bool,
}

impl OrbitControls {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            distance: distance.max(MIN_DISTANCE),
            yaw_radians: 0.0,
            pitch_radians: 0.0,
            damping_factor: 0.0,
            auto_rotate_rpm: 0.0,
            min_distance: MIN_DISTANCE,
            max_distance: f32::INFINITY,
            orbiting: false,
            panning: false,
        }
    }

    pub fn focus(mut self, target: Vec3, distance: f32) -> Self {
        self.target = target;
        self.distance = self.clamp_distance(distance.max(MIN_DISTANCE));
        self
    }

    /// Creates orbit controls from a [`Scene::frame_bounds`](crate::Scene::frame_bounds) result.
    pub fn from_framing(framing: FramingOutcome) -> Self {
        Self::new(framing.target, framing.distance).focus_on_framing(framing)
    }

    /// Adopts the target, distance, yaw, and pitch computed by
    /// [`Scene::frame_bounds`](crate::Scene::frame_bounds).
    pub fn focus_on_framing(mut self, framing: FramingOutcome) -> Self {
        self.target = framing.target;
        self.distance = self.clamp_distance(framing.distance.max(MIN_DISTANCE));
        self.yaw_radians = framing.yaw_radians;
        self.pitch_radians = framing
            .pitch_radians
            .clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
        self
    }

    /// Sets zoom limits relative to the current framed distance.
    ///
    /// `min_factor` and `max_factor` multiply the current distance. The method
    /// is designed for the common `OrbitControls::from_framing(framing)` path:
    /// choose how close and far the user can zoom relative to the initial
    /// composition instead of hard-coding scene distances.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{OrbitControls, Vec3};
    ///
    /// let controls = OrbitControls::new(Vec3::ZERO, 2.0)
    ///     .zoom_limits_bounds_relative(0.5, 4.0);
    /// assert!(controls.min_distance() <= controls.distance());
    /// assert!(controls.max_distance() >= controls.distance());
    /// ```
    pub fn zoom_limits_bounds_relative(self, min_factor: f32, max_factor: f32) -> Self {
        let base_distance = self.distance.max(MIN_DISTANCE);
        self.with_distance_limits(base_distance * min_factor, base_distance * max_factor)
    }

    /// Sets absolute scene-unit distance limits for wheel and pinch zoom.
    pub fn with_distance_limits(mut self, min_distance: f32, max_distance: f32) -> Self {
        let min_distance = sanitize_distance_limit(min_distance, MIN_DISTANCE);
        let max_distance = sanitize_distance_limit(max_distance, f32::INFINITY);
        let (min_distance, max_distance) = if min_distance <= max_distance {
            (min_distance, max_distance)
        } else {
            (max_distance, min_distance)
        };
        self.min_distance = min_distance.max(MIN_DISTANCE);
        self.max_distance = max_distance.max(self.min_distance);
        self.distance = self.clamp_distance(self.distance);
        self
    }

    pub fn with_damping(mut self, factor: f32) -> Self {
        self.damping_factor = if factor.is_finite() {
            factor.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }

    /// Applies a slow, high-damping orbit feel for product-viewer scenes.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{OrbitControls, Vec3};
    ///
    /// let controls = OrbitControls::new(Vec3::ZERO, 2.0).cinematic();
    /// assert!(controls.damping_factor() > 0.0);
    /// ```
    pub fn cinematic(self) -> Self {
        self.with_damping(CINEMATIC_DAMPING)
    }

    /// Applies a light-damping orbit feel for direct manipulation.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{OrbitControls, Vec3};
    ///
    /// let controls = OrbitControls::new(Vec3::ZERO, 2.0).snappy();
    /// assert!(controls.damping_factor() > 0.0);
    /// ```
    pub fn snappy(self) -> Self {
        self.with_damping(SNAPPY_DAMPING)
    }

    /// Applies medium damping plus a slow turntable auto-rotate.
    ///
    /// Hosts advance the turntable explicitly with [`Self::advance`] before
    /// applying the controls to the scene camera.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{OrbitControls, Vec3};
    ///
    /// let mut controls = OrbitControls::new(Vec3::ZERO, 2.0).presentation();
    /// controls.advance(1.0 / 60.0);
    /// ```
    pub fn presentation(self) -> Self {
        self.with_damping(PRESENTATION_DAMPING)
            .turntable(PRESENTATION_RPM)
    }

    /// Sets the auto-rotate speed in revolutions per minute.
    ///
    /// Negative values rotate in the opposite direction. Non-finite values
    /// disable auto-rotate.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{OrbitControls, Vec3};
    ///
    /// let mut controls = OrbitControls::new(Vec3::ZERO, 2.0).turntable(6.0);
    /// controls.advance(1.0 / 60.0);
    /// ```
    pub fn turntable(mut self, rpm: f32) -> Self {
        self.auto_rotate_rpm = if rpm.is_finite() {
            rpm.clamp(-120.0, 120.0)
        } else {
            0.0
        };
        self
    }

    /// Advances turntable auto-rotation by the provided frame duration.
    ///
    /// Returns [`OrbitControlAction::Orbit`] when the yaw changed, so host
    /// loops can call [`Self::apply_to_scene`] and schedule a redraw. User
    /// pointer or touch interaction temporarily owns the orbit state, so
    /// auto-rotate is skipped while the controls are actively orbiting or
    /// panning.
    pub fn advance(&mut self, delta_seconds: f32) -> OrbitControlAction {
        if !delta_seconds.is_finite()
            || delta_seconds <= 0.0
            || self.auto_rotate_rpm == 0.0
            || self.orbiting
            || self.panning
        {
            return OrbitControlAction::None;
        }
        self.yaw_radians += self.auto_rotate_radians_per_second() * delta_seconds;
        OrbitControlAction::Orbit
    }

    pub fn with_angles(mut self, yaw_radians: f32, pitch_radians: f32) -> Self {
        if yaw_radians.is_finite() {
            self.yaw_radians = yaw_radians;
        }
        if pitch_radians.is_finite() {
            self.pitch_radians = pitch_radians.clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
        }
        self
    }

    pub fn handle_pointer(&mut self, event: PointerEvent) -> OrbitControlAction {
        match event.kind {
            PointerEventKind::Pressed => match event.button {
                Some(PointerButton::Primary) => {
                    self.orbiting = true;
                    OrbitControlAction::BeginOrbit
                }
                Some(PointerButton::Secondary) => {
                    self.panning = true;
                    OrbitControlAction::Pan
                }
                Some(PointerButton::Auxiliary) | None => OrbitControlAction::None,
            },
            PointerEventKind::Moved if self.orbiting => {
                self.yaw_radians += event.delta.0 * ORBIT_RADIANS_PER_PIXEL;
                self.pitch_radians = (self.pitch_radians + event.delta.1 * ORBIT_RADIANS_PER_PIXEL)
                    .clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
                OrbitControlAction::Orbit
            }
            PointerEventKind::Moved if self.panning => {
                self.target.x -= event.delta.0 * PAN_UNITS_PER_PIXEL * self.distance;
                self.target.y += event.delta.1 * PAN_UNITS_PER_PIXEL * self.distance;
                OrbitControlAction::Pan
            }
            PointerEventKind::Wheel => {
                let zoom = (1.0 + event.scroll_delta * ZOOM_SCALE).max(0.05);
                self.distance = self.clamp_distance((self.distance * zoom).max(MIN_DISTANCE));
                OrbitControlAction::Zoom
            }
            PointerEventKind::Released | PointerEventKind::Cancelled => {
                self.orbiting = false;
                self.panning = false;
                OrbitControlAction::End
            }
            PointerEventKind::Moved => OrbitControlAction::None,
        }
    }

    pub fn handle_touch(&mut self, event: TouchEvent) -> OrbitControlAction {
        match event.kind {
            TouchEventKind::Started => {
                self.orbiting = true;
                OrbitControlAction::BeginOrbit
            }
            TouchEventKind::Moved if self.orbiting => {
                self.apply_orbit_delta(event.delta);
                OrbitControlAction::Orbit
            }
            TouchEventKind::Pinched => {
                self.apply_zoom_delta(event.pinch_delta);
                OrbitControlAction::Zoom
            }
            TouchEventKind::Ended | TouchEventKind::Cancelled => {
                self.orbiting = false;
                self.panning = false;
                OrbitControlAction::End
            }
            TouchEventKind::Moved => OrbitControlAction::None,
        }
    }

    pub const fn target(&self) -> Vec3 {
        self.target
    }

    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub const fn min_distance(&self) -> f32 {
        self.min_distance
    }

    pub const fn max_distance(&self) -> f32 {
        self.max_distance
    }

    pub const fn yaw_radians(&self) -> f32 {
        self.yaw_radians
    }

    pub const fn pitch_radians(&self) -> f32 {
        self.pitch_radians
    }

    pub const fn damping_factor(&self) -> f32 {
        self.damping_factor
    }

    pub const fn auto_rotate_rpm(&self) -> f32 {
        self.auto_rotate_rpm
    }

    pub fn auto_rotate_radians_per_second(&self) -> f32 {
        self.auto_rotate_rpm * TAU / 60.0
    }

    pub fn apply_to_scene(&self, scene: &mut Scene, camera: CameraKey) -> Result<(), LookupError> {
        let camera_node = scene
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let offset = self.camera_offset();
        scene.align_to(
            camera_node,
            Transform::at(Vec3::new(
                self.target.x + offset.x,
                self.target.y + offset.y,
                self.target.z + offset.z,
            )),
        )?;
        scene.ensure_camera_depth_reaches(camera, self.distance)?;
        scene.look_at_point(camera, self.target)
    }

    fn camera_offset(&self) -> Vec3 {
        let pitch_cos = self.pitch_radians.cos();
        Vec3::new(
            self.distance * self.yaw_radians.sin() * pitch_cos,
            self.distance * self.pitch_radians.sin(),
            self.distance * self.yaw_radians.cos() * pitch_cos,
        )
    }

    fn apply_orbit_delta(&mut self, delta: (f32, f32)) {
        self.yaw_radians += delta.0 * ORBIT_RADIANS_PER_PIXEL;
        self.pitch_radians = (self.pitch_radians + delta.1 * ORBIT_RADIANS_PER_PIXEL)
            .clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
    }

    fn apply_zoom_delta(&mut self, delta: f32) {
        let zoom = (1.0 + delta * ZOOM_SCALE).max(0.05);
        self.distance = self.clamp_distance((self.distance * zoom).max(MIN_DISTANCE));
    }

    fn clamp_distance(&self, distance: f32) -> f32 {
        distance.clamp(self.min_distance, self.max_distance)
    }
}

impl PointerEvent {
    pub const fn primary_pressed(x: f32, y: f32) -> Self {
        Self::pressed(x, y, PointerButton::Primary)
    }

    pub const fn secondary_pressed(x: f32, y: f32) -> Self {
        Self::pressed(x, y, PointerButton::Secondary)
    }

    pub const fn released(x: f32, y: f32) -> Self {
        Self {
            kind: PointerEventKind::Released,
            position: (x, y),
            button: None,
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        }
    }

    pub const fn moved(x: f32, y: f32, delta_x: f32, delta_y: f32) -> Self {
        Self {
            kind: PointerEventKind::Moved,
            position: (x, y),
            button: None,
            delta: (delta_x, delta_y),
            scroll_delta: 0.0,
        }
    }

    pub const fn wheel(x: f32, y: f32, scroll_delta: f32) -> Self {
        Self {
            kind: PointerEventKind::Wheel,
            position: (x, y),
            button: None,
            delta: (0.0, 0.0),
            scroll_delta,
        }
    }

    const fn pressed(x: f32, y: f32, button: PointerButton) -> Self {
        Self {
            kind: PointerEventKind::Pressed,
            position: (x, y),
            button: Some(button),
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        }
    }
}

impl TouchEvent {
    pub const fn start(x: f32, y: f32) -> Self {
        Self {
            kind: TouchEventKind::Started,
            position: (x, y),
            delta: (0.0, 0.0),
            pinch_delta: 0.0,
        }
    }

    pub const fn move_by(x: f32, y: f32, delta_x: f32, delta_y: f32) -> Self {
        Self {
            kind: TouchEventKind::Moved,
            position: (x, y),
            delta: (delta_x, delta_y),
            pinch_delta: 0.0,
        }
    }

    pub const fn pinch(x: f32, y: f32, pinch_delta: f32) -> Self {
        Self {
            kind: TouchEventKind::Pinched,
            position: (x, y),
            delta: (0.0, 0.0),
            pinch_delta,
        }
    }

    pub const fn end(x: f32, y: f32) -> Self {
        Self {
            kind: TouchEventKind::Ended,
            position: (x, y),
            delta: (0.0, 0.0),
            pinch_delta: 0.0,
        }
    }

    pub const fn cancel(x: f32, y: f32) -> Self {
        Self {
            kind: TouchEventKind::Cancelled,
            position: (x, y),
            delta: (0.0, 0.0),
            pinch_delta: 0.0,
        }
    }
}

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.01;
const PAN_UNITS_PER_PIXEL: f32 = 0.001;
const ZOOM_SCALE: f32 = 0.1;
const MIN_DISTANCE: f32 = 0.001;
const MAX_PITCH_RADIANS: f32 = 1.553_343;
fn sanitize_distance_limit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn sanitize_finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn sanitize_vec3(value: Vec3, fallback: Vec3) -> Vec3 {
    if value.is_finite() { value } else { fallback }
}
