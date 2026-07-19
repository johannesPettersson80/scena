use std::{collections::BTreeMap, path::Path};

use super::super::push_reason;
use super::depth_of_field_regions_for_expectation;

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_depth_of_field_checks(
    recipe: &scena::SceneRecipeV1,
    recipe_path: &Path,
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    expect: &scena::SceneRecipeExpectV1,
    quality: &mut scena::RenderQualityReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) {
    let Some(depth_of_field) = expect
        .expect_quality
        .as_ref()
        .and_then(|quality| quality.depth_of_field.as_ref())
    else {
        return;
    };
    let regions = match depth_of_field_regions_for_expectation(
        capture,
        introspection,
        composition,
        manifest,
        depth_of_field,
    ) {
        Ok(regions) => regions,
        Err(message) => {
            push_reason(
                reasons,
                "depth_of_field_target_unresolved",
                "quality",
                Some("expect_quality.depth_of_field".to_owned()),
                Vec::new(),
                message,
            );
            return;
        }
    };
    if recipe
        .render
        .as_ref()
        .and_then(|render| render.depth_of_field)
        .is_none()
    {
        quality.checks.push(depth_of_field_single_check(DepthOfFieldSingleCheck {
            code: "depth_of_field_not_enabled",
            severity: "error",
            region: regions.background,
            observed_key: "enabled",
            observed: 0.0,
            threshold_key: "required",
            threshold: 1.0,
            fix_hint: "set render.depth_of_field with focus_distance, aperture_f_stop, and radius_px before expecting depth-of-field quality",
        }));
        return;
    }
    match render_depth_of_field_source_capture(
        recipe,
        recipe_path,
        introspection.capabilities.backend == scena::Backend::HeadlessGpu,
    ) {
        Ok(source) => append_depth_of_field_comparison(
            source,
            capture,
            introspection,
            regions,
            depth_of_field,
            quality,
        ),
        Err(message) => push_reason(
            reasons,
            "depth_of_field_baseline_failed",
            "quality",
            Some("expect_quality.depth_of_field".to_owned()),
            Vec::new(),
            message,
        ),
    }
}

fn append_depth_of_field_comparison(
    source: DepthOfFieldSourceCapture,
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    regions: super::DepthOfFieldRegions,
    depth_of_field: &scena::SceneRecipeQualityDepthOfFieldV1,
    quality: &mut scena::RenderQualityReportV1,
) {
    if source.backend != introspection.capabilities.backend {
        quality.checks.push(depth_of_field_single_check(DepthOfFieldSingleCheck {
            code: "depth_of_field_baseline_backend_mismatch",
            severity: "error",
            region: regions.background,
            observed_key: "backend_matches",
            observed: 0.0,
            threshold_key: "required",
            threshold: 1.0,
            fix_hint: "run the baseline comparison on the same backend as the depth-of-field capture",
        }));
    } else if source.capture.descriptor.width != capture.descriptor.width
        || source.capture.descriptor.height != capture.descriptor.height
    {
        quality.checks.push(depth_of_field_single_check(DepthOfFieldSingleCheck {
            code: "depth_of_field_baseline_size_mismatch",
            severity: "error",
            region: regions.background,
            observed_key: "capture_size_matches",
            observed: 0.0,
            threshold_key: "required",
            threshold: 1.0,
            fix_hint: "render the no-DoF baseline at the same capture resolution before comparing blur",
        }));
    } else {
        quality
            .checks
            .extend(scena::evaluate_depth_of_field_region_quality(
                "expect_quality.depth_of_field",
                scena::DepthOfFieldQualityInput {
                    focused_rgba8: &capture.rgba8,
                    source_rgba8: &source.capture.rgba8,
                    width: capture.descriptor.width,
                    height: capture.descriptor.height,
                    focal_region: regions.focal,
                    background_region: regions.background,
                    expectation: depth_of_field,
                },
            ));
    }
}

struct DepthOfFieldSourceCapture {
    capture: scena::CaptureRgba8,
    backend: scena::Backend,
}

fn render_depth_of_field_source_capture(
    recipe: &scena::SceneRecipeV1,
    recipe_path: &Path,
    use_gpu: bool,
) -> Result<DepthOfFieldSourceCapture, String> {
    let mut source_recipe = recipe.clone();
    if let Some(render) = source_recipe.render.as_mut() {
        render.depth_of_field = None;
    }
    let recipe_text = serde_json::to_string(&source_recipe)
        .map_err(|error| format!("failed to serialize no-DoF baseline recipe: {error}"))?;
    let recipe_path_text = recipe_path.display().to_string();
    let build = if use_gpu {
        pollster::block_on(scena::SceneHostCore::build_recipe_json_gpu(
            &recipe_path_text,
            &recipe_text,
            scena::RecipeBuildPolicy::testing(),
        ))
    } else {
        pollster::block_on(scena::SceneHostCore::build_recipe_json(
            &recipe_path_text,
            &recipe_text,
            scena::RecipeBuildPolicy::testing(),
        ))
    }
    .map_err(|manifest| {
        serde_json::to_string_pretty(&manifest)
            .unwrap_or_else(|error| format!("failed to serialize baseline build error: {error}"))
    })?;
    let mut host = build.host;
    if !source_recipe.cameras.iter().any(|camera| camera.active) {
        host.frame_all_with_overlays()
            .map_err(|error| format!("failed to frame no-DoF baseline: {error}"))?;
    }
    host.prepare()
        .map_err(|error| format!("failed to prepare no-DoF baseline: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render no-DoF baseline: {error}"))?;
    let capture = host
        .capture()
        .map_err(|error| format!("failed to capture no-DoF baseline: {error}"))?;
    let inspection_json = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect no-DoF baseline: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| format!("failed to decode no-DoF baseline inspection: {error}"))?;
    let introspection = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    Ok(DepthOfFieldSourceCapture {
        capture,
        backend: introspection.capabilities.backend,
    })
}

struct DepthOfFieldSingleCheck<'a> {
    code: &'a str,
    severity: &'a str,
    region: scena::RenderQualityRegion,
    observed_key: &'a str,
    observed: f32,
    threshold_key: &'a str,
    threshold: f32,
    fix_hint: &'a str,
}

fn depth_of_field_single_check(input: DepthOfFieldSingleCheck<'_>) -> scena::RenderQualityCheckV1 {
    scena::RenderQualityCheckV1 {
        id: "expect_quality.depth_of_field".to_owned(),
        code: input.code.to_owned(),
        status: if input.severity == "error" {
            scena::RenderQualityStatusV1::Failed
        } else {
            scena::RenderQualityStatusV1::Checked
        },
        severity: input.severity.to_owned(),
        region: scena::RenderQualityRegionV1 {
            kind: input.region.kind.to_owned(),
            handle: input.region.handle,
            rect_css_px: Some(scena::RenderIntrospectionRectV1 {
                min_x: round3(input.region.x as f32),
                min_y: round3(input.region.y as f32),
                max_x: round3(input.region.x.saturating_add(input.region.width) as f32),
                max_y: round3(input.region.y.saturating_add(input.region.height) as f32),
                width: round3(input.region.width as f32),
                height: round3(input.region.height as f32),
            }),
        },
        observed: BTreeMap::from([(input.observed_key.to_owned(), round3(input.observed))]),
        threshold: BTreeMap::from([(input.threshold_key.to_owned(), round3(input.threshold))]),
        fix_hint: input.fix_hint.to_owned(),
    }
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
