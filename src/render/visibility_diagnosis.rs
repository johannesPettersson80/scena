mod analysis;

use serde::{Deserialize, Serialize};

use crate::diagnostics::RendererStats;
use crate::scene::SceneInspectionReportV1;

use super::Renderer;

pub const VISIBILITY_DIAGNOSIS_SCHEMA_V1: &str = "scena.visibility_diagnosis.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibilityDiagnosisOptions {
    detail: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibilityDiagnosisReportV1 {
    pub schema: String,
    pub ok: bool,
    pub target: VisibilityDiagnosisTargetV1,
    pub reasons: Vec<VisibilityDiagnosisReasonV1>,
    pub fixes: Vec<VisibilityDiagnosisFixV1>,
    pub summary: VisibilityDiagnosisSummaryV1,
    #[serde(default)]
    pub evidence: Vec<VisibilityDiagnosisEvidenceV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityDiagnosisTargetV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibilityDiagnosisReasonV1 {
    pub code: String,
    pub severity: String,
    pub confidence: String,
    pub auto_fixable: bool,
    #[serde(default)]
    pub affected_handles: Vec<u64>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibilityDiagnosisFixV1 {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<serde_json::Value>,
    pub risk: String,
    pub help: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityDiagnosisSummaryV1 {
    pub visible_nodes: usize,
    pub hidden_nodes: usize,
    pub visible_drawables: usize,
    pub culled_objects: u64,
    pub not_prepared: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibilityDiagnosisEvidenceV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<u64>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

impl VisibilityDiagnosisOptions {
    pub const fn summary() -> Self {
        Self { detail: false }
    }

    pub const fn detail() -> Self {
        Self { detail: true }
    }

    pub const fn detail_enabled(&self) -> bool {
        self.detail
    }
}

impl Default for VisibilityDiagnosisOptions {
    fn default() -> Self {
        Self::summary()
    }
}

impl VisibilityDiagnosisReportV1 {
    pub fn from_inspection(
        inspection: &SceneInspectionReportV1,
        stats: RendererStats,
        target_handle: Option<u64>,
        options: VisibilityDiagnosisOptions,
        prepared: bool,
    ) -> Self {
        analysis::from_inspection(inspection, stats, target_handle, options, prepared)
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("visibility diagnosis report is serializable")
    }
}

impl Renderer {
    pub fn diagnose_visibility(
        &self,
        inspection: &SceneInspectionReportV1,
        target_handle: Option<u64>,
        options: VisibilityDiagnosisOptions,
    ) -> VisibilityDiagnosisReportV1 {
        VisibilityDiagnosisReportV1::from_inspection(
            inspection,
            self.stats(),
            target_handle,
            options,
            self.prepared.is_some(),
        )
    }
}
