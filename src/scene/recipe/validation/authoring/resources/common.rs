use serde_json::Value;

use crate::Color;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::finite_vec3;
use crate::scene::recipe::validation::diagnostic;

pub(in crate::scene::recipe::validation::authoring) fn validate_positive_number_list(
    path: &str,
    value: Option<&Value>,
    expected_len: usize,
    message: &'static str,
    help: &'static str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match finite_number_list(value, expected_len) {
        Some(values) if values.iter().all(|value| *value > 0.0) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_vector",
            "error",
            path,
            message,
            help,
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_positive_number(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_f64) {
        Some(value) if value.is_finite() && value > 0.0 => {}
        _ => diagnostics.push(diagnostic(
            "invalid_number",
            "error",
            path,
            "field must be a finite positive number",
            "use a value greater than zero",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_positive_u32(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_u64() {
        Some(value) if value > 0 && value <= u64::from(u32::MAX) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_integer",
            "error",
            path,
            "field must be a positive u32 integer",
            "use a positive integer",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_vec3(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.and_then(finite_vec3).is_none() {
        diagnostics.push(diagnostic(
            "invalid_vector",
            "error",
            path,
            "field must be a finite [x,y,z] array",
            "use three finite numbers",
            None,
            false,
        ));
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_points(
    path: &str,
    value: Option<&Value>,
    min_len: usize,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> usize {
    let Some(array) = value.and_then(Value::as_array) else {
        diagnostics.push(diagnostic(
            "invalid_points",
            "error",
            path,
            "field must be an array of [x,y,z] points",
            "provide at least the required number of points",
            None,
            false,
        ));
        return 0;
    };
    if array.len() < min_len {
        diagnostics.push(diagnostic(
            "invalid_points",
            "error",
            path,
            format!("field must contain at least {min_len} points"),
            "add more points",
            None,
            false,
        ));
    }
    for (index, point) in array.iter().enumerate() {
        if finite_vec3(point).is_none() {
            diagnostics.push(diagnostic(
                "invalid_vector",
                "error",
                format!("{path}[{index}]"),
                "point must be a finite [x,y,z] array",
                "use three finite numbers",
                None,
                false,
            ));
        }
    }
    array.len()
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_vec3_array(
    path: &str,
    value: Option<&Value>,
    expected_len: usize,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let count = validate_points(path, Some(value), expected_len, diagnostics);
    if count != expected_len {
        diagnostics.push(diagnostic(
            "invalid_vector_count",
            "error",
            path,
            format!("field must contain exactly {expected_len} entries"),
            "match the positions length",
            None,
            false,
        ));
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_vec2_array(
    path: &str,
    value: Option<&Value>,
    expected_len: usize,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(array) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_uvs",
            "error",
            path,
            "uvs must be an array of [u,v] pairs",
            "match the positions length",
            None,
            false,
        ));
        return;
    };
    if array.len() != expected_len {
        diagnostics.push(diagnostic(
            "invalid_uv_count",
            "error",
            path,
            format!("uvs must contain exactly {expected_len} entries"),
            "match the positions length",
            None,
            false,
        ));
    }
    for (index, uv) in array.iter().enumerate() {
        if finite_number_list(Some(uv), 2).is_none() {
            diagnostics.push(diagnostic(
                "invalid_uv",
                "error",
                format!("{path}[{index}]"),
                "uv must be a finite [u,v] pair",
                "use two finite numbers",
                None,
                false,
            ));
        }
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_string_array(
    path: &str,
    value: Option<&Value>,
    expected_len: usize,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(array) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_colors",
            "error",
            path,
            "mesh colors must be an array of color refs",
            "match the positions length or omit colors",
            None,
            false,
        ));
        return;
    };
    if array.len() != expected_len {
        diagnostics.push(diagnostic(
            "invalid_color_count",
            "error",
            path,
            format!("mesh colors must contain exactly {expected_len} entries"),
            "match the positions length or omit colors",
            None,
            false,
        ));
    }
    for (index, color) in array.iter().enumerate() {
        if !color.is_string() {
            diagnostics.push(diagnostic(
                "invalid_color_ref",
                "error",
                format!("{path}[{index}]"),
                "mesh color entries must be color ids or #RRGGBB strings",
                "use a string color reference",
                None,
                false,
            ));
        }
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_indices(
    path: &str,
    value: Option<&Value>,
    vertex_count: usize,
    topology: Option<&str>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(array) = value.and_then(Value::as_array) else {
        diagnostics.push(diagnostic(
            "invalid_indices",
            "error",
            path,
            "indices must be an array of u32 integers",
            "provide explicit mesh indices",
            None,
            false,
        ));
        return;
    };
    let divisor = match topology {
        Some("lines") => 2,
        _ => 3,
    };
    if !array.len().is_multiple_of(divisor) {
        diagnostics.push(diagnostic(
            "invalid_index_count",
            "error",
            path,
            format!("index count must be a multiple of {divisor} for this topology"),
            "fix the index list",
            None,
            false,
        ));
    }
    for (index, value) in array.iter().enumerate() {
        match value.as_u64() {
            Some(value) if value < vertex_count as u64 && value <= u64::from(u32::MAX) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_index",
                "error",
                format!("{path}[{index}]"),
                "index must reference an existing vertex and fit in u32",
                "fix the mesh index",
                None,
                false,
            )),
        }
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_u8_rgb(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(array) = value.and_then(Value::as_array) else {
        diagnostics.push(diagnostic(
            "invalid_color",
            "error",
            path,
            "srgb8 color must be [r,g,b]",
            "use integer channel values 0..255",
            None,
            false,
        ));
        return;
    };
    if array.len() != 3
        || !array
            .iter()
            .all(|value| value.as_u64().is_some_and(|value| value <= 255))
    {
        diagnostics.push(diagnostic(
            "invalid_color",
            "error",
            path,
            "srgb8 color must contain exactly three 0..255 integers",
            "use integer channel values 0..255",
            None,
            false,
        ));
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_unit_vec3(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match finite_number_list(value, 3) {
        Some(values) if values.iter().all(|value| (0.0..=1.0).contains(value)) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_color",
            "error",
            path,
            "linear color must contain three finite values in [0, 1]",
            "use normalized linear RGB channels",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn finite_number_list(
    value: Option<&Value>,
    expected_len: usize,
) -> Option<Vec<f64>> {
    let array = value?.as_array()?;
    if array.len() != expected_len {
        return None;
    }
    array
        .iter()
        .map(Value::as_f64)
        .collect::<Option<Vec<_>>>()
        .filter(|values| values.iter().all(|value| value.is_finite()))
}

pub(in crate::scene::recipe::validation::authoring) fn named_color(name: &str) -> Option<Color> {
    match name {
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "blue" => Some(Color::BLUE),
        "yellow" => Some(Color::YELLOW),
        "cyan" => Some(Color::CYAN),
        "magenta" => Some(Color::MAGENTA),
        "warm_white" => Some(Color::WARM_WHITE),
        "cool_white" => Some(Color::COOL_WHITE),
        _ => None,
    }
}
