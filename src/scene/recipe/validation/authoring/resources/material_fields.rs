use std::collections::BTreeSet;

use serde_json::Value;

use crate::Color;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::validate_known_fields;
use super::common::validate_positive_number;
use crate::scene::recipe::validation::diagnostic;

const TEXTURE_SLOT_FIELDS: &[&str] = &["uri", "color_space", "optional"];
const ALPHA_MODE_FIELDS: &[&str] = &["kind", "cutoff"];

pub(in crate::scene::recipe::validation::authoring) fn validate_color_ref(
    path: &str,
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(value) if colors.contains(value) || Color::from_hex(value).is_ok() => {}
        Some(value) => diagnostics.push(diagnostic(
            "unknown_color_ref",
            "error",
            path,
            format!("base_color references unknown color '{value}'"),
            "reference a key from colors or use a direct #RRGGBB value",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_base_color",
            "error",
            path,
            "material base_color must be a color id or #RRGGBB string",
            "reference a key from colors or use a direct #RRGGBB value",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_unit_float(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_unit_value",
            "error",
            path,
            "material scalar must be finite and in [0, 1]",
            "use a normalized value such as 0.6",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_positive(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.is_some() {
        validate_positive_number(path, value, diagnostics);
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_non_negative(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(value) if value.is_finite() && value >= 0.0 => {}
        _ => diagnostics.push(diagnostic(
            "invalid_number",
            "error",
            path,
            "field must be finite and non-negative",
            "use a value greater than or equal to zero",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_range(
    path: &str,
    value: Option<&Value>,
    min: f64,
    max: f64,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(value) if value.is_finite() && (min..=max).contains(&value) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_number",
            "error",
            path,
            format!("field must be finite and in [{min}, {max}]"),
            "use a value inside the accepted range",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_alpha_mode(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_alpha_mode",
            "error",
            path,
            "alpha_mode must be an object with kind",
            "use {kind:\"opaque\"}, {kind:\"blend\"}, or {kind:\"mask\",cutoff}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, ALPHA_MODE_FIELDS, diagnostics);
    match object.get("kind").and_then(Value::as_str) {
        Some("opaque" | "blend") => {}
        Some("mask") => validate_optional_range(
            &format!("{path}.cutoff"),
            object.get("cutoff"),
            0.0,
            1.0,
            diagnostics,
        ),
        Some(kind) => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            format!("{path}.kind"),
            format!("alpha mode '{kind}' is not supported"),
            "use opaque, mask, or blend",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_alpha_mode",
            "error",
            format!("{path}.kind"),
            "alpha_mode must include kind",
            "use opaque, mask, or blend",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_texture_slot(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_texture_slot",
            "error",
            path,
            "texture slot must be an object",
            "emit {uri,color_space?,optional?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, TEXTURE_SLOT_FIELDS, diagnostics);
    match object.get("uri").and_then(Value::as_str) {
        Some(uri) if !uri.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "missing_texture_uri",
            "error",
            format!("{path}.uri"),
            "texture slot must include a non-empty uri",
            "point uri at a texture asset",
            None,
            false,
        )),
    }
    if let Some(color_space) = object.get("color_space") {
        match color_space.as_str() {
            Some("srgb" | "linear") => {}
            _ => diagnostics.push(diagnostic(
                "invalid_texture_color_space",
                "error",
                format!("{path}.color_space"),
                "texture color_space must be srgb or linear",
                "use srgb for color/emissive textures and linear for data textures",
                None,
                false,
            )),
        }
    }
    if object
        .get("optional")
        .is_some_and(|value| !value.is_boolean())
    {
        diagnostics.push(diagnostic(
            "invalid_optional",
            "error",
            format!("{path}.optional"),
            "texture optional must be a boolean",
            "set optional true only when the fallback is acceptable",
            None,
            false,
        ));
    }
}
