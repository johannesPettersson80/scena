use serde_json::Value;

use crate::material::MaterialDesc;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::diagnostic;

mod material_bindings;
pub(super) use material_bindings::validate_import_material_bindings;

pub(super) fn validate_import_material(
    path: String,
    material: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    const FIELDS: &[&str] = &[
        "preset",
        "material_pack",
        "imperfection",
        "base_color",
        "roughness",
        "metallic",
        "normal_scale",
        "occlusion_strength",
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
                "use material_pack, preset, base_color, roughness, metallic, normal_scale, occlusion_strength, or double_sided",
                None,
                false,
            ));
        }
    }
    let material_pack = object.get("material_pack");
    if let Some(imperfection) = object.get("imperfection") {
        super::super::validate_material_imperfection(
            &format!("{path}.imperfection"),
            imperfection,
            diagnostics,
        );
        if material_pack.is_none() {
            diagnostics.push(diagnostic(
                "material_imperfection_requires_surface_maps",
                "error",
                format!("{path}.imperfection"),
                "import material imperfection requires a material_pack with normal and roughness maps",
                "add material_pack or remove imperfection",
                None,
                false,
            ));
        }
    }
    if let Some(pack) = material_pack {
        validate_material_pack(&path, pack, diagnostics);
        for field in ["preset", "roughness", "metallic"] {
            if object.contains_key(field) {
                diagnostics.push(diagnostic(
                    "conflicting_import_material_pack_field",
                    "error",
                    format!("{path}.{field}"),
                    format!("{field} cannot override a source-locked import material pack"),
                    "remove the field; use base_color only as a tint or tile_size_m inside material_pack",
                    None,
                    false,
                ));
            }
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
        None if object.get("preset").is_none() && material_pack.is_none() => {
            diagnostics.push(diagnostic(
                "invalid_import_material",
                "error",
                format!("{path}.base_color"),
                "import material must include base_color unless it uses a preset",
                "provide base_color or use a supported material preset with an optional tint",
                None,
                false,
            ))
        }
        None => {}
    }
    validate_unit_scalar(&path, object.get("roughness"), "roughness", diagnostics);
    validate_unit_scalar(&path, object.get("metallic"), "metallic", diagnostics);
    validate_unit_scalar(
        &path,
        object.get("normal_scale"),
        "normal_scale",
        diagnostics,
    );
    validate_unit_scalar(
        &path,
        object.get("occlusion_strength"),
        "occlusion_strength",
        diagnostics,
    );
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

fn validate_material_pack(
    material_path: &str,
    value: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    const FIELDS: &[&str] = &["uri", "expected_archive_sha256", "tile_size_m"];
    let path = format!("{material_path}.material_pack");
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import_material_pack",
            "error",
            path,
            "import material_pack must be an object",
            "use material_pack:{uri:\"materials/steel/scena-material-pack.json\"}",
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
                format!("import material_pack field '{key}' is not part of scena.scene_recipe.v1"),
                "use uri, expected_archive_sha256, or tile_size_m",
                None,
                false,
            ));
        }
    }
    match object.get("uri").and_then(Value::as_str) {
        Some(uri) if !uri.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_import_material_pack",
            "error",
            format!("{path}.uri"),
            "import material pack uri must be a non-empty string",
            "reference a compiled scena-material-pack.json file",
            None,
            false,
        )),
    }
    if let Some(hash) = object.get("expected_archive_sha256") {
        match hash.as_str() {
            Some(hash) if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            }
            _ => diagnostics.push(diagnostic(
                "invalid_material_pack_source_sha256",
                "error",
                format!("{path}.expected_archive_sha256"),
                "expected material archive SHA-256 must contain exactly 64 hexadecimal digits",
                "copy source.archive_sha256 from the compiled material pack manifest",
                None,
                false,
            )),
        }
    }
    if let Some(tile_size_m) = object.get("tile_size_m") {
        match tile_size_m.as_f64() {
            Some(value) if value.is_finite() && value > 0.0 => {}
            _ => diagnostics.push(diagnostic(
                "invalid_import_material_pack",
                "error",
                format!("{path}.tile_size_m"),
                "import material pack tile_size_m must be finite and positive",
                "use the real-world width of one texture tile in metres",
                None,
                false,
            )),
        }
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

pub(super) fn validate_import_edge_rounding(
    path: String,
    edge_rounding: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    const FIELDS: &[&str] = &[
        "enabled",
        "radius_fraction",
        "segments",
        "edge_angle_threshold_degrees",
        "max_derived_triangles",
    ];
    let Some(object) = edge_rounding.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import_edge_rounding",
            "error",
            &path,
            "import edge_rounding must be an object",
            "use edge_rounding:{enabled:true,radius_fraction:0.0025,segments:3}",
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
                format!("import edge_rounding field '{key}' is not part of scena.scene_recipe.v1"),
                "use enabled, radius_fraction, segments, edge_angle_threshold_degrees, or max_derived_triangles",
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
            "invalid_import_edge_rounding",
            "error",
            format!("{path}.enabled"),
            "import edge_rounding enabled must be a boolean",
            "set enabled:true or remove edge_rounding",
            None,
            false,
        ));
    }
    if object.get("radius_fraction").is_some_and(|value| {
        !value
            .as_f64()
            .is_some_and(|value| value.is_finite() && value > 0.0 && value <= 0.02)
    }) {
        diagnostics.push(diagnostic(
            "invalid_import_edge_rounding",
            "error",
            format!("{path}.radius_fraction"),
            "import edge_rounding radius_fraction must be finite and in (0, 0.02]",
            "use 0.0025 for a subtle manufactured-edge radius",
            None,
            false,
        ));
    }
    if object
        .get("segments")
        .is_some_and(|value| !value.as_u64().is_some_and(|value| (1..=8).contains(&value)))
    {
        diagnostics.push(diagnostic(
            "invalid_import_edge_rounding",
            "error",
            format!("{path}.segments"),
            "import edge_rounding segments must be an integer from 1 through 8",
            "use 3 segments for final product photography",
            None,
            false,
        ));
    }
    if object
        .get("edge_angle_threshold_degrees")
        .is_some_and(|value| {
            !value
                .as_f64()
                .is_some_and(|value| value.is_finite() && value > 0.0 && value < 180.0)
        })
    {
        diagnostics.push(diagnostic(
            "invalid_import_edge_rounding",
            "error",
            format!("{path}.edge_angle_threshold_degrees"),
            "import edge_rounding edge_angle_threshold_degrees must be finite and in (0, 180)",
            "use 30 degrees to retain smooth tessellated curvature while rounding hard edges",
            None,
            false,
        ));
    }
    if object
        .get("max_derived_triangles")
        .is_some_and(|value| value.as_u64().is_none_or(|value| value == 0))
    {
        diagnostics.push(diagnostic(
            "invalid_import_edge_rounding",
            "error",
            format!("{path}.max_derived_triangles"),
            "import edge_rounding max_derived_triangles must be a positive integer",
            "use an explicit derived-geometry budget such as 250000",
            None,
            false,
        ));
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
