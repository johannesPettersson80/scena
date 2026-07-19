use serde_json::Value;

use crate::material::MaterialDesc;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::diagnostic;

pub(super) fn validate_import_material(
    path: String,
    material: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    const FIELDS: &[&str] = &[
        "preset",
        "base_color",
        "roughness",
        "metallic",
        "double_sided",
    ];
    let Some(object) = material.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import_material",
            "error",
            &path,
            "import material must be an object with preset or base_color plus optional roughness and metallic",
            "use material:{preset:\"clearcoat_plastic\",base_color:\"#D8C69A\"} or material:{base_color:\"#3A3D42\",roughness:0.8,metallic:0.0}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("import material field '{key}' is not part of scena.scene_recipe.v1"),
                "use preset, base_color, roughness, metallic, or double_sided",
                None,
                false,
            ));
        }
    }
    match object.get("preset").and_then(Value::as_str) {
        Some(preset) if MaterialDesc::PRESET_NAMES.contains(&preset) => {}
        Some(preset) => diagnostics.push(diagnostic(
            "invalid_material_preset",
            "error",
            format!("{path}.preset"),
            format!("import material preset '{preset}' is not supported"),
            format!("use one of: {}", MaterialDesc::PRESET_NAMES.join(", ")),
            None,
            false,
        )),
        None if object.get("preset").is_some() => diagnostics.push(diagnostic(
            "invalid_material_preset",
            "error",
            format!("{path}.preset"),
            "import material preset must be a string",
            format!("use one of: {}", MaterialDesc::PRESET_NAMES.join(", ")),
            None,
            false,
        )),
        None => {}
    }
    match object.get("base_color").and_then(Value::as_str) {
        Some(color) if !color.trim().is_empty() => {}
        Some(_) => diagnostics.push(diagnostic(
            "invalid_import_material",
            "error",
            format!("{path}.base_color"),
            "import material base_color must be a non-empty color string",
            "use a declared color id, named Color constant, or direct #RRGGBB value",
            None,
            false,
        )),
        None if object.get("preset").is_none() => diagnostics.push(diagnostic(
            "invalid_import_material",
            "error",
            format!("{path}.base_color"),
            "import material must include base_color unless it uses a preset",
            "provide base_color or use a supported material preset with an optional tint",
            None,
            false,
        )),
        None => {}
    }
    validate_unit_scalar(&path, object.get("roughness"), "roughness", diagnostics);
    validate_unit_scalar(&path, object.get("metallic"), "metallic", diagnostics);
    if object
        .get("double_sided")
        .is_some_and(|double_sided| !double_sided.is_boolean())
    {
        diagnostics.push(diagnostic(
            "invalid_import_material",
            "error",
            format!("{path}.double_sided"),
            "import material double_sided must be a boolean",
            "set double_sided:true when a CAD import must be reviewable from the back side",
            None,
            false,
        ));
    }
}

pub(super) fn validate_import_edge_emphasis(
    path: String,
    edge_emphasis: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    const FIELDS: &[&str] = &[
        "enabled",
        "base_color",
        "stroke_width_px",
        "edge_angle_threshold_degrees",
    ];
    let Some(object) = edge_emphasis.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import_edge_emphasis",
            "error",
            &path,
            "import edge_emphasis must be an object",
            "use edge_emphasis:{enabled:true,base_color:\"#FFB000\",stroke_width_px:2.0}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("import edge_emphasis field '{key}' is not part of scena.scene_recipe.v1"),
                "use enabled, base_color, stroke_width_px, or edge_angle_threshold_degrees",
                None,
                false,
            ));
        }
    }
    if object
        .get("enabled")
        .is_some_and(|enabled| !enabled.is_boolean())
    {
        diagnostics.push(diagnostic(
            "invalid_import_edge_emphasis",
            "error",
            format!("{path}.enabled"),
            "import edge_emphasis enabled must be a boolean",
            "set enabled:false to disable edge overlay generation",
            None,
            false,
        ));
    }
    if let Some(color) = object.get("base_color")
        && !matches!(color.as_str(), Some(value) if !value.trim().is_empty())
    {
        diagnostics.push(diagnostic(
            "invalid_import_edge_emphasis",
            "error",
            format!("{path}.base_color"),
            "import edge_emphasis base_color must be a non-empty color string",
            "use a declared color id, named Color constant, or direct #RRGGBB value",
            None,
            false,
        ));
    }
    validate_positive_scalar(
        &path,
        object.get("stroke_width_px"),
        "stroke_width_px",
        diagnostics,
    );
    if let Some(threshold) = object.get("edge_angle_threshold_degrees") {
        match threshold.as_f64() {
            Some(value) if value.is_finite() && (0.0..=180.0).contains(&value) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_import_edge_emphasis",
                "error",
                format!("{path}.edge_angle_threshold_degrees"),
                "import edge_emphasis edge_angle_threshold_degrees must be finite and in [0, 180]",
                "use a threshold such as 25.0",
                None,
                false,
            )),
        }
    }
}

fn validate_unit_scalar(
    path: &str,
    value: Option<&Value>,
    field: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(value) = value {
        match value.as_f64() {
            Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_import_material",
                "error",
                format!("{path}.{field}"),
                format!("import material {field} must be finite and in [0, 1]"),
                "use a normalized material scalar",
                None,
                false,
            )),
        }
    }
}

fn validate_positive_scalar(
    path: &str,
    value: Option<&Value>,
    field: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(value) = value {
        match value.as_f64() {
            Some(value) if value.is_finite() && value > 0.0 => {}
            _ => diagnostics.push(diagnostic(
                "invalid_import_edge_emphasis",
                "error",
                format!("{path}.{field}"),
                format!("import edge_emphasis {field} must be finite and positive"),
                "use a positive scalar",
                None,
                false,
            )),
        }
    }
}
