use std::collections::BTreeSet;

use serde_json::Value;

use crate::Color;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::common::{
    TransformUse, validate_optional_angle, validate_optional_non_negative, validate_transform,
};
use crate::scene::recipe::validation::diagnostic;

const LIGHT_FIELDS: &[&str] = &[
    "id",
    "kind",
    "preset",
    "color",
    "illuminance_lux",
    "intensity_candela",
    "range",
    "inner_cone_degrees",
    "outer_cone_degrees",
    "transform",
];

pub(in crate::scene::recipe::validation::authoring) fn validate_lights(
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(lights) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_lights",
            "error",
            "$.lights",
            "lights must be an array",
            "emit lights:[{id,kind,...}]",
            None,
            false,
        ));
        return;
    };
    for (index, light) in lights.iter().enumerate() {
        let path = format!("$.lights[{index}]");
        let Some(object) = light.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_light",
                "error",
                &path,
                "light entry must be an object",
                "emit light entries as {id, kind}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, LIGHT_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        match object.get("kind").and_then(Value::as_str) {
            Some("directional" | "point" | "spot") => {}
            Some(kind) => diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("light kind '{kind}' is not supported"),
                "use directional, point, or spot",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_light_kind",
                "error",
                format!("{path}.kind"),
                "light must include kind",
                "use directional, point, or spot",
                None,
                false,
            )),
        }
        if let Some(color) = object.get("color") {
            match color.as_str() {
                Some(value) if colors.contains(value) || Color::from_hex(value).is_ok() => {}
                Some(_) => diagnostics.push(diagnostic(
                    "unknown_color_ref",
                    "error",
                    format!("{path}.color"),
                    "light color must reference a declared color or direct #RRGGBB string",
                    "declare the color under colors or use #RRGGBB",
                    None,
                    false,
                )),
                None => diagnostics.push(diagnostic(
                    "invalid_color_ref",
                    "error",
                    format!("{path}.color"),
                    "light color must be a string",
                    "use a color id or #RRGGBB",
                    None,
                    false,
                )),
            }
        }
        for field in ["illuminance_lux", "intensity_candela", "range"] {
            validate_optional_non_negative(
                &format!("{path}.{field}"),
                object.get(field),
                diagnostics,
            );
        }
        validate_optional_angle(
            &format!("{path}.inner_cone_degrees"),
            object.get("inner_cone_degrees"),
            diagnostics,
        );
        validate_optional_angle(
            &format!("{path}.outer_cone_degrees"),
            object.get("outer_cone_degrees"),
            diagnostics,
        );
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{path}.transform"),
                transform,
                TransformUse::Node,
                &BTreeSet::new(),
                &BTreeSet::new(),
                diagnostics,
            );
        }
    }
}
