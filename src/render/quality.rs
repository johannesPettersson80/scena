mod checks;
mod geometry;
mod metrics;
mod reference;
mod types;

pub use geometry::evaluate_geometry_region_quality;
pub use metrics::{
    frame_metrics, geometry_edge_metrics, label_background_metrics, label_metrics, line_metrics,
};
pub use reference::{reference_quality_metrics, ssim_grayscale};
pub use types::{
    RENDER_QUALITY_SCHEMA_V1, ReferenceQualityMetrics, RenderQualityCheckV1,
    RenderQualityFrameMetrics, RenderQualityGeometryEdgeMetrics,
    RenderQualityLabelBackgroundMetrics, RenderQualityLabelMetrics, RenderQualityLineMetrics,
    RenderQualityProfile, RenderQualityRegion, RenderQualityRegionV1, RenderQualityReportV1,
    RenderQualitySummaryV1,
};

use crate::{
    CaptureRgba8, RenderIntrospectionCapabilitiesV1, RenderIntrospectionReportV1,
    SceneRecipeQualityExpectationV1, SceneRecipeQualityLineV1, SceneRecipeQualityTextV1,
};
use checks::{
    SingleValueCheck, ThresholdCheck, push_threshold_check, region_from_rect, single_value_check,
};
use metrics::{
    frame_metrics as compute_frame_metrics,
    label_background_metrics as compute_label_background_metrics,
    label_metrics as compute_label_metrics, line_metrics as compute_line_metrics,
};

#[derive(Debug, Clone, Copy)]
pub struct RenderQualityRgba8Input<'a> {
    pub rgba8: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub capabilities: RenderIntrospectionCapabilitiesV1,
    pub visible_pixel_fraction: f32,
    pub tiny_in_frame: bool,
    pub fit_fraction: f32,
}

pub fn evaluate_render_quality(
    capture: &CaptureRgba8,
    introspection: &RenderIntrospectionReportV1,
    expectation: Option<&SceneRecipeQualityExpectationV1>,
) -> RenderQualityReportV1 {
    let full_frame =
        RenderQualityRegion::full_frame(capture.descriptor.width, capture.descriptor.height);
    let region = introspection
        .content_bbox_css_px
        .map(|rect| {
            region_from_rect(
                "subject",
                rect,
                capture.descriptor.width,
                capture.descriptor.height,
            )
        })
        .unwrap_or(full_frame);
    evaluate_render_quality_rgba8_region(
        RenderQualityRgba8Input {
            rgba8: &capture.rgba8,
            width: capture.descriptor.width,
            height: capture.descriptor.height,
            capabilities: introspection.capabilities,
            visible_pixel_fraction: introspection.visible_pixel_fraction,
            tiny_in_frame: introspection.framing.tiny_in_frame,
            fit_fraction: introspection.framing.fit_fraction,
        },
        region,
        expectation,
    )
}

pub fn evaluate_render_quality_rgba8(
    input: RenderQualityRgba8Input<'_>,
    expectation: Option<&SceneRecipeQualityExpectationV1>,
) -> RenderQualityReportV1 {
    evaluate_render_quality_rgba8_region(
        input,
        RenderQualityRegion::full_frame(input.width, input.height),
        expectation,
    )
}

