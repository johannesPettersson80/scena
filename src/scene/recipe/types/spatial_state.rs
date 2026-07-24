use serde::{Deserialize, Serialize};

use super::{SceneRecipeTransformV1, is_false};

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeSpatialTargetV1 {
    Node { id: String },
    ImportRoot { id: String },
    ImportNode { import: String, path: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeAnchorV1 {
    pub id: String,
    pub source: SceneRecipeAnchorSourceV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeAnchorSourceV1 {
    Authored {
        target: SceneRecipeSpatialTargetV1,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transform: Option<SceneRecipeTransformV1>,
    },
    Import {
        import: String,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeConnectorV1 {
    pub id: String,
    pub source: SceneRecipeConnectorSourceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_mates: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snap_tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance_hint: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll_policy: Option<SceneRecipeConnectorRollPolicyV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polarity: Option<SceneRecipeConnectorPolarityV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mate: Option<SceneRecipeConnectorMateV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeConnectorSourceV1 {
    Authored {
        target: SceneRecipeSpatialTargetV1,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transform: Option<SceneRecipeTransformV1>,
    },
    Import {
        import: String,
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeConnectorRollPolicyV1 {
    Preserve,
    ChooseNearest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeConnectorPolarityV1 {
    Plug,
    Socket,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeConnectorMateV1 {
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<SceneRecipeConnectorAlignmentV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll: Option<SceneRecipeConnectionRollV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parenting: Option<SceneRecipeConnectionParentingV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axial_gap: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeConnectorAlignmentV1 {
    ForwardToForward,
    ForwardToBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeConnectionRollV1 {
    MatchTarget,
    PreserveSource,
    ChooseNearest { step_degrees: f64 },
    Explicit { degrees: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeConnectionParentingV1 {
    PreserveSourceParent,
    ReparentSourceToTargetParent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeBoundsV1 {
    pub id: String,
    pub target: SceneRecipeSpatialTargetV1,
    pub source: SceneRecipeBoundsSourceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeBoundsSourceV1 {
    Computed,
    Imported,
    Authored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeNamedStateV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub active: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transforms: Vec<SceneRecipeStateTransformV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tints: Vec<SceneRecipeStateTintV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visibility: Vec<SceneRecipeStateVisibilityV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeStateTransformV1 {
    pub target: SceneRecipeSpatialTargetV1,
    pub transform: SceneRecipeTransformV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeStateTintV1 {
    pub target: SceneRecipeSpatialTargetV1,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeStateVisibilityV1 {
    pub target: SceneRecipeSpatialTargetV1,
    pub visible: bool,
}
