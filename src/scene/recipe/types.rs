use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scene::Transform;

pub const SCENE_RECIPE_SCHEMA_V1: &str = "scena.scene_recipe.v1";
pub const SCENE_RECIPE_VALIDATION_SCHEMA_V1: &str = "scena.scene_recipe_validation.v1";
pub const SCENE_RECIPE_BUILD_SCHEMA_V1: &str = "scena.scene_recipe_build.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeV1 {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<SceneRecipeImportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_box: Option<SceneRecipeSectionBoxV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measurements: Vec<SceneRecipeMeasurementV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callouts: Vec<SceneRecipeCalloutV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploded_view: Option<SceneRecipeExplodedViewV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<SceneRecipeCaptureV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

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
pub struct SceneRecipeSectionBoxV1 {
    pub import: String,
    #[serde(default)]
    pub margin: f32,
    #[serde(default)]
    pub inverted: bool,
    #[serde(default)]
    pub helper_wireframe: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeMeasurementV1 {
    pub id: String,
    pub kind: String,
    pub start: [f32; 3],
    pub end: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeCalloutV1 {
    pub id: String,
    pub text: String,
    pub target: SceneRecipeCalloutTargetV1,
    #[serde(default)]
    pub label_offset: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeCalloutTargetV1 {
    ImportRoot {
        import: String,
        #[serde(default)]
        local_offset: [f32; 3],
    },
    World {
        position: [f32; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeExplodedViewV1 {
    pub import: String,
    #[serde(default)]
    pub mode: SceneRecipeExplodedViewModeV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<[f32; 3]>,
    #[serde(default = "default_exploded_factor")]
    pub factor: f32,
    #[serde(default = "default_exploded_distance")]
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeExplodedViewModeV1 {
    #[default]
    DirectChildren,
    HierarchyDepth,
    Axis,
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
pub struct SceneRecipeBuildV1 {
    pub schema: String,
    pub ok: bool,
    pub imports: Vec<SceneRecipeBuildImportV1>,
    pub nodes: Vec<SceneRecipeBuildTargetV1>,
    pub cameras: Vec<SceneRecipeBuildTargetV1>,
    pub lights: Vec<SceneRecipeBuildTargetV1>,
    pub geometries: Vec<SceneRecipeBuildResourceV1>,
    pub materials: Vec<SceneRecipeBuildResourceV1>,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
    pub skipped: Vec<SceneRecipeBuildSkippedV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildImportV1 {
    pub id: String,
    pub uri: String,
    pub import_handle: u64,
    pub root_handles: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_root: Option<u64>,
    pub nodes_by_path: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildTargetV1 {
    pub id: String,
    pub handle: u64,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildResourceV1 {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildSkippedV1 {
    pub path: String,
    pub id: String,
    pub kind: String,
    pub reason: String,
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

fn default_exploded_factor() -> f32 {
    1.0
}

fn default_exploded_distance() -> f32 {
    1.0
}

fn is_false(value: &bool) -> bool {
    !*value
}
