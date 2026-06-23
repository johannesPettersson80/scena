use serde_json::Value;

use super::super::diagnostic;
use super::{validate_known_fields, validate_target};
use crate::scene::recipe::SceneRecipeDiagnosticV1;

const QUALITY_FIELDS: &[&str] = &[
    "profile",
    "exposure",
    "contrast",
    "noise",
    "text",
    "line",
    "geometry",
    "reflection",
    "area_light",
    "grounding",
    "depth_of_field",
];
const QUALITY_EXPOSURE_FIELDS: &[&str] = &["max_low_clip_fraction", "max_high_clip_fraction"];
const QUALITY_CONTRAST_FIELDS: &[&str] = &["min_luminance_range", "min_sobel_energy"];
const QUALITY_NOISE_FIELDS: &[&str] = &["max_outlier_fraction"];
const QUALITY_TEXT_FIELDS: &[&str] = &[
    "min_ink_coverage",
    "max_ink_isolation",
    "min_intermediate_edge_fraction",
    "max_background_luminance_range",
    "max_background_mean_delta",
];
const QUALITY_LINE_FIELDS: &[&str] = &["min_intermediate_edge_fraction", "max_straightness_error"];
const QUALITY_GEOMETRY_FIELDS: &[&str] = &["min_intermediate_edge_fraction"];
const QUALITY_REFLECTION_FIELDS: &[&str] = &[
    "target",
    "min_luminance_range",
    "min_sobel_energy",
    "min_chroma_range",
    "max_firefly_fraction",
];
const QUALITY_AREA_LIGHT_FIELDS: &[&str] = &[
    "target",
    "min_shadow_contrast",
    "min_penumbra_width_px",
    "min_penumbra_luma_levels",
    "min_emitter_extent_meters",
];
const QUALITY_GROUNDING_FIELDS: &[&str] = &["target", "min_contact_shadow_delta"];
const QUALITY_DEPTH_OF_FIELD_FIELDS: &[&str] = &[
    "target",
    "background_target",
    "min_source_background_sobel",
    "min_background_sobel_drop",
    "min_background_sobel_drop_fraction",
    "max_focal_mean_delta",
];
const QUALITY_REFLECTION_THRESHOLD_FIELDS: &[&str] = &[
    "min_luminance_range",
    "min_sobel_energy",
    "min_chroma_range",
    "max_firefly_fraction",
];
pub(super) const REFERENCE_FIELDS: &[&str] = &["id", "image", "metric", "mean_max", "min_ssim"];

pub(super) fn validate_quality(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_quality",
            "expect_quality must be an object",
            "emit expect_quality:{profile,exposure,contrast,noise,text,line,geometry,reflection,area_light,grounding,depth_of_field}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect.expect_quality",
        object.keys().map(String::as_str),
        QUALITY_FIELDS,
        diagnostics,
    );
    match object.get("profile").and_then(Value::as_str) {
        Some("product" | "documentation" | "cad" | "dashboard" | "twin") => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_quality.profile",
            "quality profile must be one of product, documentation, cad, dashboard, twin",
            "choose the profile that matches the intended application",
            None,
            false,
        )),
    }
    validate_threshold_object(
        object.get("exposure"),
        "$.expect.expect_quality.exposure",
        QUALITY_EXPOSURE_FIELDS,
        diagnostics,
    );
    validate_threshold_object(
        object.get("contrast"),
        "$.expect.expect_quality.contrast",
        QUALITY_CONTRAST_FIELDS,
        diagnostics,
    );
    validate_threshold_object(
        object.get("noise"),
        "$.expect.expect_quality.noise",
        QUALITY_NOISE_FIELDS,
        diagnostics,
    );
    validate_threshold_object(
        object.get("text"),
        "$.expect.expect_quality.text",
        QUALITY_TEXT_FIELDS,
        diagnostics,
    );
    validate_threshold_object(
        object.get("line"),
        "$.expect.expect_quality.line",
        QUALITY_LINE_FIELDS,
        diagnostics,
    );
    validate_threshold_object(
        object.get("geometry"),
        "$.expect.expect_quality.geometry",
        QUALITY_GEOMETRY_FIELDS,
        diagnostics,
    );
    validate_reflection_quality(object.get("reflection"), diagnostics);
    validate_area_light_quality(object.get("area_light"), diagnostics);
    validate_grounding_quality(object.get("grounding"), diagnostics);
    validate_depth_of_field_quality(object.get("depth_of_field"), diagnostics);
}

pub(super) fn validate_reference_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match object.get("image").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.image"),
            "reference image must be a non-empty path",
            "point at a committed PNG golden for this recipe",
            None,
            false,
        )),
    }
    match object.get("metric").and_then(Value::as_str) {
        Some("rgba_abs_diff" | "delta_e2000" | "ssim") => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.metric"),
            "reference metric must be rgba_abs_diff, delta_e2000, or ssim",
            "choose ssim for structure, delta_e2000 for color, or rgba_abs_diff for strict RGBA",
            None,
            false,
        )),
    }
    for field in ["mean_max", "min_ssim"] {
        if let Some(value) = object.get(field) {
            match value.as_f64() {
                Some(number) if number.is_finite() && number >= 0.0 => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_expect",
                    "error",
                    format!("{path}.{field}"),
                    format!("{field} must be a finite non-negative number"),
                    "use a threshold appropriate for the selected reference metric",
                    None,
                    false,
                )),
            }
        }
    }
}

