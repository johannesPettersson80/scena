#![cfg(feature = "inspection")]

use serde_json::json;

use scena::{
    AGENT_LOOP_RESULT_SCHEMA_V1, AgentLoopResultV1, RenderIntrospectionReportV1,
    VISUAL_REPAIR_PLAN_SCHEMA_V1, VisibilityDiagnosisReportV1, VisualRepairPlanV1,
};

#[test]
fn visual_repair_plans_presentation_and_reversible_content_repairs() {
    let render_report = render_introspection_with_frame_fix();
    let presentation = VisualRepairPlanV1::from_render_introspection(&render_report);
    assert_eq!(presentation.schema, VISUAL_REPAIR_PLAN_SCHEMA_V1);
    assert_eq!(presentation.status, "repairable");
    assert!(presentation.auto_fixable);
    assert_eq!(presentation.risk, "presentation");
    assert_eq!(presentation.root_cause.as_deref(), Some("empty_frame"));
    assert_eq!(presentation.applied_actions[0].action, "frame_bounds");
    assert_eq!(
        presentation.visual_patch,
        Some(json!({
            "schema": "scena.visual_patch.v1",
            "camera": {
                "target": [0.0, 0.0, 0.0],
                "distance": 2.8,
                "yaw_radians": 0.785,
                "pitch_radians": 0.524
            }
        }))
    );
    assert!(presentation.rerender_required);

    let diagnosis = hidden_node_diagnosis();
    let content = VisualRepairPlanV1::from_visibility_diagnosis(&diagnosis);
    assert_eq!(content.schema, VISUAL_REPAIR_PLAN_SCHEMA_V1);
    assert_eq!(content.status, "repairable");
    assert!(content.auto_fixable);
    assert_eq!(content.risk, "content");
    assert_eq!(content.root_cause.as_deref(), Some("node_hidden"));
    assert!(!content.requires_host_input);
    assert_eq!(content.applied_actions.len(), 1);
    assert_eq!(content.applied_actions[0].action, "set_visible");
    assert!(content.applied_actions[0].reversible);
    assert_eq!(
        content.applied_actions[0].before,
        Some(json!({"visibility": [{"node": 42, "visible": false}]}))
    );
    assert_eq!(
        content.visual_patch,
        Some(json!({
            "schema": "scena.visual_patch.v1",
            "visibility": [{"node": 42, "visible": true}]
        }))
    );
}

#[test]
fn visual_repair_skips_unsafe_content_repairs_and_reports_irreducible_loop() {
    let zero_scale = zero_scale_diagnosis();
    let plan = VisualRepairPlanV1::from_visibility_diagnosis(&zero_scale);
    assert_eq!(plan.status, "needs_host_input");
    assert!(!plan.auto_fixable);
    assert!(plan.requires_host_input);
    assert!(plan.applied_actions.is_empty());
    assert_eq!(plan.skipped_actions[0].action, "set_transform");
    assert_eq!(plan.skipped_actions[0].risk, "content");
    assert_eq!(plan.remaining_reasons[0].code, "zero_scale");

    let stale = stale_handle_diagnosis();
    let irreducible_plan = VisualRepairPlanV1::from_visibility_diagnosis(&stale);
    assert_eq!(irreducible_plan.status, "irreducible");
    assert!(irreducible_plan.requires_host_input);
    assert_eq!(irreducible_plan.remaining_reasons[0].code, "stale_handle");

    let loop_result = AgentLoopResultV1::irreducible(irreducible_plan, 3, 3);
    assert_eq!(loop_result.schema, AGENT_LOOP_RESULT_SCHEMA_V1);
    assert_eq!(loop_result.status, "irreducible");
    assert!(!loop_result.ok);
    assert_eq!(loop_result.iterations_used, 3);
    assert_eq!(loop_result.iteration_budget, 3);
    assert_eq!(loop_result.remaining_reasons[0].code, "stale_handle");
}

