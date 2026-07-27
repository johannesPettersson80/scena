use serde::{Deserialize, Serialize};

use crate::capture::{CaptureDescriptor, CaptureRevisions};
use crate::render::{AutoExposureConfig, AutoExposureResult, AutoExposureStatus};

pub const EXPOSURE_REPORT_SCHEMA_V1: &str = "scena.exposure_report.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExposureReportV1 {
    pub schema: String,
    pub status: String,
    pub mode: String,
    pub metering_domain: String,
    pub selected_exposure_ev: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_exposure: Option<ExposureReportAutoV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<ExposureReportSubjectV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_compensation_ev: Option<f32>,
    pub frame_key: ExposureReportFrameKeyV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExposureReportAutoV1 {
    pub status: String,
    pub measured_luminance: f32,
    pub target_luminance: f32,
    pub base_exposure_ev: f32,
    pub compensation_ev: f32,
    pub exposure_ev: f32,
    pub sample_count: u32,
    pub subject_sample_count: u32,
    pub rejected_sample_count: u32,
    pub clamped: bool,
    pub min_ev: f32,
    pub max_ev: f32,
    pub highlight_percentile: f32,
    pub highlight_target_luminance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExposureReportSubjectV1 {
    pub mean_luminance_srgb8: f64,
    pub low_clip_fraction: f64,
    pub high_clip_fraction: f64,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExposureReportFrameKeyV1 {
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

impl ExposureReportV1 {
    pub fn measured_subject(
        mode: impl Into<String>,
        selected_exposure_ev: f32,
        subject: ExposureReportSubjectV1,
        suggested_compensation_ev: f32,
        descriptor: &CaptureDescriptor,
    ) -> Self {
        Self {
            schema: EXPOSURE_REPORT_SCHEMA_V1.to_owned(),
            status: "measured".to_owned(),
            mode: mode.into(),
            metering_domain: "final_srgb8_subject_pixels".to_owned(),
            selected_exposure_ev,
            auto_exposure: None,
            subject: Some(subject),
            suggested_compensation_ev: Some(suggested_compensation_ev),
            frame_key: ExposureReportFrameKeyV1::from_capture_descriptor(descriptor),
            reason: None,
        }
    }

    pub fn from_auto_exposure(
        status: AutoExposureStatus,
        config: AutoExposureConfig,
        result: Option<AutoExposureResult>,
        selected_exposure_ev: f32,
        descriptor: &CaptureDescriptor,
    ) -> Self {
        let metering_domain = result
            .map(|result| result.metering_domain().as_str())
            .unwrap_or("renderer_auto_exposure_meter");
        let auto_exposure = result.map(|result| ExposureReportAutoV1 {
            status: auto_exposure_status_name(status).to_owned(),
            measured_luminance: result.measured_luminance(),
            target_luminance: result.target_luminance(),
            base_exposure_ev: result.base_exposure_ev(),
            compensation_ev: result.compensation_ev(),
            exposure_ev: result.exposure_ev(),
            sample_count: result.sample_count(),
            subject_sample_count: result.subject_sample_count(),
            rejected_sample_count: result.rejected_sample_count(),
            clamped: result.clamped(),
            min_ev: config.min_ev(),
            max_ev: config.max_ev(),
            highlight_percentile: config.highlight_percentile(),
            highlight_target_luminance: config.highlight_target_luminance(),
        });
        let report_status = match (status, auto_exposure.is_some()) {
            (_, true) => "measured",
            (AutoExposureStatus::Pending, false) => "pending",
            (AutoExposureStatus::Unavailable, false) => "unavailable",
            (AutoExposureStatus::Disabled, false) => "disabled",
            (AutoExposureStatus::Converged, false) => "unavailable",
        };
        Self {
            schema: EXPOSURE_REPORT_SCHEMA_V1.to_owned(),
            status: report_status.to_owned(),
            mode: "auto_exposure".to_owned(),
            metering_domain: metering_domain.to_owned(),
            selected_exposure_ev,
            auto_exposure,
            subject: None,
            suggested_compensation_ev: result.map(|result| {
                let residual =
                    (result.target_luminance() / result.measured_luminance().max(1.0e-4)).log2();
                residual - result.base_exposure_ev()
            }),
            frame_key: ExposureReportFrameKeyV1::from_capture_descriptor(descriptor),
            reason: auto_exposure_metering_reason(result, status, report_status),
        }
    }

    pub fn validate_contract(&self) -> Result<(), &'static str> {
        if self.schema != EXPOSURE_REPORT_SCHEMA_V1 {
            return Err("invalid_schema");
        }
        if self.mode.trim().is_empty() || self.metering_domain.trim().is_empty() {
            return Err("missing_exposure_mode");
        }
        if !is_known_metering_domain(self.metering_domain.as_str()) {
            return Err("invalid_metering_domain");
        }
        if !self.selected_exposure_ev.is_finite() {
            return Err("invalid_selected_exposure");
        }
        if self.frame_key.width == 0
            || self.frame_key.height == 0
            || self.frame_key.payload_fnv1a64.trim().is_empty()
            || self.frame_key.state_binding != "exact_readback_completion"
        {
            return Err("stale_frame_key");
        }
        match self.status.as_str() {
            "measured" => {
                if self.auto_exposure.is_none() && self.subject.is_none() {
                    return Err("missing_exposure_measurement");
                }
                if let Some(subject) = self.subject {
                    validate_subject(subject)?;
                }
                if let Some(auto) = self.auto_exposure.as_ref() {
                    validate_auto(auto)?;
                }
                Ok(())
            }
            "pending" | "unavailable" | "disabled" => {
                if self
                    .reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err("missing_unresolved_reason");
                }
                Ok(())
            }
            _ => Err("invalid_exposure_status"),
        }
    }
}

fn is_known_metering_domain(domain: &str) -> bool {
    matches!(
        domain,
        "scene_linear_pre_tonemap"
            | "encoded_output_feedback"
            | "final_srgb8_subject_pixels"
            | "renderer_auto_exposure_meter"
    )
}

fn auto_exposure_metering_reason(
    result: Option<AutoExposureResult>,
    status: AutoExposureStatus,
    report_status: &str,
) -> Option<String> {
    if let Some(code) = result.and_then(|result| {
        result
            .metering_domain()
            .strict_camera_behavior_rejection_code()
    }) {
        return Some(format!(
            "{code}: auto exposure was metered from encoded output feedback, not scene-linear pre-tonemap pixels"
        ));
    }
    (report_status != "measured").then(|| {
        format!(
            "auto exposure status is {} and no metered result is available",
            auto_exposure_status_name(status)
        )
    })
}

impl ExposureReportSubjectV1 {
    pub const fn new(
        mean_luminance_srgb8: f64,
        low_clip_fraction: f64,
        high_clip_fraction: f64,
        sample_count: u64,
    ) -> Self {
        Self {
            mean_luminance_srgb8,
            low_clip_fraction,
            high_clip_fraction,
            sample_count,
        }
    }
}

impl ExposureReportFrameKeyV1 {
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

fn validate_subject(subject: ExposureReportSubjectV1) -> Result<(), &'static str> {
    if !subject.mean_luminance_srgb8.is_finite()
        || !(0.0..=255.0).contains(&subject.mean_luminance_srgb8)
        || !(0.0..=1.0).contains(&subject.low_clip_fraction)
        || !(0.0..=1.0).contains(&subject.high_clip_fraction)
        || subject.sample_count == 0
    {
        return Err("invalid_subject_exposure");
    }
    Ok(())
}

fn validate_auto(auto: &ExposureReportAutoV1) -> Result<(), &'static str> {
    if !auto.measured_luminance.is_finite()
        || !auto.target_luminance.is_finite()
        || !auto.base_exposure_ev.is_finite()
        || !auto.compensation_ev.is_finite()
        || !auto.exposure_ev.is_finite()
        || !auto.min_ev.is_finite()
        || !auto.max_ev.is_finite()
        || !auto.highlight_percentile.is_finite()
        || !auto.highlight_target_luminance.is_finite()
        || auto.measured_luminance <= 0.0
        || auto.target_luminance <= 0.0
        || auto.sample_count == 0
        || auto.subject_sample_count > auto.sample_count
    {
        return Err("invalid_auto_exposure");
    }
    Ok(())
}

