use std::error::Error as StdError;
use std::f32::consts::{PI, TAU};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::OrbitControls;
use crate::geometry::Aabb;
use crate::scene::{FramingOutcome, Vec3};

/// Reusable orbit-camera state shared by controls, bookmarks, and SceneHost.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    pub target: Vec3,
    pub distance: f32,
    pub yaw_radians: f32,
    pub pitch_radians: f32,
}

/// Named camera state for tours, product views, and host-defined viewpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraBookmark {
    pub name: String,
    pub state: CameraState,
    pub target_bounds: Option<Aabb>,
    pub description: Option<String>,
}

/// Easing curve for explicit host-ticked visual transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransitionEasing {
    #[default]
    Linear,
    EaseInOut,
}

/// Host-ticked camera fly-to transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFlyTo {
    start: CameraState,
    target: CameraState,
    elapsed_seconds: f32,
    duration_seconds: f32,
    easing: TransitionEasing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CameraTransitionError {
    InvalidCameraState { message: &'static str },
    InvalidDurationSeconds,
}

impl CameraState {
    pub fn from_controls(controls: &OrbitControls) -> Self {
        Self {
            target: controls.target(),
            distance: controls.distance(),
            yaw_radians: controls.yaw_radians(),
            pitch_radians: controls.pitch_radians(),
        }
    }

    pub const fn from_framing(framing: FramingOutcome) -> Self {
        Self {
            target: framing.target,
            distance: framing.distance,
            yaw_radians: framing.yaw_radians,
            pitch_radians: framing.pitch_radians,
        }
    }

    pub fn validate(self) -> Result<(), &'static str> {
        if !self.target.to_array().into_iter().all(f32::is_finite) {
            return Err("camera target must contain finite values");
        }
        if !self.distance.is_finite() || self.distance <= 0.0 {
            return Err("camera distance must be finite and greater than zero");
        }
        if !self.yaw_radians.is_finite() {
            return Err("camera yaw must be finite");
        }
        if !self.pitch_radians.is_finite() {
            return Err("camera pitch must be finite");
        }
        Ok(())
    }

    pub fn into_controls(self) -> OrbitControls {
        OrbitControls::new(self.target, self.distance)
            .with_angles(self.yaw_radians, self.pitch_radians)
    }

    pub fn interpolate_to(self, target: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let mix = |left: f32, right: f32| left + (right - left) * amount;
        let yaw_delta = shortest_angle_delta(self.yaw_radians, target.yaw_radians);
        Self {
            target: self.target.lerp(target.target, amount),
            distance: mix(self.distance, target.distance),
            yaw_radians: self.yaw_radians + yaw_delta * amount,
            pitch_radians: mix(self.pitch_radians, target.pitch_radians),
        }
    }
}

impl CameraBookmark {
    pub fn new(name: impl Into<String>, state: CameraState) -> Self {
        Self {
            name: name.into(),
            state,
            target_bounds: None,
            description: None,
        }
    }

    pub fn from_framing(name: impl Into<String>, framing: FramingOutcome) -> Self {
        Self::new(name, CameraState::from_framing(framing))
    }

    pub fn with_target_bounds(mut self, bounds: Aabb) -> Self {
        self.target_bounds = Some(bounds);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> CameraState {
        self.state
    }

    pub const fn target_bounds(&self) -> Option<Aabb> {
        self.target_bounds
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl OrbitControls {
    pub fn camera_state(&self) -> CameraState {
        CameraState::from_controls(self)
    }

    pub fn fly_to(
        &self,
        target: CameraState,
        easing: TransitionEasing,
        duration_seconds: f64,
    ) -> Result<CameraFlyTo, CameraTransitionError> {
        let start = self.camera_state();
        start
            .validate()
            .map_err(|message| CameraTransitionError::InvalidCameraState { message })?;
        target
            .validate()
            .map_err(|message| CameraTransitionError::InvalidCameraState { message })?;
        if !duration_seconds.is_finite() || duration_seconds < 0.0 {
            return Err(CameraTransitionError::InvalidDurationSeconds);
        }
        let duration_seconds = duration_seconds.min(f32::MAX as f64) as f32;
        let elapsed_seconds = if duration_seconds == 0.0 || start == target {
            duration_seconds
        } else {
            0.0
        };
        Ok(CameraFlyTo {
            start,
            target,
            elapsed_seconds,
            duration_seconds,
            easing,
        })
    }
}

impl CameraFlyTo {
    pub fn advance(&mut self, delta_seconds: f32) -> OrbitControls {
        if delta_seconds.is_finite() && delta_seconds > 0.0 {
            self.elapsed_seconds =
                (self.elapsed_seconds + delta_seconds).min(self.duration_seconds);
        }
        self.sample()
    }

    pub fn sample(&self) -> OrbitControls {
        self.sample_state().into_controls()
    }

    pub fn sample_state(&self) -> CameraState {
        if self.is_complete() {
            return self.target;
        }
        let amount = eased_amount(
            self.elapsed_seconds / self.duration_seconds.max(f32::EPSILON),
            self.easing,
        );
        self.start.interpolate_to(self.target, amount)
    }

    pub fn is_complete(&self) -> bool {
        self.elapsed_seconds >= self.duration_seconds
    }
}

impl fmt::Display for CameraTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCameraState { message } => write!(formatter, "{message}"),
            Self::InvalidDurationSeconds => {
                write!(
                    formatter,
                    "camera transition duration_seconds must be finite and non-negative"
                )
            }
        }
    }
}

impl StdError for CameraTransitionError {}

pub(crate) fn eased_amount(amount: f32, easing: TransitionEasing) -> f32 {
    let amount = amount.clamp(0.0, 1.0);
    match easing {
        TransitionEasing::Linear => amount,
        TransitionEasing::EaseInOut => {
            if amount < 0.5 {
                4.0 * amount * amount * amount
            } else {
                1.0 - (-2.0 * amount + 2.0).powi(3) / 2.0
            }
        }
    }
}

fn shortest_angle_delta(start: f32, target: f32) -> f32 {
    let delta = (target - start).rem_euclid(TAU);
    if delta > PI { delta - TAU } else { delta }
}