#[test]
fn visual_repair_golden_fixtures_match_live_schema() {
    let fixture: VisualRepairPlanV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/visual_repair_plan.v1.json"
    ))
    .expect("visual repair fixture deserializes");
    assert_eq!(fixture.schema, VISUAL_REPAIR_PLAN_SCHEMA_V1);
    assert_eq!(fixture.status, "repairable");
    assert!(fixture.auto_fixable);
    assert_eq!(fixture.root_cause.as_deref(), Some("node_hidden"));

    let loop_fixture: AgentLoopResultV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/agent_loop_result.v1.json"
    ))
    .expect("agent loop fixture deserializes");
    assert_eq!(loop_fixture.schema, AGENT_LOOP_RESULT_SCHEMA_V1);
    assert_eq!(loop_fixture.status, "irreducible");
    assert!(!loop_fixture.ok);
}

fn render_introspection_with_frame_fix() -> RenderIntrospectionReportV1 {
    serde_json::from_value(json!({
        "schema": "scena.render_introspection.v1",
        "ok": false,
        "reasons": [{
            "code": "empty_frame",
            "severity": "error",
            "affected_handles": [],
            "message": "rendered frame has no non-background pixels"
        }],
        "fixes": [{
            "action": "frame_bounds",
            "patch": {
                "camera": {
                    "target": [0.0, 0.0, 0.0],
                    "distance": 2.8,
                    "yaw_radians": 0.785,
                    "pitch_radians": 0.524
                }
            },
            "help": "frame the scene or target bounds before rendering again"
        }],
        "visible_pixel_fraction": 0.0,
        "luminance": {"min": 0.0, "max": 0.0, "mean": 0.0, "p05": 0.0, "p50": 0.0, "p95": 0.0},
        "framing": {
            "center_offset_fraction": [0.0, 0.0],
            "fit_fraction": 0.0,
            "cropped": false,
            "tiny_in_frame": false
        },
        "nodes_summary": {
            "visible": 1,
            "hidden": 0,
            "drawn": 1,
            "culled": 0,
            "transparent": 0,
            "failed_material": 0
        },
        "nodes_detail": [],
        "artifacts": {
            "capture": {
                "schema": "scena.capture.v1",
                "width": 64,
                "height": 64,
                "payload_fnv1a64": "0000000000000000"
            }
        },
        "capabilities": {
            "backend": "headless",
            "gpu_device": false,
            "surface_attached": false,
            "hardware_tier": "low",
            "forward_pbr": "degraded",
            "readback_headless_screenshots": "supported"
        }
    }))
    .expect("render introspection JSON builds")
}

fn hidden_node_diagnosis() -> VisibilityDiagnosisReportV1 {
    serde_json::from_value(json!({
        "schema": "scena.visibility_diagnosis.v1",
        "ok": false,
        "target": {"kind": "node", "handle": 42},
        "reasons": [{
            "code": "node_hidden",
            "severity": "error",
            "confidence": "high",
            "auto_fixable": true,
            "affected_handles": [42],
            "message": "target node is hidden"
        }],
        "fixes": [{
            "action": "set_visible",
            "target_handle": 42,
            "patch": {"visibility": [{"node": 42, "visible": true}]},
            "risk": "content",
            "help": "set the target node visible, then render and diagnose again"
        }],
        "summary": {
            "visible_nodes": 0,
            "hidden_nodes": 1,
            "visible_drawables": 0,
            "culled_objects": 0,
            "not_prepared": false
        },
        "evidence": []
    }))
    .expect("hidden-node diagnosis JSON builds")
}

fn zero_scale_diagnosis() -> VisibilityDiagnosisReportV1 {
    let mut diagnosis = hidden_node_diagnosis();
    diagnosis.reasons[0].code = "zero_scale".to_owned();
    diagnosis.reasons[0].message = "target node has a zero scale component".to_owned();
    diagnosis.fixes[0].action = "set_transform".to_owned();
    diagnosis.fixes[0].patch = Some(json!({
        "transforms": [{
            "node": 42,
            "transform": {
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }
        }]
    }));
    diagnosis.fixes[0].help =
        "replace the zero scale with a visible non-zero scale, then diagnose again".to_owned();
    diagnosis
}

fn stale_handle_diagnosis() -> VisibilityDiagnosisReportV1 {
    let mut diagnosis = hidden_node_diagnosis();
    diagnosis.target.handle = Some(999);
    diagnosis.reasons[0].code = "stale_handle".to_owned();
    diagnosis.reasons[0].auto_fixable = false;
    diagnosis.reasons[0].affected_handles = vec![999];
    diagnosis.reasons[0].message =
        "target handle is not present in the inspection report".to_owned();
    diagnosis.fixes.clear();
    diagnosis
}
