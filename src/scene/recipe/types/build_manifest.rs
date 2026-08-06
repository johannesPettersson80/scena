use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::scene::recipe::RecipeBuildPolicyReportV1;

use super::spatial_state::SceneRecipeSpatialTargetV1;

pub const RECIPE_BUILD_RESULT_SCHEMA_V1: &str = "scena.recipe_build_result.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBuildExecutionV1 {
    pub asset_fetches: u64,
    pub renderer_constructions: u64,
    pub gpu_context_constructions: u64,
    pub prepare_calls: u64,
    pub render_calls: u64,
    pub capture_constructions: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeBuildResultV1 {
    pub schema: String,
    pub ok: bool,
    pub build: SceneRecipeBuildV1,
    pub policy: RecipeBuildPolicyReportV1,
    pub execution: RecipeBuildExecutionV1,
}

impl RecipeBuildResultV1 {
    #[cfg(feature = "scene-host")]
    pub(crate) fn manifest_only(
        build: SceneRecipeBuildV1,
        policy: RecipeBuildPolicyReportV1,
        asset_fetches: u64,
    ) -> Self {
        Self {
            schema: RECIPE_BUILD_RESULT_SCHEMA_V1.to_owned(),
            ok: build.ok,
            build,
            policy,
            execution: RecipeBuildExecutionV1 {
                asset_fetches,
                renderer_constructions: 0,
                gpu_context_constructions: 0,
                prepare_calls: 0,
                render_calls: 0,
                capture_constructions: 0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeValidationReportV1 {
    pub schema: String,
    pub ok: bool,
    #[serde(default)]
    pub validation_mode: RecipeValidationModeV1,
    #[serde(default)]
    pub execution_equivalent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<Box<RecipeBuildPolicyReportV1>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<SceneRecipeResourceResolutionV1>,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeValidationModeV1 {
    #[default]
    SyntaxOnly,
    FullResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeResourceStatusV1 {
    NotChecked,
    Resolved,
    Builtin,
    Loaded,
    OptionalSkipped,
    ResolutionFailed,
    LoadFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeResourceResolutionV1 {
    pub path: String,
    pub kind: String,
    pub authored_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_uri: Option<String>,
    pub required: bool,
    pub status: SceneRecipeResourceStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeDiagnosticResourceV1 {
    pub kind: String,
    pub authored_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_uri: Option<String>,
    pub required: bool,
    pub allowed_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildV1 {
    pub schema: String,
    pub ok: bool,
    pub imports: Vec<SceneRecipeBuildImportV1>,
    pub nodes: Vec<SceneRecipeBuildTargetV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<SceneRecipeBuildInstanceV1>,
    pub cameras: Vec<SceneRecipeBuildTargetV1>,
    pub lights: Vec<SceneRecipeBuildTargetV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<SceneRecipeBuildAnimationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<SceneRecipeBuildAnchorV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<SceneRecipeBuildConnectorV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<SceneRecipeBuildConnectionV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<SceneRecipeBuildBoundsV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub named_states: Vec<SceneRecipeBuildNamedStateV1>,
    pub geometries: Vec<SceneRecipeBuildResourceV1>,
    pub materials: Vec<SceneRecipeBuildResourceV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<SceneRecipeBuildResourceV1>,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
    pub skipped: Vec<SceneRecipeBuildSkippedV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildAnchorV1 {
    pub id: String,
    pub identity_scope: String,
    pub source: String,
    pub target: SceneRecipeSpatialTargetV1,
    pub node_handle: u64,
    pub source_units: String,
    pub source_coordinate_system: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildConnectorV1 {
    pub id: String,
    pub identity_scope: String,
    pub source: String,
    pub target: SceneRecipeSpatialTargetV1,
    pub node_handle: u64,
    pub source_units: String,
    pub source_coordinate_system: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildConnectionV1 {
    pub source: String,
    pub target: String,
    pub status: String,
    pub snap_distance_scene_meters: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeBuildBoundsV1 {
    pub id: String,
    pub identity_scope: String,
    pub target: SceneRecipeSpatialTargetV1,
    pub source: String,
    pub space: String,
    pub units: String,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildNamedStateV1 {
    pub id: String,
    pub identity_scope: String,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_from: Option<String>,
    pub transform_count: usize,
    pub tint_count: usize,
    pub visibility_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildInstanceV1 {
    pub set_id: String,
    pub id: String,
    pub set_handle: u64,
    pub instance_id: u64,
    pub identity_scope: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_rounding: Option<SceneRecipeImportEdgeRoundingReportV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeImportEdgeRoundingReportV1 {
    pub enabled: bool,
    pub inspected_meshes: usize,
    pub rounded_meshes: usize,
    pub skipped_meshes: usize,
    pub eligible_edges: usize,
    pub rounded_edges: usize,
    pub skipped_edges: usize,
    pub rejected_edges: usize,
    #[serde(default)]
    pub removed_degenerate_triangles: usize,
    pub source_triangles: usize,
    pub derived_triangles: usize,
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
    pub visible: Option<bool>,
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
    /// Deterministically ranked valid names for lookup failures.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    #[serde(default)]
    pub auto_fixable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<SceneRecipeDiagnosticResourceV1>,
}

impl SceneRecipeDiagnosticV1 {
    pub(crate) fn with_candidates(mut self, candidates: Vec<String>) -> Self {
        if self.suggestion.is_none() {
            self.suggestion = candidates.first().cloned();
        }
        self.candidates = candidates;
        self
    }
}
