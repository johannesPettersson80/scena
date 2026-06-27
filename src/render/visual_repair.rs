use serde::{Deserialize, Serialize};
use serde_json::Value;

mod actions;
use actions::{
    aggregate_visual_patch, patchless_presentation_skip, plan_risk, presentation_action,
    reversible_content_action, risk_for_render_fix,
};

use super::introspection::{RenderIntrospectionReasonV1, RenderIntrospectionReportV1};
use super::visibility_diagnosis::{VisibilityDiagnosisReasonV1, VisibilityDiagnosisReportV1};

pub const VISUAL_REPAIR_PLAN_SCHEMA_V1: &str = "scena.visual_repair_plan.v1";
pub const AGENT_LOOP_RESULT_SCHEMA_V1: &str = "scena.agent_loop_result.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualRepairPlanV1 {
    pub schema: String,
    pub status: String,
    pub auto_fixable: bool,
    pub confidence: String,
    pub risk: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_cause: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_patch: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_update: Option<Value>,
    #[serde(default)]
    pub applied_actions: Vec<VisualRepairActionV1>,
    #[serde(default)]
    pub skipped_actions: Vec<VisualRepairSkippedActionV1>,
    #[serde(default)]
    pub remaining_reasons: Vec<VisualRepairRemainingReasonV1>,
    pub requires_host_input: bool,
    pub rerender_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualRepairActionV1 {
    pub action: String,
    pub source: String,
    pub risk: String,
    pub confidence: String,
    pub root_cause: String,
    pub auto_fixable: bool,
    pub reversible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<Value>,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualRepairSkippedActionV1 {
    pub action: String,
    pub source: String,
    pub risk: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_cause: Option<String>,
    pub reason: String,
    pub auto_fixable: bool,
    pub requires_host_input: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_handle: Option<u64>,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRepairRemainingReasonV1 {
    pub code: String,
    pub source: String,
    pub severity: String,
    pub auto_fixable: bool,
    #[serde(default)]
    pub affected_handles: Vec<u64>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentLoopResultV1 {
    pub schema: String,
    pub status: String,
    pub ok: bool,
    pub iterations_used: u32,
    pub iteration_budget: u32,
    pub auto_fixable: bool,
    pub requires_host_input: bool,
    pub confidence: String,
    pub reason: String,
    pub plan_status: String,
    #[serde(default)]
    pub remaining_reasons: Vec<VisualRepairRemainingReasonV1>,
    #[serde(default)]
    pub skipped_actions: Vec<VisualRepairSkippedActionV1>,
}

impl VisualRepairPlanV1 {
    pub fn from_visibility_diagnosis(report: &VisibilityDiagnosisReportV1) -> Self {
        let reasons = report
            .reasons
            .iter()
            .map(ReasonView::from_visibility)
            .collect::<Vec<_>>();
        let mut planner = RepairPlanner::new("visibility_diagnosis", reasons);

        for fix in &report.fixes {
            let root_cause = planner.root_cause_for_fix(fix.target_handle);
            let risk = fix.risk.as_str();
            if risk == "presentation" {
                if let Some(action) = presentation_action(
                    "visibility_diagnosis",
                    &fix.action,
                    risk,
                    fix.target_handle,
                    fix.patch.clone(),
                    fix.help.clone(),
                    root_cause.as_ref(),
                ) {
                    planner.apply(action);
                } else {
                    planner.skip(patchless_presentation_skip(
                        "visibility_diagnosis",
                        &fix.action,
                        risk,
                        fix.target_handle,
                        fix.help.clone(),
                        root_cause.as_ref(),
                    ));
                }
                continue;
            }

            if risk == "content" {
                if let Some(action) =
                    reversible_content_action("visibility_diagnosis", fix, root_cause.as_ref())
                {
                    planner.apply(action);
                } else {
                    planner.skip(VisualRepairSkippedActionV1 {
                        action: fix.action.clone(),
                        source: "visibility_diagnosis".to_owned(),
                        risk: risk.to_owned(),
                        root_cause: root_cause.as_ref().map(|reason| reason.code.clone()),
                        reason: "content repair is not safely reversible from the diagnosis report"
                            .to_owned(),
                        auto_fixable: false,
                        requires_host_input: true,
                        target_handle: fix.target_handle,
                        help: fix.help.clone(),
                    });
                }
            }
        }

        planner.finish()
    }

    pub fn from_render_introspection(report: &RenderIntrospectionReportV1) -> Self {
        let reasons = report
            .reasons
            .iter()
            .map(ReasonView::from_introspection)
            .collect::<Vec<_>>();
        let mut planner = RepairPlanner::new("render_introspection", reasons);

        for fix in &report.fixes {
            let root_cause = planner.root_cause_for_fix(fix.target_handle);
            let risk = risk_for_render_fix(&fix.action);
            if risk == "presentation" {
                if let Some(action) = presentation_action(
                    "render_introspection",
                    &fix.action,
                    risk,
                    fix.target_handle,
                    fix.patch.clone(),
                    fix.help.clone(),
                    root_cause.as_ref(),
                ) {
                    planner.apply(action);
                } else {
                    planner.skip(patchless_presentation_skip(
                        "render_introspection",
                        &fix.action,
                        risk,
                        fix.target_handle,
                        fix.help.clone(),
                        root_cause.as_ref(),
                    ));
                }
            } else {
                planner.skip(VisualRepairSkippedActionV1 {
                    action: fix.action.clone(),
                    source: "render_introspection".to_owned(),
                    risk: risk.to_owned(),
                    root_cause: root_cause.as_ref().map(|reason| reason.code.clone()),
                    reason: "render introspection did not provide a reversible content patch"
                        .to_owned(),
                    auto_fixable: false,
                    requires_host_input: true,
                    target_handle: fix.target_handle,
                    help: fix.help.clone(),
                });
            }
        }

        planner.finish()
    }

    pub fn to_schema_json(&self) -> Value {
        serde_json::to_value(self).expect("visual repair plan is serializable")
    }
}

impl AgentLoopResultV1 {
    pub fn irreducible(
        plan: VisualRepairPlanV1,
        iterations_used: u32,
        iteration_budget: u32,
    ) -> Self {
        Self {
            schema: AGENT_LOOP_RESULT_SCHEMA_V1.to_owned(),
            status: "irreducible".to_owned(),
            ok: false,
            iterations_used,
            iteration_budget,
            auto_fixable: false,
            requires_host_input: true,
            confidence: plan.confidence,
            reason:
                "repair did not converge within the iteration budget or has no safe automatic fix"
                    .to_owned(),
            plan_status: plan.status,
            remaining_reasons: plan.remaining_reasons,
            skipped_actions: plan.skipped_actions,
        }
    }

    pub fn to_schema_json(&self) -> Value {
        serde_json::to_value(self).expect("agent loop result is serializable")
    }
}

struct RepairPlanner {
    source: &'static str,
    reasons: Vec<ReasonView>,
    applied_actions: Vec<VisualRepairActionV1>,
    skipped_actions: Vec<VisualRepairSkippedActionV1>,
}

impl RepairPlanner {
    fn new(source: &'static str, reasons: Vec<ReasonView>) -> Self {
        Self {
            source,
            reasons,
            applied_actions: Vec::new(),
            skipped_actions: Vec::new(),
        }
    }

    fn root_cause_for_fix(&self, target_handle: Option<u64>) -> Option<ReasonView> {
        self.reasons
            .iter()
            .find(|reason| {
                reason.severity == "error"
                    && target_handle.is_some_and(|handle| {
                        reason.affected_handles.is_empty()
                            || reason.affected_handles.contains(&handle)
                    })
            })
            .or_else(|| {
                self.reasons
                    .iter()
                    .find(|reason| reason.severity == "error")
            })
            .or_else(|| self.reasons.first())
            .cloned()
    }

    fn apply(&mut self, action: VisualRepairActionV1) {
        self.applied_actions.push(action);
    }

    fn skip(&mut self, action: VisualRepairSkippedActionV1) {
        self.skipped_actions.push(action);
    }

    fn finish(self) -> VisualRepairPlanV1 {
        let covered = self
            .applied_actions
            .iter()
            .map(|action| action.root_cause.as_str())
            .collect::<Vec<_>>();
        let remaining_reasons = self
            .reasons
            .iter()
            .filter(|reason| reason.severity == "error" && !covered.contains(&reason.code.as_str()))
            .map(|reason| reason.to_remaining(self.source))
            .collect::<Vec<_>>();
        let has_applied = !self.applied_actions.is_empty();
        let has_skipped = !self.skipped_actions.is_empty();
        let has_remaining = !remaining_reasons.is_empty();
        let status = if has_applied && !has_skipped && !has_remaining {
            "repairable"
        } else if has_applied {
            "partial"
        } else if has_skipped {
            "needs_host_input"
        } else {
            "irreducible"
        };
        let visual_patch = aggregate_visual_patch(&self.applied_actions);
        let requires_host_input =
            has_skipped || has_remaining || !has_applied || visual_patch.is_none();
        let auto_fixable = has_applied && !requires_host_input;
        let risk = plan_risk(&self.applied_actions, &self.skipped_actions);
        let root_cause = self
            .applied_actions
            .first()
            .map(|action| action.root_cause.clone())
            .or_else(|| remaining_reasons.first().map(|reason| reason.code.clone()))
            .or_else(|| {
                self.skipped_actions
                    .first()
                    .and_then(|action| action.root_cause.clone())
            });
        let confidence = self
            .applied_actions
            .first()
            .map(|action| action.confidence.clone())
            .or_else(|| {
                self.reasons
                    .iter()
                    .find(|reason| reason.severity == "error")
                    .map(|reason| reason.confidence.clone())
            })
            .unwrap_or_else(|| "medium".to_owned());
        VisualRepairPlanV1 {
            schema: VISUAL_REPAIR_PLAN_SCHEMA_V1.to_owned(),
            status: status.to_owned(),
            auto_fixable,
            confidence,
            risk,
            root_cause,
            visual_patch,
            recipe_update: None,
            applied_actions: self.applied_actions,
            skipped_actions: self.skipped_actions,
            remaining_reasons,
            requires_host_input,
            rerender_required: has_applied,
        }
    }
}

#[derive(Clone)]
struct ReasonView {
    code: String,
    severity: String,
    confidence: String,
    auto_fixable: bool,
    affected_handles: Vec<u64>,
    message: String,
}

impl ReasonView {
    fn from_visibility(reason: &VisibilityDiagnosisReasonV1) -> Self {
        Self {
            code: reason.code.clone(),
            severity: reason.severity.clone(),
            confidence: reason.confidence.clone(),
            auto_fixable: reason.auto_fixable,
            affected_handles: reason.affected_handles.clone(),
            message: reason.message.clone(),
        }
    }

    fn from_introspection(reason: &RenderIntrospectionReasonV1) -> Self {
        Self {
            code: reason.code.clone(),
            severity: reason.severity.clone(),
            confidence: if reason.severity == "error" {
                "high".to_owned()
            } else {
                "medium".to_owned()
            },
            auto_fixable: reason.severity == "error",
            affected_handles: reason.affected_handles.clone(),
            message: reason.message.clone(),
        }
    }

    fn to_remaining(&self, source: &str) -> VisualRepairRemainingReasonV1 {
        VisualRepairRemainingReasonV1 {
            code: self.code.clone(),
            source: source.to_owned(),
            severity: self.severity.clone(),
            auto_fixable: self.auto_fixable,
            affected_handles: self.affected_handles.clone(),
            message: self.message.clone(),
        }
    }
}
