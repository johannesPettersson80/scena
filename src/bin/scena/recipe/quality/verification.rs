use std::{collections::BTreeMap, path::Path};

use super::super::{push_reason, reference_quality};
use super::{
    append_depth_of_field_checks, area_light_region_for_expectation,
    expectation_without_region_specific_checks, geometry_expectation, geometry_region,
    grid_floor_region, grid_line_quality_thresholds, grounding_region_for_expectation,
    max_area_light_emitter_extent_meters, reflection_region_for_expectation,
};

pub(crate) struct QualityVerificationInput<'a> {
    pub(crate) host: &'a mut scena::SceneHostCore,
    pub(crate) recipe: &'a scena::SceneRecipeV1,
    pub(crate) manifest: &'a scena::SceneRecipeBuildV1,
    pub(crate) expect: Option<&'a scena::SceneRecipeExpectV1>,
    pub(crate) capture: &'a scena::CaptureRgba8,
    pub(crate) introspection: &'a scena::RenderIntrospectionReportV1,
    pub(crate) composition: &'a scena::SceneCompositionReportV1,
    pub(crate) subject_observations: &'a [scena::SubjectObservationV1],
    pub(crate) recipe_path: &'a Path,
    pub(crate) recipe_dir: &'a Path,
}

pub(crate) fn verify_quality_expectations(
    input: QualityVerificationInput<'_>,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> Result<scena::RenderQualityReportV1, String> {
    let QualityVerificationInput {
        host,
        recipe,
        manifest,
        expect,
        capture,
        introspection,
        composition,
        subject_observations,
        recipe_path,
        recipe_dir,
    } = input;
    let quality_failures_are_blocking = expect.is_some_and(|expect| {
        expect.expect_quality.is_some() || !expect.expect_reference.is_empty()
    });
    let baseline_quality_is_diagnostic = !quality_failures_are_blocking
        || baseline_quality_is_diagnostic(expect)
        || recipe_photo_intent_owns_quality(recipe, expect);
    let quality_expectation_without_text = expectation_without_region_specific_checks(expect);
    let mut quality = scena::evaluate_render_quality_rgba8_region(
        scena::RenderQualityRgba8Input {
            rgba8: &capture.rgba8,
            width: capture.descriptor.width,
            height: capture.descriptor.height,
            capabilities: introspection.capabilities,
            visible_pixel_fraction: introspection.visible_pixel_fraction,
            tiny_in_frame: introspection.framing.tiny_in_frame,
            fit_fraction: introspection.framing.fit_fraction,
        },
        geometry_region(capture, introspection, composition),
        quality_expectation_without_text.as_ref(),
    );
    if let Some(expect) = expect {
        append_label_checks(host, capture, expect, &mut quality);
        append_line_checks(host, capture, expect, &mut quality);
        append_grid_checks(recipe, capture, expect, &mut quality);
        append_geometry_checks(capture, introspection, composition, expect, &mut quality);
        append_reflection_checks(
            capture,
            composition,
            manifest,
            expect,
            &mut quality,
            reasons,
        );
        append_area_light_checks(
            recipe,
            capture,
            introspection,
            composition,
            manifest,
            expect,
            &mut quality,
            reasons,
        );
        append_grounding_checks(
            capture,
            introspection,
            composition,
            manifest,
            expect,
            &mut quality,
            reasons,
        );
        append_depth_of_field_checks(
            recipe,
            recipe_path,
            capture,
            introspection,
            composition,
            manifest,
            expect,
            &mut quality,
            reasons,
        );
        append_subject_observation_quality_checks(expect, subject_observations, &mut quality);
        quality
            .checks
            .extend(reference_quality::verify_reference_expectations(
                &expect.expect_reference,
                capture,
                recipe_dir,
            )?);
    }
    if baseline_quality_is_diagnostic {
        downgrade_baseline_quality_checks(&mut quality);
    }
    reference_quality::refresh_quality_summary(&mut quality);
    if quality_failures_are_blocking {
        reasons.extend(
            quality
                .checks
                .iter()
                .filter(|check| !matches!(check.status, scena::RenderQualityStatusV1::Checked))
                .filter(|check| matches!(check.severity.as_str(), "error" | "warning"))
                .map(|check| scena::SceneRecipeVerificationReasonV1 {
                    code: check.code.clone(),
                    severity: check.severity.clone(),
                    source: "quality".to_owned(),
                    expectation_id: Some(check.id.clone()),
                    affected_handles: check.region.handle.into_iter().collect(),
                    message: quality_failure_message(check),
                }),
        );
    }
    Ok(quality)
}

