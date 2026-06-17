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

fn is_false(value: &bool) -> bool {
    !*value
}
