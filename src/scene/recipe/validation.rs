use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::Transform;

mod overlays;
mod suggestions;

use super::types::{
    SCENE_RECIPE_SCHEMA_V1, SCENE_RECIPE_VALIDATION_SCHEMA_V1, SceneRecipeDiagnosticV1,
    SceneRecipeV1, SceneRecipeValidationReportV1,
};
use overlays::{
    validate_callouts, validate_exploded_view, validate_measurements, validate_section_box,
};
use suggestions::{
    CAPTURE_FIELDS, EXPECTED_EXTENT_FIELDS, IMPORT_FIELDS, ROOT_FIELDS, UNSUPPORTED_SECTION_FIELDS,
    UNSUPPORTED_WORKFLOW_FIELDS, nearest_capture_field, nearest_import_field, nearest_root_field,
};

pub fn validate_scene_recipe_json(text: &str) -> SceneRecipeValidationReportV1 {
    match serde_json::from_str::<Value>(text) {
        Ok(value) => validate_scene_recipe_value(value),
        Err(error) => validation_report(vec![diagnostic(
            "invalid_json",
            "error",
            "$",
            format!("recipe is not valid JSON: {error}"),
            "emit a JSON object with schema scena.scene_recipe.v1",
            None,
            false,
        )]),
    }
}

pub fn validate_scene_recipe_value(value: Value) -> SceneRecipeValidationReportV1 {
    let mut diagnostics = Vec::new();
    validate_scene_recipe_value_inner(&value, &mut diagnostics);
    validation_report(diagnostics)
}

pub fn parse_valid_scene_recipe_json(
    text: &str,
) -> Result<SceneRecipeV1, SceneRecipeValidationReportV1> {
    let value = match serde_json::from_str::<Value>(text) {
        Ok(value) => value,
        Err(_) => return Err(validate_scene_recipe_json(text)),
    };
    let report = validate_scene_recipe_value(value.clone());
    if !report.ok {
        return Err(report);
    }
    serde_json::from_value::<SceneRecipeV1>(value).map_err(|error| {
        validation_report(vec![diagnostic(
            "invalid_shape",
            "error",
            "$",
            format!("recipe shape did not match scena.scene_recipe.v1: {error}"),
            "use `scena schema get scena.scene_recipe.v1` for the accepted shape",
            None,
            false,
        )])
    })
}

fn validate_scene_recipe_value_inner(
    value: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_shape",
            "error",
            "$",
            "recipe must be a JSON object",
            "emit a JSON object with schema, imports, and optional capture fields",
            None,
            false,
        ));
        return;
    };

    validate_root_fields(object.keys().map(String::as_str), diagnostics);
    validate_schema(object.get("schema"), diagnostics);
    validate_imports(object.get("imports"), diagnostics);
    let import_ids = import_ids(object.get("imports"));
    validate_section_box(object.get("section_box"), &import_ids, diagnostics);
    validate_measurements(object.get("measurements"), diagnostics);
    validate_callouts(object.get("callouts"), &import_ids, diagnostics);
    validate_exploded_view(object.get("exploded_view"), &import_ids, diagnostics);
    validate_capture(object.get("capture"), diagnostics);
    validate_metadata(object.get("metadata"), diagnostics);
}

fn validate_root_fields<'a>(
    keys: impl Iterator<Item = &'a str>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in keys {
        if ROOT_FIELDS.contains(&key) {
            continue;
        }
        if UNSUPPORTED_WORKFLOW_FIELDS.contains(&key) {
            diagnostics.push(diagnostic(
                "unsupported_workflow",
                "error",
                format!("$.{key}"),
                format!("recipe field '{key}' would make the recipe a workflow script"),
                "keep recipes as declarative snapshots; the host owns sequencing, loops, and time",
                None,
                false,
            ));
        } else if UNSUPPORTED_SECTION_FIELDS.contains(&key) {
            diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("$.{key}"),
                format!("recipe section '{key}' is not implemented in this scena build"),
                "remove the section or wait for the feature slice that owns it",
                None,
                false,
            ));
        } else {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("$.{key}"),
                format!("recipe field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field or use metadata for caller-owned opaque data",
                nearest_root_field(key).map(str::to_owned),
                false,
            ));
        }
    }
}

fn validate_schema(schema: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    match schema.and_then(Value::as_str) {
        Some(SCENE_RECIPE_SCHEMA_V1) => {}
        Some(found) => diagnostics.push(diagnostic(
            "schema_mismatch",
            "error",
            "$.schema",
            format!("expected schema '{SCENE_RECIPE_SCHEMA_V1}', got '{found}'"),
            "set schema to scena.scene_recipe.v1",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_schema",
            "error",
            "$.schema",
            "recipe must declare schema scena.scene_recipe.v1",
            "add `\"schema\": \"scena.scene_recipe.v1\"`",
            None,
            true,
        )),
    }
}

fn validate_imports(imports: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(imports) = imports else {
        diagnostics.push(diagnostic(
            "missing_imports",
            "error",
            "$.imports",
            "recipe must contain at least one import in the current slice",
            "add imports:[{id, uri}] or use a direct asset path",
            None,
            false,
        ));
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
    if imports.is_empty() {
        diagnostics.push(diagnostic(
            "missing_imports",
            "error",
            "$.imports",
            "recipe must contain at least one import in the current slice",
            "add imports:[{id, uri}] or use a direct asset path",
            None,
            false,
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, import) in imports.iter().enumerate() {
        validate_import(index, import, &mut ids, diagnostics);
    }
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

fn import_ids(imports: Option<&Value>) -> BTreeSet<String> {
    imports
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn validate_capture(capture: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(capture) = capture else {
        return;
    };
    let Some(object) = capture.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_capture",
            "error",
            "$.capture",
            "capture must be an object",
            "emit capture:{width,height}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !CAPTURE_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("$.capture.{key}"),
                format!("capture field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field; CLI --out owns artifact paths",
                nearest_capture_field(key).map(str::to_owned),
                false,
            ));
        }
    }
    for field in ["width", "height"] {
        match object.get(field).and_then(Value::as_u64) {
            Some(value) if value > 0 && value <= u64::from(u32::MAX) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_capture",
                "error",
                format!("$.capture.{field}"),
                format!("capture {field} must be a positive integer"),
                "use a positive pixel dimension",
                None,
                false,
            )),
        }
    }
}

fn validate_metadata(metadata: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(metadata) = metadata else {
        return;
    };
    if !metadata.is_object() {
        diagnostics.push(diagnostic(
            "invalid_metadata",
            "error",
            "$.metadata",
            "metadata must be an object when present",
            "put caller-owned opaque values under metadata object keys",
            None,
            false,
        ));
    }
}

fn validation_report(diagnostics: Vec<SceneRecipeDiagnosticV1>) -> SceneRecipeValidationReportV1 {
    SceneRecipeValidationReportV1 {
        schema: SCENE_RECIPE_VALIDATION_SCHEMA_V1.to_owned(),
        ok: !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error"),
        diagnostics,
    }
}

pub(super) fn diagnostic(
    code: impl Into<String>,
    severity: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
    suggestion: Option<String>,
    auto_fixable: bool,
) -> SceneRecipeDiagnosticV1 {
    SceneRecipeDiagnosticV1 {
        code: code.into(),
        severity: severity.into(),
        path: path.into(),
        message: message.into(),
        help: help.into(),
        suggestion,
        auto_fixable,
    }
}