const PRODUCT_SUBJECT_TARGET_MEAN_LUMA: f32 = 90.0;
const PRODUCT_SUBJECT_MIN_MEAN_LUMA: f32 = 80.0;
const PRODUCT_SUBJECT_MAX_MEAN_LUMA: f32 = 100.0;
const PRODUCT_SUBJECT_MAX_LOW_CLIP: f32 = 0.20;
const PRODUCT_SUBJECT_MAX_HIGH_CLIP: f32 = 0.05;
const PRODUCT_SUBJECT_MIN_LUMA_STDDEV: f32 = 6.0;
const PRODUCT_SUBJECT_MIN_LUMA_RANGE: f32 = 32.0;

fn append_subject_observation_quality_checks(
    expect: &scena::SceneRecipeExpectV1,
    subject_observations: &[scena::SubjectObservationV1],
    quality: &mut scena::RenderQualityReportV1,
) {
    let Some(expect_quality) = expect.expect_quality.as_ref() else {
        return;
    };
    if expect_quality.profile != "product" {
        return;
    }
    let Some(observation) = product_subject_observation(subject_observations) else {
        return;
    };
    let Some(pixel_quality) = observation.pixel_quality else {
        return;
    };

    let exposure = expect_quality.exposure;
    let min_mean_luma = exposure
        .and_then(|exposure| exposure.min_mean_luminance_srgb8)
        .unwrap_or(PRODUCT_SUBJECT_MIN_MEAN_LUMA as f64) as f32;
    let max_mean_luma = exposure
        .and_then(|exposure| exposure.max_mean_luminance_srgb8)
        .unwrap_or(PRODUCT_SUBJECT_MAX_MEAN_LUMA as f64) as f32;
    let max_low_clip = exposure
        .and_then(|exposure| exposure.max_low_clip_fraction)
        .unwrap_or(PRODUCT_SUBJECT_MAX_LOW_CLIP as f64) as f32;
    let max_high_clip = exposure
        .and_then(|exposure| exposure.max_high_clip_fraction)
        .unwrap_or(PRODUCT_SUBJECT_MAX_HIGH_CLIP as f64) as f32;
    let mean_luma = pixel_quality.mean_luminance_srgb8 as f32;
    let low_clip = pixel_quality.low_clip_fraction as f32;
    let high_clip = pixel_quality.high_clip_fraction as f32;
    let target_mean_luma = PRODUCT_SUBJECT_TARGET_MEAN_LUMA.clamp(min_mean_luma, max_mean_luma);
    let suggested_compensation_ev = (target_mean_luma / mean_luma.max(1.0)).log2();
    let region = subject_quality_region(observation);
    let (status, code, fix_hint) = if low_clip > max_low_clip {
        (
            scena::RenderQualityStatusV1::Failed,
            "subject_black_crushed",
            "raise render.exposure_compensation, use subject metering/photo intent, or change staging so the product is not a silhouette",
        )
    } else if high_clip > max_high_clip {
        (
            scena::RenderQualityStatusV1::Failed,
            "subject_blown_out",
            "lower render.exposure_compensation or soften the light/staging so product highlights keep detail",
        )
    } else if mean_luma < min_mean_luma {
        (
            scena::RenderQualityStatusV1::Failed,
            "subject_luminance_below_min",
            "increase render.exposure_compensation or select brighter camera-behavior staging",
        )
    } else if mean_luma > max_mean_luma {
        (
            scena::RenderQualityStatusV1::Failed,
            "subject_luminance_above_max",
            "decrease render.exposure_compensation or reduce direct lighting on the subject",
        )
    } else {
        (
            scena::RenderQualityStatusV1::Checked,
            "subject_exposure_sane",
            "no action needed",
        )
    };
    quality.checks.push(scena::RenderQualityCheckV1 {
        id: "expect_quality.subject.pixel_exposure".to_owned(),
        code: code.to_owned(),
        status,
        severity: if matches!(status, scena::RenderQualityStatusV1::Failed) {
            "error".to_owned()
        } else {
            "info".to_owned()
        },
        region: region.clone(),
        observed: quality_observed([
            ("mean_luminance_srgb8", mean_luma),
            ("low_clip_fraction", low_clip),
            ("high_clip_fraction", high_clip),
            ("sample_count", pixel_quality.sample_count as f32),
            ("suggested_compensation_ev", suggested_compensation_ev),
        ]),
        threshold: quality_observed([
            ("min_mean_luminance_srgb8", min_mean_luma),
            ("max_mean_luminance_srgb8", max_mean_luma),
            ("max_low_clip_fraction", max_low_clip),
            ("max_high_clip_fraction", max_high_clip),
        ]),
        fix_hint: fix_hint.to_owned(),
    });

    let stddev = pixel_quality.luminance_stddev_srgb8 as f32;
    let range = pixel_quality.luminance_range_srgb8 as f32;
    let material_status =
        if stddev < PRODUCT_SUBJECT_MIN_LUMA_STDDEV || range < PRODUCT_SUBJECT_MIN_LUMA_RANGE {
            scena::RenderQualityStatusV1::Failed
        } else {
            scena::RenderQualityStatusV1::Checked
        };
    quality.checks.push(scena::RenderQualityCheckV1 {
        id: "expect_quality.subject.material_readability".to_owned(),
        code: if matches!(material_status, scena::RenderQualityStatusV1::Failed) {
            "subject_luminance_structure_below_min"
        } else {
            "subject_material_readability_sane"
        }
        .to_owned(),
        status: material_status,
        severity: if matches!(material_status, scena::RenderQualityStatusV1::Failed) {
            "error".to_owned()
        } else {
            "info".to_owned()
        },
        region,
        observed: quality_observed([
            ("luminance_stddev_srgb8", stddev),
            ("luminance_range_srgb8", range),
            ("sample_count", pixel_quality.sample_count as f32),
        ]),
        threshold: quality_observed([
            ("min_luminance_stddev_srgb8", PRODUCT_SUBJECT_MIN_LUMA_STDDEV),
            ("min_luminance_range_srgb8", PRODUCT_SUBJECT_MIN_LUMA_RANGE),
        ]),
        fix_hint: if matches!(material_status, scena::RenderQualityStatusV1::Failed) {
            "add reflective studio lighting, rotate the product/camera, or use material settings that reveal steel/detail structure"
        } else {
            "no action needed"
        }
        .to_owned(),
    });
}

