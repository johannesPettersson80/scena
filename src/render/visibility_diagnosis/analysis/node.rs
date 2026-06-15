use std::collections::BTreeMap;

use serde_json::json;

use crate::scene::{SceneInspectionReportV1, SceneNodeInspectionV1, Transform};

use super::super::{
    VisibilityDiagnosisEvidenceV1, VisibilityDiagnosisFixV1, VisibilityDiagnosisOptions,
    VisibilityDiagnosisReasonV1,
};
use super::reason::{ReasonSpec, push_evidence, push_fix, push_reason};

pub(super) fn diagnose_node(
    node: &SceneNodeInspectionV1,
    inspection: &SceneInspectionReportV1,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
    evidence: &mut Vec<VisibilityDiagnosisEvidenceV1>,
    options: VisibilityDiagnosisOptions,
) {
    diagnose_single_node(node, inspection, reasons, fixes, evidence, options);
    for descendant in descendants_of(node.handle, inspection) {
        diagnose_single_node(descendant, inspection, reasons, fixes, evidence, options);
    }
}

fn diagnose_single_node(
    node: &SceneNodeInspectionV1,
    inspection: &SceneInspectionReportV1,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
    evidence: &mut Vec<VisibilityDiagnosisEvidenceV1>,
    options: VisibilityDiagnosisOptions,
) {
    let hidden_ancestor = hidden_ancestor(node, inspection);
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

    if let Some(ancestor) = hidden_ancestor {
        push_reason(
            reasons,
            ReasonSpec {
                code: "parent_hidden",
                severity: "error",
                confidence: "high",
                auto_fixable: true,
                affected_handles: vec![node.handle, ancestor.handle],
                message: "an ancestor of the target node is hidden",
            },
        );
        push_fix(
            fixes,
            "set_visible",
            Some(ancestor.handle),
            Some(json!({
                "visibility": [
                    {
                        "node": ancestor.handle,
                        "visible": true
                    }
                ]
            })),
            "content",
            "set the hidden ancestor visible, then render and diagnose again",
        );
        if options.detail_enabled() {
            push_evidence(
                evidence,
                "visibility_ancestor",
                Some(ancestor.handle),
                "hidden ancestor prevents the target subtree from drawing",
                Some(json!({
                    "target": node.handle,
                    "ancestor": ancestor.handle,
                    "ancestor_kind": ancestor.kind
                })),
            );
        }
    }

    if !transform_is_finite(node.local_transform) || !transform_is_finite(node.world_transform) {
        push_reason(
            reasons,
            ReasonSpec {
                code: "nan_transform",
                severity: "error",
                confidence: "high",
                auto_fixable: true,
                affected_handles: vec![node.handle],
                message: "target node has a non-finite transform component",
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
                        "transform": Transform::IDENTITY
                    }
                ]
            })),
            "content",
            "replace the non-finite transform with a finite transform, then diagnose again",
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
                        "transform": {
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
                    }
                ]
            })),
            "content",
            "replace the zero scale with a visible non-zero scale, then diagnose again",
        );
    }

    diagnose_draw_and_material(node, inspection, reasons, fixes);

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

fn descendants_of(root: u64, inspection: &SceneInspectionReportV1) -> Vec<&SceneNodeInspectionV1> {
    let mut descendants = Vec::new();
    for node in &inspection.nodes {
        let mut current = node.parent;
        while let Some(parent) = current {
            if parent == root {
                descendants.push(node);
                break;
            }
            current = inspection
                .nodes
                .iter()
                .find(|candidate| candidate.handle == parent)
                .and_then(|candidate| candidate.parent);
        }
    }
    descendants
}

