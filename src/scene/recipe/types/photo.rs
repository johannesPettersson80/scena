use serde::{Deserialize, Serialize};

use super::subject::SceneRecipeSubjectV1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePhotoV1 {
    pub intent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<SceneRecipePhotoSubjectV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<SceneRecipePhotoCompositionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<SceneRecipePhotoExposureV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<SceneRecipePhotoFocusV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<SceneRecipePhotoStagingV1>,
}

pub type SceneRecipePhotoSubjectV1 = SceneRecipeSubjectV1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePhotoCompositionV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_fraction: Option<SceneRecipePhotoRangeV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_center_offset_fraction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePhotoExposureV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metering: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_luminance_srgb8: Option<SceneRecipePhotoRangeV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_low_clip_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_high_clip_fraction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePhotoFocusV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePhotoStagingV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePhotoRangeV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}
