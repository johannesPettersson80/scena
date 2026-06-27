use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

pub(super) fn validate_values(
    path: &str,
    value: Option<&Value>,
    channel_kind: Option<&str>,
    interpolation: &str,
    time_count: Option<usize>,
    authored_weight_count: Option<usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(values) = value
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_animation_values",
            "error",
            path,
            "animation values must be a non-empty array",
            "emit one value per time, or three values per time for cubic_spline",
            None,
            false,
        ));
        return;
    };
    let expected_values = time_count.map(|count| {
        if interpolation == "cubic_spline" {
            count.saturating_mul(3)
        } else {
            count
        }
    });
    if let Some(expected) = expected_values
        && values.len() != expected
    {
        diagnostics.push(diagnostic(
            "invalid_animation_values",
            "error",
            path,
            format!(
                "animation values length {} does not match expected {expected}",
                values.len()
            ),
            "emit one value per time, or three values per time for cubic_spline",
            None,
            false,
        ));
    }
    let Some(component_count) = component_count(channel_kind) else {
        return;
    };
    for (index, value) in values.iter().enumerate() {
        let Some(components) = value.as_array() else {
            diagnostics.push(diagnostic(
                "invalid_animation_value",
                "error",
                format!("{path}[{index}]"),
                "animation value must be an array",
                "emit vector components as numbers",
                None,
                false,
            ));
            continue;
        };
        if channel_kind == Some("weights") {
            if components.is_empty() {
                diagnostics.push(diagnostic(
                    "invalid_animation_value",
                    "error",
                    format!("{path}[{index}]"),
                    "weights animation value must include at least one weight",
                    "emit one numeric component per morph target",
                    None,
                    false,
                ));
            } else if let Some(expected) = authored_weight_count
                && components.len() != expected
            {
                diagnostics.push(diagnostic(
                    "invalid_animation_value",
                    "error",
                    format!("{path}[{index}]"),
                    format!(
                        "weights animation value has {} components but target has {expected} morph targets",
                        components.len()
                    ),
                    "emit one numeric component per authored morph target",
                    None,
                    false,
                ));
            }
        } else if components.len() != component_count {
            diagnostics.push(diagnostic(
                "invalid_animation_value",
                "error",
                format!("{path}[{index}]"),
                format!(
                    "animation value for {} must have {component_count} components",
                    channel_kind.unwrap_or("unknown")
                ),
                "emit translation/scale as [x,y,z] and rotation as [x,y,z,w]",
                None,
                false,
            ));
        }
        for (component_index, component) in components.iter().enumerate() {
            match component.as_f64() {
                Some(component)
                    if component.is_finite() && component.abs() <= f64::from(f32::MAX) => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_animation_value",
                    "error",
                    format!("{path}[{index}][{component_index}]"),
                    "animation components must be finite f32-compatible numbers",
                    "emit finite numeric components",
                    None,
                    false,
                )),
            }
        }
    }
}

fn component_count(channel_kind: Option<&str>) -> Option<usize> {
    match channel_kind {
        Some("translation" | "scale") => Some(3),
        Some("rotation") => Some(4),
        Some("weights") => Some(1),
        _ => None,
    }
}
