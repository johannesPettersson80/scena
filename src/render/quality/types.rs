use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{RenderIntrospectionCapabilitiesV1, RenderIntrospectionRectV1};

pub const RENDER_QUALITY_SCHEMA_V1: &str = "scena.render_quality.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderQualityReportV1 {
    pub schema: String,
    pub ok: bool,
    pub profile: String,
    pub summary: RenderQualitySummaryV1,
    pub checks: Vec<RenderQualityCheckV1>,
    pub capabilities: RenderIntrospectionCapabilitiesV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderQualitySummaryV1 {
    pub checks: usize,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderQualityCheckV1 {
    pub id: String,
    pub code: String,
    pub status: RenderQualityStatusV1,
    pub severity: String,
    pub region: RenderQualityRegionV1,
    pub observed: BTreeMap<String, f32>,
    pub threshold: BTreeMap<String, f32>,
    pub fix_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderQualityStatusV1 {
    Checked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderQualityRegionV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect_css_px: Option<RenderIntrospectionRectV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderQualityProfile {
    Product,
    Documentation,
    Cad,
    Dashboard,
    Twin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderQualityRegion {
    pub kind: &'static str,
    pub handle: Option<u64>,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityFrameMetrics {
    pub low_clip_fraction: f32,
    pub high_clip_fraction: f32,
    pub clipped_highlight_fraction: f32,
    pub p01: f32,
    pub p05: f32,
    pub p50: f32,
    pub p95: f32,
    pub p99: f32,
    pub luminance_range: f32,
    pub luminance_stddev: f32,
    pub sobel_energy: f32,
    pub noise_outlier_fraction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityLabelMetrics {
    pub ink_coverage: f32,
    pub ink_isolation: f32,
    pub intermediate_edge_fraction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityLabelBackgroundMetrics {
    pub luminance_range: f32,
    pub mean_rgb_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityLineMetrics {
    pub ink_coverage: f32,
    pub intermediate_edge_fraction: f32,
    pub straightness_error: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityGridLineMetrics {
    pub intermediate_px_per_edge: f32,
    pub unique_luma_levels: usize,
    pub transition_width_px: f32,
    pub halo_overshoot: f32,
    pub contrast_range: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityAreaLightMetrics {
    pub shadow_contrast: f32,
    pub penumbra_width_px: f32,
    pub penumbra_luma_levels: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityGeometryEdgeMetrics {
    pub intermediate_edge_fraction: f32,
    pub edge_candidate_fraction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityDepthOfFieldMetrics {
    pub source_background_sobel: f32,
    pub focused_background_sobel: f32,
    pub background_sobel_drop: f32,
    pub background_sobel_drop_fraction: f32,
    pub focal_mean_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceQualityMetrics {
    pub mean_abs_diff: f32,
    pub mean_delta_e2000: f32,
    pub ssim: f32,
}

impl RenderQualityReportV1 {
    pub(super) fn from_checks(
        profile: RenderQualityProfile,
        capabilities: RenderIntrospectionCapabilitiesV1,
        checks: Vec<RenderQualityCheckV1>,
    ) -> Self {
        let errors = checks
            .iter()
            .filter(|check| check.severity == "error")
            .count();
        let warnings = checks
            .iter()
            .filter(|check| check.severity == "warning")
            .count();
        Self {
            schema: RENDER_QUALITY_SCHEMA_V1.to_owned(),
            ok: errors == 0,
            profile: profile.as_str().to_owned(),
            summary: RenderQualitySummaryV1 {
                checks: checks.len(),
                errors,
                warnings,
            },
            checks,
            capabilities,
        }
    }
}

impl RenderQualityProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "product" => Some(Self::Product),
            "documentation" => Some(Self::Documentation),
            "cad" => Some(Self::Cad),
            "dashboard" => Some(Self::Dashboard),
            "twin" => Some(Self::Twin),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Documentation => "documentation",
            Self::Cad => "cad",
            Self::Dashboard => "dashboard",
            Self::Twin => "twin",
        }
    }

    pub(super) const fn severe_black_crush_max(self) -> f32 {
        match self {
            Self::Documentation | Self::Dashboard | Self::Cad => 0.55,
            Self::Product | Self::Twin => 0.80,
        }
    }

    pub(super) const fn severe_high_clip_max(self) -> f32 {
        match self {
            Self::Documentation | Self::Dashboard => 0.35,
            Self::Product | Self::Cad | Self::Twin => 0.65,
        }
    }

    pub(super) const fn default_min_luminance_range(self) -> f32 {
        match self {
            Self::Cad => 0.08,
            Self::Documentation | Self::Dashboard => 0.16,
            Self::Product | Self::Twin => 0.12,
        }
    }

    pub const fn default_min_geometry_intermediate_edge_fraction(self) -> f32 {
        match self {
            Self::Product => 0.25,
            Self::Documentation | Self::Cad | Self::Twin => 0.02,
            Self::Dashboard => 0.01,
        }
    }
}

impl RenderQualityRegion {
    pub const fn full_frame(width: u32, height: u32) -> Self {
        Self {
            kind: "frame",
            handle: None,
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    pub(super) fn to_report(self) -> RenderQualityRegionV1 {
        RenderQualityRegionV1 {
            kind: self.kind.to_owned(),
            handle: self.handle,
            rect_css_px: Some(RenderIntrospectionRectV1 {
                min_x: round3(self.x as f32),
                min_y: round3(self.y as f32),
                max_x: round3(self.x.saturating_add(self.width) as f32),
                max_y: round3(self.y.saturating_add(self.height) as f32),
                width: round3(self.width as f32),
                height: round3(self.height as f32),
            }),
        }
    }
}

pub(super) fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
