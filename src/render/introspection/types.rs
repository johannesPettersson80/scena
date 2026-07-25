use serde::{Deserialize, Serialize};

use crate::diagnostics::{Backend, CapabilityStatus, HardwareTier};

const DEFAULT_BACKGROUND_RGBA8: [u8; 4] = [0, 0, 0, 255];
const DEFAULT_CONTENT_TOLERANCE_RGBA8: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderIntrospectionOptions {
    pub(super) detail: bool,
    pub(super) capture_png_path: Option<String>,
    pub(super) capture_descriptor_path: Option<String>,
    pub(super) contact_sheet_path: Option<String>,
    pub(super) background_rgba8: [u8; 4],
    pub(super) content_tolerance_rgba8: u8,
    pub(super) timings: Option<RenderIntrospectionTimingsV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionReportV1 {
    pub schema: String,
    pub ok: bool,
    pub reasons: Vec<RenderIntrospectionReasonV1>,
    pub fixes: Vec<RenderIntrospectionFixV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_bbox_css_px: Option<RenderIntrospectionRectV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_bbox_fraction: Option<RenderIntrospectionRectV1>,
    pub visible_pixel_fraction: f32,
    pub luminance: RenderIntrospectionLuminanceV1,
    pub framing: RenderIntrospectionFramingV1,
    pub nodes_summary: RenderIntrospectionNodesSummaryV1,
    #[serde(default)]
    pub nodes_detail: Vec<RenderIntrospectionNodeDetailV1>,
    pub artifacts: RenderIntrospectionArtifactsV1,
    pub capabilities: RenderIntrospectionCapabilitiesV1,
    #[serde(default)]
    pub timings: RenderIntrospectionTimingsV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderIntrospectionTimingsV1 {
    pub status: String,
    pub clock: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepare_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

impl RenderIntrospectionTimingsV1 {
    pub fn unavailable() -> Self {
        Self {
            status: "unavailable".to_owned(),
            clock: "unavailable".to_owned(),
            source: "not_measured".to_owned(),
            prepare_ms: None,
            render_ms: None,
            capture_ms: None,
            total_ms: None,
            unavailable_reason: Some(
                "request CLI observational timing with --timings; deterministic reports omit wall-clock measurements"
                    .to_owned(),
            ),
        }
    }

    pub fn measured_monotonic(
        prepare_ms: u64,
        render_ms: u64,
        capture_ms: u64,
        total_ms: u64,
    ) -> Self {
        Self {
            status: "measured".to_owned(),
            clock: "monotonic".to_owned(),
            source: "scena_cli_stage_timer".to_owned(),
            prepare_ms: Some(prepare_ms),
            render_ms: Some(render_ms),
            capture_ms: Some(capture_ms),
            total_ms: Some(total_ms),
            unavailable_reason: None,
        }
    }
}

impl Default for RenderIntrospectionTimingsV1 {
    fn default() -> Self {
        Self::unavailable()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionReasonV1 {
    pub code: String,
    pub severity: String,
    #[serde(default)]
    pub affected_handles: Vec<u64>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionFixV1 {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<serde_json::Value>,
    pub help: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionRectV1 {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionLuminanceV1 {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub p05: f32,
    pub p50: f32,
    pub p95: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionFramingV1 {
    pub center_offset_fraction: [f32; 2],
    pub fit_fraction: f32,
    pub cropped: bool,
    pub tiny_in_frame: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_camera: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderIntrospectionNodesSummaryV1 {
    /// Every visible scene node, including cameras, lights, and empties.
    ///
    /// G05: this is **not** comparable to `drawn`. Use `visible_drawable`,
    /// which counts the population `drawn` is drawn from.
    pub visible: usize,
    pub hidden: usize,
    /// Visible nodes that can rasterize — the population `drawn` comes from.
    ///
    /// Added in 1.9.1; defaults to `0` when deserializing an older fixture.
    #[serde(default)]
    pub visible_drawable: usize,
    /// Length of the renderer's draw list. Excludes overlay and label draws.
    pub drawn: usize,
    pub culled: u64,
    pub transparent: u64,
    pub failed_material: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionNodeDetailV1 {
    pub handle: u64,
    pub kind: String,
    pub visible: bool,
    #[serde(default)]
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderIntrospectionArtifactsV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_png_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_descriptor_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_sheet_path: Option<String>,
    pub capture: RenderIntrospectionCaptureSummaryV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderIntrospectionCaptureSummaryV1 {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub payload_fnv1a64: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderIntrospectionCapabilitiesV1 {
    pub backend: Backend,
    pub gpu_device: bool,
    pub surface_attached: bool,
    pub hardware_tier: HardwareTier,
    pub forward_pbr: CapabilityStatus,
    pub readback_headless_screenshots: CapabilityStatus,
}

impl RenderIntrospectionOptions {
    pub const fn summary() -> Self {
        Self {
            detail: false,
            capture_png_path: None,
            capture_descriptor_path: None,
            contact_sheet_path: None,
            background_rgba8: DEFAULT_BACKGROUND_RGBA8,
            content_tolerance_rgba8: DEFAULT_CONTENT_TOLERANCE_RGBA8,
            timings: None,
        }
    }

    pub const fn detail() -> Self {
        Self {
            detail: true,
            capture_png_path: None,
            capture_descriptor_path: None,
            contact_sheet_path: None,
            background_rgba8: DEFAULT_BACKGROUND_RGBA8,
            content_tolerance_rgba8: DEFAULT_CONTENT_TOLERANCE_RGBA8,
            timings: None,
        }
    }

    pub fn with_capture_png_path(mut self, path: impl Into<String>) -> Self {
        self.capture_png_path = Some(path.into());
        self
    }

    pub fn with_capture_descriptor_path(mut self, path: impl Into<String>) -> Self {
        self.capture_descriptor_path = Some(path.into());
        self
    }

    pub fn with_contact_sheet_path(mut self, path: impl Into<String>) -> Self {
        self.contact_sheet_path = Some(path.into());
        self
    }

    /// Uses the supplied shader-encoded clear color for content detection.
    pub fn with_background_rgba8(mut self, background_rgba8: [u8; 4]) -> Self {
        self.background_rgba8 = background_rgba8;
        self
    }

    /// Uses this per-channel byte tolerance when comparing pixels to background.
    pub const fn with_content_tolerance_rgba8(mut self, tolerance: u8) -> Self {
        self.content_tolerance_rgba8 = tolerance;
        self
    }

    pub fn with_timings(mut self, timings: RenderIntrospectionTimingsV1) -> Self {
        self.timings = Some(timings);
        self
    }

    pub const fn detail_enabled(&self) -> bool {
        self.detail
    }
}

impl Default for RenderIntrospectionOptions {
    fn default() -> Self {
        Self::summary()
    }
}
