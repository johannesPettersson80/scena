use serde_json::{Value, json};

use super::super::visibility_diagnosis::VisibilityDiagnosisFixV1;
use super::{ReasonView, VisualRepairActionV1, VisualRepairSkippedActionV1};

pub(super) fn reversible_content_action(
    source: &str,
    fix: &VisibilityDiagnosisFixV1,
    root_cause: Option<&ReasonView>,
) -> Option<VisualRepairActionV1> {
    if fix.action != "set_visible" {
        return None;
    }
    let patch = fix.patch.clone()?;
    let entry = patch.get("visibility")?.as_array()?.first()?;
    let node = entry.get("node")?.as_u64()?;
    let visible = entry.get("visible")?.as_bool()?;
    if !visible {
        return None;
    }
    let before = json!({
        "visibility": [
            {
                "node": node,
                "visible": false
            }
        ]
    });
    Some(VisualRepairActionV1 {
        action: fix.action.clone(),
        source: source.to_owned(),
        risk: "content".to_owned(),
        confidence: root_cause
            .map(|reason| reason.confidence.clone())
            .unwrap_or_else(|| "medium".to_owned()),
        root_cause: root_cause
            .map(|reason| reason.code.clone())
            .unwrap_or_else(|| "node_hidden".to_owned()),
        auto_fixable: true,
        reversible: true,
        target_handle: fix.target_handle,
        before: Some(before),
        after: Some(patch.clone()),
        patch: Some(patch),
        help: fix.help.clone(),
    })
}

pub(super) fn presentation_action(
    source: &str,
    action: &str,
    risk: &str,
    target_handle: Option<u64>,
    patch: Option<Value>,
    help: String,
    root_cause: Option<&ReasonView>,
) -> Option<VisualRepairActionV1> {
    let patch = patch?;
    Some(VisualRepairActionV1 {
        action: action.to_owned(),
        source: source.to_owned(),
        risk: risk.to_owned(),
        confidence: root_cause
            .map(|reason| reason.confidence.clone())
            .unwrap_or_else(|| "medium".to_owned()),
        root_cause: root_cause
            .map(|reason| reason.code.clone())
            .unwrap_or_else(|| action.to_owned()),
        auto_fixable: true,
        reversible: false,
        target_handle,
        before: None,
        after: Some(patch.clone()),
        patch: Some(patch),
        help,
    })
}

pub(super) fn patchless_presentation_skip(
    source: &str,
    action: &str,
    risk: &str,
    target_handle: Option<u64>,
    help: String,
    root_cause: Option<&ReasonView>,
) -> VisualRepairSkippedActionV1 {
    VisualRepairSkippedActionV1 {
        action: action.to_owned(),
        source: source.to_owned(),
        risk: risk.to_owned(),
        root_cause: root_cause.map(|reason| reason.code.clone()),
        reason: "presentation repair did not include a machine-applicable visual patch".to_owned(),
        auto_fixable: false,
        requires_host_input: true,
        target_handle,
        help,
    }
}

pub(super) fn risk_for_render_fix(action: &str) -> &'static str {
    match action {
        "frame_bounds" | "set_camera" | "prepare" | "resize_capture" | "set_view_preset" => {
            "presentation"
        }
        _ => "content",
    }
}

pub(super) fn plan_risk(
    applied: &[VisualRepairActionV1],
    skipped: &[VisualRepairSkippedActionV1],
) -> String {
    if applied.iter().any(|action| action.risk == "content")
        || skipped.iter().any(|action| action.risk == "content")
    {
        "content".to_owned()
    } else if applied.iter().any(|action| action.risk == "presentation")
        || skipped.iter().any(|action| action.risk == "presentation")
    {
        "presentation".to_owned()
    } else {
        "irreducible".to_owned()
    }
}

pub(super) fn aggregate_visual_patch(actions: &[VisualRepairActionV1]) -> Option<Value> {
    let mut object = serde_json::Map::new();
    object.insert("schema".to_owned(), json!("scena.visual_patch.v1"));
    let mut has_patch = false;
    for action in actions {
        let Some(Value::Object(patch)) = &action.patch else {
            continue;
        };
        for (key, value) in patch {
            if key == "schema" {
                continue;
            }
            has_patch = true;
            match (object.get_mut(key), value) {
                (Some(Value::Array(existing)), Value::Array(next)) => {
                    existing.extend(next.iter().cloned());
                }
                _ => {
                    object.insert(key.clone(), value.clone());
                }
            }
        }
    }
    has_patch.then_some(Value::Object(object))
}
