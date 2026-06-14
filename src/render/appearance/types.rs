use serde::{Deserialize, Serialize};

use crate::scene::SceneMaterialInspectionV1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceIntrospectionOptions {
    pub(super) detail: bool,
    pub(super) background_rgba8: [u8; 4],
    pub(super) content_tolerance_rgba8: u8,
    pub(super) active_variant: Option<String>,
    pub(super) available_variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceExpectationV1 {
    pub schema: String,
    #[serde(default)]
    pub targets: Vec<AppearanceTargetExpectationV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearanceTargetExpectationV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swatch_srgb8: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_mode: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_source_material: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_base_color_texture: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceIntrospectionReportV1 {
    pub schema: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_variant: Option<String>,
    #[serde(default)]
    pub available_variants: Vec<String>,
    pub summary: AppearanceSummaryV1,
    pub targets: Vec<AppearanceTargetReportV1>,
    pub reasons: Vec<AppearanceReasonV1>,
    pub fixes: Vec<AppearanceFixV1>,
    pub artifacts: AppearanceArtifactsV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AppearanceSummaryV1 {
    pub targets: usize,
    pub matched: usize,
    pub errors: usize,
    pub warnings: usize,
    pub sampled_pixels: u64,
    pub luminance_mean: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceTargetReportV1 {
    pub id: String,
    pub matched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<SceneMaterialInspectionV1>,
    pub sampled_region: AppearanceSampleRegionV1,
    pub sampled_color_srgb8: [u8; 4],
    pub sampled_color_family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swatch_distance: Option<f32>,
    pub alpha: AppearanceAlphaSummaryV1,
    pub expected: AppearanceTargetExpectationV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceSampleRegionV1 {
    pub kind: String,
    pub sampled_pixels: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox_css_px: Option<AppearanceRectV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AppearanceRectV1 {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceAlphaSummaryV1 {
    pub mode: String,
    pub base_color_alpha: f32,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearanceReasonV1 {
    pub code: String,
    pub severity: String,
    pub target_id: String,
    #[serde(default)]
    pub affected_handles: Vec<u64>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceFixV1 {
    pub action: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<serde_json::Value>,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceArtifactsV1 {
    pub capture: AppearanceCaptureSummaryV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearanceCaptureSummaryV1 {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub payload_fnv1a64: String,
}

impl AppearanceIntrospectionOptions {
    pub const fn summary() -> Self {
        Self {
            detail: false,
            background_rgba8: [0, 0, 0, 255],
            content_tolerance_rgba8: 2,
            active_variant: None,
            available_variants: Vec::new(),
        }
    }

    pub const fn detail() -> Self {
        Self {
            detail: true,
            background_rgba8: [0, 0, 0, 255],
            content_tolerance_rgba8: 2,
            active_variant: None,
            available_variants: Vec::new(),
        }
    }

    pub fn with_active_material_variant(mut self, variant: Option<String>) -> Self {
        self.active_variant = variant;
        self
    }

    pub fn with_available_material_variants(mut self, variants: Vec<String>) -> Self {
        self.available_variants = variants;
        self
    }

    pub fn with_background_rgba8(mut self, background_rgba8: [u8; 4]) -> Self {
        self.background_rgba8 = background_rgba8;
        self
    }

    pub const fn with_content_tolerance_rgba8(mut self, tolerance: u8) -> Self {
        self.content_tolerance_rgba8 = tolerance;
        self
    }

    pub const fn detail_enabled(&self) -> bool {
        self.detail
    }
}

impl Default for AppearanceIntrospectionOptions {
    fn default() -> Self {
        Self::summary()
    }
}

const fn is_false(value: &bool) -> bool {
    !*value
}
