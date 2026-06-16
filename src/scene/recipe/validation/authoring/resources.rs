use std::collections::BTreeSet;

use serde_json::Value;

use crate::Color;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::diagnostic;
use super::{finite_vec3, validate_known_fields, validate_required_id};

const GEOMETRY_FIELDS: &[&str] = &["id", "primitive"];
const PRIMITIVE_FIELDS: &[&str] = &["kind", "size"];
const MATERIAL_FIELDS: &[&str] = &["id", "kind", "base_color", "metallic", "roughness"];

pub(super) fn validate_colors(
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
        match color.as_str() {
            Some(color) if Color::from_hex(color).is_ok() => {}
            Some(_) => diagnostics.push(diagnostic(
                "invalid_color",
                "error",
                format!("$.colors.{id}"),
                "Slice 1 colors must be #RRGGBB sRGB hex strings",
                "use a six-digit hex string such as #3A7BD5",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "invalid_color",
                "error",
                format!("$.colors.{id}"),
                "color values must be strings in this slice",
                "use a six-digit hex string such as #3A7BD5",
                None,
                false,
            )),
        }
    }
}

pub(super) fn validate_geometries(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(geometries) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_geometries",
            "error",
            "$.geometries",
            "geometries must be an array",
            "emit geometries:[{id,primitive}]",
            None,
            false,
        ));
        return;
    };
    for (index, geometry) in geometries.iter().enumerate() {
        let path = format!("$.geometries[{index}]");
        let Some(object) = geometry.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_geometry",
                "error",
                &path,
                "geometry entry must be an object",
                "emit geometry entries as {id, primitive}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, GEOMETRY_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        validate_primitive(
            &format!("{path}.primitive"),
            object.get("primitive"),
            diagnostics,
        );
    }
}

fn validate_primitive(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "missing_primitive",
            "error",
            path,
            "geometry must include a primitive object",
            "emit primitive:{kind:\"box\",size:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_primitive",
            "error",
            path,
            "primitive must be an object",
            "emit primitive:{kind:\"box\",size:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, PRIMITIVE_FIELDS, diagnostics);
    match object.get("kind").and_then(Value::as_str) {
        Some("box") => validate_positive_vec3(
            &format!("{path}.size"),
            object.get("size"),
            "box primitive size must contain three finite positive dimensions",
            "use size:[width,height,depth] in meters",
            diagnostics,
        ),
        Some(kind) => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            format!("{path}.kind"),
            format!("primitive kind '{kind}' is not implemented in this slice"),
            "use kind:\"box\" until the primitive-coverage slice lands",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_primitive_kind",
            "error",
            format!("{path}.kind"),
            "primitive must include a kind string",
            "use kind:\"box\"",
            None,
            false,
        )),
    }
}

pub(super) fn validate_materials(
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(materials) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_materials",
            "error",
            "$.materials",
            "materials must be an array",
            "emit materials:[{id,kind,base_color}]",
            None,
            false,
        ));
        return;
    };
    for (index, material) in materials.iter().enumerate() {
        let path = format!("$.materials[{index}]");
        let Some(object) = material.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_material",
                "error",
                &path,
                "material entry must be an object",
                "emit material entries as {id, kind, base_color}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, MATERIAL_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let kind = object.get("kind").and_then(Value::as_str);
        match kind {
            Some("unlit" | "pbr_metallic_roughness") => {}
            Some(kind) => diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("material kind '{kind}' is not implemented in this slice"),
                "use kind:\"unlit\" or kind:\"pbr_metallic_roughness\"",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_material_kind",
                "error",
                format!("{path}.kind"),
                "material must include a kind string",
                "use kind:\"unlit\" or kind:\"pbr_metallic_roughness\"",
                None,
                false,
            )),
        }
        validate_color_ref(
            &format!("{path}.base_color"),
            object.get("base_color"),
            colors,
            diagnostics,
        );
        match kind {
            Some("unlit") => {
                for field in ["metallic", "roughness"] {
                    if object.contains_key(field) {
                        diagnostics.push(diagnostic(
                            "unsupported_feature",
                            "error",
                            format!("{path}.{field}"),
                            format!("unlit materials do not use {field}"),
                            "remove the field or use kind:\"pbr_metallic_roughness\"",
                            None,
                            false,
                        ));
                    }
                }
            }
            Some("pbr_metallic_roughness") => {
                validate_unit_float(
                    &format!("{path}.metallic"),
                    object.get("metallic"),
                    diagnostics,
                );
                validate_unit_float(
                    &format!("{path}.roughness"),
                    object.get("roughness"),
                    diagnostics,
                );
            }
            Some(_) | None => {}
        }
    }
}

fn validate_color_ref(
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

fn validate_unit_float(
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

fn validate_positive_vec3(
    path: &str,
    value: Option<&Value>,
    message: &'static str,
    help: &'static str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(finite_vec3) {
        Some([x, y, z]) if x > 0.0 && y > 0.0 && z > 0.0 => {}
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
