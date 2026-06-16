use serde_json::Value;

use crate::Color;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::common::{named_color, validate_positive_number, validate_u8_rgb, validate_unit_vec3};
use crate::scene::recipe::validation::diagnostic;

pub(in crate::scene::recipe::validation::authoring) fn validate_colors(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(colors) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_colors",
            "error",
            "$.colors",
            "colors must be an object mapping ids to color values",
            "emit colors:{\"accent\":\"#3A7BD5\"}",
            None,
            false,
        ));
        return;
    };
    for (id, color) in colors {
        validate_color_value(&format!("$.colors.{id}"), color, diagnostics);
    }
}

fn validate_color_value(path: &str, value: &Value, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    if let Some(color) = value.as_str() {
        if Color::from_hex(color).is_ok() || named_color(color).is_some() {
            return;
        }
        diagnostics.push(diagnostic(
            "invalid_color",
            "error",
            path,
            "color string must be #RRGGBB or a supported named color",
            "use #3A7BD5, white, black, red, green, blue, yellow, cyan, magenta, warm_white, or cool_white",
            None,
            false,
        ));
        return;
    }
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_color",
            "error",
            path,
            "color values must be strings or {srgb8|linear|kelvin} objects",
            "use #3A7BD5 or {\"srgb8\":[58,123,213]}",
            None,
            false,
        ));
        return;
    };
    match object.len() {
        1 if object.contains_key("srgb8") => {
            validate_u8_rgb(path, object.get("srgb8"), diagnostics)
        }
        1 if object.contains_key("linear") => {
            validate_unit_vec3(path, object.get("linear"), diagnostics)
        }
        1 if object.contains_key("kelvin") => {
            validate_positive_number(path, object.get("kelvin"), diagnostics)
        }
        _ => diagnostics.push(diagnostic(
            "invalid_color",
            "error",
            path,
            "color object must contain exactly one of srgb8, linear, or kelvin",
            "use {\"srgb8\":[58,123,213]}, {\"linear\":[0.1,0.2,0.3]}, or {\"kelvin\":5600}",
            None,
            false,
        )),
    }
}
