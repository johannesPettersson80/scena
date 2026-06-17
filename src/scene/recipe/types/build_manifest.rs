use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeValidationReportV1 {
    pub schema: String,
    pub ok: bool,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildV1 {
    pub schema: String,
    pub ok: bool,
    pub imports: Vec<SceneRecipeBuildImportV1>,
    pub nodes: Vec<SceneRecipeBuildTargetV1>,
    pub cameras: Vec<SceneRecipeBuildTargetV1>,
    pub lights: Vec<SceneRecipeBuildTargetV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<SceneRecipeBuildAnimationV1>,
    pub geometries: Vec<SceneRecipeBuildResourceV1>,
    pub materials: Vec<SceneRecipeBuildResourceV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<SceneRecipeBuildResourceV1>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildAnimationV1 {
    pub id: String,
    pub handle: u64,
    pub duration_seconds: f32,
    pub channel_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildResourceV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertex_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_count: Option<usize>,
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
