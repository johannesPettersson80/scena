mod area_light;
mod checks;
mod depth_of_field;
mod geometry;
mod grid;
mod grounding;
mod metrics;
mod reference;
mod reflection;
mod types;

pub use area_light::{area_light_shadow_metrics, evaluate_area_light_region_quality};
pub use depth_of_field::{
    DepthOfFieldQualityInput, depth_of_field_metrics, evaluate_depth_of_field_region_quality,
};
pub use geometry::{evaluate_geometry_region_quality, geometry_edge_metrics};
pub use grid::{evaluate_grid_line_region_quality, grid_line_metrics};
pub use grounding::{
    RenderQualityGroundingMetrics, contact_shadow_metrics, evaluate_grounding_region_quality,
};
pub use metrics::{frame_metrics, label_background_metrics, label_metrics, line_metrics};
pub use reference::{reference_quality_metrics, ssim_grayscale};
pub use reflection::evaluate_reflection_region_quality;
pub use types::{
    RENDER_QUALITY_SCHEMA_V1, ReferenceQualityMetrics, RenderQualityAreaLightMetrics,
    RenderQualityCheckV1, RenderQualityDepthOfFieldMetrics, RenderQualityFrameMetrics,
    RenderQualityGeometryEdgeMetrics, RenderQualityGridLineMetrics,
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
mod tests;
