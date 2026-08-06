use std::collections::BTreeSet;

use serde_json::Value;

mod authoring;
mod expectations;
mod imports;
mod material_imperfection;
mod overlays;
mod photo;
mod setup;
mod spatial_state;
mod suggestions;

use super::RecipeBuildPolicy;
use super::types::{
    SCENE_RECIPE_SCHEMA_V1, SCENE_RECIPE_VALIDATION_SCHEMA_V1, SceneRecipeDiagnosticV1,
    SceneRecipeV1, SceneRecipeValidationReportV1,
};
use authoring::{has_authored_renderable_nodes, validate_authoring_sections};
use material_imperfection::validate_material_imperfection;
use overlays::{
    validate_callouts, validate_exploded_view, validate_measurements, validate_section_box,
};
use suggestions::{
    CAPTURE_FIELDS, ROOT_FIELDS, UNSUPPORTED_SECTION_FIELDS, UNSUPPORTED_WORKFLOW_FIELDS,
    nearest_capture_field, nearest_root_field,
};

pub fn validate_scene_recipe_json(text: &str) -> SceneRecipeValidationReportV1 {
    validate_scene_recipe_json_with_policy(text, &RecipeBuildPolicy::default())
}

pub fn validate_scene_recipe_json_with_policy(
    text: &str,
    policy: &RecipeBuildPolicy,
) -> SceneRecipeValidationReportV1 {
    if text.len() > policy.max_recipe_bytes() {
        return recipe_too_large_report(text.len(), policy.max_recipe_bytes());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value) => validate_scene_recipe_value_with_policy(value, policy),
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

/// Performs structural validation and inventories authored resources without I/O.
///
/// The returned report is explicitly marked `syntax_only` and never claims
/// execution equivalence. Use the asset-aware validator for full resolution.
pub fn validate_scene_recipe_json_syntax_with_policy(
    text: &str,
    policy: &RecipeBuildPolicy,
) -> SceneRecipeValidationReportV1 {
    let mut report = validate_scene_recipe_json_with_policy(text, policy);
    report.policy = Some(Box::new(policy.to_schema_report()));
    if report.ok
        && let Ok(recipe) = serde_json::from_str::<SceneRecipeV1>(text)
    {
        report.resources = policy.recipe_resource_inventory(&recipe).reports();
    }
    report
}

pub fn validate_scene_recipe_value(value: Value) -> SceneRecipeValidationReportV1 {
    validate_scene_recipe_value_with_policy(value, &RecipeBuildPolicy::default())
}

pub fn validate_scene_recipe_value_with_policy(
    value: Value,
    policy: &RecipeBuildPolicy,
) -> SceneRecipeValidationReportV1 {
    let mut diagnostics = Vec::new();
    validate_scene_recipe_value_inner(&value, policy, &mut diagnostics);
    validation_report(diagnostics)
}

pub fn parse_valid_scene_recipe_json(
    text: &str,
) -> Result<SceneRecipeV1, SceneRecipeValidationReportV1> {
    parse_valid_scene_recipe_json_with_policy(text, &RecipeBuildPolicy::default())
}

pub fn parse_valid_scene_recipe_json_with_policy(
    text: &str,
    policy: &RecipeBuildPolicy,
) -> Result<SceneRecipeV1, SceneRecipeValidationReportV1> {
    if text.len() > policy.max_recipe_bytes() {
        return Err(recipe_too_large_report(
            text.len(),
            policy.max_recipe_bytes(),
        ));
    }
    let value = match serde_json::from_str::<Value>(text) {
        Ok(value) => value,
        Err(_) => return Err(validate_scene_recipe_json_with_policy(text, policy)),
    };
    let report = validate_scene_recipe_value_with_policy(value.clone(), policy);
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
    policy: &RecipeBuildPolicy,
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
    let allow_empty_imports = has_authored_renderable_nodes(object);
    imports::validate_imports(
        object.get("imports"),
        allow_empty_imports,
        policy,
        diagnostics,
    );
    validate_authoring_sections(object, policy, diagnostics);
    spatial_state::validate_spatial_state_sections(object, diagnostics);
    let import_ids = imports::import_ids(object.get("imports"));
    let node_ids = authored_node_ids(object.get("nodes"));
    validate_section_box(object.get("section_box"), &import_ids, diagnostics);
    validate_measurements(object.get("measurements"), diagnostics);
    validate_callouts(object.get("callouts"), &import_ids, diagnostics);
    validate_exploded_view(object.get("exploded_view"), &import_ids, diagnostics);
    setup::validate_scene_setup(object.get("scene"), diagnostics);
    setup::validate_render_setup(object.get("render"), &import_ids, &node_ids, diagnostics);
    photo::validate_photo(object, &import_ids, &node_ids, diagnostics);
    expectations::validate_expectations(object.get("expect"), diagnostics);
    validate_capture(object.get("capture"), diagnostics);
    validate_metadata(object.get("metadata"), diagnostics);
}

fn authored_node_ids(nodes: Option<&Value>) -> BTreeSet<String> {
    nodes
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|node| node.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

pub fn recipe_too_large_report(
    byte_len: usize,
    max_recipe_bytes: usize,
) -> SceneRecipeValidationReportV1 {
    validation_report(vec![diagnostic(
        "policy_violation",
        "error",
        "$",
        format!(
            "recipe document is {byte_len} bytes, exceeding RecipeBuildPolicy max_recipe_bytes {max_recipe_bytes}"
        ),
        "use a smaller recipe document or raise the operator-owned max_recipe_bytes policy",
        None,
        false,
    )])
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
        validation_mode: super::types::RecipeValidationModeV1::SyntaxOnly,
        execution_equivalent: false,
        policy: None,
        resources: Vec::new(),
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
        candidates: Vec::new(),
        auto_fixable,
        resource: None,
    }
}
