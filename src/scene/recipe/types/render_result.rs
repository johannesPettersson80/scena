use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::SceneRecipeBuildV1;
use crate::{
    AppearanceIntrospectionReportV1, CaptureDescriptor, InteractionVerificationReportV1,
    RenderIntrospectionRectV1, RenderIntrospectionReportV1, RenderQualityReportV1,
};

pub const SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1: &str = "scena.recipe_render_result.v1";
pub const SCENE_COMPOSITION_SCHEMA_V1: &str = "scena.scene_composition.v1";

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
    pub composition: Option<SceneCompositionReportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<RenderQualityReportV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeVerificationSummaryV1 {
    pub render_checks: usize,
    pub appearance_targets: usize,
    pub interaction_steps: usize,
    pub composition_checks: usize,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneCompositionReportV1 {
    pub schema: String,
    pub ok: bool,
    pub summary: SceneCompositionSummaryV1,
    pub checks: Vec<SceneCompositionCheckV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneCompositionSummaryV1 {
    pub checks: usize,
    pub checked: usize,
    pub failed: usize,
    pub skipped: usize,
    pub unsupported: usize,
    pub not_applicable: usize,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneCompositionCheckV1 {
    pub id: String,
    pub category: String,
    pub code: String,
    pub status: SceneCompositionStatusV1,
    pub severity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<SceneCompositionRegionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default)]
    pub affected_handles: Vec<u64>,
    #[serde(default)]
    pub observed: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Value>,
    pub message: String,
    pub fix_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneCompositionStatusV1 {
    Checked,
    Failed,
    SkippedNoDeclaredIntent,
    SkippedNoBackendSupport,
    SkippedImportUnknown,
    Unsupported,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneCompositionRegionV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect_css_px: Option<RenderIntrospectionRectV1>,
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
        composition: Option<SceneCompositionReportV1>,
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
        let composition_checks = composition
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
                composition_checks,
                quality_checks,
                errors,
                warnings,
            },
            reasons,
            appearance,
            interaction,
            composition,
            quality,
        }
    }
}

impl SceneCompositionReportV1 {
    pub fn new(checks: Vec<SceneCompositionCheckV1>) -> Self {
        let checked = checks
            .iter()
            .filter(|check| check.status == SceneCompositionStatusV1::Checked)
            .count();
        let failed = checks
            .iter()
            .filter(|check| check.status == SceneCompositionStatusV1::Failed)
            .count();
        let skipped = checks
            .iter()
            .filter(|check| {
                matches!(
                    check.status,
                    SceneCompositionStatusV1::SkippedNoDeclaredIntent
                        | SceneCompositionStatusV1::SkippedNoBackendSupport
                        | SceneCompositionStatusV1::SkippedImportUnknown
                )
            })
            .count();
        let unsupported = checks
            .iter()
            .filter(|check| check.status == SceneCompositionStatusV1::Unsupported)
            .count();
        let not_applicable = checks
            .iter()
            .filter(|check| check.status == SceneCompositionStatusV1::NotApplicable)
            .count();
        let errors = checks
            .iter()
            .filter(|check| check.severity == "error")
            .count();
        let warnings = checks
            .iter()
            .filter(|check| check.severity == "warning")
            .count();
        Self {
            schema: SCENE_COMPOSITION_SCHEMA_V1.to_owned(),
            ok: failed == 0 && errors == 0,
            summary: SceneCompositionSummaryV1 {
                checks: checks.len(),
                checked,
                failed,
                skipped,
                unsupported,
                not_applicable,
                errors,
                warnings,
            },
            checks,
        }
    }
}