fn validate_threshold_object(
    value: Option<&Value>,
    path: &str,
    fields: &[&str],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "quality threshold block must be an object",
            "emit finite normalized thresholds between 0 and 1",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object.keys().map(String::as_str), fields, diagnostics);
    for field in fields {
        if let Some(value) = object.get(*field) {
            match value.as_f64() {
                Some(number) if number.is_finite() && (0.0..=1.0).contains(&number) => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_expect",
                    "error",
                    format!("{path}.{field}"),
                    format!("{field} must be finite and between 0 and 1"),
                    "use a normalized quality threshold",
                    None,
                    false,
                )),
            }
        }
    }
}

fn validate_reflection_quality(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_quality.reflection",
            "quality threshold block must be an object",
            "emit finite normalized thresholds between 0 and 1",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect.expect_quality.reflection",
        object.keys().map(String::as_str),
        QUALITY_REFLECTION_FIELDS,
        diagnostics,
    );
    if object.contains_key("target") {
        validate_target(
            "$.expect.expect_quality.reflection.target",
            object.get("target"),
            false,
            diagnostics,
        );
    }
    for field in QUALITY_REFLECTION_THRESHOLD_FIELDS {
        if let Some(value) = object.get(*field) {
            match value.as_f64() {
                Some(number) if number.is_finite() && (0.0..=1.0).contains(&number) => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_expect",
                    "error",
                    format!("$.expect.expect_quality.reflection.{field}"),
                    format!("{field} must be finite and between 0 and 1"),
                    "use a normalized quality threshold",
                    None,
                    false,
                )),
            }
        }
    }
}

fn validate_area_light_quality(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_quality.area_light",
            "area-light quality threshold block must be an object",
            "emit area_light:{target?,min_shadow_contrast?,min_penumbra_width_px?,min_penumbra_luma_levels?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect.expect_quality.area_light",
        object.keys().map(String::as_str),
        QUALITY_AREA_LIGHT_FIELDS,
        diagnostics,
    );
    if object.contains_key("target") {
        validate_target(
            "$.expect.expect_quality.area_light.target",
            object.get("target"),
            false,
            diagnostics,
        );
    }
    validate_unit_threshold(
        "$.expect.expect_quality.area_light.min_shadow_contrast",
        object.get("min_shadow_contrast"),
        diagnostics,
    );
    validate_non_negative_threshold(
        "$.expect.expect_quality.area_light.min_penumbra_width_px",
        object.get("min_penumbra_width_px"),
        diagnostics,
    );
    validate_non_negative_threshold(
        "$.expect.expect_quality.area_light.min_penumbra_luma_levels",
        object.get("min_penumbra_luma_levels"),
        diagnostics,
    );
    validate_non_negative_threshold(
        "$.expect.expect_quality.area_light.min_emitter_extent_meters",
        object.get("min_emitter_extent_meters"),
        diagnostics,
    );
}

fn validate_grounding_quality(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_quality.grounding",
            "grounding quality expectation must be an object",
            "emit grounding:{target?,min_contact_shadow_delta?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect.expect_quality.grounding",
        object.keys().map(String::as_str),
        QUALITY_GROUNDING_FIELDS,
        diagnostics,
    );
    if object.contains_key("target") {
        validate_target(
            "$.expect.expect_quality.grounding.target",
            object.get("target"),
            false,
            diagnostics,
        );
    }
    validate_unit_threshold(
        "$.expect.expect_quality.grounding.min_contact_shadow_delta",
        object.get("min_contact_shadow_delta"),
        diagnostics,
    );
}

fn validate_depth_of_field_quality(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_quality.depth_of_field",
            "depth-of-field quality expectation must be an object",
            "emit depth_of_field:{target?,background_target?,min_background_sobel_drop?,max_focal_mean_delta?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect.expect_quality.depth_of_field",
        object.keys().map(String::as_str),
        QUALITY_DEPTH_OF_FIELD_FIELDS,
        diagnostics,
    );
    if object.contains_key("target") {
        validate_target(
            "$.expect.expect_quality.depth_of_field.target",
            object.get("target"),
            false,
            diagnostics,
        );
    }
    if object.contains_key("background_target") {
        validate_target(
            "$.expect.expect_quality.depth_of_field.background_target",
            object.get("background_target"),
            false,
            diagnostics,
        );
    }
    validate_unit_threshold(
        "$.expect.expect_quality.depth_of_field.min_background_sobel_drop_fraction",
        object.get("min_background_sobel_drop_fraction"),
        diagnostics,
    );
    validate_unit_threshold(
        "$.expect.expect_quality.depth_of_field.max_focal_mean_delta",
        object.get("max_focal_mean_delta"),
        diagnostics,
    );
    validate_non_negative_threshold(
        "$.expect.expect_quality.depth_of_field.min_source_background_sobel",
        object.get("min_source_background_sobel"),
        diagnostics,
    );
    validate_non_negative_threshold(
        "$.expect.expect_quality.depth_of_field.min_background_sobel_drop",
        object.get("min_background_sobel_drop"),
        diagnostics,
    );
}

fn validate_unit_threshold(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(number) if number.is_finite() && (0.0..=1.0).contains(&number) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "threshold must be finite and between 0 and 1",
            "use a normalized threshold",
            None,
            false,
        )),
    }
}

fn validate_non_negative_threshold(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(number) if number.is_finite() && number >= 0.0 => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "threshold must be a finite non-negative number",
            "use a non-negative threshold appropriate for the measured quality metric",
            None,
            false,
        )),
    }
}
