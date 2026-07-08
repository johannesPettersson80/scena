use super::super::scena_input::appearance_introspection_options;

mod bbox_fit;
mod interaction;
mod quality;
mod reference_quality;
mod target_fit;

use interaction::{compile_interaction_expectation, run_interaction_verification};
use std::path::Path;

pub(crate) struct RecipeVerificationInput<'a> {
    pub(crate) host: &'a mut scena::SceneHostCore,
    pub(crate) manifest: &'a scena::SceneRecipeBuildV1,
    pub(crate) recipe: &'a scena::SceneRecipeV1,
    pub(crate) expect: Option<&'a scena::SceneRecipeExpectV1>,
    pub(crate) capture: &'a scena::CaptureRgba8,
    pub(crate) inspection: &'a scena::SceneInspectionReportV1,
    pub(crate) introspection: &'a scena::RenderIntrospectionReportV1,
    pub(crate) detail: bool,
    pub(crate) recipe_path: &'a Path,
    pub(crate) recipe_dir: &'a Path,
}

pub(crate) fn verify_recipe_expectations(
    input: RecipeVerificationInput<'_>,
) -> Result<scena::SceneRecipeVerificationReportV1, String> {
    let RecipeVerificationInput {
        host,
        manifest,
        recipe,
        expect,
        capture,
        inspection,
        introspection,
        detail,
        recipe_path,
        recipe_dir,
    } = input;
    let mut reasons = Vec::new();
    let mut render_checks = 0;

    if let Some(expect) = expect {
        render_checks += verify_visible(expect, manifest, inspection, &mut reasons);
        render_checks += bbox_fit::verify_bbox_fit(
            expect.expect_bbox_fit,
            manifest,
            capture,
            inspection,
            introspection,
            &mut reasons,
        );
        render_checks += target_fit::verify_target_fit(
            &expect.expect_target_fit,
            manifest,
            capture,
            host.renderer().background_color(),
            &mut reasons,
        );
        render_checks += verify_no_warnings(expect, introspection, &mut reasons);
    }

    let appearance_expectation = expect
        .map(|expect| compile_appearance_expectation(expect, manifest, &mut reasons))
        .unwrap_or_else(|| scena::AppearanceExpectationV1 {
            schema: scena::APPEARANCE_EXPECTATION_SCHEMA_V1.to_owned(),
            targets: Vec::new(),
        });
    let appearance = if appearance_expectation.targets.is_empty() {
        None
    } else {
        let report = host.renderer().introspect_appearance(
            capture,
            inspection,
            &appearance_expectation,
            appearance_introspection_options(detail),
        );
        reasons.extend(report.reasons.iter().map(|reason| {
            scena::SceneRecipeVerificationReasonV1 {
                code: reason.code.clone(),
                severity: reason.severity.clone(),
                source: "appearance".to_owned(),
                expectation_id: Some(reason.target_id.clone()),
                affected_handles: reason.affected_handles.clone(),
                message: reason.message.clone(),
            }
        }));
        Some(report)
    };

    let interaction_expectation = expect
        .map(|expect| compile_interaction_expectation(expect, manifest, capture, &mut reasons))
        .unwrap_or_else(|| scena::InteractionExpectationV1 {
            schema: scena::INTERACTION_EXPECTATION_SCHEMA_V1.to_owned(),
            viewport: scena::InteractionViewportV1 {
                width_css_px: capture.descriptor.width as f32,
                height_css_px: capture.descriptor.height as f32,
                device_pixel_ratio: 1.0,
            },
            steps: Vec::new(),
        });
    let interaction = if interaction_expectation.steps.is_empty() {
        None
    } else {
        let report = run_interaction_verification(host, interaction_expectation)?;
        reasons.extend(report.reasons.iter().map(|reason| {
            scena::SceneRecipeVerificationReasonV1 {
                code: reason.code.clone(),
                severity: reason.severity.clone(),
                source: "interaction".to_owned(),
                expectation_id: Some(format!("step:{}", reason.step_index)),
                affected_handles: Vec::new(),
                message: reason.message.clone(),
            }
        }));
        Some(report)
    };

    let composition =
        host.composition_report(recipe, manifest, capture, inspection, introspection, expect);
    push_composition_reasons(&composition, &mut reasons);

    let quality = quality::verify_quality_expectations(
        quality::QualityVerificationInput {
            host,
            recipe,
            manifest,
            expect,
            capture,
            introspection,
            composition: &composition,
            recipe_path,
            recipe_dir,
        },
        &mut reasons,
    )?;

    Ok(scena::SceneRecipeVerificationReportV1::new(
        render_checks,
        reasons,
        appearance,
        interaction,
        Some(composition),
        Some(quality),
    ))
}

