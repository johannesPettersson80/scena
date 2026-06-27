use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSectionBoxV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    #[serde(default)]
    pub margin: f32,
    #[serde(default)]
    pub inverted: bool,
    #[serde(default)]
    pub helper_wireframe: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeTargetV1 {
    Node { id: String },
    Import { id: String },
    World { position: [f32; 3] },
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
    Node {
        id: String,
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

fn default_exploded_factor() -> f32 {
    1.0
}

fn default_exploded_distance() -> f32 {
    1.0
}
