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
const DIRECTIONAL_PRESETS: &[&str] = &["sun", "key", "fill", "rim"];
const POINT_PRESETS: &[&str] = &["softbox", "bulb_warm", "bulb_cool"];

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
        let kind = object.get("kind").and_then(Value::as_str);
        match kind {
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
        validate_light_preset(&path, kind, object.get("preset"), diagnostics);
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

fn validate_light_preset(
    path: &str,
    kind: Option<&str>,
    preset: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(preset) = preset else {
        return;
    };
    let preset_path = format!("{path}.preset");
    let Some(preset) = preset.as_str() else {
        diagnostics.push(diagnostic(
            "invalid_light_preset",
            "error",
            preset_path,
            "light preset must be a string",
            "use a documented preset for the light kind",
            None,
            false,
        ));
        return;
    };
    match kind {
        Some("directional") if DIRECTIONAL_PRESETS.contains(&preset) => {}
        Some("point") if POINT_PRESETS.contains(&preset) => {}
        Some("spot") => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            preset_path,
            "spot light presets are not supported",
            "omit preset and set spot light intensity, range, and cone angles explicitly",
            None,
            false,
        )),
        Some("directional") => diagnostics.push(diagnostic(
            "invalid_light_preset",
            "error",
            preset_path,
            format!("preset '{preset}' is not valid for directional lights"),
            "use sun, key, fill, or rim",
            None,
            false,
        )),
        Some("point") => diagnostics.push(diagnostic(
            "invalid_light_preset",
            "error",
            preset_path,
            format!("preset '{preset}' is not valid for point lights"),
            "use softbox, bulb_warm, or bulb_cool",
            None,
            false,
        )),
        _ => {}
    }
}
