use serde::{Deserialize, Serialize};

use super::overlays::SceneRecipeTargetV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeSubjectFallbackPolicyV1 {
    Error,
    AverageMeteringWithWarning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSubjectSpecV1 {
    pub target: SceneRecipeTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<SceneRecipeSubjectFallbackPolicyV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SceneRecipeSubjectV1 {
    Spec(SceneRecipeSubjectSpecV1),
    Target(SceneRecipeTargetV1),
}

impl SceneRecipeSubjectV1 {
    pub const fn target(&self) -> &SceneRecipeTargetV1 {
        match self {
            Self::Spec(spec) => &spec.target,
            Self::Target(target) => target,
        }
    }

    pub fn fallback(&self) -> SceneRecipeSubjectFallbackPolicyV1 {
        match self {
            Self::Spec(spec) => spec
                .fallback
                .unwrap_or(SceneRecipeSubjectFallbackPolicyV1::Error),
            Self::Target(_) => SceneRecipeSubjectFallbackPolicyV1::Error,
        }
    }
}
