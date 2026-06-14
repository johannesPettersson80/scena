use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::diagnostics::RendererStats;
use crate::scene::{SceneInspectionReportV1, SceneNodeInspectionV1, Transform};

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
        let target = VisibilityDiagnosisTargetV1 {
            kind: target_handle.map_or("scene", |_| "node").to_owned(),
            handle: target_handle,
        };
        let visible_nodes = inspection.nodes.iter().filter(|node| node.visible).count();
        let hidden_nodes = inspection.nodes.len().saturating_sub(visible_nodes);
        let summary = VisibilityDiagnosisSummaryV1 {
            visible_nodes,
            hidden_nodes,
            visible_drawables: inspection.counts.visible_drawable,
            culled_objects: stats.culled_objects,
            not_prepared: !prepared,
        };
        let mut reasons = Vec::new();
        let mut fixes = Vec::new();
        let mut evidence = Vec::new();

        if !prepared {
            push_reason(
                &mut reasons,
                ReasonSpec {
                    code: "not_prepared",
                    severity: "error",
                    confidence: "high",
                    auto_fixable: true,
                    affected_handles: Vec::new(),
                    message: "renderer has not prepared this scene",
                },
            );
            push_fix(
                &mut fixes,
                "prepare",
                None,
                None,
                "presentation",
                "call prepare_with_assets before rendering or diagnosing visibility",
            );
        }

        if inspection.active_camera.is_none() {
            push_reason(
                &mut reasons,
                ReasonSpec {
                    code: "missing_camera",
                    severity: "error",
                    confidence: "high",
                    auto_fixable: true,
                    affected_handles: Vec::new(),
                    message: "inspection report has no active camera",
                },
            );
            push_fix(
                &mut fixes,
                "set_camera",
                None,
                None,
                "presentation",
                "create or select an active camera before rendering again",
            );
        }

        if inspection.counts.visible_drawable == 0 {
            push_reason(
                &mut reasons,
                ReasonSpec {
                    code: "no_visible_drawables",
                    severity: "error",
                    confidence: "high",
                    auto_fixable: false,
                    affected_handles: Vec::new(),
                    message: "inspection reports no visible drawable nodes",
                },
            );
        }

        let visible_drawables = inspection.counts.visible_drawable as u64;
        let all_visible_drawables_culled =
            visible_drawables > 0 && stats.culled_objects >= visible_drawables;
        if all_visible_drawables_culled {
            push_reason(
                &mut reasons,
                ReasonSpec {
                    code: "all_culled",
                    severity: "error",
                    confidence: "medium",
                    auto_fixable: true,
                    affected_handles: Vec::new(),
                    message: "renderer stats report every visible drawable was culled for this frame",
                },
            );
            push_fix(
                &mut fixes,
                "frame_bounds",
                None,
                None,
                "presentation",
                "frame the target bounds and render again",
            );
        }

        if let Some(handle) = target_handle {
            match inspection.nodes.iter().find(|node| node.handle == handle) {
                Some(node) => diagnose_node(node, &mut reasons, &mut fixes, &mut evidence, options),
                None => {
                    push_reason(
                        &mut reasons,
                        ReasonSpec {
                            code: "stale_handle",
                            severity: "error",
                            confidence: "high",
                            auto_fixable: false,
                            affected_handles: vec![handle],
                            message: "target handle is not present in the inspection report",
                        },
                    );
                    if options.detail_enabled() {
                        push_evidence(
                            &mut evidence,
                            "handle_lookup",
                            Some(handle),
                            "no node row has this stable handle",
                            None,
                        );
                    }
                }
            }
        }

        Self {
            schema: VISIBILITY_DIAGNOSIS_SCHEMA_V1.to_owned(),
            ok: !has_error_reasons(&reasons),
            target,
            reasons,
            fixes,
            summary,
            evidence,
        }
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

fn has_error_reasons(reasons: &[VisibilityDiagnosisReasonV1]) -> bool {
    reasons.iter().any(|reason| reason.severity == "error")
}

