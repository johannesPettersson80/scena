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
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeQualityLineV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_intermediate_edge_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_straightness_error: Option<f64>,
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
