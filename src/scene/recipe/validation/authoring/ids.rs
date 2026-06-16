use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::diagnostic;

pub(super) fn validate_global_ids(
    object: &Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut ids = BTreeSet::new();
    collect_import_ids(object.get("imports"), &mut ids, diagnostics);
    collect_map_ids("colors", object.get("colors"), &mut ids, diagnostics);
    for section in ["geometries", "materials", "nodes", "cameras"] {
        collect_array_ids(section, object.get(section), &mut ids, diagnostics);
    }
}

fn collect_import_ids(
    value: Option<&Value>,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(imports) = value.and_then(Value::as_array) else {
        return;
    };
    for (index, import) in imports.iter().enumerate() {
        let path = format!("$.imports[{index}].id");
        if let Some(id) = import.get("id").and_then(Value::as_str) {
            collect_id(&path, id, ids, diagnostics);
        }
    }
}

fn collect_map_ids(
    section: &str,
    value: Option<&Value>,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(object) = value.and_then(Value::as_object) else {
        return;
    };
    for id in object.keys() {
        collect_id(&format!("$.{section}.{id}"), id, ids, diagnostics);
    }
}

fn collect_array_ids(
    section: &str,
    value: Option<&Value>,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(array) = value.and_then(Value::as_array) else {
        return;
    };
    for (index, entry) in array.iter().enumerate() {
        if let Some(id) = entry.get("id").and_then(Value::as_str) {
            collect_id(&format!("$.{section}[{index}].id"), id, ids, diagnostics);
        }
    }
}

fn collect_id(
    path: &str,
    id: &str,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if id.trim().is_empty() {
        diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            path,
            "recipe id must not be empty",
            "use a stable caller-owned id such as `plate` or `main_camera`",
            None,
            false,
        ));
    } else if !ids.insert(id.to_owned()) {
        diagnostics.push(diagnostic(
            "duplicate_id",
            "error",
            path,
            format!("recipe id '{id}' is used more than once"),
            "make every recipe id globally unique across imports, colors, resources, and nodes",
            None,
            false,
        ));
    }
}

pub(super) fn id_set_from_map(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| object.keys())
        .cloned()
        .collect()
}

pub(super) fn id_set_from_array(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}
