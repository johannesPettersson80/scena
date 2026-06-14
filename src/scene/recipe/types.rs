use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scene::Transform;

pub const SCENE_RECIPE_SCHEMA_V1: &str = "scena.scene_recipe.v1";
pub const SCENE_RECIPE_VALIDATION_SCHEMA_V1: &str = "scena.scene_recipe_validation.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeV1 {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<SceneRecipeImportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<SceneRecipeCaptureV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeImportV1 {
    pub id: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_extent: Option<SceneRecipeExpectedExtentV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeExpectedExtentV1 {
    pub min: f64,
    pub max: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeCaptureV1 {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeValidationReportV1 {
    pub schema: String,
    pub ok: bool,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeDiagnosticV1 {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub help: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default)]
    pub auto_fixable: bool,
}
