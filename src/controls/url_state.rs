use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{MIN_DISTANCE, OrbitControls};
use crate::scene::{FramingOutcome, Vec3};

const CAMERA_ORBIT_PARAM: &str = "camera-orbit";
const CAMERA_TARGET_PARAM: &str = "camera-target";
const TARGET_ZERO_EPSILON: f32 = 1.0e-6;

type DecodedQueryPair<'a> = (Cow<'a, str>, Cow<'a, str>);

/// Shareable orbit-camera state encoded with model-viewer-style query keys.
///
/// The URL form intentionally serializes only camera state. Asset paths,
/// source URLs, tokens, and arbitrary query parameters are ignored on parse
/// and never emitted on encode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraOrbitUrlState {
    yaw_degrees: f32,
    pitch_degrees: f32,
    distance: f32,
    target: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CameraOrbitUrlStateError {
    MissingCameraOrbit,
    InvalidPercentEncoding { component: String },
    InvalidCameraOrbit { value: String },
    InvalidCameraTarget { value: String },
    InvalidField { field: &'static str },
}

impl CameraOrbitUrlState {
    pub fn new(
        yaw_degrees: f32,
        pitch_degrees: f32,
        distance: f32,
    ) -> Result<Self, CameraOrbitUrlStateError> {
        Self::with_target(yaw_degrees, pitch_degrees, distance, Vec3::ZERO)
    }

    pub fn with_target(
        yaw_degrees: f32,
        pitch_degrees: f32,
        distance: f32,
        target: Vec3,
    ) -> Result<Self, CameraOrbitUrlStateError> {
        validate_finite("yaw_degrees", yaw_degrees)?;
        validate_finite("pitch_degrees", pitch_degrees)?;
        validate_distance(distance)?;
        validate_target(target)?;

        Ok(Self {
            yaw_degrees,
            pitch_degrees,
            distance,
            target: target.to_array(),
        })
    }

    pub const fn yaw_degrees(self) -> f32 {
        self.yaw_degrees
    }

    pub const fn pitch_degrees(self) -> f32 {
        self.pitch_degrees
    }

    pub const fn distance(self) -> f32 {
        self.distance
    }

    pub fn target(self) -> Vec3 {
        Vec3::from_array(self.target)
    }

    /// Parses a full URL, query string, or fragment and extracts only
    /// `camera-orbit` plus optional `camera-target`.
    pub fn from_url_query(input: &str) -> Result<Self, CameraOrbitUrlStateError> {
        let mut camera_orbit = None;
        let mut camera_target = None;

        for (key, value) in query_pairs(input)? {
            match key.as_ref() {
                CAMERA_ORBIT_PARAM => camera_orbit = Some(value.into_owned()),
                CAMERA_TARGET_PARAM => camera_target = Some(value.into_owned()),
                _ => {}
            }
        }

        let camera_orbit = camera_orbit.ok_or(CameraOrbitUrlStateError::MissingCameraOrbit)?;
        let (yaw_degrees, pitch_degrees, distance) = parse_camera_orbit(&camera_orbit)?;
        let target = match camera_target {
            Some(value) => parse_camera_target(&value)?,
            None => Vec3::ZERO,
        };

        Self::with_target(yaw_degrees, pitch_degrees, distance, target)
    }

    /// Encodes state as a shareable query string using concrete
    /// model-viewer-style units: `camera-orbit=<yaw>deg <pitch>deg <distance>m`.
    pub fn to_query_string(self) -> String {
        let mut query = format!(
            "?{CAMERA_ORBIT_PARAM}={}",
            urlencoding::encode(&self.camera_orbit_value())
        );
        if let Some(target) = self.camera_target_value() {
            query.push('&');
            query.push_str(CAMERA_TARGET_PARAM);
            query.push('=');
            query.push_str(&urlencoding::encode(&target));
        }
        query
    }

    pub fn camera_orbit_value(self) -> String {
        format!(
            "{}deg {}deg {}m",
            format_scalar(self.yaw_degrees),
            format_scalar(self.pitch_degrees),
            format_scalar(self.distance)
        )
    }

    pub fn camera_target_value(self) -> Option<String> {
        let target = self.target();
        if target.abs_diff_eq(Vec3::ZERO, TARGET_ZERO_EPSILON) {
            return None;
        }
        Some(format!(
            "{}m {}m {}m",
            format_scalar(target.x),
            format_scalar(target.y),
            format_scalar(target.z)
        ))
    }
}

impl OrbitControls {
    pub fn url_state(&self) -> CameraOrbitUrlState {
        CameraOrbitUrlState {
            yaw_degrees: finite_or(self.yaw_radians.to_degrees(), 0.0),
            pitch_degrees: finite_or(self.pitch_radians.to_degrees(), 0.0),
            distance: sanitize_distance(self.distance),
            target: sanitize_vec3(self.target).to_array(),
        }
    }

    pub fn with_url_state(
        self,
        state: CameraOrbitUrlState,
    ) -> Result<Self, CameraOrbitUrlStateError> {
        let state = CameraOrbitUrlState::with_target(
            state.yaw_degrees,
            state.pitch_degrees,
            state.distance,
            state.target(),
        )?;
        Ok(self.focus(state.target(), state.distance).with_angles(
            state.yaw_degrees.to_radians(),
            state.pitch_degrees.to_radians(),
        ))
    }
}

impl FramingOutcome {
    pub fn url_state(&self) -> CameraOrbitUrlState {
        CameraOrbitUrlState {
            yaw_degrees: finite_or(self.yaw_radians.to_degrees(), 0.0),
            pitch_degrees: finite_or(self.pitch_radians.to_degrees(), 0.0),
            distance: sanitize_distance(self.distance),
            target: sanitize_vec3(self.target).to_array(),
        }
    }
}

impl fmt::Display for CameraOrbitUrlStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCameraOrbit => write!(formatter, "missing camera-orbit query parameter"),
            Self::InvalidPercentEncoding { component } => {
                write!(formatter, "invalid percent encoding in '{component}'")
            }
            Self::InvalidCameraOrbit { value } => {
                write!(formatter, "invalid camera-orbit value '{value}'")
            }
            Self::InvalidCameraTarget { value } => {
                write!(formatter, "invalid camera-target value '{value}'")
            }
            Self::InvalidField { field } => {
                write!(formatter, "invalid {field} in URL camera state")
            }
        }
    }
}

