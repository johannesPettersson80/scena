use serde::{Deserialize, Serialize};

use super::SceneRecipeTargetV1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeExpectV1 {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_visible: Vec<SceneRecipeVisibleExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_color: Vec<SceneRecipeColorExpectationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_bbox_fit: Option<SceneRecipeBboxFitExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_target_fit: Vec<SceneRecipeTargetFitExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_grounded: Vec<SceneRecipeGroundedExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_helper_occluded: Vec<SceneRecipeHelperOcclusionExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_occlusion: Vec<SceneRecipeOcclusionExpectationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_backend: Option<SceneRecipeBackendExpectationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_clipping: Option<SceneRecipeClippingExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_state: Vec<SceneRecipeStateExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_transform: Vec<SceneRecipeTransformExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_separation: Vec<SceneRecipeSeparationExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_pick: Vec<SceneRecipePickExpectationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_quality: Option<SceneRecipeQualityExpectationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect_reference: Vec<SceneRecipeReferenceExpectationV1>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub expect_no_warnings: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeVisibleExpectationV1 {
    pub id: String,
    pub target: SceneRecipeTargetV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeColorExpectationV1 {
    pub id: String,
    pub target: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swatch_srgb8: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_source_material: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_base_color_texture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeBboxFitExpectationV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeTargetBoundsV1 {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeTargetRegionV1 {
    pub bounds: SceneRecipeTargetBoundsV1,
    pub centroid: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeTargetFitExpectationV1 {
    pub id: String,
    pub target: SceneRecipeTargetV1,
    pub bounds: SceneRecipeTargetBoundsV1,
    pub centroid: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_fit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_visible_coverage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeGroundedExpectationV1 {
    pub id: String,
    pub target: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plane_y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeHelperOcclusionExpectationV1 {
    pub id: String,
    pub helper: SceneRecipeTargetV1,
    pub occluder: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_pixels: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeOcclusionExpectationV1 {
    pub id: String,
    pub front: SceneRecipeTargetV1,
    pub back: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_pixels: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeBackendExpectationV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_device: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeClippingExpectationV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_clipping_planes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_box_active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_box_inverted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeStateExpectationV1 {
    pub id: String,
    pub import: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_material_variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeTransformExpectationV1 {
    pub id: String,
    pub target: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_degrees: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_tolerance_degrees: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSeparationExpectationV1 {
    pub id: String,
    pub a: SceneRecipeTargetV1,
    pub b: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_gap: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePickExpectationV1 {
    pub id: String,
    pub x_css_px: f64,
    pub y_css_px: f64,
    pub target: SceneRecipeTargetV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityExpectationV1 {
    pub profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<SceneRecipeQualityExposureV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<SceneRecipeQualityContrastV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<SceneRecipeQualityNoiseV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<SceneRecipeQualityTextV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<SceneRecipeQualityLineV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<SceneRecipeQualityGeometryV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflection: Option<SceneRecipeQualityReflectionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_light: Option<SceneRecipeQualityAreaLightV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounding: Option<SceneRecipeQualityGroundingV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_of_field: Option<SceneRecipeQualityDepthOfFieldV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityExposureV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_low_clip_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_high_clip_fraction: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityContrastV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_luminance_range: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_sobel_energy: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityNoiseV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_outlier_fraction: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityTextV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_ink_coverage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ink_isolation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_intermediate_edge_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_background_luminance_range: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_background_mean_delta: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityLineV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_intermediate_edge_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_straightness_error: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityGeometryV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_intermediate_edge_fraction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityReflectionV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_luminance_range: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_sobel_energy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_chroma_range: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_firefly_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_bright_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_dark_fraction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityAreaLightV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_shadow_contrast: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_penumbra_width_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_penumbra_luma_levels: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_emitter_extent_meters: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityGroundingV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_contact_shadow_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityDepthOfFieldV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_source_background_sobel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_background_sobel_drop: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_background_sobel_drop_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_focal_mean_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeReferenceExpectationV1 {
    pub id: String,
    pub image: String,
    pub metric: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_ssim: Option<f64>,
}

fn is_false(value: &bool) -> bool {
    !*value
}
