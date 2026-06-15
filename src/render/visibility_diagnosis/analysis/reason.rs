use super::super::{
    VisibilityDiagnosisEvidenceV1, VisibilityDiagnosisFixV1, VisibilityDiagnosisReasonV1,
};

pub(super) struct ReasonSpec {
    pub(super) code: &'static str,
    pub(super) severity: &'static str,
    pub(super) confidence: &'static str,
    pub(super) auto_fixable: bool,
    pub(super) affected_handles: Vec<u64>,
    pub(super) message: &'static str,
}

pub(super) fn has_error_reasons(reasons: &[VisibilityDiagnosisReasonV1]) -> bool {
    reasons.iter().any(|reason| reason.severity == "error")
}

pub(super) fn push_reason(reasons: &mut Vec<VisibilityDiagnosisReasonV1>, spec: ReasonSpec) {
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

pub(super) fn push_fix(
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

pub(super) fn push_evidence(
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
