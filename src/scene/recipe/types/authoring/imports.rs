use serde::{Deserialize, Serialize};

use crate::scene::Transform;

use super::super::{default_true, is_false, is_true};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeImportV1 {
    pub id: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_extent: Option<SceneRecipeExpectedExtentV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<SceneRecipeImportMaterialV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_emphasis: Option<SceneRecipeImportEdgeEmphasisV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeExpectedExtentV1 {
    pub min: f64,
    pub max: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeImportMaterialV1 {
    pub base_color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeImportEdgeEmphasisV1 {
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_angle_threshold_degrees: Option<f64>,
}
