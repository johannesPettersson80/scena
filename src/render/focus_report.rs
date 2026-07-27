use serde::{Deserialize, Serialize};

use crate::capture::{CaptureDescriptor, CaptureRevisions};

pub const FOCUS_REPORT_SCHEMA_V1: &str = "scena.focus_report.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusReportV1 {
    pub schema: String,
    pub status: String,
    pub mode: String,
    pub target: FocusReportTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<FocusReportResolvedV1>,
    pub frame_key: FocusReportFrameKeyV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusReportTargetV1 {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handles: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FocusReportResolvedV1 {
    pub focus_distance_m: f32,
    pub near_depth_m: f32,
    pub far_depth_m: f32,
    pub visible_pixel_count: u64,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusReportFrameKeyV1 {
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

impl FocusReportV1 {
    pub fn resolved(
        mode: impl Into<String>,
        target: FocusReportTargetV1,
        coverage: Option<String>,
        strength: Option<String>,
        resolved: FocusReportResolvedV1,
        descriptor: &CaptureDescriptor,
    ) -> Self {
        Self {
            schema: FOCUS_REPORT_SCHEMA_V1.to_owned(),
            status: "resolved".to_owned(),
            mode: mode.into(),
            target,
            coverage,
            strength,
            resolved: Some(resolved),
            frame_key: FocusReportFrameKeyV1::from_capture_descriptor(descriptor),
            reason: None,
        }
    }

    pub fn unresolved(
        mode: impl Into<String>,
        target: FocusReportTargetV1,
        coverage: Option<String>,
        strength: Option<String>,
        reason: impl Into<String>,
        descriptor: &CaptureDescriptor,
    ) -> Self {
        Self {
            schema: FOCUS_REPORT_SCHEMA_V1.to_owned(),
            status: "unresolved".to_owned(),
            mode: mode.into(),
            target,
            coverage,
            strength,
            resolved: None,
            frame_key: FocusReportFrameKeyV1::from_capture_descriptor(descriptor),
            reason: Some(reason.into()),
        }
    }

    pub fn not_requested(
        target: FocusReportTargetV1,
        reason: impl Into<String>,
        descriptor: &CaptureDescriptor,
    ) -> Self {
        Self {
            schema: FOCUS_REPORT_SCHEMA_V1.to_owned(),
            status: "not_requested".to_owned(),
            mode: "none".to_owned(),
            target,
            coverage: None,
            strength: None,
            resolved: None,
            frame_key: FocusReportFrameKeyV1::from_capture_descriptor(descriptor),
            reason: Some(reason.into()),
        }
    }

    pub fn validate_contract(&self) -> Result<(), &'static str> {
        if self.schema != FOCUS_REPORT_SCHEMA_V1 {
            return Err("invalid_schema");
        }
        if self.target.kind.trim().is_empty() || self.target.id.trim().is_empty() {
            return Err("missing_focus_target");
        }
        if self.frame_key.width == 0
            || self.frame_key.height == 0
            || self.frame_key.payload_fnv1a64.trim().is_empty()
            || self.frame_key.state_binding != "exact_readback_completion"
        {
            return Err("stale_frame_key");
        }
        match self.status.as_str() {
            "resolved" => {
                let Some(resolved) = self.resolved else {
                    return Err("missing_resolved_focus");
                };
                if self.reason.is_some() {
                    return Err("resolved_focus_has_reason");
                }
                if !resolved.focus_distance_m.is_finite()
                    || !resolved.near_depth_m.is_finite()
                    || !resolved.far_depth_m.is_finite()
                    || resolved.focus_distance_m <= 0.0
                    || resolved.near_depth_m <= 0.0
                    || resolved.far_depth_m <= 0.0
                    || resolved.visible_pixel_count == 0
                    || !(0.0..=1.0).contains(&resolved.confidence)
                {
                    return Err("invalid_resolved_focus");
                }
                Ok(())
            }
            "unresolved" | "not_requested" => {
                if self
                    .reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err("missing_unresolved_reason");
                }
                if self.resolved.is_some() {
                    return Err("unresolved_focus_has_resolved_payload");
                }
                Ok(())
            }
            _ => Err("invalid_focus_status"),
        }
    }
}

impl FocusReportTargetV1 {
    pub fn new(
        kind: impl Into<String>,
        id: impl Into<String>,
        handles: impl IntoIterator<Item = u64>,
    ) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            handles: handles.into_iter().collect(),
        }
    }

    pub fn import(id: impl Into<String>, handles: impl IntoIterator<Item = u64>) -> Self {
        Self::new("import", id, handles)
    }
}

impl FocusReportFrameKeyV1 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_frame_key() -> FocusReportFrameKeyV1 {
        FocusReportFrameKeyV1 {
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

    fn valid_resolved_report() -> FocusReportV1 {
        FocusReportV1 {
            schema: FOCUS_REPORT_SCHEMA_V1.to_owned(),
            status: "resolved".to_owned(),
            mode: "subject".to_owned(),
            target: FocusReportTargetV1::import("subject", [10, 11]),
            coverage: Some("all".to_owned()),
            strength: Some("subtle".to_owned()),
            resolved: Some(FocusReportResolvedV1 {
                focus_distance_m: 2.5,
                near_depth_m: 2.0,
                far_depth_m: 3.0,
                visible_pixel_count: 512,
                confidence: 1.0,
            }),
            frame_key: valid_frame_key(),
            reason: None,
        }
    }

    #[test]
    fn focus_report_contract_rejects_missing_target_stale_frame_and_unresolved_reason() {
        assert_eq!(valid_resolved_report().validate_contract(), Ok(()));

        let mut missing_target = valid_resolved_report();
        missing_target.target.id.clear();
        assert_eq!(
            missing_target.validate_contract(),
            Err("missing_focus_target")
        );

        let mut stale_frame = valid_resolved_report();
        stale_frame.frame_key.state_binding = "unverified".to_owned();
        assert_eq!(stale_frame.validate_contract(), Err("stale_frame_key"));

        let mut unresolved_without_reason = valid_resolved_report();
        unresolved_without_reason.status = "unresolved".to_owned();
        unresolved_without_reason.resolved = None;
        assert_eq!(
            unresolved_without_reason.validate_contract(),
            Err("missing_unresolved_reason")
        );
    }
}
