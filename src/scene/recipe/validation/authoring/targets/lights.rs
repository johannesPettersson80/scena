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
    "shape",
    "color",
    "illuminance_lux",
    "intensity_candela",
    "luminous_flux_lumens",
    "range",
    "width",
    "height",
    "radius",
    "inner_cone_degrees",
    "outer_cone_degrees",
    "transform",
];
const DIRECTIONAL_PRESETS: &[&str] = &["sun", "key", "fill", "rim"];
const POINT_PRESETS: &[&str] = &["softbox", "bulb_warm", "bulb_cool"];
const AREA_PRESETS: &[&str] = &["softbox"];

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
            Some("directional" | "point" | "spot" | "area" | "studio_rig") => {}
            Some(kind) => diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("light kind '{kind}' is not supported"),
                "use directional, point, spot, area, or studio_rig",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_light_kind",
                "error",
                format!("{path}.kind"),
                "light must include kind",
                "use directional, point, spot, area, or studio_rig",
                None,
                false,
            )),
        }
        validate_light_preset(&path, kind, object.get("preset"), diagnostics);
        validate_light_field_compatibility(&path, kind, object, diagnostics);
        if let Some(color) = object.get("color") {
            match color.as_str() {
                Some(value)
                    if colors.contains(value)
                        || Color::from_named_constant(value).is_some()
                        || Color::from_hex(value).is_ok() => {}
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
        for field in [
            "illuminance_lux",
            "intensity_candela",
            "luminous_flux_lumens",
            "range",
        ] {
            validate_optional_non_negative(
                &format!("{path}.{field}"),
                object.get(field),
                diagnostics,
            );
        }
        validate_area_shape(&path, kind, object, diagnostics);
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
        Some("area") if AREA_PRESETS.contains(&preset) => {}
        Some("studio_rig") if preset == "studio_rig" => {}
        Some("studio_rig") => diagnostics.push(diagnostic(
            "invalid_light_preset",
            "error",
            preset_path,
            format!("preset '{preset}' is not valid for studio_rig lights"),
            "use studio_rig or omit preset for the default studio rig",
            None,
            false,
        )),
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
        Some("area") => diagnostics.push(diagnostic(
            "invalid_light_preset",
            "error",
            preset_path,
            format!("preset '{preset}' is not valid for area lights"),
            "use softbox",
            None,
            false,
        )),
        _ => {}
    }
}

fn validate_light_field_compatibility(
    path: &str,
    kind: Option<&str>,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(kind) = kind else {
        return;
    };
    let incompatible: &[&str] = match kind {
        "studio_rig" => &[
            "shape",
            "color",
            "illuminance_lux",
            "intensity_candela",
            "luminous_flux_lumens",
            "range",
            "width",
            "height",
            "radius",
            "inner_cone_degrees",
            "outer_cone_degrees",
            "transform",
        ],
        "directional" => &[
            "shape",
            "intensity_candela",
            "luminous_flux_lumens",
            "range",
            "width",
            "height",
            "radius",
            "inner_cone_degrees",
            "outer_cone_degrees",
        ],
        "point" => &[
            "shape",
            "illuminance_lux",
            "luminous_flux_lumens",
            "width",
            "height",
            "radius",
            "inner_cone_degrees",
            "outer_cone_degrees",
        ],
        "spot" => &[
            "shape",
            "illuminance_lux",
            "luminous_flux_lumens",
            "width",
            "height",
            "radius",
        ],
        "area" => &[
            "illuminance_lux",
            "intensity_candela",
            "inner_cone_degrees",
            "outer_cone_degrees",
        ],
        _ => &[],
    };
    for field in incompatible {
        if object.contains_key(*field) {
            diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.{field}"),
                format!("field '{field}' is not valid for {kind} lights"),
                "remove the field or switch to the light kind that owns it",
                None,
                false,
            ));
        }
    }
}

fn validate_area_shape(
    path: &str,
    kind: Option<&str>,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if kind != Some("area") {
        return;
    }
    let shape = object
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("rect");
    match shape {
        "rect" => {
            validate_positive_dimension(path, object.get("width"), "width", diagnostics);
            validate_positive_dimension(path, object.get("height"), "height", diagnostics);
            if object.contains_key("radius") {
                diagnostics.push(diagnostic(
                    "invalid_area_light_shape",
                    "error",
                    format!("{path}.radius"),
                    "rect area lights use width and height, not radius",
                    "remove radius or use shape:\"disc\"/\"sphere\"",
                    None,
                    false,
                ));
            }
        }
        "disc" | "sphere" => {
            validate_positive_dimension(path, object.get("radius"), "radius", diagnostics);
            for field in ["width", "height"] {
                if object.contains_key(field) {
                    diagnostics.push(diagnostic(
                        "invalid_area_light_shape",
                        "error",
                        format!("{path}.{field}"),
                        format!("{shape} area lights use radius, not {field}"),
                        "remove width/height or use shape:\"rect\"",
                        None,
                        false,
                    ));
                }
            }
        }
        other => diagnostics.push(diagnostic(
            "invalid_area_light_shape",
            "error",
            format!("{path}.shape"),
            format!("area light shape '{other}' is not supported"),
            "use rect, disc, or sphere",
            None,
            false,
        )),
    }
}

fn validate_positive_dimension(
    path: &str,
    value: Option<&Value>,
    field: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(value) if value.is_finite() && value > 0.0 => {}
        _ => diagnostics.push(diagnostic(
            "invalid_area_light_shape",
            "error",
            format!("{path}.{field}"),
            format!("area light {field} must be finite and > 0"),
            "use a positive area-light dimension",
            None,
            false,
        )),
    }
}
