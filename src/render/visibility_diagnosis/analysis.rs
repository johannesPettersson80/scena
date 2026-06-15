mod node;
mod reason;

use crate::diagnostics::{Diagnostic, DiagnosticCode, RendererStats};
use crate::scene::{SceneImportInspectionV1, SceneInspectionReportV1, SceneNodeInspectionV1};

use super::{
    VISIBILITY_DIAGNOSIS_SCHEMA_V1, VisibilityDiagnosisEvidenceV1, VisibilityDiagnosisFixV1,
    VisibilityDiagnosisOptions, VisibilityDiagnosisReasonV1, VisibilityDiagnosisReportV1,
    VisibilityDiagnosisSummaryV1, VisibilityDiagnosisTargetV1,
};
use reason::{ReasonSpec, has_error_reasons, push_evidence, push_fix, push_reason};

pub(super) fn from_inspection(
    inspection: &SceneInspectionReportV1,
    stats: RendererStats,
    target_handle: Option<u64>,
    options: VisibilityDiagnosisOptions,
    prepared: bool,
    diagnostics: &[Diagnostic],
) -> VisibilityDiagnosisReportV1 {
    let mut target = VisibilityDiagnosisTargetV1 {
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

    diagnose_renderer_diagnostics(diagnostics, target_handle, &mut reasons, &mut fixes);
    diagnose_active_clipping_planes(inspection, target_handle, &mut reasons, &mut fixes);

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
        match target_for_handle(inspection, handle) {
            VisibilityTarget::Node(node) => node::diagnose_node(
                node,
                inspection,
                &mut reasons,
                &mut fixes,
                &mut evidence,
                options,
            ),
            VisibilityTarget::Import(import) => {
                target.kind = "import".to_owned();
                diagnose_import(
                    import,
                    inspection,
                    &mut reasons,
                    &mut fixes,
                    &mut evidence,
                    options,
                );
            }
            VisibilityTarget::Missing => {
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

    VisibilityDiagnosisReportV1 {
        schema: VISIBILITY_DIAGNOSIS_SCHEMA_V1.to_owned(),
        ok: !has_error_reasons(&reasons),
        target,
        reasons,
        fixes,
        summary,
        evidence,
    }
}

fn diagnose_renderer_diagnostics(
    diagnostics: &[Diagnostic],
    target_handle: Option<u64>,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
) {
    for diagnostic in diagnostics {
        match diagnostic.code() {
            DiagnosticCode::ObjectsBehindCamera => {
                push_reason(
                    reasons,
                    ReasonSpec {
                        code: "behind_camera",
                        severity: "error",
                        confidence: "high",
                        auto_fixable: true,
                        affected_handles: target_handle.into_iter().collect(),
                        message: "renderer diagnostics report visible bounds behind the camera",
                    },
                );
                push_fix(
                    fixes,
                    "frame_bounds",
                    target_handle,
                    None,
                    "presentation",
                    "move or frame the camera so visible content is in front of it",
                );
            }
            DiagnosticCode::SceneOutsideCameraFrustum => {
                push_reason(
                    reasons,
                    ReasonSpec {
                        code: "outside_frustum",
                        severity: "error",
                        confidence: "high",
                        auto_fixable: true,
                        affected_handles: target_handle.into_iter().collect(),
                        message: "renderer diagnostics report visible bounds outside the camera frustum",
                    },
                );
                push_fix(
                    fixes,
                    "frame_bounds",
                    target_handle,
                    None,
                    "presentation",
                    "frame the scene or target bounds before rendering again",
                );
            }
            code if backend_degradation_code(code) => {
                push_reason(
                    reasons,
                    ReasonSpec {
                        code: "backend_capability_degraded",
                        severity: "warning",
                        confidence: "medium",
                        auto_fixable: false,
                        affected_handles: target_handle.into_iter().collect(),
                        message: "renderer diagnostics report a degraded or disabled backend capability",
                    },
                );
                push_fix(
                    fixes,
                    "inspect_capabilities",
                    None,
                    None,
                    "presentation",
                    "inspect the capability report before relying on this visual result",
                );
            }
            _ => {}
        }
    }
}

fn diagnose_active_clipping_planes(
    inspection: &SceneInspectionReportV1,
    target_handle: Option<u64>,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
) {
    if inspection.counts.clipping_planes == 0 {
        return;
    }
    push_reason(
        reasons,
        ReasonSpec {
            code: "clipped_by_active_clipping_plane",
            severity: "warning",
            confidence: "medium",
            auto_fixable: true,
            affected_handles: target_handle.into_iter().collect(),
            message: "active clipping planes may be hiding the target",
        },
    );
    push_fix(
        fixes,
        "clear_clipping_planes",
        target_handle,
        None,
        "presentation",
        "clear active clipping planes or widen the section box, then render again",
    );
}

fn backend_degradation_code(code: DiagnosticCode) -> bool {
    matches!(
        code,
        DiagnosticCode::WebGl2DepthCompatibility
            | DiagnosticCode::ForwardPbrDegraded
            | DiagnosticCode::DirectionalShadowsDegraded
            | DiagnosticCode::PointShadowsDisabled
            | DiagnosticCode::SpotShadowsDisabled
            | DiagnosticCode::BloomDisabled
            | DiagnosticCode::AmbientOcclusionDisabled
            | DiagnosticCode::OrderIndependentTransparencyDisabled
            | DiagnosticCode::PhysicalGlassTransmissionDegraded
            | DiagnosticCode::WideGamutOutputUnavailable
            | DiagnosticCode::GpuCullingDisabled
    )
}

enum VisibilityTarget<'a> {
    Node(&'a SceneNodeInspectionV1),
    Import(&'a SceneImportInspectionV1),
    Missing,
}

fn target_for_handle(inspection: &SceneInspectionReportV1, handle: u64) -> VisibilityTarget<'_> {
    if let Some(import) = inspection
        .imports
        .as_ref()
        .and_then(|imports| imports.iter().find(|import| import.handle == handle))
    {
        return VisibilityTarget::Import(import);
    }
    if let Some(node) = inspection.nodes.iter().find(|node| node.handle == handle) {
        return VisibilityTarget::Node(node);
    }
    VisibilityTarget::Missing
}

fn diagnose_import(
    import: &SceneImportInspectionV1,
    inspection: &SceneInspectionReportV1,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
    evidence: &mut Vec<VisibilityDiagnosisEvidenceV1>,
    options: VisibilityDiagnosisOptions,
) {
    if import.root_handles.is_empty() {
        push_reason(
            reasons,
            ReasonSpec {
                code: "import_has_no_roots",
                severity: "error",
                confidence: "high",
                auto_fixable: false,
                affected_handles: vec![import.handle],
                message: "target import has no registered root nodes",
            },
        );
        return;
    }

    let mut diagnosed_any_root = false;
    for root in &import.root_handles {
        if let Some(node) = inspection.nodes.iter().find(|node| node.handle == *root) {
            diagnosed_any_root = true;
            node::diagnose_node(node, inspection, reasons, fixes, evidence, options);
        }
    }
    if !diagnosed_any_root {
        push_reason(
            reasons,
            ReasonSpec {
                code: "import_roots_stale",
                severity: "error",
                confidence: "high",
                auto_fixable: false,
                affected_handles: import.root_handles.clone(),
                message: "target import roots are not present in the inspection report",
            },
        );
    }
}
