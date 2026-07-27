use serde::{Deserialize, Serialize};

use crate::capture::{CaptureDescriptor, CaptureRevisions};

pub const SUBJECT_OBSERVATION_SCHEMA_V1: &str = "scena.subject_observation.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectObservationV1 {
    pub schema: String,
    pub status: String,
    pub source: String,
    pub target: SubjectObservationTargetV1,
    pub frame_key: SubjectObservationFrameKeyV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projected_bounds: Option<SubjectObservationBoundsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_bounds: Option<SubjectObservationBoundsV1>,
    pub visible_pixel_count: u64,
    pub projected_area_px: u64,
    pub visible_fill_fraction: f32,
    pub visible_fraction_of_projected: f32,
    pub occlusion_estimate: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<SubjectObservationDepthV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixel_quality: Option<SubjectObservationPixelQualityV1>,
    pub fallback: SubjectObservationFallbackV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectObservationTargetV1 {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub handles: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectObservationFrameKeyV1 {
    pub width: u32,
    pub height: u32,
    pub payload_fnv1a64: String,
    pub pixel_source: String,
    pub state_binding: String,
    pub render_generation: u64,
    pub target_revision: u64,
    pub output_resources_revision: u64,
    pub revisions: CaptureRevisions,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubjectObservationBoundsV1 {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub width: f32,
    pub height: f32,
    pub area_px: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubjectObservationDepthV1 {
    pub near_m: f32,
    pub p50_m: f32,
    pub far_m: f32,
    pub sample_count: u64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubjectObservationPixelQualityV1 {
    pub mean_luminance_srgb8: f64,
    pub luminance_stddev_srgb8: f64,
    pub luminance_range_srgb8: f64,
    pub low_clip_fraction: f64,
    pub high_clip_fraction: f64,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectObservationFallbackV1 {
    pub degraded: bool,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub reason_codes: Vec<String>,
}

impl SubjectObservationV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn observed(
        source: impl Into<String>,
        target: SubjectObservationTargetV1,
        descriptor: &CaptureDescriptor,
        projected_bounds: SubjectObservationBoundsV1,
        visible_bounds: SubjectObservationBoundsV1,
        metrics: SubjectObservationMetricsV1,
        depth: Option<SubjectObservationDepthV1>,
        fallback: SubjectObservationFallbackV1,
    ) -> Self {
        Self {
            schema: SUBJECT_OBSERVATION_SCHEMA_V1.to_owned(),
            status: "observed".to_owned(),
            source: source.into(),
            target,
            frame_key: SubjectObservationFrameKeyV1::from_capture_descriptor(descriptor),
            projected_bounds: Some(projected_bounds),
            visible_bounds: Some(visible_bounds),
            visible_pixel_count: metrics.visible_pixel_count,
            projected_area_px: metrics.projected_area_px,
            visible_fill_fraction: metrics.visible_fill_fraction,
            visible_fraction_of_projected: metrics.visible_fraction_of_projected,
            occlusion_estimate: metrics.occlusion_estimate,
            depth,
            pixel_quality: None,
            fallback,
        }
    }

    pub fn degraded(
        source: impl Into<String>,
        target: SubjectObservationTargetV1,
        descriptor: &CaptureDescriptor,
        reason_codes: Vec<String>,
        flags: Vec<String>,
    ) -> Self {
        Self {
            schema: SUBJECT_OBSERVATION_SCHEMA_V1.to_owned(),
            status: "degraded".to_owned(),
            source: source.into(),
            target,
            frame_key: SubjectObservationFrameKeyV1::from_capture_descriptor(descriptor),
            projected_bounds: None,
            visible_bounds: None,
            visible_pixel_count: 0,
            projected_area_px: 0,
            visible_fill_fraction: 0.0,
            visible_fraction_of_projected: 0.0,
            occlusion_estimate: 0.0,
            depth: None,
            pixel_quality: None,
            fallback: SubjectObservationFallbackV1 {
                degraded: true,
                flags,
                reason_codes,
            },
        }
    }

    pub fn with_pixel_quality(mut self, pixel_quality: SubjectObservationPixelQualityV1) -> Self {
        self.pixel_quality = Some(pixel_quality);
        self
    }

    pub fn validate_contract(&self) -> Result<(), &'static str> {
        if self.schema != SUBJECT_OBSERVATION_SCHEMA_V1 {
            return Err("invalid_schema");
        }
        if self.status.trim().is_empty() || self.source.trim().is_empty() {
            return Err("missing_subject_observation_status");
        }
        if self.target.kind.trim().is_empty() || self.target.id.trim().is_empty() {
            return Err("missing_subject_observation_target");
        }
        if self.target.handles.is_empty() {
            return Err("missing_resolved_subject_handles");
        }
        if self.frame_key.width == 0
            || self.frame_key.height == 0
            || self.frame_key.payload_fnv1a64.trim().is_empty()
            || self.frame_key.state_binding != "exact_readback_completion"
        {
            return Err("stale_subject_observation");
        }
        match self.status.as_str() {
            "observed" => self.validate_observed(),
            "degraded" | "unavailable" => {
                if !self.fallback.degraded || self.fallback.reason_codes.is_empty() {
                    return Err("missing_subject_observation_reason");
                }
                Ok(())
            }
            _ => Err("invalid_subject_observation_status"),
        }
    }

    fn validate_observed(&self) -> Result<(), &'static str> {
        let Some(projected) = self.projected_bounds else {
            return Err("missing_projected_subject_bounds");
        };
        let Some(visible) = self.visible_bounds else {
            return Err("missing_visible_subject_bounds");
        };
        if !projected.valid() || !visible.valid() {
            return Err("invalid_subject_observation_bounds");
        }
        if self.visible_pixel_count == 0 || self.projected_area_px == 0 {
            return Err("missing_visible_subject_pixels");
        }
        if !unit_fraction(self.visible_fill_fraction)
            || !unit_fraction(self.visible_fraction_of_projected)
            || !unit_fraction(self.occlusion_estimate)
        {
            return Err("invalid_subject_observation_fractions");
        }
        if let Some(depth) = self.depth {
            depth.validate()?;
        }
        if let Some(pixel_quality) = self.pixel_quality {
            pixel_quality.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubjectObservationMetricsV1 {
    pub visible_pixel_count: u64,
    pub projected_area_px: u64,
    pub visible_fill_fraction: f32,
    pub visible_fraction_of_projected: f32,
    pub occlusion_estimate: f32,
}

impl SubjectObservationTargetV1 {
    pub fn new(
        kind: impl Into<String>,
        id: impl Into<String>,
        handles: impl IntoIterator<Item = u64>,
    ) -> Self {
        let mut handles = handles.into_iter().collect::<Vec<_>>();
        handles.sort_unstable();
        handles.dedup();
        Self {
            kind: kind.into(),
            id: id.into(),
            handles,
        }
    }
}

impl SubjectObservationPixelQualityV1 {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.mean_luminance_srgb8.is_finite()
            || !self.luminance_stddev_srgb8.is_finite()
            || !self.luminance_range_srgb8.is_finite()
            || !(0.0..=255.0).contains(&self.mean_luminance_srgb8)
            || !(0.0..=255.0).contains(&self.luminance_stddev_srgb8)
            || !(0.0..=255.0).contains(&self.luminance_range_srgb8)
            || !(0.0..=1.0).contains(&self.low_clip_fraction)
            || !(0.0..=1.0).contains(&self.high_clip_fraction)
            || self.sample_count == 0
        {
            return Err("invalid_subject_pixel_quality");
        }
        Ok(())
    }
}

impl SubjectObservationFrameKeyV1 {
    pub fn from_capture_descriptor(descriptor: &CaptureDescriptor) -> Self {
        Self {
            width: descriptor.width,
            height: descriptor.height,
            payload_fnv1a64: descriptor.payload.fnv1a64.clone(),
            pixel_source: descriptor.frame.pixel_source.clone(),
            state_binding: descriptor.frame.state_binding.clone(),
            render_generation: descriptor.frame.render_generation,
            target_revision: descriptor.frame.target_revision,
            output_resources_revision: descriptor.frame.output_resources_revision,
            revisions: descriptor.revisions,
        }
    }
}

impl SubjectObservationBoundsV1 {
    fn valid(self) -> bool {
        self.min_x.is_finite()
            && self.min_y.is_finite()
            && self.max_x.is_finite()
            && self.max_y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
            && self.max_x >= self.min_x
            && self.max_y >= self.min_y
            && self.area_px > 0
    }
}

impl SubjectObservationDepthV1 {
    fn validate(self) -> Result<(), &'static str> {
        if !self.near_m.is_finite()
            || !self.p50_m.is_finite()
            || !self.far_m.is_finite()
            || self.near_m <= 0.0
            || self.p50_m <= 0.0
            || self.far_m <= 0.0
            || self.near_m > self.p50_m
            || self.p50_m > self.far_m
            || self.sample_count == 0
            || !unit_fraction(self.confidence)
        {
            return Err("invalid_subject_observation_depth");
        }
        Ok(())
    }
}

fn unit_fraction(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
