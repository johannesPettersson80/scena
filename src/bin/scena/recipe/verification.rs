use super::super::scena_input::appearance_introspection_options;

mod bbox_fit;
mod interaction;
mod quality;
mod reference_quality;
mod target_fit;

use interaction::{compile_interaction_expectation, run_interaction_verification};
use serde_json::Value;
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
    let subject_observations = subject_observations_from_composition(capture, &composition);

    let quality = quality::verify_quality_expectations(
        quality::QualityVerificationInput {
            host,
            recipe,
            manifest,
            expect,
            capture,
            introspection,
            composition: &composition,
            subject_observations: &subject_observations,
            recipe_path,
            recipe_dir,
        },
        &mut reasons,
    )?;

    let mut report = scena::SceneRecipeVerificationReportV1::new(
        render_checks,
        reasons,
        appearance,
        interaction,
        Some(composition),
        Some(quality),
    );
    report.subject_observations = subject_observations;
    Ok(report)
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
        let handles = match scena::resolve_scene_recipe_target_handles(
            manifest,
            &visible.target,
            scena::SceneRecipeTargetResolutionMode::Subject,
        ) {
            Ok(handles) => handles,
            Err(error) if error.kind == scena::SceneRecipeTargetResolutionErrorKind::Hidden => {
                push_reason(
                    reasons,
                    "target_not_visible",
                    "render",
                    Some(visible.id.clone()),
                    Vec::new(),
                    error.message,
                );
                continue;
            }
            Err(error) => {
                let message = if error.candidates.is_empty() {
                    error.message
                } else {
                    format!(
                        "{}; nearest candidates: {}",
                        error.message,
                        error.candidates.join(", ")
                    )
                };
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
    let mode = if allow_import {
        scena::SceneRecipeTargetResolutionMode::Subject
    } else {
        scena::SceneRecipeTargetResolutionMode::SingleHandle
    };
    scena::resolve_scene_recipe_target_handles(manifest, target, mode).map_err(|error| {
        if error.candidates.is_empty() {
            error.message
        } else {
            format!(
                "{}; nearest candidates: {}",
                error.message,
                error.candidates.join(", ")
            )
        }
    })
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

fn subject_observations_from_composition(
    capture: &scena::CaptureRgba8,
    composition: &scena::SceneCompositionReportV1,
) -> Vec<scena::SubjectObservationV1> {
    composition
        .checks
        .iter()
        .filter(|check| check.id.starts_with("subject.") && check.id.ends_with(".projected_bounds"))
        .filter_map(|projected| {
            if projected.affected_handles.is_empty() {
                return None;
            }
            let prefix = projected
                .id
                .strip_suffix(".projected_bounds")
                .expect("filter ensures projected suffix");
            let visible_id = format!("{prefix}.visible_mask");
            let visible = composition
                .checks
                .iter()
                .find(|check| check.id == visible_id);
            Some(subject_observation_from_checks(capture, projected, visible))
        })
        .collect()
}

fn subject_observation_from_checks(
    capture: &scena::CaptureRgba8,
    projected: &scena::SceneCompositionCheckV1,
    visible: Option<&scena::SceneCompositionCheckV1>,
) -> scena::SubjectObservationV1 {
    let source = observed_str(&projected.observed, "source").unwrap_or("unknown");
    let target = scena::SubjectObservationTargetV1::new(
        observed_str(&projected.observed, "target_kind").unwrap_or("unknown"),
        projected
            .target_id
            .as_deref()
            .or_else(|| observed_str(&projected.observed, "target_id"))
            .unwrap_or("world"),
        projected.affected_handles.iter().copied(),
    );
    let Some(visible) = visible else {
        return scena::SubjectObservationV1::degraded(
            source,
            target,
            &capture.descriptor,
            vec!["subject_visible_mask_missing".to_owned()],
            vec!["visible_mask_unavailable".to_owned()],
        );
    };
    let mut reason_codes = Vec::new();
    if projected.status != scena::SceneCompositionStatusV1::Checked {
        reason_codes.push(projected.code.clone());
    }
    if visible.status != scena::SceneCompositionStatusV1::Checked {
        reason_codes.push(visible.code.clone());
    }
    let projected_bounds = observation_bounds(projected);
    let visible_bounds = observation_bounds(visible);
    let visible_pixel_count = observed_u64(&visible.observed, "visible_pixels");
    let projected_area_px = observed_u64(&visible.observed, "projected_area_px")
        .or_else(|| projected_bounds.map(|bounds| bounds.area_px));
    let Some(projected_bounds) = projected_bounds else {
        return degraded_subject_observation(
            source,
            target,
            &capture.descriptor,
            reason_codes,
            "projected_bounds_unavailable",
        );
    };
    let Some(visible_bounds) = visible_bounds else {
        return degraded_subject_observation(
            source,
            target,
            &capture.descriptor,
            reason_codes,
            "visible_bounds_unavailable",
        );
    };
    let Some(visible_pixel_count) = visible_pixel_count else {
        return degraded_subject_observation(
            source,
            target,
            &capture.descriptor,
            reason_codes,
            "visible_pixels_unavailable",
        );
    };
    let Some(projected_area_px) = projected_area_px else {
        return degraded_subject_observation(
            source,
            target,
            &capture.descriptor,
            reason_codes,
            "projected_area_unavailable",
        );
    };

    let depth = subject_observation_depth(visible);
    let mut flags = Vec::new();
    if depth.is_none() {
        reason_codes.push("subject_depth_unavailable".to_owned());
        flags.push("depth_unavailable".to_owned());
    }
    let confidence = observed_str(&visible.observed, "confidence");
    if confidence != Some("exact_opaque_semantic_aov") {
        if let Some(confidence) = confidence {
            flags.push(format!("confidence:{confidence}"));
        }
        if !reason_codes
            .iter()
            .any(|code| code == "subject_mask_degraded")
        {
            reason_codes.push("subject_mask_degraded".to_owned());
        }
    }
    let fallback = scena::SubjectObservationFallbackV1 {
        degraded: !flags.is_empty() || !reason_codes.is_empty(),
        flags,
        reason_codes,
    };
    let mut observation = scena::SubjectObservationV1::observed(
        source,
        target,
        &capture.descriptor,
        projected_bounds,
        visible_bounds,
        scena::SubjectObservationMetricsV1 {
            visible_pixel_count,
            projected_area_px,
            visible_fill_fraction: observed_f32(&visible.observed, "visible_fill_fraction")
                .unwrap_or(0.0),
            visible_fraction_of_projected: observed_f32(
                &visible.observed,
                "visible_fraction_of_projected",
            )
            .unwrap_or(0.0),
            occlusion_estimate: observed_f32(&visible.observed, "occlusion_estimate")
                .unwrap_or(0.0),
        },
        depth,
        fallback,
    );
    if let Some(pixel_quality) = subject_observation_pixel_quality(visible) {
        observation = observation.with_pixel_quality(pixel_quality);
    }
    observation
}

fn degraded_subject_observation(
    source: &str,
    target: scena::SubjectObservationTargetV1,
    descriptor: &scena::CaptureDescriptor,
    mut reason_codes: Vec<String>,
    flag: &str,
) -> scena::SubjectObservationV1 {
    reason_codes.push(flag.to_owned());
    scena::SubjectObservationV1::degraded(
        source,
        target,
        descriptor,
        reason_codes,
        vec![flag.to_owned()],
    )
}

fn observation_bounds(
    check: &scena::SceneCompositionCheckV1,
) -> Option<scena::SubjectObservationBoundsV1> {
    let rect = check.region.as_ref()?.rect_css_px.as_ref()?;
    let area_px = (rect.width.max(0.0) * rect.height.max(0.0)).round() as u64;
    Some(scena::SubjectObservationBoundsV1 {
        min_x: rect.min_x,
        min_y: rect.min_y,
        max_x: rect.max_x,
        max_y: rect.max_y,
        width: rect.width,
        height: rect.height,
        area_px,
    })
}

fn subject_observation_depth(
    visible: &scena::SceneCompositionCheckV1,
) -> Option<scena::SubjectObservationDepthV1> {
    Some(scena::SubjectObservationDepthV1 {
        near_m: observed_f32(&visible.observed, "depth_near_m")?,
        p50_m: observed_f32(&visible.observed, "depth_p50_m")?,
        far_m: observed_f32(&visible.observed, "depth_far_m")?,
        sample_count: observed_u64(&visible.observed, "depth_sample_count")?,
        confidence: observed_f32(&visible.observed, "depth_confidence")?,
    })
}

fn subject_observation_pixel_quality(
    visible: &scena::SceneCompositionCheckV1,
) -> Option<scena::SubjectObservationPixelQualityV1> {
    Some(scena::SubjectObservationPixelQualityV1 {
        mean_luminance_srgb8: observed_f64(&visible.observed, "mean_luminance_srgb8")?,
        luminance_stddev_srgb8: observed_f64(&visible.observed, "luminance_stddev_srgb8")?,
        luminance_range_srgb8: observed_f64(&visible.observed, "luminance_range_srgb8")?,
        low_clip_fraction: observed_f64(&visible.observed, "low_clip_fraction")?,
        high_clip_fraction: observed_f64(&visible.observed, "high_clip_fraction")?,
        sample_count: observed_u64(&visible.observed, "subject_pixel_sample_count")?,
    })
}

fn observed_str<'a>(
    observed: &'a std::collections::BTreeMap<String, Value>,
    key: &str,
) -> Option<&'a str> {
    observed.get(key)?.as_str()
}

fn observed_u64(observed: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<u64> {
    observed.get(key)?.as_u64()
}

fn observed_f32(observed: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<f32> {
    observed.get(key)?.as_f64().map(|value| value as f32)
}

fn observed_f64(observed: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<f64> {
    observed.get(key)?.as_f64()
}
