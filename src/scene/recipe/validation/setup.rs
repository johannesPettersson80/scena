use serde_json::Value;

use super::super::types::SceneRecipeDiagnosticV1;
use super::diagnostic;

mod render;
mod scene;

pub(super) use render::validate_render_setup;
pub(super) use scene::validate_scene_setup;

fn validate_known_fields(
    path: &str,
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("field '{key}' is not accepted here"),
                "remove the field or wait for the feature slice that owns it",
                None,
                false,
            ));
        }
    }
}

fn validate_enum(
    path: &str,
    value: Option<&Value>,
    allowed: &[&str],
    code: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_str() {
        Some(value) if allowed.contains(&value) => {}
        _ => diagnostics.push(diagnostic(
            code,
            "error",
            path,
            format!("field must be one of {}", allowed.join(", ")),
            "use one of the documented enum strings",
            None,
            false,
        )),
    }
}

fn validate_optional_string(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(value) = value
        && value.as_str().is_none_or(|value| value.trim().is_empty())
    {
        diagnostics.push(diagnostic(
            "invalid_string",
            "error",
            path,
            "field must be a non-empty string",
            "use a recipe color id or direct #RRGGBB value",
            None,
            false,
        ));
    }
}

fn validate_finite_number_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    if !value.as_f64().is_some_and(f64::is_finite) {
        diagnostics.push(number_diagnostic(path, "field must be a finite number"));
    }
}

fn validate_non_negative_number_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    validate_non_negative_number_required(path, Some(value), diagnostics);
}

fn validate_positive_number_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    if !value
        .as_f64()
        .is_some_and(|value| value.is_finite() && value > 0.0)
    {
        diagnostics.push(number_diagnostic(
            path,
            "field must be finite and greater than zero",
        ));
    }
}

fn validate_unit_number_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    validate_unit_number_required(path, Some(value), diagnostics);
}

fn validate_unit_number_required(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if !value
        .and_then(Value::as_f64)
        .is_some_and(|value| value.is_finite() && (0.0..=1.0).contains(&value))
    {
        diagnostics.push(number_diagnostic(
            path,
            "field must be finite and in [0, 1]",
        ));
    }
}

fn validate_non_negative_number_required(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if !value
        .and_then(Value::as_f64)
        .is_some_and(|value| value.is_finite() && value >= 0.0)
    {
        diagnostics.push(number_diagnostic(
            path,
            "field must be finite and non-negative",
        ));
    }
}

fn validate_u8(path: &str, value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    if value
        .and_then(Value::as_u64)
        .is_none_or(|value| value > u64::from(u8::MAX))
    {
        diagnostics.push(number_diagnostic(path, "field must fit in u8"));
    }
}

fn validate_u8_max(
    path: &str,
    value: Option<&Value>,
    max: u8,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value
        .and_then(Value::as_u64)
        .is_none_or(|value| value > u64::from(max))
    {
        diagnostics.push(number_diagnostic(
            path,
            format!("field must be an integer in [0, {max}]"),
        ));
    }
}

fn number_diagnostic(path: &str, message: impl Into<String>) -> SceneRecipeDiagnosticV1 {
    diagnostic(
        "invalid_number",
        "error",
        path,
        message,
        "use a finite value in the documented range",
        None,
        false,
    )
}