fn auto_exposure_status_name(status: AutoExposureStatus) -> &'static str {
    match status {
        AutoExposureStatus::Disabled => "disabled",
        AutoExposureStatus::Pending => "pending",
        AutoExposureStatus::Converged => "converged",
        AutoExposureStatus::Unavailable => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_frame_key() -> ExposureReportFrameKeyV1 {
        ExposureReportFrameKeyV1 {
            width: 320,
            height: 240,
            payload_fnv1a64: "0123456789abcdef".to_owned(),
            pixel_source: "renderer_owned_readback".to_owned(),
            state_binding: "exact_readback_completion".to_owned(),
            render_generation: 3,
            target_revision: 1,
            output_resources_revision: 2,
            revisions: CaptureRevisions {
                structure: 1,
                transform: 2,
                camera: 3,
                appearance: 4,
                interaction: 5,
            },
        }
    }

    fn valid_report() -> ExposureReportV1 {
        ExposureReportV1 {
            schema: EXPOSURE_REPORT_SCHEMA_V1.to_owned(),
            status: "measured".to_owned(),
            mode: "camera_behavior_retry".to_owned(),
            metering_domain: "final_srgb8_subject_pixels".to_owned(),
            selected_exposure_ev: 1.25,
            auto_exposure: None,
            subject: Some(ExposureReportSubjectV1::new(90.0, 0.0, 0.0, 24_576)),
            suggested_compensation_ev: Some(0.0),
            frame_key: valid_frame_key(),
            reason: None,
        }
    }

    #[test]
    fn exposure_report_contract_rejects_missing_measurement_and_stale_frame() {
        assert_eq!(valid_report().validate_contract(), Ok(()));

        let mut missing_measurement = valid_report();
        missing_measurement.subject = None;
        assert_eq!(
            missing_measurement.validate_contract(),
            Err("missing_exposure_measurement")
        );

        let mut stale_frame = valid_report();
        stale_frame.frame_key.state_binding = "unverified".to_owned();
        assert_eq!(stale_frame.validate_contract(), Err("stale_frame_key"));

        let mut unresolved_without_reason = valid_report();
        unresolved_without_reason.status = "unavailable".to_owned();
        unresolved_without_reason.subject = None;
        assert_eq!(
            unresolved_without_reason.validate_contract(),
            Err("missing_unresolved_reason")
        );

        let mut invalid_domain = valid_report();
        invalid_domain.metering_domain = "guessed_by_prose".to_owned();
        assert_eq!(
            invalid_domain.validate_contract(),
            Err("invalid_metering_domain")
        );
    }

    #[test]
    fn encoded_output_metering_gets_strict_camera_behavior_rejection_reason() {
        let encoded = crate::render::estimate_auto_exposure_from_srgb8(
            &[84, 84, 84, 255, 84, 84, 84, 255],
            AutoExposureConfig::mixed(),
        )
        .expect("encoded output frame meters");
        let reason =
            auto_exposure_metering_reason(Some(encoded), AutoExposureStatus::Converged, "measured")
                .expect("encoded metering is degraded for strict evidence");
        assert!(
            reason.contains("metering_domain_encoded_output_feedback"),
            "{reason}"
        );

        let linear = crate::render::estimate_auto_exposure_from_linear_colors(
            &[crate::Color::from_linear_rgb(0.09, 0.09, 0.09); 2],
            AutoExposureConfig::mixed(),
        )
        .expect("scene-linear frame meters");
        assert_eq!(
            auto_exposure_metering_reason(Some(linear), AutoExposureStatus::Converged, "measured"),
            None
        );
    }
}
