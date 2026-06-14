use crate::scene::{
    SceneDrawInspectionV1, SceneInspectionReportV1, SceneMaterialInspectionV1,
    SceneNodeInspectionV1,
};

use super::sampling::{ContentSample, swatch_distance};
use super::types::{
    AppearanceAlphaSummaryV1, AppearanceFixV1, AppearanceIntrospectionOptions, AppearanceReasonV1,
    AppearanceTargetExpectationV1,
};

const SWATCH_DISTANCE_THRESHOLD: f32 = 0.42;

pub(super) struct MatchedTarget<'a> {
    pub(super) node: u64,
    pub(super) material: &'a SceneMaterialInspectionV1,
}

pub(super) fn material_for_target<'a>(
    inspection: &'a SceneInspectionReportV1,
    expected: &AppearanceTargetExpectationV1,
) -> Option<MatchedTarget<'a>> {
    if let Some(handle) = expected.node {
        return material_for_node(inspection, handle);
    }
    if let Some(tag) = &expected.tag {
        return inspection
            .find_by_tag(tag)
            .into_iter()
            .find_map(|node| material_for_node(inspection, node.handle));
    }
    inspection
        .draw_list
        .iter()
        .find(|draw| draw.visible && draw.material.is_some())
        .or_else(|| {
            inspection
                .draw_list
                .iter()
                .find(|draw| draw.material.is_some())
        })
        .and_then(draw_material)
        .or_else(|| {
            inspection
                .nodes
                .iter()
                .find(|node| node.material.is_some())
                .and_then(node_material)
        })
}

pub(super) fn evaluate_target(
    expected: &AppearanceTargetExpectationV1,
    node_handle: Option<u64>,
    material: Option<&SceneMaterialInspectionV1>,
    sample: &ContentSample,
    options: &AppearanceIntrospectionOptions,
    reasons: &mut Vec<AppearanceReasonV1>,
    fixes: &mut Vec<AppearanceFixV1>,
) {
    let affected = node_handle.into_iter().collect::<Vec<_>>();
    if material.is_none() {
        push_reason(
            reasons,
            "target_not_found",
            "error",
            &expected.id,
            affected.clone(),
            "no material-bearing draw matched the appearance target selector",
        );
        push_fix(
            fixes,
            "inspect_material_assignment",
            &expected.id,
            node_handle,
            "inspect the target selector and material assignment before verifying appearance",
        );
        return;
    }
    let material = material.expect("checked above");

    if let Some(variant) = &expected.variant {
        if !options
            .available_variants
            .iter()
            .any(|candidate| candidate == variant)
        {
            push_reason(
                reasons,
                "variant_missing",
                "error",
                &expected.id,
                affected.clone(),
                "intended material variant is not declared by the loaded asset",
            );
            push_fix(
                fixes,
                "list_material_variants",
                &expected.id,
                node_handle,
                "list declared material variants and choose one of the available names",
            );
        } else if options.active_variant.as_deref() != Some(variant.as_str()) {
            push_reason(
                reasons,
                "variant_not_active",
                "error",
                &expected.id,
                affected.clone(),
                "intended material variant is declared but not active in the rendered frame",
            );
            push_fix(
                fixes,
                "apply_variant",
                &expected.id,
                node_handle,
                "apply the intended variant before rendering and verifying appearance",
            );
        }
    }

    if expected.require_source_material && material.source.kind != "source_material" {
        push_reason(
            reasons,
            "generated_fallback",
            "error",
            &expected.id,
            affected.clone(),
            "target material uses generated fallback provenance where source material was required",
        );
        push_fix(
            fixes,
            "inspect_material_assignment",
            &expected.id,
            node_handle,
            "inspect the source glTF primitive material assignment and reload the source material",
        );
    }

    if expected.require_base_color_texture
        && !material
            .textures
            .iter()
            .any(|slot| slot.slot == "baseColorTexture" && slot.has_decoded_pixels)
    {
        push_reason(
            reasons,
            "texture_provenance_missing",
            "error",
            &expected.id,
            affected.clone(),
            "target expected a decoded base-color texture but none was present",
        );
        push_fix(
            fixes,
            "load_missing_texture",
            &expected.id,
            node_handle,
            "load the missing base-color texture or relax the texture expectation",
        );
    }

    if material.base_color.a <= 0.01 {
        push_reason(
            reasons,
            "alpha_hidden",
            "error",
            &expected.id,
            affected.clone(),
            "target material base-color alpha is near zero",
        );
        push_fix(
            fixes,
            "set_alpha",
            &expected.id,
            node_handle,
            "raise the material alpha or use an opaque material for this target",
        );
    }

    if let Some(alpha_mode) = &expected.alpha_mode
        && material.alpha_mode != *alpha_mode
    {
        push_reason(
            reasons,
            "alpha_mode_mismatch",
            "error",
            &expected.id,
            affected.clone(),
            "target material alpha mode differs from the expected alpha mode",
        );
        push_fix(
            fixes,
            "set_alpha_mode",
            &expected.id,
            node_handle,
            "set the target material alpha mode to match the appearance expectation",
        );
    }

    if let Some(expected_family) = &expected.color_family
        && sample.color_family != *expected_family
    {
        push_reason(
            reasons,
            "color_mismatch",
            "error",
            &expected.id,
            affected.clone(),
            "sampled rendered color family differs from the expected color family",
        );
        push_fix(
            fixes,
            "inspect_material_assignment",
            &expected.id,
            node_handle,
            "inspect material assignment, variant selection, tint, and color-space inputs",
        );
    }

    if let Some(swatch) = expected.swatch_srgb8 {
        let distance = swatch_distance(sample.average_rgba8, swatch);
        if distance > SWATCH_DISTANCE_THRESHOLD {
            push_reason(
                reasons,
                "color_mismatch",
                "error",
                &expected.id,
                affected,
                "sampled rendered swatch is outside the expected color tolerance",
            );
            push_fix(
                fixes,
                "inspect_material_assignment",
                &expected.id,
                node_handle,
                "inspect material assignment, variant selection, tint, and color-space inputs",
            );
        }
    }
}

