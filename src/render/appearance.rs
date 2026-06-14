use std::collections::BTreeSet;

use crate::capture::CaptureRgba8;
use crate::scene::SceneInspectionReportV1;

use super::Renderer;
use super::color_contract::linear_rgba_to_srgb8;

mod evaluation;
mod regions;
mod sampling;
mod types;
use evaluation::{alpha_summary, evaluate_target, material_for_target, push_fix, push_reason};
use regions::TargetSample;
use sampling::{ContentSample, round3, swatch_distance};
pub use types::{
    AppearanceAlphaSummaryV1, AppearanceArtifactsV1, AppearanceCaptureSummaryV1,
    AppearanceExpectationV1, AppearanceFixV1, AppearanceIntrospectionOptions,
    AppearanceIntrospectionReportV1, AppearanceReasonV1, AppearanceRectV1,
    AppearanceSampleRegionV1, AppearanceSummaryV1, AppearanceTargetExpectationV1,
    AppearanceTargetReportV1,
};

pub const APPEARANCE_EXPECTATION_SCHEMA_V1: &str = "scena.appearance_expectation.v1";
pub const APPEARANCE_INTROSPECTION_SCHEMA_V1: &str = "scena.appearance_introspection.v1";

impl AppearanceExpectationV1 {
    pub fn validate_schema(&self) -> Result<(), String> {
        if self.schema != APPEARANCE_EXPECTATION_SCHEMA_V1 {
            return Err(format!(
                "appearance expectation schema must be '{}', got '{}'",
                APPEARANCE_EXPECTATION_SCHEMA_V1, self.schema
            ));
        }
        for target in &self.targets {
            if let Some(tolerance) = target.swatch_tolerance
                && (!tolerance.is_finite() || tolerance < 0.0)
            {
                return Err(format!(
                    "appearance target '{}' swatch_tolerance must be a finite non-negative number",
                    target.id
                ));
            }
        }
        Ok(())
    }

    pub fn first_requested_variant(&self) -> Option<&str> {
        self.targets
            .iter()
            .find_map(|target| target.variant.as_deref())
    }
}

impl AppearanceIntrospectionReportV1 {
    pub fn from_capture(
        capture: &CaptureRgba8,
        inspection: &SceneInspectionReportV1,
        expectation: &AppearanceExpectationV1,
        options: AppearanceIntrospectionOptions,
    ) -> Self {
        let frame_sample = ContentSample::from_rgba8(
            capture.descriptor.width,
            capture.descriptor.height,
            capture.rgba8.as_slice(),
            options.background_rgba8,
            options.content_tolerance_rgba8,
        );
        let mut targets = Vec::new();
        let mut reasons = Vec::new();
        let mut fixes = Vec::new();

        if expectation.targets.is_empty() {
            push_reason(
                &mut reasons,
                "no_expectations",
                "error",
                "appearance",
                Vec::new(),
                "appearance expectation contains no targets",
            );
            push_fix(
                &mut fixes,
                "add_appearance_target",
                "appearance",
                None,
                "add at least one target with an intended color, variant, alpha, or material source policy",
            );
        }
        push_conflicting_variant_warning(expectation, &mut reasons);

        for expected in &expectation.targets {
            let matched = material_for_target(inspection, expected);
            let node_handle = matched.as_ref().map(|target| target.node);
            let material = matched.as_ref().map(|target| target.material.clone());
            let sample = matched
                .as_ref()
                .and_then(|target| {
                    target.draw.map(|draw| {
                        TargetSample::node_bbox(
                            capture,
                            draw,
                            options.background_rgba8,
                            options.content_tolerance_rgba8,
                        )
                    })
                })
                .unwrap_or_else(|| TargetSample::frame_content(frame_sample.clone()));
            let alpha = material
                .as_ref()
                .map(alpha_summary)
                .unwrap_or(AppearanceAlphaSummaryV1 {
                    mode: "unknown".to_owned(),
                    base_color_alpha: 0.0,
                    hidden: true,
                });
            let swatch_distance = expected
                .swatch_srgb8
                .map(|swatch| swatch_distance(sample.content.average_rgba8, swatch));

            targets.push(AppearanceTargetReportV1 {
                id: expected.id.clone(),
                matched: matched.is_some(),
                node: node_handle,
                material: material.clone(),
                sampled_region: AppearanceSampleRegionV1 {
                    kind: sample.kind.to_owned(),
                    sampled_pixels: sample.content.sampled_pixels,
                    bbox_css_px: sample.region_bbox.map(AppearanceRectV1::from),
                },
                sampled_color_srgb8: sample.content.average_rgba8,
                sampled_color_family: sample.content.color_family.clone(),
                swatch_distance,
                alpha,
                expected: expected.clone(),
            });

            evaluate_target(
                expected,
                node_handle,
                material.as_ref(),
                &sample.content,
                &options,
                &mut reasons,
                &mut fixes,
            );
        }

        let errors = reasons
            .iter()
            .filter(|reason| reason.severity == "error")
            .count();
        let warnings = reasons
            .iter()
            .filter(|reason| reason.severity == "warning")
            .count();

        Self {
            schema: APPEARANCE_INTROSPECTION_SCHEMA_V1.to_owned(),
            ok: errors == 0,
            active_variant: options.active_variant,
            available_variants: options.available_variants,
            summary: AppearanceSummaryV1 {
                targets: expectation.targets.len(),
                matched: targets.iter().filter(|target| target.matched).count(),
                errors,
                warnings,
                sampled_pixels: frame_sample.sampled_pixels,
                luminance_mean: round3(frame_sample.luminance_mean),
            },
            targets,
            reasons,
            fixes,
            artifacts: AppearanceArtifactsV1 {
                capture: AppearanceCaptureSummaryV1 {
                    schema: capture.descriptor.schema.clone(),
                    width: capture.descriptor.width,
                    height: capture.descriptor.height,
                    payload_fnv1a64: capture.descriptor.payload.fnv1a64.clone(),
                },
            },
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("appearance introspection report is serializable")
    }
}

fn push_conflicting_variant_warning(
    expectation: &AppearanceExpectationV1,
    reasons: &mut Vec<AppearanceReasonV1>,
) {
    let variants = expectation
        .targets
        .iter()
        .filter_map(|target| target.variant.as_deref())
        .collect::<BTreeSet<_>>();
    if variants.len() <= 1 {
        return;
    }
    push_reason(
        reasons,
        "conflicting_variant_expectations",
        "warning",
        "appearance",
        Vec::new(),
        "appearance expectation requests multiple material variants, but material variants are applied asset-wide for the rendered frame",
    );
}

impl Renderer {
    pub fn introspect_appearance(
        &self,
        capture: &CaptureRgba8,
        inspection: &SceneInspectionReportV1,
        expectation: &AppearanceExpectationV1,
        options: AppearanceIntrospectionOptions,
    ) -> AppearanceIntrospectionReportV1 {
        let options = options.with_background_rgba8(linear_rgba_to_srgb8(self.background_color()));
        AppearanceIntrospectionReportV1::from_capture(capture, inspection, expectation, options)
    }
}