fn evaluate_render_quality_rgba8_region(
    input: RenderQualityRgba8Input<'_>,
    region: RenderQualityRegion,
    expectation: Option<&SceneRecipeQualityExpectationV1>,
) -> RenderQualityReportV1 {
    let profile = expectation
        .and_then(|expectation| RenderQualityProfile::parse(&expectation.profile))
        .unwrap_or(RenderQualityProfile::Product);
    let metrics = compute_frame_metrics(input.rgba8, input.width, input.height, region);
    let mut checks = Vec::new();

    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id: "baseline.black_crush",
            code: "severe_black_crush",
            severity: "error",
            region,
            observed_key: "low_clip_fraction",
            observed: metrics.low_clip_fraction,
            threshold_key: "max_low_clip_fraction",
            threshold: profile.severe_black_crush_max(),
            fails: metrics.low_clip_fraction > profile.severe_black_crush_max(),
            fix_hint: "add lights, raise exposure, or use a lighter background for the subject",
        },
    );
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id: "baseline.blown_out",
            code: "severe_blown_out",
            severity: "error",
            region,
            observed_key: "high_clip_fraction",
            observed: metrics.high_clip_fraction,
            threshold_key: "max_high_clip_fraction",
            threshold: profile.severe_high_clip_max(),
            fails: metrics.high_clip_fraction > profile.severe_high_clip_max(),
            fix_hint: "lower exposure or reduce light intensity so highlights retain detail",
        },
    );
    if input.visible_pixel_fraction <= 0.001 {
        checks.push(single_value_check(SingleValueCheck {
            id: "baseline.blank",
            code: "blank_frame",
            severity: "error",
            region,
            observed_key: "visible_pixel_fraction",
            observed: input.visible_pixel_fraction,
            threshold_key: "min_visible_pixel_fraction",
            threshold: 0.001,
            fix_hint: "frame the subject or make the target visible before capturing",
        }));
    }
    if input.tiny_in_frame {
        checks.push(single_value_check(SingleValueCheck {
            id: "baseline.tiny",
            code: "subject_tiny_in_frame",
            severity: "warning",
            region,
            observed_key: "fit_fraction",
            observed: input.fit_fraction,
            threshold_key: "min_fit_fraction",
            threshold: 0.05,
            fix_hint: "frame the subject tighter or use frame_all_with_overlays for overlay-heavy scenes",
        }));
    }

    if let Some(expectation) = expectation {
        if let Some(exposure) = expectation.exposure {
            if let Some(max_low) = exposure.max_low_clip_fraction {
                push_threshold_check(
                    &mut checks,
                    ThresholdCheck {
                        id: "expect_quality.exposure.low_clip",
                        code: "low_clip_fraction_too_high",
                        severity: "error",
                        region,
                        observed_key: "low_clip_fraction",
                        observed: metrics.low_clip_fraction,
                        threshold_key: "max_low_clip_fraction",
                        threshold: max_low as f32,
                        fails: metrics.low_clip_fraction > max_low as f32,
                        fix_hint: "add lighting, increase exposure, or separate the subject from a black background",
                    },
                );
            }
            if let Some(max_high) = exposure.max_high_clip_fraction {
                push_threshold_check(
                    &mut checks,
                    ThresholdCheck {
                        id: "expect_quality.exposure.high_clip",
                        code: "high_clip_fraction_too_high",
                        severity: "error",
                        region,
                        observed_key: "high_clip_fraction",
                        observed: metrics.high_clip_fraction,
                        threshold_key: "max_high_clip_fraction",
                        threshold: max_high as f32,
                        fails: metrics.high_clip_fraction > max_high as f32,
                        fix_hint: "lower exposure or reduce light intensity",
                    },
                );
            }
        }

        if let Some(contrast) = expectation.contrast {
            let min_range = contrast
                .min_luminance_range
                .map(|value| value as f32)
                .unwrap_or_else(|| profile.default_min_luminance_range());
            push_threshold_check(
                &mut checks,
                ThresholdCheck {
                    id: "expect_quality.contrast.range",
                    code: "contrast_too_flat",
                    severity: "error",
                    region,
                    observed_key: "luminance_range",
                    observed: metrics.luminance_range,
                    threshold_key: "min_luminance_range",
                    threshold: min_range,
                    fails: metrics.luminance_range < min_range,
                    fix_hint: "add a light rig/environment or use material/background colors with more separation",
                },
            );
            if let Some(min_sobel) = contrast.min_sobel_energy {
                push_threshold_check(
                    &mut checks,
                    ThresholdCheck {
                        id: "expect_quality.contrast.edges",
                        code: "edge_energy_too_low",
                        severity: "error",
                        region,
                        observed_key: "sobel_energy",
                        observed: metrics.sobel_energy,
                        threshold_key: "min_sobel_energy",
                        threshold: min_sobel as f32,
                        fails: metrics.sobel_energy < min_sobel as f32,
                        fix_hint: "increase capture resolution or add lighting/background contrast at object edges",
                    },
                );
            }
        }

        if let Some(noise) = expectation.noise
            && let Some(max_noise) = noise.max_outlier_fraction
        {
            push_threshold_check(
                &mut checks,
                ThresholdCheck {
                    id: "expect_quality.noise.outliers",
                    code: "noise_outlier_fraction_too_high",
                    severity: "error",
                    region,
                    observed_key: "noise_outlier_fraction",
                    observed: metrics.noise_outlier_fraction,
                    threshold_key: "max_outlier_fraction",
                    threshold: max_noise as f32,
                    fails: metrics.noise_outlier_fraction > max_noise as f32,
                    fix_hint: "raise sample quality, disable unstable effects, or inspect firefly-producing materials",
                },
            );
        }

        if let Some(text) = expectation.text {
            checks.extend(evaluate_label_region_quality(
                "expect_quality.text",
                input.rgba8,
                input.width,
                input.height,
                region,
                text,
            ));
        }
        if let Some(geometry) = expectation.geometry {
            checks.extend(evaluate_geometry_region_quality(
                "expect_quality.geometry",
                input.rgba8,
                input.width,
                input.height,
                region,
                geometry,
            ));
        }
    }

    RenderQualityReportV1::from_checks(profile, input.capabilities, checks)
}