fn diagnose_draw_and_material(
    node: &SceneNodeInspectionV1,
    inspection: &SceneInspectionReportV1,
    reasons: &mut Vec<VisibilityDiagnosisReasonV1>,
    fixes: &mut Vec<VisibilityDiagnosisFixV1>,
) {
    let draw = inspection
        .draw_list
        .iter()
        .find(|draw| draw.node == node.handle);
    let hidden_by_visibility = !node.visible || hidden_ancestor(node, inspection).is_some();
    if draw.is_none()
        && !hidden_by_visibility
        && transform_is_finite(node.local_transform)
        && transform_is_finite(node.world_transform)
        && !is_zero_scale(node.local_transform)
        && !is_zero_scale(node.world_transform)
    {
        if inspection.active_camera.is_some() && node.layer_mask != u64::MAX {
            push_reason(
                reasons,
                ReasonSpec {
                    code: "layer_masked",
                    severity: "error",
                    confidence: "medium",
                    auto_fixable: true,
                    affected_handles: vec![node.handle],
                    message: "target node is excluded from the active camera layer mask",
                },
            );
            push_fix(
                fixes,
                "set_layer_mask",
                Some(node.handle),
                None,
                "content",
                "put the target node on a layer visible to the active camera, then render again",
            );
        } else if node.kind.eq_ignore_ascii_case("mesh")
            || node.kind.eq_ignore_ascii_case("instance_set")
            || node.kind.eq_ignore_ascii_case("model")
        {
            push_reason(
                reasons,
                ReasonSpec {
                    code: "missing_geometry",
                    severity: "error",
                    confidence: "medium",
                    auto_fixable: false,
                    affected_handles: vec![node.handle],
                    message: "target drawable has no geometry in the inspection draw list",
                },
            );
            push_fix(
                fixes,
                "inspect_assets",
                Some(node.handle),
                None,
                "content",
                "inspect the Assets store used for diagnosis and prepare with the matching asset handles",
            );
        }
    }

    let Some(draw) = draw else {
        return;
    };
    let Some(material) = draw.material.as_ref().or(node.material.as_ref()) else {
        push_reason(
            reasons,
            ReasonSpec {
                code: "missing_material_upload",
                severity: "error",
                confidence: "medium",
                auto_fixable: false,
                affected_handles: vec![node.handle],
                message: "target drawable has no material preview in the inspection draw list",
            },
        );
        push_fix(
            fixes,
            "inspect_assets",
            Some(node.handle),
            None,
            "content",
            "inspect the Assets store used for diagnosis and ensure the material handle resolves",
        );
        return;
    };

    if material.base_color.a <= 1.0e-6 {
        push_reason(
            reasons,
            ReasonSpec {
                code: "alpha_zero",
                severity: "error",
                confidence: "high",
                auto_fixable: true,
                affected_handles: vec![node.handle],
                message: "target material base color alpha is zero",
            },
        );
        push_fix(
            fixes,
            "set_material_alpha",
            Some(node.handle),
            None,
            "content",
            "raise the target material alpha or replace the material, then render again",
        );
    } else if material.alpha_mode != "opaque" {
        push_reason(
            reasons,
            ReasonSpec {
                code: "transparent_material",
                severity: "warning",
                confidence: "medium",
                auto_fixable: false,
                affected_handles: vec![node.handle],
                message: "target material uses a transparent alpha mode",
            },
        );
    }
}

fn hidden_ancestor<'a>(
    node: &SceneNodeInspectionV1,
    inspection: &'a SceneInspectionReportV1,
) -> Option<&'a SceneNodeInspectionV1> {
    let nodes = nodes_by_handle(inspection);
    let mut current = node.parent;
    while let Some(handle) = current {
        let ancestor = nodes.get(&handle).copied()?;
        if !ancestor.visible {
            return Some(ancestor);
        }
        current = ancestor.parent;
    }
    None
}

fn nodes_by_handle(inspection: &SceneInspectionReportV1) -> BTreeMap<u64, &SceneNodeInspectionV1> {
    inspection
        .nodes
        .iter()
        .map(|node| (node.handle, node))
        .collect()
}

fn is_zero_scale(transform: Transform) -> bool {
    const EPSILON: f32 = 1.0e-6;
    transform.scale.x.abs() <= EPSILON
        || transform.scale.y.abs() <= EPSILON
        || transform.scale.z.abs() <= EPSILON
}

fn transform_is_finite(transform: Transform) -> bool {
    transform.translation.is_finite()
        && transform.rotation.is_finite()
        && transform.scale.is_finite()
}