impl StdError for CameraOrbitUrlStateError {}

fn query_pairs(input: &str) -> Result<Vec<DecodedQueryPair<'_>>, CameraOrbitUrlStateError> {
    let query = extract_query(input);
    let mut pairs = Vec::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_component(raw_key)?;
        let value = decode_component(raw_value)?;
        pairs.push((key, value));
    }
    Ok(pairs)
}

fn extract_query(input: &str) -> &str {
    let trimmed = input.trim();
    let without_prefix = trimmed
        .split_once('?')
        .map_or(trimmed, |(_, query)| query)
        .strip_prefix('#')
        .unwrap_or_else(|| {
            trimmed
                .strip_prefix('?')
                .or_else(|| trimmed.strip_prefix('#'))
                .unwrap_or(trimmed)
        });
    without_prefix.split('#').next().unwrap_or(without_prefix)
}

fn decode_component(value: &str) -> Result<Cow<'_, str>, CameraOrbitUrlStateError> {
    urlencoding::decode(value).map_err(|_| CameraOrbitUrlStateError::InvalidPercentEncoding {
        component: value.to_string(),
    })
}

fn parse_camera_orbit(value: &str) -> Result<(f32, f32, f32), CameraOrbitUrlStateError> {
    let parts =
        split_triplet(value).ok_or_else(|| CameraOrbitUrlStateError::InvalidCameraOrbit {
            value: value.to_string(),
        })?;
    let yaw_degrees = parse_angle_degrees(parts[0]).ok_or_else(|| {
        CameraOrbitUrlStateError::InvalidCameraOrbit {
            value: value.to_string(),
        }
    })?;
    let pitch_degrees = parse_angle_degrees(parts[1]).ok_or_else(|| {
        CameraOrbitUrlStateError::InvalidCameraOrbit {
            value: value.to_string(),
        }
    })?;
    let distance = parse_distance_meters(parts[2]).ok_or_else(|| {
        CameraOrbitUrlStateError::InvalidCameraOrbit {
            value: value.to_string(),
        }
    })?;
    Ok((yaw_degrees, pitch_degrees, distance))
}