pub fn evaluate_label_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    expectation: SceneRecipeQualityTextV1,
) -> Vec<RenderQualityCheckV1> {
    evaluate_label_region_quality_with_background(
        id,
        rgba8,
        width,
        height,
        region,
        expectation,
        None,
    )
}

pub fn evaluate_label_region_quality_with_background(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    expectation: SceneRecipeQualityTextV1,
    expected_background_srgb8: Option<[u8; 3]>,
) -> Vec<RenderQualityCheckV1> {
    let metrics = compute_label_metrics(rgba8, width, height, region);
    let max_isolation = expectation.max_ink_isolation.unwrap_or(0.01) as f32;
    let min_coverage = expectation.min_ink_coverage.unwrap_or(0.10) as f32;
    let min_intermediate = expectation.min_intermediate_edge_fraction.unwrap_or(0.01) as f32;
    let mut checks = Vec::new();
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id,
            code: "label_ink_isolation",
            severity: "error",
            region,
            observed_key: "ink_isolation",
            observed: metrics.ink_isolation,
            threshold_key: "max_ink_isolation",
            threshold: max_isolation,
            fails: metrics.ink_isolation > max_isolation,
            fix_hint: "use the TrueType coverage path and render labels at native resolution; increase label size if strokes are fragmented",
        },
    );
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id,
            code: "label_ink_coverage_too_low",
            severity: "error",
            region,
            observed_key: "ink_coverage",
            observed: metrics.ink_coverage,
            threshold_key: "min_ink_coverage",
            threshold: min_coverage,
            fails: metrics.ink_coverage < min_coverage,
            fix_hint: "increase label size, foreground contrast, or capture resolution",
        },
    );
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id,
            code: "label_missing_antialiasing",
            severity: "error",
            region,
            observed_key: "intermediate_edge_fraction",
            observed: metrics.intermediate_edge_fraction,
            threshold_key: "min_intermediate_edge_fraction",
            threshold: min_intermediate,
            fails: metrics.intermediate_edge_fraction < min_intermediate,
            fix_hint: "preserve font coverage and avoid thresholding glyph alpha into 1-bit cells",
        },
    );
    if let Some(expected_background_srgb8) = expected_background_srgb8 {
        let background = compute_label_background_metrics(
            rgba8,
            width,
            height,
            region,
            expected_background_srgb8,
        );
        let max_luminance_range = expectation.max_background_luminance_range.unwrap_or(0.03) as f32;
        let max_mean_delta = expectation.max_background_mean_delta.unwrap_or(0.03) as f32;
        push_threshold_check(
            &mut checks,
            ThresholdCheck {
                id,
                code: "label_background_not_uniform",
                severity: "error",
                region,
                observed_key: "background_luminance_range",
                observed: background.luminance_range,
                threshold_key: "max_background_luminance_range",
                threshold: max_luminance_range,
                fails: background.luminance_range > max_luminance_range,
                fix_hint: "render label backgrounds in the final flat overlay pass as one opaque unlit fill",
            },
        );
        push_threshold_check(
            &mut checks,
            ThresholdCheck {
                id,
                code: "label_background_color_mismatch",
                severity: "error",
                region,
                observed_key: "background_mean_rgb_delta",
                observed: background.mean_rgb_delta,
                threshold_key: "max_background_mean_delta",
                threshold: max_mean_delta,
                fails: background.mean_rgb_delta > max_mean_delta,
                fix_hint: "draw the label background after post-processing using the authored label background color",
            },
        );
    }
    checks
}