fn verify_visible(
    expect: &scena::SceneRecipeExpectV1,
    manifest: &scena::SceneRecipeBuildV1,
    inspection: &scena::SceneInspectionReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> usize {
    let mut checks = 0;
    for visible in &expect.expect_visible {
        checks += 1;
        let handles = match resolve_target_handles(&visible.target, manifest, true) {
            Ok(handles) => handles,
            Err(message) => {
                push_reason(
                    reasons,
                    "target_not_found",
                    "render",
                    Some(visible.id.clone()),
                    Vec::new(),
                    message,
                );
                continue;
            }
        };
        for handle in handles {
            let node = inspection.node_by_handle(handle);
            let drawn = inspection.draw_list.iter().any(|draw| draw.node == handle);
            if node.is_none_or(|node| !node.visible) || !drawn {
                push_reason(
                    reasons,
                    "target_not_visible",
                    "render",
                    Some(visible.id.clone()),
                    vec![handle],
                    format!("expected target handle {handle} to be visible and drawn"),
                );
            }
        }
    }
    checks
}

fn verify_no_warnings(
    expect: &scena::SceneRecipeExpectV1,
    introspection: &scena::RenderIntrospectionReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> usize {
    if !expect.expect_no_warnings {
        return 0;
    }
    for reason in &introspection.reasons {
        if reason.severity == "warning" {
            push_reason(
                reasons,
                "render_warning",
                "render",
                None,
                reason.affected_handles.clone(),
                reason.message.clone(),
            );
        }
    }
    1
}

fn compile_appearance_expectation(
    expect: &scena::SceneRecipeExpectV1,
    manifest: &scena::SceneRecipeBuildV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> scena::AppearanceExpectationV1 {
    let mut targets = Vec::new();
    for color in &expect.expect_color {
        let handle = match resolve_target_handle(&color.target, manifest) {
            Ok(handle) => handle,
            Err(message) => {
                push_reason(
                    reasons,
                    "target_not_found",
                    "appearance",
                    Some(color.id.clone()),
                    Vec::new(),
                    message,
                );
                continue;
            }
        };
        targets.push(scena::AppearanceTargetExpectationV1 {
            id: color.id.clone(),
            node: Some(handle),
            tag: None,
            variant: None,
            color_family: color.color_family.clone(),
            swatch_srgb8: color.swatch_srgb8,
            swatch_tolerance: color.tolerance.map(|value| value as f32),
            alpha_mode: None,
            require_source_material: color.require_source_material,
            require_base_color_texture: color.require_base_color_texture,
        });
    }
    scena::AppearanceExpectationV1 {
        schema: scena::APPEARANCE_EXPECTATION_SCHEMA_V1.to_owned(),
        targets,
    }
}

fn resolve_target_handle(
    target: &scena::SceneRecipeTargetV1,
    manifest: &scena::SceneRecipeBuildV1,
) -> Result<u64, String> {
    let mut handles = resolve_target_handles(target, manifest, false)?;
    handles
        .pop()
        .ok_or_else(|| "target resolved to no handles".to_owned())
}

pub(super) fn resolve_target_handles(
    target: &scena::SceneRecipeTargetV1,
    manifest: &scena::SceneRecipeBuildV1,
    allow_import: bool,
) -> Result<Vec<u64>, String> {
    match target {
        scena::SceneRecipeTargetV1::Node { id } => manifest
            .nodes
            .iter()
            .find(|node| node.id == *id)
            .map(|node| vec![node.handle])
            .or_else(|| {
                manifest.imports.iter().find_map(|import| {
                    import
                        .nodes_by_path
                        .get(id)
                        .copied()
                        .map(|handle| vec![handle])
                })
            })
            .ok_or_else(|| {
                format!("expectation target node id '{id}' was not in the build manifest")
            }),
        scena::SceneRecipeTargetV1::Import { id } if allow_import => manifest
            .imports
            .iter()
            .find(|import| import.id == *id)
            .map(|import| import.root_handles.clone())
            .ok_or_else(|| {
                format!("expectation target import id '{id}' was not in the build manifest")
            }),
        scena::SceneRecipeTargetV1::Import { id } => Err(format!(
            "expectation target import id '{id}' requires a specific node target"
        )),
        scena::SceneRecipeTargetV1::World { .. } => {
            Err("expectation target kind 'world' cannot resolve to a stable handle".to_owned())
        }
    }
}

fn push_reason(
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
    code: &str,
    source: &str,
    expectation_id: Option<String>,
    affected_handles: Vec<u64>,
    message: String,
) {
    reasons.push(scena::SceneRecipeVerificationReasonV1 {
        code: code.to_owned(),
        severity: "error".to_owned(),
        source: source.to_owned(),
        expectation_id,
        affected_handles,
        message,
    });
}

fn push_composition_reasons(
    composition: &scena::SceneCompositionReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) {
    for check in &composition.checks {
        if check.severity != "error" && check.severity != "warning" {
            continue;
        }
        if check.severity == "warning" && check.status == scena::SceneCompositionStatusV1::Checked {
            continue;
        }
        reasons.push(scena::SceneRecipeVerificationReasonV1 {
            code: check.code.clone(),
            severity: check.severity.clone(),
            source: "composition".to_owned(),
            expectation_id: Some(check.id.clone()),
            affected_handles: check.affected_handles.clone(),
            message: format!("{}; fix: {}", check.message, check.fix_hint),
        });
    }
}