fn product_subject_observation(
    subject_observations: &[scena::SubjectObservationV1],
) -> Option<&scena::SubjectObservationV1> {
    subject_observations
        .iter()
        .filter(|observation| {
            observation.status == "observed"
                && !observation.fallback.degraded
                && observation.pixel_quality.is_some()
        })
        .min_by_key(|observation| match observation.source.as_str() {
            "render.metering" => 0,
            "photo.subject" => 1,
            "render.depth_of_field.focus" => 2,
            _ => 3,
        })
}

fn subject_quality_region(
    observation: &scena::SubjectObservationV1,
) -> scena::RenderQualityRegionV1 {
    let rect_css_px = observation
        .visible_bounds
        .or(observation.projected_bounds)
        .map(|bounds| scena::RenderIntrospectionRectV1 {
            min_x: bounds.min_x,
            min_y: bounds.min_y,
            max_x: bounds.max_x,
            max_y: bounds.max_y,
            width: bounds.width,
            height: bounds.height,
        });
    scena::RenderQualityRegionV1 {
        kind: "subject_visible".to_owned(),
        handle: observation.target.handles.first().copied(),
        rect_css_px,
    }
}

fn quality_observed<const N: usize>(pairs: [(&str, f32); N]) -> BTreeMap<String, f32> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), round3_f32(value)))
        .collect()
}