pub(super) fn alpha_summary(material: &SceneMaterialInspectionV1) -> AppearanceAlphaSummaryV1 {
    AppearanceAlphaSummaryV1 {
        mode: material.alpha_mode.clone(),
        base_color_alpha: super::sampling::round3(material.base_color.a),
        hidden: material.base_color.a <= 0.01,
    }
}

pub(super) fn push_reason(
    reasons: &mut Vec<AppearanceReasonV1>,
    code: &str,
    severity: &str,
    target_id: &str,
    affected_handles: Vec<u64>,
    message: &str,
) {
    reasons.push(AppearanceReasonV1 {
        code: code.to_owned(),
        severity: severity.to_owned(),
        target_id: target_id.to_owned(),
        affected_handles,
        message: message.to_owned(),
    });
}

pub(super) fn push_fix(
    fixes: &mut Vec<AppearanceFixV1>,
    action: &str,
    target_id: &str,
    target_handle: Option<u64>,
    help: &str,
) {
    if fixes
        .iter()
        .any(|fix| fix.action == action && fix.target_id == target_id)
    {
        return;
    }
    fixes.push(AppearanceFixV1 {
        action: action.to_owned(),
        target_id: target_id.to_owned(),
        target_handle,
        patch: None,
        help: help.to_owned(),
    });
}

fn material_for_node<'a>(
    inspection: &'a SceneInspectionReportV1,
    handle: u64,
) -> Option<MatchedTarget<'a>> {
    inspection
        .draw_list
        .iter()
        .find(|draw| draw.node == handle && draw.material.is_some())
        .and_then(draw_material)
        .or_else(|| inspection.node_by_handle(handle).and_then(node_material))
}

fn draw_material(draw: &SceneDrawInspectionV1) -> Option<MatchedTarget<'_>> {
    Some(MatchedTarget {
        node: draw.node,
        material: draw.material.as_ref()?,
    })
}

fn node_material(node: &SceneNodeInspectionV1) -> Option<MatchedTarget<'_>> {
    Some(MatchedTarget {
        node: node.handle,
        material: node.material.as_ref()?,
    })
}
