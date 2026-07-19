use serde_json::Value;

use super::{
    diagnostic, push_swatch_error, validate_fraction, validate_known_fields, validate_target,
    validate_vec3,
};
use crate::scene::recipe::SceneRecipeDiagnosticV1;

pub(super) const VISIBLE_FIELDS: &[&str] = &["id", "target"];
pub(super) const COLOR_FIELDS: &[&str] = &[
    "id",
    "target",
    "color_family",
    "swatch_srgb8",
    "tolerance",
    "require_source_material",
    "require_base_color_texture",
];
const BBOX_FIT_FIELDS: &[&str] = &["min", "max"];
pub(super) const TARGET_FIT_FIELDS: &[&str] = &[
    "id",
    "target",
    "bounds",
    "centroid",
    "min_fit",
    "max_fit",
    "min_visible_coverage",
];
const TARGET_BOUNDS_FIELDS: &[&str] = &["min", "max"];

pub(super) fn validate_visible_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.target"),
        object.get("target"),
        true,
        diagnostics,
    );
}

pub(super) fn validate_color_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.target"),
        object.get("target"),
        false,
        diagnostics,
    );
    if let Some(family) = object.get("color_family")
        && family.as_str().is_none_or(str::is_empty)
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.color_family"),
            "color_family must be a non-empty string",
            "use a rendered color family such as red, green, blue, gray, or mixed",
            None,
            false,
        ));
    }
    if let Some(swatch) = object.get("swatch_srgb8") {
        let Some(values) = swatch.as_array() else {
            push_swatch_error(path, diagnostics);
            return;
        };
        if values.len() != 3
            || !values
                .iter()
                .all(|value| value.as_u64().is_some_and(|value| value <= 255))
        {
            push_swatch_error(path, diagnostics);
        }
    }
    if let Some(tolerance) = object.get("tolerance") {
        match tolerance.as_f64() {
            Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("{path}.tolerance"),
                "color tolerance must be finite and between 0 and 1",
                "use normalized RGB Euclidean tolerance such as 0.05 or 0.20",
                None,
                false,
            )),
        }
    }
    for field in ["require_source_material", "require_base_color_texture"] {
        if let Some(value) = object.get(field)
            && !value.is_boolean()
        {
            diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("{path}.{field}"),
                format!("{field} must be a boolean"),
                "set the material provenance expectation to true or false",
                None,
                false,
            ));
        }
    }
}

pub(super) fn validate_bbox_fit(
    bbox_fit: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(bbox_fit) = bbox_fit else {
        return;
    };
    let Some(object) = bbox_fit.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_bbox_fit",
            "expect_bbox_fit must be an object",
            "emit expect_bbox_fit:{min,max}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect.expect_bbox_fit",
        object.keys().map(String::as_str),
        BBOX_FIT_FIELDS,
        diagnostics,
    );
    let mut min = None;
    let mut max = None;
    for field in ["min", "max"] {
        if let Some(value) = object.get(field) {
            match value.as_f64() {
                Some(number) if number.is_finite() && (0.0..=1.0).contains(&number) => {
                    if field == "min" {
                        min = Some(number);
                    } else {
                        max = Some(number);
                    }
                }
                _ => diagnostics.push(diagnostic(
                    "invalid_expect",
                    "error",
                    format!("$.expect.expect_bbox_fit.{field}"),
                    format!("expect_bbox_fit {field} must be finite and between 0 and 1"),
                    "use a normalized frame fraction",
                    None,
                    false,
                )),
            }
        }
    }
    if let (Some(min), Some(max)) = (min, max)
        && min > max
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_bbox_fit",
            "expect_bbox_fit min must be <= max",
            "lower min or raise max",
            None,
            false,
        ));
    }
}

pub(super) fn validate_target_fit_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.target"),
        object.get("target"),
        true,
        diagnostics,
    );
    validate_target_bounds(&format!("{path}.bounds"), object.get("bounds"), diagnostics);
    validate_vec3(
        &format!("{path}.centroid"),
        object.get("centroid"),
        "target-fit centroid",
        diagnostics,
    );
    let min_fit = validate_fraction(
        &format!("{path}.min_fit"),
        object.get("min_fit"),
        "min_fit",
        diagnostics,
    );
    let max_fit = validate_fraction(
        &format!("{path}.max_fit"),
        object.get("max_fit"),
        "max_fit",
        diagnostics,
    );
    if let (Some(min_fit), Some(max_fit)) = (min_fit, max_fit)
        && min_fit > max_fit
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "target fit min_fit must be <= max_fit",
            "lower min_fit or raise max_fit",
            None,
            false,
        ));
    }
    validate_fraction(
        &format!("{path}.min_visible_coverage"),
        object.get("min_visible_coverage"),
        "min_visible_coverage",
        diagnostics,
    );
}

fn validate_target_bounds(
    path: &str,
    bounds: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(bounds) = bounds else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "target-fit bounds are required",
            "emit bounds:{min:[x,y,z],max:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = bounds.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "target-fit bounds must be an object",
            "emit bounds:{min:[x,y,z],max:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        path,
        object.keys().map(String::as_str),
        TARGET_BOUNDS_FIELDS,
        diagnostics,
    );
    let min = validate_vec3(
        &format!("{path}.min"),
        object.get("min"),
        "target-fit bounds min",
        diagnostics,
    );
    let max = validate_vec3(
        &format!("{path}.max"),
        object.get("max"),
        "target-fit bounds max",
        diagnostics,
    );
    if let (Some(min), Some(max)) = (min, max)
        && min.iter().zip(max).any(|(min, max)| *min > max)
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "target-fit bounds min must be <= max on every axis",
            "emit finite axis-aligned bounds around the target region",
            None,
            false,
        ));
    }
}