fn round3_f32(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn baseline_quality_is_diagnostic(expect: Option<&scena::SceneRecipeExpectV1>) -> bool {
    let Some(quality) = expect.and_then(|expect| expect.expect_quality.as_ref()) else {
        return false;
    };
    let has_frame_quality_thresholds =
        quality.exposure.is_some() || quality.contrast.is_some() || quality.noise.is_some();
    let has_region_specific_quality = quality.text.is_some()
        || quality.line.is_some()
        || quality.geometry.is_some()
        || quality.reflection.is_some()
        || quality.area_light.is_some()
        || quality.grounding.is_some()
        || quality.depth_of_field.is_some();
    has_region_specific_quality && !has_frame_quality_thresholds
}

fn recipe_photo_intent_owns_quality(
    recipe: &scena::SceneRecipeV1,
    expect: Option<&scena::SceneRecipeExpectV1>,
) -> bool {
    recipe.photo.is_some()
        && expect
            .and_then(|expect| expect.expect_quality.as_ref())
            .is_none()
}

fn downgrade_baseline_quality_checks(quality: &mut scena::RenderQualityReportV1) {
    for check in &mut quality.checks {
        if check.id.starts_with("baseline.") {
            check.status = scena::RenderQualityStatusV1::Checked;
            check.severity = "info".to_owned();
        }
    }
}

fn quality_failure_message(check: &scena::RenderQualityCheckV1) -> String {
    let Some((observed_key, observed)) = check.observed.iter().next() else {
        return format!("{}; fix: {}", check.code, check.fix_hint);
    };
    let Some((threshold_key, threshold)) = check.threshold.iter().next() else {
        return format!(
            "{}: {}={:.3}; fix: {}",
            check.code, observed_key, observed, check.fix_hint
        );
    };
    let comparison = if threshold_key.starts_with("min_") || check.code.ends_with("_too_low") {
        "<"
    } else if threshold_key.starts_with("max_") || check.code.ends_with("_too_high") {
        ">"
    } else {
        "vs"
    };
    format!(
        "{}: {}={:.3} {} {}={:.3}; fix: {}",
        check.code, observed_key, observed, comparison, threshold_key, threshold, check.fix_hint
    )
}

fn append_label_checks(
    host: &scena::SceneHostCore,
    capture: &scena::CaptureRgba8,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
) {
    let Some(text) = expect
        .expect_quality
        .as_ref()
        .and_then(|quality| quality.text)
    else {
        return;
    };
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

fn append_line_checks(
    host: &scena::SceneHostCore,
    capture: &scena::CaptureRgba8,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
) {
    let Some(line) = expect
        .expect_quality
        .as_ref()
        .and_then(|quality| quality.line)
    else {
        return;
    };
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

fn append_grid_checks(
    recipe: &scena::SceneRecipeV1,
    capture: &scena::CaptureRgba8,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
) {
    let Some(thresholds) = grid_line_quality_thresholds(recipe, expect.expect_quality.as_ref())
    else {
        return;
    };
    quality
        .checks
        .extend(scena::evaluate_grid_line_region_quality(
            "expect_quality.grid_floor_lines",
            &capture.rgba8,
            capture.descriptor.width,
            capture.descriptor.height,
            grid_floor_region(capture),
            thresholds.min_intermediate_px_per_edge,
            thresholds.min_unique_luma_levels,
            thresholds.max_halo_overshoot,
            thresholds.min_contrast_range,
        ));
}

fn append_geometry_checks(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
) {
    let Some(geometry) = geometry_expectation(expect.expect_quality.as_ref()) else {
        return;
    };
    quality
        .checks
        .extend(scena::evaluate_geometry_region_quality(
            "expect_quality.geometry",
            &capture.rgba8,
            capture.descriptor.width,
            capture.descriptor.height,
            geometry_region(capture, introspection, composition),
            geometry,
        ));
}

fn append_reflection_checks(
    capture: &scena::CaptureRgba8,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) {
    let Some(reflection) = expect
        .expect_quality
        .as_ref()
        .and_then(|quality| quality.reflection.as_ref())
    else {
        return;
    };
    match reflection_region_for_expectation(capture, composition, manifest, reflection) {
        Ok((id, region)) => {
            quality
                .checks
                .extend(scena::evaluate_reflection_region_quality(
                    &id,
                    &capture.rgba8,
                    capture.descriptor.width,
                    capture.descriptor.height,
                    region,
                    reflection.clone(),
                ));
        }
        Err(message) => push_reason(
            reasons,
            "reflection_target_unresolved",
            "quality",
            Some("expect_quality.reflection.target".to_owned()),
            Vec::new(),
            message,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn append_area_light_checks(
    recipe: &scena::SceneRecipeV1,
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) {
    let Some(area_light) = expect
        .expect_quality
        .as_ref()
        .and_then(|quality| quality.area_light.as_ref())
    else {
        return;
    };
    match area_light_region_for_expectation(
        capture,
        introspection,
        composition,
        manifest,
        area_light,
    ) {
        Ok((id, region)) => {
            quality
                .checks
                .extend(scena::evaluate_area_light_region_quality(
                    &id,
                    &capture.rgba8,
                    capture.descriptor.width,
                    capture.descriptor.height,
                    region,
                    area_light.clone(),
                    max_area_light_emitter_extent_meters(recipe),
                ));
        }
        Err(message) => push_reason(
            reasons,
            "area_light_target_unresolved",
            "quality",
            Some("expect_quality.area_light.target".to_owned()),
            Vec::new(),
            message,
        ),
    }
}

fn append_grounding_checks(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) {
    let Some(grounding) = expect
        .expect_quality
        .as_ref()
        .and_then(|quality| quality.grounding.as_ref())
    else {
        return;
    };
    match grounding_region_for_expectation(capture, introspection, composition, manifest, grounding)
    {
        Ok((id, region)) => {
            quality
                .checks
                .extend(scena::evaluate_grounding_region_quality(
                    &id,
                    &capture.rgba8,
                    capture.descriptor.width,
                    capture.descriptor.height,
                    region,
                    grounding.clone(),
                ));
        }
        Err(message) => push_reason(
            reasons,
            "grounding_target_unresolved",
            "quality",
            Some("expect_quality.grounding.target".to_owned()),
            Vec::new(),
            message,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn quality_failure_message_includes_metric_value_and_threshold() {
        let check = scena::RenderQualityCheckV1 {
            id: "baseline.clipped_highlights".to_owned(),
            code: "clipped_highlight_fraction_too_high".to_owned(),
            status: scena::RenderQualityStatusV1::Failed,
            severity: "error".to_owned(),
            region: scena::RenderQualityRegionV1 {
                kind: "subject".to_owned(),
                handle: None,
                rect_css_px: None,
            },
            observed: BTreeMap::from([("clipped_highlight_fraction".to_owned(), 0.41)]),
            threshold: BTreeMap::from([("max_clipped_highlight_fraction".to_owned(), 0.05)]),
            fix_hint: "lower exposure".to_owned(),
        };

        assert_eq!(
            quality_failure_message(&check),
            "clipped_highlight_fraction_too_high: clipped_highlight_fraction=0.410 > max_clipped_highlight_fraction=0.050; fix: lower exposure"
        );
    }
}
