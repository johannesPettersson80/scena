use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::Transform;
use crate::scene::recipe::{RecipeBuildPolicy, types::SceneRecipeDiagnosticV1};

use super::diagnostic;
use super::suggestions::{EXPECTED_EXTENT_FIELDS, IMPORT_FIELDS, nearest_import_field};

pub(super) fn validate_imports(
    imports: Option<&Value>,
    allow_empty: bool,
    policy: &RecipeBuildPolicy,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(imports) = imports else {
        if !allow_empty {
            diagnostics.push(missing_imports_diagnostic());
        }
        return;
    };
    let Some(imports) = imports.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_imports",
            "error",
            "$.imports",
            "imports must be an array",
            "emit imports as an array of {id, uri} objects",
            None,
            false,
        ));
        return;
    };
    if imports.is_empty() && !allow_empty {
        diagnostics.push(missing_imports_diagnostic());
    }
    if imports.len() > policy.max_imports() {
        diagnostics.push(diagnostic(
            "policy_violation",
            "error",
            "$.imports",
            format!(
                "recipe imports {} assets, exceeding RecipeBuildPolicy max_imports {}",
                imports.len(),
                policy.max_imports()
            ),
            "split the recipe or raise the operator-owned max_imports policy",
            None,
            false,
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, import) in imports.iter().enumerate() {
        validate_import(index, import, &mut ids, diagnostics);
    }
}

fn missing_imports_diagnostic() -> SceneRecipeDiagnosticV1 {
    diagnostic(
        "missing_imports",
        "error",
        "$.imports",
        "recipe must contain at least one import or authored renderable node",
        "add imports:[{id, uri}] or author a node with geometry and material",
        None,
        false,
    )
}

fn validate_import(
    index: usize,
    import: &Value,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let path = format!("$.imports[{index}]");
    let Some(object) = import.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import",
            "error",
            &path,
            "import entry must be an object",
            "emit each import as {id, uri}",
            None,
            false,
        ));
        return;
    };

    validate_import_fields(&path, object.keys().map(String::as_str), diagnostics);
    validate_import_id(&path, object.get("id"), ids, diagnostics);
    validate_import_uri(&path, object.get("uri"), diagnostics);
    validate_import_optional(&path, object.get("optional"), diagnostics);
    if let Some(transform) = object.get("transform") {
        validate_transform(format!("{path}.transform"), transform, diagnostics);
    }
    if let Some(extent) = object.get("expected_extent") {
        validate_expected_extent(format!("{path}.expected_extent"), extent, diagnostics);
    }
    if let Some(material) = object.get("material") {
        validate_import_material(format!("{path}.material"), material, diagnostics);
    }
    if let Some(edge_emphasis) = object.get("edge_emphasis") {
        validate_import_edge_emphasis(format!("{path}.edge_emphasis"), edge_emphasis, diagnostics);
    }
}

fn validate_import_fields<'a>(
    path: &str,
    keys: impl Iterator<Item = &'a str>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in keys {
        if !IMPORT_FIELDS.contains(&key) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("import field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field or move caller-owned data to metadata",
                nearest_import_field(key).map(str::to_owned),
                false,
            ));
        }
    }
}

fn validate_import_id(
    path: &str,
    id: Option<&Value>,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match id.and_then(Value::as_str) {
        Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            format!("{path}.id"),
            "import id must not be empty",
            "use a stable caller-owned id such as `body` or `part_1`",
            None,
            false,
        )),
        Some(id) if !ids.insert(id.to_owned()) => diagnostics.push(diagnostic(
            "duplicate_id",
            "error",
            format!("{path}.id"),
            format!("import id '{id}' is used more than once"),
            "make recipe ids unique so diagnostics and future patches can name one target",
            None,
            false,
        )),
        Some(_) => {}
        None => diagnostics.push(diagnostic(
            "missing_id",
            "error",
            format!("{path}.id"),
            "import entry must include an id string",
            "add a stable caller-owned id",
            None,
            false,
        )),
    }
}

fn validate_import_uri(
    path: &str,
    uri: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match uri.and_then(Value::as_str) {
        Some(uri) if uri.trim().is_empty() => diagnostics.push(diagnostic(
            "missing_asset",
            "error",
            format!("{path}.uri"),
            "import uri must not be empty",
            "point uri at a glTF/GLB asset",
            None,
            false,
        )),
        Some(_) => {}
        None => diagnostics.push(diagnostic(
            "missing_asset",
            "error",
            format!("{path}.uri"),
            "import entry must include a uri string",
            "point uri at a glTF/GLB asset",
            None,
            false,
        )),
    }
}

fn validate_import_optional(
    path: &str,
    optional: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if optional.is_some_and(|optional| !optional.is_boolean()) {
        diagnostics.push(diagnostic(
            "invalid_optional",
            "error",
            format!("{path}.optional"),
            "import optional must be a boolean when present",
            "set optional to true only when a missing import may be skipped",
            None,
            false,
        ));
    }
}

fn validate_expected_extent(
    path: String,
    extent: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(object) = extent.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expected_extent",
            "error",
            &path,
            "expected_extent must be an object with min and max",
            "emit expected_extent:{min,max,unit}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !EXPECTED_EXTENT_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("expected_extent field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field; expected_extent accepts min, max, and optional unit",
                None,
                false,
            ));
        }
    }
    let min = object.get("min").and_then(Value::as_f64);
    let max = object.get("max").and_then(Value::as_f64);
    match (min, max) {
        (Some(min), Some(max)) if min.is_finite() && max.is_finite() && min > 0.0 && max >= min => {
        }
        _ => diagnostics.push(diagnostic(
            "invalid_expected_extent",
            "error",
            &path,
            "expected_extent requires finite positive min and max with max >= min",
            "use a finite positive size range, for example {min:0.1,max:10.0}",
            None,
            false,
        )),
    }
    if object
        .get("unit")
        .is_some_and(|unit| !unit.is_string() && !unit.is_null())
    {
        diagnostics.push(diagnostic(
            "invalid_expected_extent",
            "error",
            format!("{path}.unit"),
            "expected_extent unit must be a string when present",
            "use a unit label such as `m`, `cm`, or `mm`, or omit unit",
            None,
            false,
        ));
    }
}

fn validate_import_material(
    path: String,
    material: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    const FIELDS: &[&str] = &["base_color", "roughness", "metallic", "double_sided"];
    let Some(object) = material.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import_material",
            "error",
            &path,
            "import material must be an object with base_color, roughness, and metallic",
            "use material:{base_color:\"#3A3D42\",roughness:0.8,metallic:0.0}",
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
                "use base_color, roughness, metallic, or double_sided",
                None,
                false,
            ));
        }
    }
    match object.get("base_color").and_then(Value::as_str) {
        Some(color) if !color.trim().is_empty() => {}
        Some(_) | None => diagnostics.push(diagnostic(
            "invalid_import_material",
            "error",
            format!("{path}.base_color"),
            "import material base_color must be a non-empty color string",
            "use a declared color id, named Color constant, or direct #RRGGBB value",
            None,
            false,
        )),
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

fn validate_import_edge_emphasis(
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

fn validate_transform(
    path: String,
    transform: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if serde_json::from_value::<Transform>(transform.clone()).is_err() {
        diagnostics.push(diagnostic(
            "invalid_transform",
            "error",
            path,
            "transform must match scena's stable Transform JSON shape",
            "emit translation, rotation, and scale arrays, or omit transform for identity",
            None,
            false,
        ));
    }
}

pub(super) fn import_ids(imports: Option<&Value>) -> BTreeSet<String> {
    imports
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}
