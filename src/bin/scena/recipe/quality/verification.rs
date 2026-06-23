use std::path::Path;

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
        recipe_path,
        recipe_dir,
    } = input;
    let quality_expectation_without_text = expectation_without_region_specific_checks(expect);
    let mut quality = scena::evaluate_render_quality(
        capture,
        introspection,
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
    Ok(quality)
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
