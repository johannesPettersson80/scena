//! Platform-neutral orbit, pan, fly, and focus controls.

use crate::scene::Vec3;

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
            orbiting: false,
            panning: false,
        }
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
                self.distance = (self.distance * zoom).max(MIN_DISTANCE);
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

    pub const fn target(&self) -> Vec3 {
        self.target
    }

    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub const fn yaw_radians(&self) -> f32 {
        self.yaw_radians
    }

    pub const fn pitch_radians(&self) -> f32 {
        self.pitch_radians
    }
}

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.01;
const PAN_UNITS_PER_PIXEL: f32 = 0.001;
const ZOOM_SCALE: f32 = 0.1;
const MIN_DISTANCE: f32 = 0.001;
const MAX_PITCH_RADIANS: f32 = 1.553_343;
