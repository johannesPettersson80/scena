use super::super::scena_input::appearance_introspection_options;

mod bbox_fit;
mod reference_quality;

use std::path::Path;

pub(crate) struct RecipeVerificationInput<'a> {
    pub(crate) host: &'a mut scena::SceneHostCore,
    pub(crate) manifest: &'a scena::SceneRecipeBuildV1,
    pub(crate) expect: Option<&'a scena::SceneRecipeExpectV1>,
    pub(crate) capture: &'a scena::CaptureRgba8,
    pub(crate) inspection: &'a scena::SceneInspectionReportV1,
    pub(crate) introspection: &'a scena::RenderIntrospectionReportV1,
    pub(crate) detail: bool,
    pub(crate) recipe_dir: &'a Path,
}

pub(crate) fn verify_recipe_expectations(
    input: RecipeVerificationInput<'_>,
) -> Result<scena::SceneRecipeVerificationReportV1, String> {
    let RecipeVerificationInput {
        host,
        manifest,
        expect,
        capture,
        inspection,
        introspection,
        detail,
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

    let quality_expectation_without_text = expect
        .and_then(|expect| expect.expect_quality.as_ref())
        .cloned()
        .map(|mut expectation| {
            expectation.text = None;
            expectation.line = None;
            expectation
        });
    let mut quality = scena::evaluate_render_quality(
        capture,
        introspection,
        quality_expectation_without_text.as_ref(),
    );
    if let Some(expect) = expect {
        if let Some(text) = expect
            .expect_quality
            .as_ref()
            .and_then(|quality| quality.text)
        {
            let label_targets =
                host.label_quality_targets(capture.descriptor.width, capture.descriptor.height);
            for (index, target) in label_targets.into_iter().enumerate() {
                quality
                    .checks
                    .extend(scena::evaluate_label_region_quality_with_background(
                        &format!("expect_quality.text.label[{index}]"),
                        &capture.rgba8,
                        capture.descriptor.width,
                        capture.descriptor.height,
                        target.region,
                        text,
                        target.background_srgb8,
                    ));
            }
        }
        if let Some(line) = expect
            .expect_quality
            .as_ref()
            .and_then(|quality| quality.line)
        {
            let line_regions =
                host.line_quality_regions(capture.descriptor.width, capture.descriptor.height);
            for (index, region) in line_regions.into_iter().enumerate() {
                quality.checks.extend(scena::evaluate_line_region_quality(
                    &format!("expect_quality.line.segment[{index}]"),
                    &capture.rgba8,
                    capture.descriptor.width,
                    capture.descriptor.height,
                    region,
                    line,
                ));
            }
        }
        quality
            .checks
            .extend(reference_quality::verify_reference_expectations(
                &expect.expect_reference,
                capture,
                recipe_dir,
            )?);
        reference_quality::refresh_quality_summary(&mut quality);
    }
    reasons.extend(
        quality
            .checks
            .iter()
            .map(|check| scena::SceneRecipeVerificationReasonV1 {
                code: check.code.clone(),
                severity: check.severity.clone(),
                source: "quality".to_owned(),
                expectation_id: Some(check.id.clone()),
                affected_handles: check.region.handle.into_iter().collect(),
                message: format!("{}; fix: {}", check.code, check.fix_hint),
            }),
    );

    Ok(scena::SceneRecipeVerificationReportV1::new(
        render_checks,
        reasons,
        appearance,
        interaction,
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

fn compile_interaction_expectation(
    expect: &scena::SceneRecipeExpectV1,
    manifest: &scena::SceneRecipeBuildV1,
    capture: &scena::CaptureRgba8,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> scena::InteractionExpectationV1 {
    let mut steps = Vec::new();
    for pick in &expect.expect_pick {
        let handle = match resolve_target_handle(&pick.target, manifest) {
            Ok(handle) => handle,
            Err(message) => {
                push_reason(
                    reasons,
                    "target_not_found",
                    "interaction",
                    Some(pick.id.clone()),
                    Vec::new(),
                    message,
                );
                continue;
            }
        };
        steps.push(scena::InteractionStepExpectationV1 {
            action: "pick".to_owned(),
            x_css_px: pick.x_css_px as f32,
            y_css_px: pick.y_css_px as f32,
            coordinate_space: "css".to_owned(),
            expect_hit: Some(true),
            expected_handle: Some(handle),
            expect_hover: None,
            expect_selection: None,
            expected_events: vec!["pick".to_owned()],
        });
    }
    scena::InteractionExpectationV1 {
        schema: scena::INTERACTION_EXPECTATION_SCHEMA_V1.to_owned(),
        viewport: scena::InteractionViewportV1 {
            width_css_px: capture.descriptor.width as f32,
            height_css_px: capture.descriptor.height as f32,
            device_pixel_ratio: 1.0,
        },
        steps,
    }
}

fn run_interaction_verification(
    host: &mut scena::SceneHostCore,
    expectation: scena::InteractionExpectationV1,
) -> Result<scena::InteractionVerificationReportV1, String> {
    let artifacts = scena::InteractionVerificationArtifactsV1::from_viewport(expectation.viewport);
    let _ = host.drain_events();
    let mut steps = Vec::with_capacity(expectation.steps.len());
    for (index, step) in expectation.steps.iter().enumerate() {
        let coordinates = scena::InteractionCoordinatesV1::from_step(step, expectation.viewport)?;
        let handle = host
            .pick(coordinates.x_css_px, coordinates.y_css_px)
            .map_err(|error| format!("recipe interaction pick failed: {error}"))?;
        let events = host
            .drain_events()
            .iter()
            .map(scena::host_event_kind_name)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        steps.push(scena::InteractionStepReportV1 {
            index,
            action: step.action.clone(),
            coordinates,
            expected: scena::InteractionStepExpectedV1::from(step),
            observed: scena::InteractionStepObservedV1 {
                hit: handle.is_some(),
                handle,
                hover_handle: host.hover_handle(),
                selection_handle: host.primary_selection_handle(),
                events,
            },
        });
    }
    Ok(scena::InteractionVerificationReportV1::from_steps(
        artifacts, steps,
    ))
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

fn resolve_target_handles(
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
