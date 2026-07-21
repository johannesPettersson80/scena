use std::collections::BTreeSet;

use serde_json::Value;

use crate::diagnostics::nearest_name_candidates;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

use super::super::finite_vec3;

pub(in crate::scene::recipe::validation::authoring) fn validate_look_at_target(
    path: &str,
    value: Option<&Value>,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value {
        Some(Value::String(target))
            if is_declared_node_or_import_path(target, node_ids, import_ids) => {}
        Some(Value::String(target)) if is_import_path_ref(target) => {
            diagnostics.push(unknown_import_path_diagnostic(path, target, import_ids));
        }
        Some(Value::String(target)) => diagnostics.push(
            diagnostic(
                "unknown_node_ref",
                "error",
                path,
                format!("look_at target references unknown node '{target}'"),
                "target an authored node id, imported node path, or provide a [x,y,z] position",
                None,
                false,
            )
            .with_candidates(nearest_name_candidates(target, node_ids, 3)),
        ),
        Some(value) if finite_vec3(value).is_some() => {}
        Some(_) => diagnostics.push(diagnostic(
            "invalid_look_at_target",
            "error",
            path,
            "look_at target must be an authored node id, imported node path, or finite [x,y,z] position",
            "target an authored node id, imported node path such as part:/Node, or provide a [x,y,z] position",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_look_at_target",
            "error",
            path,
            "look_at transform requires a target",
            "target an authored node id, imported node path, or provide a [x,y,z] position",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_node_ref(
    path: &str,
    value: Option<&Value>,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(value) if is_declared_node_or_import_path(value, node_ids, import_ids) => {}
        Some(value) if is_import_path_ref(value) => {
            diagnostics.push(unknown_import_path_diagnostic(path, value, import_ids));
        }
        Some(value) => diagnostics.push(
            diagnostic(
                "unknown_node_ref",
                "error",
                path,
                format!("node reference '{value}' does not name a declared authored node or imported node path"),
                "declare a node with this id or target an imported node path such as part:/Node",
                None,
                false,
            )
            .with_candidates(nearest_name_candidates(value, node_ids, 3)),
        ),
        None => diagnostics.push(diagnostic(
            "missing_node_ref",
            "error",
            path,
            "node reference must be a string",
            "set node to an authored node id or imported node path such as part:/Node",
            None,
            false,
        )),
    }
}

fn is_declared_node_or_import_path(
    value: &str,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
) -> bool {
    node_ids.contains(value)
        || import_path_import(value).is_some_and(|import| import_ids.contains(import))
}

fn is_import_path_ref(value: &str) -> bool {
    import_path_import(value).is_some()
}

fn import_path_import(value: &str) -> Option<&str> {
    let (import, _path) = value.split_once(":/")?;
    (!import.trim().is_empty()).then_some(import)
}

fn unknown_import_path_diagnostic(
    path: &str,
    value: &str,
    import_ids: &BTreeSet<String>,
) -> SceneRecipeDiagnosticV1 {
    let import = import_path_import(value).unwrap_or(value);
    let suffix = value.split_once(":/").map(|(_, suffix)| suffix);
    let candidates = nearest_name_candidates(import, import_ids, 3)
        .into_iter()
        .map(|candidate| match suffix {
            Some(suffix) => format!("{candidate}:/{suffix}"),
            None => candidate,
        })
        .collect();
    diagnostic(
        "unknown_import_ref",
        "error",
        path,
        format!("imported node path '{value}' references unknown import '{import}'"),
        "declare the import id before targeting imported node paths",
        None,
        false,
    )
    .with_candidates(candidates)
}
