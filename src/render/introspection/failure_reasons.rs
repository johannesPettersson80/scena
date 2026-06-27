use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::scene::{SceneInspectionReportV1, Transform};
use serde_json::json;

use super::{
    RenderIntrospectionFixV1, RenderIntrospectionReasonV1, push_fix, push_frame_bounds_fix,
    push_reason,
};

pub(super) fn push_failure_reasons(
    reasons: &mut Vec<RenderIntrospectionReasonV1>,
    fixes: &mut Vec<RenderIntrospectionFixV1>,
    inspection: &SceneInspectionReportV1,
    visible_pixels: u64,
    diagnostics: &[Diagnostic],
) {
    push_inspection_failure_reasons(reasons, fixes, inspection, visible_pixels);
    push_renderer_diagnostic_reasons(reasons, fixes, inspection, diagnostics);
}

fn push_inspection_failure_reasons(
    reasons: &mut Vec<RenderIntrospectionReasonV1>,
    fixes: &mut Vec<RenderIntrospectionFixV1>,
    inspection: &SceneInspectionReportV1,
    visible_pixels: u64,
) {
    for node in &inspection.nodes {
        if node.visible
            && (!transform_is_finite(node.local_transform)
                || !transform_is_finite(node.world_transform))
        {
            push_reason(
                reasons,
                "nan_transform",
                "error",
                vec![node.handle],
                "a visible node has a non-finite transform component",
            );
            push_fix(
                fixes,
                "set_transform",
                Some(node.handle),
                Some(json!({
                    "transforms": [{
                        "node": node.handle,
                        "transform": Transform::IDENTITY
                    }]
                })),
                "replace non-finite transforms with finite values before rendering again",
            );
        }
    }

    for draw in &inspection.draw_list {
        let Some(material) = draw.material.as_ref() else {
            continue;
        };
        if material.base_color.a <= 1.0e-6 {
            push_reason(
                reasons,
                "alpha_zero",
                "error",
                vec![draw.node],
                "a drawable material has zero base-color alpha",
            );
        }
    }

    if visible_pixels == 0 && inspection.counts.clipping_planes > 0 {
        push_reason(
            reasons,
            "clipped_by_active_clipping_plane",
            "error",
            inspection.draw_list.iter().map(|draw| draw.node).collect(),
            "active clipping planes may be removing all visible content",
        );
        push_fix(
            fixes,
            "clear_clipping_planes",
            None,
            Some(json!({"section_box": {"mode": "disable"}})),
            "clear active clipping planes or widen the section box before rendering again",
        );
    }
}

fn push_renderer_diagnostic_reasons(
    reasons: &mut Vec<RenderIntrospectionReasonV1>,
    fixes: &mut Vec<RenderIntrospectionFixV1>,
    inspection: &SceneInspectionReportV1,
    diagnostics: &[Diagnostic],
) {
    for diagnostic in diagnostics {
        match diagnostic.code() {
            DiagnosticCode::ObjectsBehindCamera => {
                push_reason(
                    reasons,
                    "behind_camera",
                    "error",
                    Vec::new(),
                    "renderer diagnostics report all visible bounds behind the camera",
                );
                push_frame_bounds_fix(
                    fixes,
                    "move or frame the camera so visible content is in front of it",
                    inspection,
                );
            }
            DiagnosticCode::SceneOutsideCameraFrustum => {
                push_reason(
                    reasons,
                    "outside_frustum",
                    "error",
                    Vec::new(),
                    "renderer diagnostics report visible bounds outside the camera frustum",
                );
                push_frame_bounds_fix(
                    fixes,
                    "frame the scene or target bounds before rendering again",
                    inspection,
                );
            }
            code if backend_degradation_code(code) => {
                push_reason(
                    reasons,
                    "backend_capability_degraded",
                    "warning",
                    Vec::new(),
                    "renderer diagnostics report a degraded or disabled backend capability",
                );
                push_fix(
                    fixes,
                    "inspect_capabilities",
                    None,
                    None,
                    "inspect the capability report before relying on this visual result",
                );
            }
            _ => {}
        }
    }
}

fn transform_is_finite(transform: Transform) -> bool {
    transform.translation.is_finite()
        && transform.rotation.is_finite()
        && transform.scale.is_finite()
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
