use serde_json::Value;

use super::diagnostic;
use crate::scene::recipe::SceneRecipeDiagnosticV1;

mod composition;
mod quality;

const EXPECT_FIELDS: &[&str] = &[
    "expect_visible",
    "expect_color",
    "expect_bbox_fit",
    "expect_grounded",
    "expect_helper_occluded",
    "expect_occlusion",
    "expect_backend",
    "expect_clipping",
    "expect_state",
    "expect_transform",
    "expect_separation",
    "expect_pick",
    "expect_quality",
    "expect_reference",
    "expect_no_warnings",
];
const VISIBLE_FIELDS: &[&str] = &["id", "target"];
const COLOR_FIELDS: &[&str] = &[
    "id",
    "target",
    "color_family",
    "swatch_srgb8",
    "tolerance",
    "require_source_material",
    "require_base_color_texture",
];
const BBOX_FIT_FIELDS: &[&str] = &["min", "max"];
const PICK_FIELDS: &[&str] = &["id", "x_css_px", "y_css_px", "target"];
const TARGET_FIELDS: &[&str] = &["kind", "id", "position"];

pub(super) fn validate_expectations(
    expect: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(expect) = expect else {
        return;
    };
    let Some(object) = expect.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect",
            "expect must be an object",
            "emit expect:{expect_color,expect_bbox_fit,expect_pick}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.expect",
        object.keys().map(String::as_str),
        EXPECT_FIELDS,
        diagnostics,
    );
    validate_array(
        object.get("expect_visible"),
        "$.expect.expect_visible",
        VISIBLE_FIELDS,
        validate_visible_expectation,
        diagnostics,
    );
    validate_array(
        object.get("expect_color"),
        "$.expect.expect_color",
        COLOR_FIELDS,
        validate_color_expectation,
        diagnostics,
    );
    validate_bbox_fit(object.get("expect_bbox_fit"), diagnostics);
    validate_array(
        object.get("expect_grounded"),
        "$.expect.expect_grounded",
        composition::GROUNDED_FIELDS,
        composition::validate_grounded_expectation,
        diagnostics,
    );
    validate_array(
        object.get("expect_helper_occluded"),
        "$.expect.expect_helper_occluded",
        composition::HELPER_OCCLUDED_FIELDS,
        composition::validate_helper_occluded_expectation,
        diagnostics,
    );
    validate_array(
        object.get("expect_occlusion"),
        "$.expect.expect_occlusion",
        composition::OCCLUSION_FIELDS,
        composition::validate_occlusion_expectation,
        diagnostics,
    );
    composition::validate_backend_expectation(object.get("expect_backend"), diagnostics);
    composition::validate_clipping_expectation(object.get("expect_clipping"), diagnostics);
    validate_array(
        object.get("expect_state"),
        "$.expect.expect_state",
        composition::STATE_FIELDS,
        composition::validate_state_expectation,
        diagnostics,
    );
    validate_array(
        object.get("expect_transform"),
        "$.expect.expect_transform",
        composition::TRANSFORM_FIELDS,
        composition::validate_transform_expectation,
        diagnostics,
    );
    validate_array(
        object.get("expect_separation"),
        "$.expect.expect_separation",
        composition::SEPARATION_FIELDS,
        composition::validate_separation_expectation,
        diagnostics,
    );
    validate_array(
        object.get("expect_pick"),
        "$.expect.expect_pick",
        PICK_FIELDS,
        validate_pick_expectation,
        diagnostics,
    );
    quality::validate_quality(object.get("expect_quality"), diagnostics);
    validate_array(
        object.get("expect_reference"),
        "$.expect.expect_reference",
        quality::REFERENCE_FIELDS,
        quality::validate_reference_expectation,
        diagnostics,
    );
    if let Some(value) = object.get("expect_no_warnings")
        && !value.is_boolean()
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_no_warnings",
            "expect_no_warnings must be a boolean",
            "set expect_no_warnings to true or false",
            None,
            false,
        ));
    }
}

fn validate_array(
    value: Option<&Value>,
    path: &str,
    fields: &[&str],
    validate_entry: fn(&str, &serde_json::Map<String, Value>, &mut Vec<SceneRecipeDiagnosticV1>),
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(entries) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "expectation list must be an array",
            "emit an array of expectation objects",
            None,
            false,
        ));
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        let Some(object) = entry.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                &entry_path,
                "expectation entry must be an object",
                "emit an object with id and target",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(
            &entry_path,
            object.keys().map(String::as_str),
            fields,
            diagnostics,
        );
        validate_id(&entry_path, object.get("id"), diagnostics);
        validate_entry(&entry_path, object, diagnostics);
    }
}

fn validate_visible_expectation(
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

fn validate_color_expectation(
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

fn validate_bbox_fit(bbox_fit: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
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

fn validate_pick_expectation(
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
    for field in ["x_css_px", "y_css_px"] {
        match object.get(field).and_then(Value::as_f64) {
            Some(value) if value.is_finite() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("{path}.{field}"),
                format!("{field} must be a finite CSS-pixel number"),
                "use finite CSS-pixel coordinates inside the capture viewport",
                None,
                false,
            )),
        }
    }
}

fn validate_id(path: &str, value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    match value.and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.id"),
            "expectation id must be a non-empty string",
            "use a stable expectation id",
            None,
            false,
        )),
    }
}

pub(super) fn validate_target(
    path: &str,
    target: Option<&Value>,
    allow_import: bool,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(target) = target else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "expectation target is required",
            "target an authored or imported node id",
            None,
            false,
        ));
        return;
    };
    let Some(object) = target.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "expectation target must be an object",
            "emit target:{kind:\"node\",id:\"...\"}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        path,
        object.keys().map(String::as_str),
        TARGET_FIELDS,
        diagnostics,
    );
    match object.get("kind").and_then(Value::as_str) {
        Some("node") => validate_target_id(path, object.get("id"), diagnostics),
        Some("import") if allow_import => validate_target_id(path, object.get("id"), diagnostics),
        Some("import") => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            path,
            "this expectation target does not support whole-import matching",
            "target a specific node id from scene_recipe_build.nodes or imports[].nodes_by_path",
            None,
            false,
        )),
        Some("world") => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            path,
            "world-position expectation targets are not supported",
            "target a node id so the verifier can compare stable handles",
            None,
            false,
        )),
        Some(other) => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.kind"),
            format!("unsupported expectation target kind '{other}'"),
            "use kind:\"node\"",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.kind"),
            "expectation target kind is required",
            "use kind:\"node\"",
            None,
            false,
        )),
    }
}

fn validate_target_id(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.id"),
            "expectation target id must be a non-empty string",
            "use a stable authored node id or imported node path id",
            None,
            false,
        )),
    }
}

fn validate_known_fields<'a>(
    path: &str,
    keys: impl Iterator<Item = &'a str>,
    allowed: &[&str],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in keys {
        if !allowed.contains(&key) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("field '{key}' is not accepted here"),
                "remove the field or use the documented expectation shape",
                None,
                false,
            ));
        }
    }
}

fn push_swatch_error(path: &str, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    diagnostics.push(diagnostic(
        "invalid_expect",
        "error",
        format!("{path}.swatch_srgb8"),
        "swatch_srgb8 must contain exactly three 0..255 integers",
        "emit swatch_srgb8:[r,g,b]",
        None,
        false,
    ));
}
