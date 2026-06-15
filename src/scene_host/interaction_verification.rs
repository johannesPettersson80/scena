use serde::{Deserialize, Serialize};

use super::{HOST_EVENT_SCHEMA_V1, HostEventV1};

pub const INTERACTION_EXPECTATION_SCHEMA_V1: &str = "scena.interaction_expectation.v1";
pub const INTERACTION_VERIFICATION_SCHEMA_V1: &str = "scena.interaction_verification.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionExpectationV1 {
    pub schema: String,
    pub viewport: InteractionViewportV1,
    pub steps: Vec<InteractionStepExpectationV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InteractionViewportV1 {
    pub width_css_px: f32,
    pub height_css_px: f32,
    pub device_pixel_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionStepExpectationV1 {
    pub action: String,
    pub x_css_px: f32,
    pub y_css_px: f32,
    #[serde(default = "default_coordinate_space")]
    pub coordinate_space: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_hit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_hover: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_selection: Option<bool>,
    #[serde(default)]
    pub expected_events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionVerificationReportV1 {
    pub schema: String,
    pub ok: bool,
    pub summary: InteractionVerificationSummaryV1,
    pub steps: Vec<InteractionStepReportV1>,
    pub reasons: Vec<InteractionVerificationReasonV1>,
    pub fixes: Vec<InteractionVerificationFixV1>,
    pub artifacts: InteractionVerificationArtifactsV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionVerificationSummaryV1 {
    pub step_count: usize,
    pub failed_step_count: usize,
    pub hit_count: usize,
    pub miss_count: usize,
    pub event_count: usize,
    pub rendered_feedback_checked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionStepReportV1 {
    pub index: usize,
    pub action: String,
    pub coordinates: InteractionCoordinatesV1,
    pub expected: InteractionStepExpectedV1,
    pub observed: InteractionStepObservedV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InteractionCoordinatesV1 {
    pub coordinate_space: InteractionCoordinateSpaceV1,
    pub x_css_px: f32,
    pub y_css_px: f32,
    pub x_physical_px: f32,
    pub y_physical_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionCoordinateSpaceV1 {
    Css,
    Physical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionStepExpectedV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_hit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_hover: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_selection: Option<bool>,
    pub expected_events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionStepObservedV1 {
    pub hit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_handle: Option<u64>,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionVerificationReasonV1 {
    pub code: String,
    pub severity: String,
    pub step_index: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionVerificationFixV1 {
    pub action: String,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionVerificationArtifactsV1 {
    pub host_event_schema: String,
    pub width_css_px: f32,
    pub height_css_px: f32,
    pub width_physical_px: u32,
    pub height_physical_px: u32,
    pub device_pixel_ratio: f32,
}

impl InteractionExpectationV1 {
    pub fn validate_schema(&self) -> Result<(), String> {
        if self.schema != INTERACTION_EXPECTATION_SCHEMA_V1 {
            return Err(format!(
                "expected schema {INTERACTION_EXPECTATION_SCHEMA_V1}, got {}",
                self.schema
            ));
        }
        if !self.viewport.width_css_px.is_finite()
            || !self.viewport.height_css_px.is_finite()
            || !self.viewport.device_pixel_ratio.is_finite()
            || self.viewport.width_css_px <= 0.0
            || self.viewport.height_css_px <= 0.0
            || self.viewport.device_pixel_ratio <= 0.0
        {
            return Err("interaction viewport must be finite and positive".to_string());
        }
        if self.steps.is_empty() {
            return Err("interaction expectation requires at least one step".to_string());
        }
        for (index, step) in self.steps.iter().enumerate() {
            step.coordinate_space()?;
            if !matches!(step.action.as_str(), "pick" | "hover" | "select") {
                return Err(format!(
                    "interaction step {index} action must be pick, hover, or select"
                ));
            }
            if !step.x_css_px.is_finite() || !step.y_css_px.is_finite() {
                return Err(format!(
                    "interaction step {index} coordinates must be finite"
                ));
            }
        }
        Ok(())
    }
}

impl InteractionStepExpectationV1 {
    pub fn coordinate_space(&self) -> Result<InteractionCoordinateSpaceV1, String> {
        match self.coordinate_space.as_str() {
            "css" => Ok(InteractionCoordinateSpaceV1::Css),
            "physical" => Ok(InteractionCoordinateSpaceV1::Physical),
            other => Err(format!(
                "interaction coordinate_space must be css or physical, got '{other}'"
            )),
        }
    }
}

impl InteractionVerificationReportV1 {
    pub fn from_steps(
        viewport: InteractionVerificationArtifactsV1,
        steps: Vec<InteractionStepReportV1>,
    ) -> Self {
        let mut reasons = Vec::new();
        let mut fixes = Vec::new();
        for step in &steps {
            collect_step_reasons(step, &mut reasons, &mut fixes);
        }
        let failed_step_count = reasons
            .iter()
            .filter(|reason| reason.severity == "error")
            .map(|reason| reason.step_index)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let errors = reasons
            .iter()
            .filter(|reason| reason.severity == "error")
            .count();
        let hit_count = steps.iter().filter(|step| step.observed.hit).count();
        let miss_count = steps.len().saturating_sub(hit_count);
        let event_count = steps
            .iter()
            .map(|step| step.observed.events.len())
            .sum::<usize>();
        Self {
            schema: INTERACTION_VERIFICATION_SCHEMA_V1.to_owned(),
            ok: errors == 0,
            summary: InteractionVerificationSummaryV1 {
                step_count: steps.len(),
                failed_step_count,
                hit_count,
                miss_count,
                event_count,
                rendered_feedback_checked: false,
            },
            steps,
            reasons,
            fixes,
            artifacts: viewport,
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("interaction verification report is serializable")
    }
}

impl From<&InteractionStepExpectationV1> for InteractionStepExpectedV1 {
    fn from(value: &InteractionStepExpectationV1) -> Self {
        Self {
            expect_hit: value.expect_hit,
            expected_handle: value.expected_handle,
            expect_hover: value.expect_hover,
            expect_selection: value.expect_selection,
            expected_events: value.expected_events.clone(),
        }
    }
}

impl InteractionCoordinatesV1 {
    pub fn from_step(
        step: &InteractionStepExpectationV1,
        viewport: InteractionViewportV1,
    ) -> Result<Self, String> {
        let coordinate_space = step.coordinate_space()?;
        let (x_css_px, y_css_px, x_physical_px, y_physical_px) = match coordinate_space {
            InteractionCoordinateSpaceV1::Css => (
                step.x_css_px,
                step.y_css_px,
                step.x_css_px * viewport.device_pixel_ratio,
                step.y_css_px * viewport.device_pixel_ratio,
            ),
            InteractionCoordinateSpaceV1::Physical => (
                step.x_css_px / viewport.device_pixel_ratio,
                step.y_css_px / viewport.device_pixel_ratio,
                step.x_css_px,
                step.y_css_px,
            ),
        };
        Ok(Self {
            coordinate_space,
            x_css_px: round3(x_css_px),
            y_css_px: round3(y_css_px),
            x_physical_px: round3(x_physical_px),
            y_physical_px: round3(y_physical_px),
        })
    }
}

impl InteractionVerificationArtifactsV1 {
    pub fn from_viewport(viewport: InteractionViewportV1) -> Self {
        Self {
            host_event_schema: HOST_EVENT_SCHEMA_V1.to_owned(),
            width_css_px: round3(viewport.width_css_px),
            height_css_px: round3(viewport.height_css_px),
            width_physical_px: physical_px(viewport.width_css_px, viewport.device_pixel_ratio),
            height_physical_px: physical_px(viewport.height_css_px, viewport.device_pixel_ratio),
            device_pixel_ratio: round3(viewport.device_pixel_ratio),
        }
    }
}

pub fn host_event_kind_name(event: &HostEventV1) -> &'static str {
    match event {
        HostEventV1::Pick { .. } => "pick",
        HostEventV1::Hover { .. } => "hover",
        HostEventV1::SelectionChanged { .. } => "selection_changed",
        HostEventV1::LoadProgress { .. } => "load_progress",
        HostEventV1::AssetLoaded { .. } => "asset_loaded",
        HostEventV1::Diagnostic { .. } => "diagnostic",
        HostEventV1::CaptureReady { .. } => "capture_ready",
        HostEventV1::SurfaceResized { .. } => "surface_resized",
        HostEventV1::ContextLost { .. } => "context_lost",
        HostEventV1::ContextRestored => "context_restored",
        HostEventV1::DeviceLost { .. } => "device_lost",
        HostEventV1::DeviceRecovered => "device_recovered",
        HostEventV1::CapabilityChanged { .. } => "capability_changed",
    }
}

pub fn physical_px(css_px: f32, device_pixel_ratio: f32) -> u32 {
    (css_px * device_pixel_ratio).round().max(1.0) as u32
}

fn collect_step_reasons(
    step: &InteractionStepReportV1,
    reasons: &mut Vec<InteractionVerificationReasonV1>,
    fixes: &mut Vec<InteractionVerificationFixV1>,
) {
    if let Some(expect_hit) = step.expected.expect_hit
        && step.observed.hit != expect_hit
    {
        push_reason(
            reasons,
            "hit_mismatch",
            step.index,
            format!(
                "expected hit={}, observed hit={}",
                expect_hit, step.observed.hit
            ),
        );
        push_fix(
            fixes,
            "frame_target",
            "frame the target and verify the pointer coordinates are in CSS pixels",
        );
    }
    if let Some(expected) = step.expected.expected_handle
        && step.observed.handle != Some(expected)
    {
        push_reason(
            reasons,
            "handle_mismatch",
            step.index,
            format!(
                "expected handle {}, observed {:?}",
                expected, step.observed.handle
            ),
        );
        push_fix(
            fixes,
            "update_expected_handle",
            "inspect the scene and use the stable handle reported for the picked target",
        );
    }
    if let Some(expect_hover) = step.expected.expect_hover {
        let observed_hover = step.observed.hover_handle.is_some();
        if observed_hover != expect_hover {
            if expect_hover {
                push_reason(
                    reasons,
                    "hover_missing",
                    step.index,
                    "expected hover state to be set after the interaction".to_string(),
                );
                push_fix(
                    fixes,
                    "use_hover_action",
                    "run a hover or select action at a coordinate that hits the target",
                );
            } else {
                push_reason(
                    reasons,
                    "hover_unexpected",
                    step.index,
                    format!(
                        "expected hover state to be clear, observed {:?}",
                        step.observed.hover_handle
                    ),
                );
                push_fix(
                    fixes,
                    "clear_hover",
                    "run a hover action at a coordinate that misses the scene or update the expectation",
                );
            }
        }
    }
    if let Some(expect_selection) = step.expected.expect_selection {
        let observed_selection = step.observed.selection_handle.is_some();
        if observed_selection != expect_selection {
            if expect_selection {
                push_reason(
                    reasons,
                    "selection_missing",
                    step.index,
                    "expected primary selection to be set after the interaction".to_string(),
                );
                push_fix(
                    fixes,
                    "use_select_action",
                    "run a select action at a coordinate that hits the target",
                );
            } else {
                push_reason(
                    reasons,
                    "selection_unexpected",
                    step.index,
                    format!(
                        "expected primary selection to be clear, observed {:?}",
                        step.observed.selection_handle
                    ),
                );
                push_fix(
                    fixes,
                    "clear_selection",
                    "run a select action at a coordinate that misses the scene or update the expectation",
                );
            }
        }
    }
    if !step.expected.expected_events.is_empty()
        && step.observed.events != step.expected.expected_events
    {
        push_reason(
            reasons,
            "event_sequence_mismatch",
            step.index,
            format!(
                "expected events {:?}, observed {:?}",
                step.expected.expected_events, step.observed.events
            ),
        );
        push_fix(
            fixes,
            "check_event_contract",
            "use the documented host event sequence for the chosen interaction action",
        );
    }
}

fn push_reason(
    reasons: &mut Vec<InteractionVerificationReasonV1>,
    code: &str,
    step_index: usize,
    message: String,
) {
    reasons.push(InteractionVerificationReasonV1 {
        code: code.to_owned(),
        severity: "error".to_owned(),
        step_index,
        message,
    });
}

fn push_fix(fixes: &mut Vec<InteractionVerificationFixV1>, action: &str, help: &str) {
    if fixes.iter().any(|fix| fix.action == action) {
        return;
    }
    fixes.push(InteractionVerificationFixV1 {
        action: action.to_owned(),
        help: help.to_owned(),
    });
}

fn default_coordinate_space() -> String {
    "css".to_string()
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