fn parse_camera_target(value: &str) -> Result<Vec3, CameraOrbitUrlStateError> {
    let parts =
        split_triplet(value).ok_or_else(|| CameraOrbitUrlStateError::InvalidCameraTarget {
            value: value.to_string(),
        })?;
    let x = parse_distance_meters(parts[0]).ok_or_else(|| {
        CameraOrbitUrlStateError::InvalidCameraTarget {
            value: value.to_string(),
        }
    })?;
    let y = parse_distance_meters(parts[1]).ok_or_else(|| {
        CameraOrbitUrlStateError::InvalidCameraTarget {
            value: value.to_string(),
        }
    })?;
    let z = parse_distance_meters(parts[2]).ok_or_else(|| {
        CameraOrbitUrlStateError::InvalidCameraTarget {
            value: value.to_string(),
        }
    })?;
    Ok(Vec3::new(x, y, z))
}

fn split_triplet(value: &str) -> Option<Vec<&str>> {
    let parts: Vec<&str> = if value.contains(',') {
        value.split(',').map(str::trim).collect()
    } else {
        value.split_whitespace().map(str::trim).collect()
    };
    if parts.len() == 3 && parts.iter().all(|part| !part.is_empty()) {
        Some(parts)
    } else {
        None
    }
}

fn parse_angle_degrees(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(degrees) = value.strip_suffix("deg") {
        parse_finite(degrees)
    } else if let Some(radians) = value.strip_suffix("rad") {
        parse_finite(radians).map(f32::to_degrees)
    } else {
        parse_finite(value)
    }
}

fn parse_distance_meters(value: &str) -> Option<f32> {
    let value = value.trim();
    let meters = if let Some(mm) = value.strip_suffix("mm") {
        parse_finite(mm)? * 0.001
    } else if let Some(cm) = value.strip_suffix("cm") {
        parse_finite(cm)? * 0.01
    } else if let Some(m) = value.strip_suffix('m') {
        parse_finite(m)?
    } else {
        parse_finite(value)?
    };
    if meters.is_finite() {
        Some(meters)
    } else {
        None
    }
}

fn parse_finite(value: &str) -> Option<f32> {
    let parsed = value.trim().parse::<f32>().ok()?;
    parsed.is_finite().then_some(parsed)
}

fn validate_finite(field: &'static str, value: f32) -> Result<(), CameraOrbitUrlStateError> {
    value
        .is_finite()
        .then_some(())
        .ok_or(CameraOrbitUrlStateError::InvalidField { field })
}

fn validate_distance(distance: f32) -> Result<(), CameraOrbitUrlStateError> {
    (distance.is_finite() && distance > 0.0)
        .then_some(())
        .ok_or(CameraOrbitUrlStateError::InvalidField { field: "distance" })
}

fn validate_target(target: Vec3) -> Result<(), CameraOrbitUrlStateError> {
    (target.x.is_finite() && target.y.is_finite() && target.z.is_finite())
        .then_some(())
        .ok_or(CameraOrbitUrlStateError::InvalidField { field: "target" })
}

fn sanitize_distance(distance: f32) -> f32 {
    if distance.is_finite() && distance > 0.0 {
        distance
    } else {
        MIN_DISTANCE
    }
}

fn sanitize_vec3(value: Vec3) -> Vec3 {
    Vec3::new(
        finite_or(value.x, 0.0),
        finite_or(value.y, 0.0),
        finite_or(value.z, 0.0),
    )
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn format_scalar(value: f32) -> String {
    let mut formatted = format!("{value:.6}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }
    if formatted == "-0" {
        "0".to_string()
    } else {
        formatted
    }
}