fn diagnose_node(
    node: &SceneNodeInspectionV1,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
    evidence: &mut Vec<VisibilityDiagnosisEvidenceV1>,
    options: VisibilityDiagnosisOptions,
) {
    if !node.visible {
        push_reason(
            reasons,
            ReasonSpec {
                code: "node_hidden",
                severity: "error",
                confidence: "high",
                auto_fixable: true,
                affected_handles: vec![node.handle],
                message: "target node is hidden",
            },
        );
        push_fix(
            fixes,
            "set_visible",
            Some(node.handle),
            Some(json!({
                "visibility": [
                    {
                        "node": node.handle,
                        "visible": true
                    }
                ]
            })),
            "content",
            "set the target node visible, then render and diagnose again",
        );
    }

    if is_zero_scale(node.local_transform) || is_zero_scale(node.world_transform) {
        push_reason(
            reasons,
            ReasonSpec {
                code: "zero_scale",
                severity: "error",
                confidence: "high",
                auto_fixable: true,
                affected_handles: vec![node.handle],
                message: "target node has a zero scale component",
            },
        );
        push_fix(
            fixes,
            "set_transform",
            Some(node.handle),
            Some(json!({
                "transforms": [
                    {
                        "node": node.handle,
                        "translation": [
                            node.local_transform.translation.x,
                            node.local_transform.translation.y,
                            node.local_transform.translation.z
                        ],
                        "rotation": [
                            node.local_transform.rotation.x,
                            node.local_transform.rotation.y,
                            node.local_transform.rotation.z,
                            node.local_transform.rotation.w
                        ],
                        "scale": [1.0, 1.0, 1.0]
                    }
                ]
            })),
            "content",
            "replace the zero scale with a visible non-zero scale, then diagnose again",
        );
    }

    if options.detail_enabled() {
        push_evidence(
            evidence,
            "node_state",
            Some(node.handle),
            "inspection row used for target diagnosis",
            Some(json!({
                "visible": node.visible,
                "kind": node.kind,
                "local_scale": [
                    node.local_transform.scale.x,
                    node.local_transform.scale.y,
                    node.local_transform.scale.z
                ],
                "world_scale": [
                    node.world_transform.scale.x,
                    node.world_transform.scale.y,
                    node.world_transform.scale.z
                ]
            })),
        );
    }
}

fn is_zero_scale(transform: Transform) -> bool {
    const EPSILON: f32 = 1.0e-6;
    transform.scale.x.abs() <= EPSILON
        || transform.scale.y.abs() <= EPSILON
        || transform.scale.z.abs() <= EPSILON
}

struct ReasonSpec {
    code: &'static str,
    severity: &'static str,
    confidence: &'static str,
    auto_fixable: bool,
    affected_handles: Vec<u64>,
    message: &'static str,
}

fn push_reason(reasons: &mut Vec<VisibilityDiagnosisReasonV1>, spec: ReasonSpec) {
    if reasons.iter().any(|reason| reason.code == spec.code) {
        return;
    }
    reasons.push(VisibilityDiagnosisReasonV1 {
        code: spec.code.to_owned(),
        severity: spec.severity.to_owned(),
        confidence: spec.confidence.to_owned(),
        auto_fixable: spec.auto_fixable,
        affected_handles: spec.affected_handles,
        message: spec.message.to_owned(),
    });
}

fn push_fix(
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
    action: &str,
    target_handle: Option<u64>,
    patch: Option<serde_json::Value>,
    risk: &str,
    help: &str,
) {
    if fixes
        .iter()
        .any(|fix| fix.action == action && fix.target_handle == target_handle)
    {
        return;
    }
    fixes.push(VisibilityDiagnosisFixV1 {
        action: action.to_owned(),
        target_handle,
        patch,
        risk: risk.to_owned(),
        help: help.to_owned(),
    });
}

fn push_evidence(
    evidence: &mut Vec<VisibilityDiagnosisEvidenceV1>,
    kind: &str,
    handle: Option<u64>,
    message: &str,
    value: Option<serde_json::Value>,
) {
    evidence.push(VisibilityDiagnosisEvidenceV1 {
        kind: kind.to_owned(),
        handle,
        message: message.to_owned(),
        value,
    });
}
