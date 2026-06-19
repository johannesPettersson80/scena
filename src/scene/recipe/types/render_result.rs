use serde::{Deserialize, Serialize};

use super::SceneRecipeBuildV1;
use crate::{
    AppearanceIntrospectionReportV1, CaptureDescriptor, InteractionVerificationReportV1,
    RenderIntrospectionReportV1, RenderQualityReportV1,
};

pub const SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1: &str = "scena.recipe_render_result.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeRenderResultV1 {
    pub schema: String,
    pub ok: bool,
    pub build: SceneRecipeBuildV1,
    pub capture: Option<CaptureDescriptor>,
    pub introspection: Option<RenderIntrospectionReportV1>,
    pub verification: SceneRecipeVerificationReportV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeVerificationReportV1 {
    pub ok: bool,
    pub summary: SceneRecipeVerificationSummaryV1,
    pub reasons: Vec<SceneRecipeVerificationReasonV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<AppearanceIntrospectionReportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<InteractionVerificationReportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<RenderQualityReportV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeVerificationSummaryV1 {
    pub render_checks: usize,
    pub appearance_targets: usize,
    pub interaction_steps: usize,
    pub quality_checks: usize,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeVerificationReasonV1 {
    pub code: String,
    pub severity: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expectation_id: Option<String>,
    #[serde(default)]
    pub affected_handles: Vec<u64>,
    pub message: String,
}

impl SceneRecipeRenderResultV1 {
    pub fn new(
        build: SceneRecipeBuildV1,
        capture: CaptureDescriptor,
        introspection: RenderIntrospectionReportV1,
        verification: SceneRecipeVerificationReportV1,
    ) -> Self {
        let ok = build.ok && introspection.ok && verification.ok;
        Self {
            schema: SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1.to_owned(),
            ok,
            build,
            capture: Some(capture),
            introspection: Some(introspection),
            verification,
        }
    }

    pub fn build_failed(build: SceneRecipeBuildV1) -> Self {
        let verification = SceneRecipeVerificationReportV1::new(
            0,
            vec![SceneRecipeVerificationReasonV1 {
                code: "build_failed".to_owned(),
                severity: "error".to_owned(),
                source: "build".to_owned(),
                expectation_id: None,
                affected_handles: Vec::new(),
                message: "recipe build failed before render verification could run".to_owned(),
            }],
            None,
            None,
            None,
        );
        Self {
            schema: SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1.to_owned(),
            ok: false,
            build,
            capture: None,
            introspection: None,
            verification,
        }
    }
}

impl SceneRecipeVerificationReportV1 {
    pub fn new(
        render_checks: usize,
        reasons: Vec<SceneRecipeVerificationReasonV1>,
        appearance: Option<AppearanceIntrospectionReportV1>,
        interaction: Option<InteractionVerificationReportV1>,
        quality: Option<RenderQualityReportV1>,
    ) -> Self {
        let appearance_targets = appearance
            .as_ref()
            .map(|report| report.summary.targets)
            .unwrap_or(0);
        let interaction_steps = interaction
            .as_ref()
            .map(|report| report.summary.step_count)
            .unwrap_or(0);
        let quality_checks = quality
            .as_ref()
            .map(|report| report.summary.checks)
            .unwrap_or(0);
        let errors = reasons
            .iter()
            .filter(|reason| reason.severity == "error")
            .count();
        let warnings = reasons
            .iter()
            .filter(|reason| reason.severity == "warning")
            .count();
        Self {
            ok: errors == 0,
            summary: SceneRecipeVerificationSummaryV1 {
                render_checks,
                appearance_targets,
                interaction_steps,
                quality_checks,
                errors,
                warnings,
            },
            reasons,
            appearance,
            interaction,
            quality,
        }
    }
}