pub fn evaluate_line_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    expectation: SceneRecipeQualityLineV1,
) -> Vec<RenderQualityCheckV1> {
    let metrics = compute_line_metrics(rgba8, width, height, region);
    let min_intermediate = expectation.min_intermediate_edge_fraction.unwrap_or(0.02) as f32;
    let max_straightness = expectation.max_straightness_error.unwrap_or(0.08) as f32;
    let mut checks = Vec::new();
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id,
            code: "line_missing_antialiasing",
            severity: "error",
            region,
            observed_key: "intermediate_edge_fraction",
            observed: metrics.intermediate_edge_fraction,
            threshold_key: "min_intermediate_edge_fraction",
            threshold: min_intermediate,
            fails: metrics.intermediate_edge_fraction < min_intermediate,
            fix_hint: "render dimension and leader lines through the antialiased stroke pass instead of hard 1px geometry",
        },
    );
    push_threshold_check(
        &mut checks,
        ThresholdCheck {
            id,
            code: "line_not_straight",
            severity: "error",
            region,
            observed_key: "straightness_error",
            observed: metrics.straightness_error,
            threshold_key: "max_straightness_error",
            threshold: max_straightness,
            fails: metrics.straightness_error > max_straightness,
            fix_hint: "use one continuous stroke segment for each dimension or leader line and avoid segmented stair-step geometry",
        },
    );
    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn label_quality_known_bad_fails_exact_reason_and_good_passes() {
        let width = 32;
        let height = 16;
        let region = RenderQualityRegion::full_frame(width, height);
        let expectation = SceneRecipeQualityTextV1 {
            min_ink_coverage: Some(0.10),
            max_ink_isolation: Some(0.01),
            min_intermediate_edge_fraction: Some(0.01),
            max_background_luminance_range: None,
            max_background_mean_delta: None,
        };
        let bad = eroded_label_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/label-known-bad-eroded.ppm",
            &bad,
            width,
            height,
        );
        let bad_checks =
            evaluate_label_region_quality("bad-label", &bad, width, height, region, expectation);
        assert!(
            bad_checks
                .iter()
                .any(|check| check.code == "label_ink_isolation"),
            "eroded label fixture must fail with label_ink_isolation: {bad_checks:#?}"
        );

        let good = antialiased_label_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/label-known-good-antialiased.ppm",
            &good,
            width,
            height,
        );
        let good_checks =
            evaluate_label_region_quality("good-label", &good, width, height, region, expectation);
        assert!(
            good_checks.is_empty(),
            "known-good antialiased label should pass quality checks: {good_checks:#?}"
        );
    }

    #[test]
    fn label_quality_known_bad_gpu_eroded_fixture_fails_exact_reason() {
        let (rgba8, width, height) = read_ppm_fixture(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/assets/render-quality/label-known-bad-gpu-eroded.ppm"),
        );
        let region = RenderQualityRegion {
            kind: "label",
            handle: None,
            x: 95,
            y: 55,
            width: 230,
            height: 30,
        };
        let expectation = SceneRecipeQualityTextV1 {
            min_ink_coverage: Some(0.30),
            max_ink_isolation: Some(0.01),
            min_intermediate_edge_fraction: Some(0.01),
            max_background_luminance_range: None,
            max_background_mean_delta: None,
        };
        let checks =
            evaluate_label_region_quality("gpu-eroded", &rgba8, width, height, region, expectation);
        write_ppm_crop_artifact(
            "target/gate-artifacts/render-quality/label-known-bad-gpu-eroded-crop.ppm",
            &rgba8,
            width,
            region,
        );
        assert!(
            checks
                .iter()
                .any(|check| check.code == "label_ink_coverage_too_low"),
            "old GPU-eroded label fixture must fail exact label_ink_coverage_too_low: {checks:#?}"
        );
    }

    #[test]
    fn label_quality_known_bad_bitmap_fixture_fails_antialiasing_reason() {
        let width = 80;
        let height = 28;
        let region = RenderQualityRegion::full_frame(width, height);
        let expectation = SceneRecipeQualityTextV1 {
            min_ink_coverage: Some(0.08),
            max_ink_isolation: Some(0.02),
            min_intermediate_edge_fraction: Some(0.01),
            max_background_luminance_range: None,
            max_background_mean_delta: None,
        };
        let bad = blocky_text_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/label-known-bad-bitmap.ppm",
            &bad,
            width,
            height,
        );
        let checks = evaluate_label_region_quality(
            "bad-bitmap-label",
            &bad,
            width,
            height,
            region,
            expectation,
        );
        assert!(
            checks
                .iter()
                .any(|check| check.code == "label_missing_antialiasing"),
            "old blocky bitmap-style label fixture must fail exact label_missing_antialiasing: {checks:#?}"
        );
    }

    #[test]
    fn label_background_quality_known_bad_fails_exact_reason_and_good_passes() {
        let width = 48;
        let height = 24;
        let region = RenderQualityRegion::full_frame(width, height);
        let expected = [29, 39, 51];
        let expectation = SceneRecipeQualityTextV1 {
            min_ink_coverage: Some(0.0),
            max_ink_isolation: Some(1.0),
            min_intermediate_edge_fraction: Some(0.0),
            max_background_luminance_range: Some(0.02),
            max_background_mean_delta: Some(0.02),
        };

        let bad = nonuniform_label_background_fixture(width, height, expected);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/label-background-known-bad.ppm",
            &bad,
            width,
            height,
        );
        let bad_checks = evaluate_label_region_quality_with_background(
            "bad-label-background",
            &bad,
            width,
            height,
            region,
            expectation,
            Some(expected),
        );
        assert!(
            bad_checks
                .iter()
                .any(|check| check.code == "label_background_not_uniform"),
            "non-uniform label background fixture must fail exact label_background_not_uniform: {bad_checks:#?}"
        );

        let good = solid_label_background_fixture(width, height, expected);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/label-background-known-good.ppm",
            &good,
            width,
            height,
        );
        let good_checks = evaluate_label_region_quality_with_background(
            "good-label-background",
            &good,
            width,
            height,
            region,
            expectation,
            Some(expected),
        );
        assert!(
            good_checks.is_empty(),
            "flat authored label background should pass quality checks: {good_checks:#?}"
        );
    }

    #[test]
    fn line_quality_known_bad_fails_exact_reason_and_good_passes() {
        let width = 64;
        let height = 32;
        let region = RenderQualityRegion::full_frame(width, height);
        let expectation = SceneRecipeQualityLineV1 {
            min_intermediate_edge_fraction: Some(0.05),
            max_straightness_error: Some(0.08),
        };
        let bad = aliased_line_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/line-known-bad-aliased.ppm",
            &bad,
            width,
            height,
        );
        let bad_checks =
            evaluate_line_region_quality("bad-line", &bad, width, height, region, expectation);
        assert!(
            bad_checks
                .iter()
                .any(|check| check.code == "line_missing_antialiasing"),
            "hard 1px line fixture must fail exact line_missing_antialiasing: {bad_checks:#?}"
        );

        let good = antialiased_line_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/line-known-good-antialiased.ppm",
            &good,
            width,
            height,
        );
        let good_checks =
            evaluate_line_region_quality("good-line", &good, width, height, region, expectation);
        assert!(
            good_checks.is_empty(),
            "known-good antialiased straight line should pass quality checks: {good_checks:#?}"
        );
    }

    #[test]
    fn geometry_edge_quality_known_bad_fails_exact_reason_and_good_passes() {
        let width = 32;
        let height = 24;
        let region = RenderQualityRegion::full_frame(width, height);
        let expectation = crate::SceneRecipeQualityGeometryV1 {
            min_intermediate_edge_fraction: Some(0.05),
        };

        let bad = hard_geometry_edge_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/geometry-edge-known-bad-hard.ppm",
            &bad,
            width,
            height,
        );
        let bad_checks = evaluate_geometry_region_quality(
            "bad-geometry-edge",
            &bad,
            width,
            height,
            region,
            expectation,
        );
        assert!(
            bad_checks
                .iter()
                .any(|check| check.code == "geometry_missing_antialiasing"),
            "hard 1-bit geometry edge fixture must fail exact geometry_missing_antialiasing: {bad_checks:#?}"
        );

        let good = antialiased_geometry_edge_fixture(width, height);
        write_ppm_artifact(
            "target/gate-artifacts/render-quality/geometry-edge-known-good-sampled.ppm",
            &good,
            width,
            height,
        );
        let good_checks = evaluate_geometry_region_quality(
            "good-geometry-edge",
            &good,
            width,
            height,
            region,
            expectation,
        );
        assert!(
            good_checks.is_empty(),
            "known-good sampled geometry edge should pass quality checks: {good_checks:#?}"
        );
    }

    #[test]
    fn exposure_quality_known_bad_fails_exact_black_crush_reason() {
        let width = 20;
        let height = 20;
        let mut bad = vec![0; (width * height * 4) as usize];
        for pixel in bad.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        let report = evaluate_render_quality_rgba8(
            quality_input(&bad, width, height, 0.2, false, 0.5),
            None,
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == "severe_black_crush"),
            "pitch-black fixture must fail with severe_black_crush: {report:#?}"
        );
    }

    #[test]
    fn baseline_quality_fixtures_cover_blank_blown_out_and_tiny_codes() {
        let width = 16;
        let height = 16;
        let white = solid_frame(width, height, [255, 255, 255, 255]);
        let blown_out = evaluate_render_quality_rgba8(
            quality_input(&white, width, height, 0.5, false, 0.5),
            None,
        );
        assert_has_code(&blown_out, "severe_blown_out");

        let gray = solid_frame(width, height, [128, 128, 128, 255]);
        let blank = evaluate_render_quality_rgba8(
            quality_input(&gray, width, height, 0.0, false, 0.5),
            None,
        );
        assert_has_code(&blank, "blank_frame");

        let tiny = evaluate_render_quality_rgba8(
            quality_input(&gray, width, height, 0.5, true, 0.02),
            None,
        );
        assert_has_code(&tiny, "subject_tiny_in_frame");

        let good = evaluate_render_quality_rgba8(
            quality_input(&gray, width, height, 0.5, false, 0.5),
            None,
        );
        assert_lacks_code(&good, "severe_blown_out");
        assert_lacks_code(&good, "blank_frame");
        assert_lacks_code(&good, "subject_tiny_in_frame");
    }

    #[test]
    fn opt_in_quality_fixtures_cover_exposure_contrast_and_noise_codes() {
        let width = 32;
        let height = 32;
        let exposure_expectation = SceneRecipeQualityExpectationV1 {
            profile: "product".to_owned(),
            exposure: Some(crate::SceneRecipeQualityExposureV1 {
                max_low_clip_fraction: Some(0.40),
                max_high_clip_fraction: Some(0.40),
            }),
            contrast: None,
            noise: None,
            text: None,
            line: None,
            geometry: None,
        };
        let mut low_clip = solid_frame(width, height, [96, 96, 96, 255]);
        for pixel in low_clip
            .chunks_exact_mut(4)
            .take((width * height / 2) as usize)
        {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        }
        let low_report = evaluate_render_quality_rgba8(
            quality_input(&low_clip, width, height, 0.5, false, 0.5),
            Some(&exposure_expectation),
        );
        assert_has_code(&low_report, "low_clip_fraction_too_high");
        let mut high_clip = solid_frame(width, height, [96, 96, 96, 255]);
        for pixel in high_clip
            .chunks_exact_mut(4)
            .take((width * height / 2) as usize)
        {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
        }
        let high_report = evaluate_render_quality_rgba8(
            quality_input(&high_clip, width, height, 0.5, false, 0.5),
            Some(&exposure_expectation),
        );
        assert_has_code(&high_report, "high_clip_fraction_too_high");
        let exposure_good_frame = solid_frame(width, height, [96, 96, 96, 255]);
        let exposure_good = evaluate_render_quality_rgba8(
            quality_input(&exposure_good_frame, width, height, 0.5, false, 0.5),
            Some(&exposure_expectation),
        );
        assert_lacks_code(&exposure_good, "low_clip_fraction_too_high");
        assert_lacks_code(&exposure_good, "high_clip_fraction_too_high");

        let contrast_expectation = SceneRecipeQualityExpectationV1 {
            profile: "product".to_owned(),
            exposure: None,
            contrast: Some(crate::SceneRecipeQualityContrastV1 {
                min_luminance_range: Some(0.25),
                min_sobel_energy: Some(0.05),
            }),
            noise: None,
            text: None,
            line: None,
            geometry: None,
        };
        let flat_frame = solid_frame(width, height, [128, 128, 128, 255]);
        let flat = evaluate_render_quality_rgba8(
            quality_input(&flat_frame, width, height, 0.5, false, 0.5),
            Some(&contrast_expectation),
        );
        assert_has_code(&flat, "contrast_too_flat");
        assert_has_code(&flat, "edge_energy_too_low");
        let checker = checker_frame(width, height);
        let contrast_good = evaluate_render_quality_rgba8(
            quality_input(&checker, width, height, 0.5, false, 0.5),
            Some(&contrast_expectation),
        );
        assert_lacks_code(&contrast_good, "contrast_too_flat");
        assert_lacks_code(&contrast_good, "edge_energy_too_low");

        let noise_expectation = SceneRecipeQualityExpectationV1 {
            profile: "product".to_owned(),
            exposure: None,
            contrast: None,
            noise: Some(crate::SceneRecipeQualityNoiseV1 {
                max_outlier_fraction: Some(0.01),
            }),
            text: None,
            line: None,
            geometry: None,
        };
        let mut noisy = solid_frame(width, height, [128, 128, 128, 255]);
        for y in (2..height - 2).step_by(4) {
            for x in (2..width - 2).step_by(4) {
                set_gray(&mut noisy, width, x, y, 255);
            }
        }
        let noisy_report = evaluate_render_quality_rgba8(
            quality_input(&noisy, width, height, 0.5, false, 0.5),
            Some(&noise_expectation),
        );
        assert_has_code(&noisy_report, "noise_outlier_fraction_too_high");
        let noise_good_frame = solid_frame(width, height, [128, 128, 128, 255]);
        let noise_good = evaluate_render_quality_rgba8(
            quality_input(&noise_good_frame, width, height, 0.5, false, 0.5),
            Some(&noise_expectation),
        );
        assert_lacks_code(&noise_good, "noise_outlier_fraction_too_high");
    }

    #[test]
    fn reference_quality_fixtures_cover_abs_delta_e_and_ssim_metrics() {
        let width = 8;
        let height = 8;
        let expected = solid_frame(width, height, [80, 120, 160, 255]);
        let actual = solid_frame(width, height, [160, 80, 80, 255]);
        let changed = reference_quality_metrics(&actual, &expected, width, height)
            .expect("reference metrics compute for matching dimensions");
        assert!(changed.mean_abs_diff > 20.0, "{changed:#?}");
        assert!(changed.mean_delta_e2000 > 10.0, "{changed:#?}");
        assert!(changed.ssim < 0.99, "{changed:#?}");

        let identical = reference_quality_metrics(&expected, &expected, width, height)
            .expect("reference metrics compute for identical fixtures");
        assert_eq!(identical.mean_abs_diff, 0.0);
        assert_eq!(identical.mean_delta_e2000, 0.0);
        assert_eq!(identical.ssim, 1.0);
    }

    #[test]
    fn grayscale_ssim_matches_reference_anchors() {
        let width = 8;
        let height = 8;
        let mut a = vec![0; (width * height * 4) as usize];
        for pixel in a.chunks_exact_mut(4) {
            pixel[0] = 128;
            pixel[1] = 128;
            pixel[2] = 128;
            pixel[3] = 255;
        }
        let mut b = a.clone();
        for pixel in b.chunks_exact_mut(4).take(8) {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
        }
        let identical = ssim_grayscale(&a, &a, width, height).expect("SSIM computes");
        let changed = ssim_grayscale(&a, &b, width, height).expect("SSIM computes");
        assert_eq!(types::round3(identical), 1.0);
        assert!(
            changed < 0.9,
            "changed reference pair must produce lower SSIM before SSIM gates anything: {changed}"
        );
    }

    fn eroded_label_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        for x in (2..width.saturating_sub(2)).step_by(3) {
            for y in (3..height.saturating_sub(3)).step_by(4) {
                set_gray(&mut rgba, width, x, y, 255);
            }
        }
        rgba
    }

    fn antialiased_label_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        for y in 4..12 {
            for x in 3..29 {
                let edge = x == 3 || x == 28 || y == 4 || y == 11;
                set_gray(&mut rgba, width, x, y, if edge { 96 } else { 230 });
            }
        }
        rgba
    }

    fn blocky_text_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        for cell_y in 0..7 {
            for cell_x in 0..18 {
                let on = cell_x % 3 != 1 || cell_y == 0 || cell_y == 3 || cell_y == 6;
                if !on {
                    continue;
                }
                for y in 0..3 {
                    for x in 0..3 {
                        set_gray(
                            &mut rgba,
                            width,
                            8 + cell_x * 3 + x,
                            4 + cell_y * 3 + y,
                            255,
                        );
                    }
                }
            }
        }
        rgba
    }

    fn solid_label_background_fixture(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..width.saturating_mul(height) {
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
        rgba
    }

    fn nonuniform_label_background_fixture(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
        let mut rgba = solid_label_background_fixture(width, height, rgb);
        for y in 0..height {
            for x in 0..width {
                if x > width / 2 || y > height * 2 / 3 {
                    let offset = ((y * width + x) * 4) as usize;
                    rgba[offset] = rgba[offset].saturating_add(58);
                    rgba[offset + 1] = rgba[offset + 1].saturating_add(58);
                    rgba[offset + 2] = rgba[offset + 2].saturating_add(58);
                }
            }
        }
        rgba
    }

    fn aliased_line_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        for x in 6..width.saturating_sub(6) {
            let t = (x - 6) as f32 / width.saturating_sub(12).max(1) as f32;
            let y = (height as f32 * 0.75 + (height as f32 * -0.5) * t).round() as u32;
            set_gray(&mut rgba, width, x, y.min(height.saturating_sub(1)), 255);
        }
        rgba
    }

    fn antialiased_line_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        let start = (6.0f32, height as f32 * 0.75);
        let end = (width as f32 - 6.0, height as f32 * 0.25);
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let length_squared = dx * dx + dy * dy;
        for y in 0..height {
            for x in 0..width {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let t =
                    (((px - start.0) * dx + (py - start.1) * dy) / length_squared).clamp(0.0, 1.0);
                let closest_x = start.0 + dx * t;
                let closest_y = start.1 + dy * t;
                let distance = ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt();
                let coverage = if distance <= 1.0 {
                    1.0
                } else {
                    (2.0 - distance).clamp(0.0, 1.0)
                };
                if coverage > 0.0 {
                    set_gray(&mut rgba, width, x, y, (coverage * 255.0).round() as u8);
                }
            }
        }
        rgba
    }

    fn hard_geometry_edge_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        for y in 3..height.saturating_sub(3) {
            for x in width / 2..width.saturating_sub(3) {
                set_gray(&mut rgba, width, x, y, 255);
            }
        }
        rgba
    }

    fn antialiased_geometry_edge_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        let edge = width / 2;
        for y in 3..height.saturating_sub(3) {
            for x in edge.saturating_sub(1)..width.saturating_sub(3) {
                let value = if x == edge.saturating_sub(1) {
                    72
                } else if x == edge {
                    192
                } else {
                    255
                };
                set_gray(&mut rgba, width, x, y, value);
            }
        }
        rgba
    }

    fn black_frame(width: u32, height: u32) -> Vec<u8> {
        solid_frame(width, height, [0, 0, 0, 255])
    }

    fn solid_frame(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        let mut rgba = vec![0; (width * height * 4) as usize];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        rgba
    }

    fn checker_frame(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = black_frame(width, height);
        for y in 0..height {
            for x in 0..width {
                let value = if (x / 4 + y / 4) % 2 == 0 { 32 } else { 224 };
                set_gray(&mut rgba, width, x, y, value);
            }
        }
        rgba
    }

    fn set_gray(rgba: &mut [u8], width: u32, x: u32, y: u32, value: u8) {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset] = value;
        rgba[offset + 1] = value;
        rgba[offset + 2] = value;
        rgba[offset + 3] = 255;
    }

    fn minimal_capabilities() -> RenderIntrospectionCapabilitiesV1 {
        RenderIntrospectionCapabilitiesV1 {
            backend: crate::diagnostics::Backend::Headless,
            gpu_device: false,
            surface_attached: false,
            hardware_tier: crate::diagnostics::HardwareTier::Low,
            forward_pbr: crate::diagnostics::CapabilityStatus::ErrorIfRequired,
            readback_headless_screenshots: crate::diagnostics::CapabilityStatus::Supported,
        }
    }

    fn quality_input<'a>(
        rgba8: &'a [u8],
        width: u32,
        height: u32,
        visible_pixel_fraction: f32,
        tiny_in_frame: bool,
        fit_fraction: f32,
    ) -> RenderQualityRgba8Input<'a> {
        RenderQualityRgba8Input {
            rgba8,
            width,
            height,
            capabilities: minimal_capabilities(),
            visible_pixel_fraction,
            tiny_in_frame,
            fit_fraction,
        }
    }

    fn assert_has_code(report: &RenderQualityReportV1, code: &str) {
        assert!(
            report.checks.iter().any(|check| check.code == code),
            "expected quality code {code} in report: {report:#?}"
        );
    }

    fn assert_lacks_code(report: &RenderQualityReportV1, code: &str) {
        assert!(
            report.checks.iter().all(|check| check.code != code),
            "expected no quality code {code} in report: {report:#?}"
        );
    }

    fn write_ppm_crop_artifact(
        path: &str,
        rgba: &[u8],
        frame_width: u32,
        region: RenderQualityRegion,
    ) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("render-quality artifact dir exists");
        }
        let mut ppm = format!("P6\n{} {}\n255\n", region.width, region.height).into_bytes();
        for y in region.y..region.y.saturating_add(region.height) {
            for x in region.x..region.x.saturating_add(region.width) {
                let offset = ((y * frame_width + x) * 4) as usize;
                ppm.extend_from_slice(&rgba[offset..offset + 3]);
            }
        }
        std::fs::write(path, ppm).expect("render-quality crop artifact writes");
    }

    fn write_ppm_artifact(path: &str, rgba8: &[u8], width: u32, height: u32) {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("artifact directory is created");
        }
        let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
        for pixel in rgba8.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(path, ppm).expect("quality fixture artifact writes");
    }

    fn read_ppm_fixture(path: impl AsRef<Path>) -> (Vec<u8>, u32, u32) {
        let bytes = std::fs::read(path.as_ref()).expect("PPM fixture reads");
        let mut cursor = 0;
        let magic = next_ppm_token(&bytes, &mut cursor).expect("PPM magic");
        assert_eq!(magic, "P6");
        let width = next_ppm_token(&bytes, &mut cursor)
            .expect("PPM width")
            .parse::<u32>()
            .expect("PPM width parses");
        let height = next_ppm_token(&bytes, &mut cursor)
            .expect("PPM height")
            .parse::<u32>()
            .expect("PPM height parses");
        let max = next_ppm_token(&bytes, &mut cursor).expect("PPM max value");
        assert_eq!(max, "255");
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        let rgb = &bytes[cursor..];
        assert_eq!(rgb.len(), (width * height * 3) as usize);
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for pixel in rgb.chunks_exact(3) {
            rgba.extend_from_slice(pixel);
            rgba.push(255);
        }
        (rgba, width, height)
    }

    fn next_ppm_token(bytes: &[u8], cursor: &mut usize) -> Option<String> {
        while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
            *cursor += 1;
        }
        let start = *cursor;
        while bytes
            .get(*cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace())
        {
            *cursor += 1;
        }
        (start < *cursor).then(|| String::from_utf8_lossy(&bytes[start..*cursor]).into_owned())
    }
}
