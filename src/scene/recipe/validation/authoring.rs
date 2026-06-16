use serde_json::{Map, Value};

use super::super::types::SceneRecipeDiagnosticV1;
use super::diagnostic;

mod ids;
mod resources;
mod targets;

pub(super) fn has_authored_renderable_nodes(object: &Map<String, Value>) -> bool {
    targets::has_authored_renderable_nodes(object)
}

pub(super) fn validate_authoring_sections(
    object: &Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    ids::validate_global_ids(object, diagnostics);
    resources::validate_colors(object.get("colors"), diagnostics);
    let color_ids = ids::id_set_from_map(object.get("colors"));
    resources::validate_geometries(object.get("geometries"), diagnostics);
    let geometry_ids = ids::id_set_from_array(object.get("geometries"));
    resources::validate_materials(object.get("materials"), &color_ids, diagnostics);
    let material_ids = ids::id_set_from_array(object.get("materials"));
    targets::validate_nodes(
        object.get("nodes"),
        &geometry_ids,
        &material_ids,
        diagnostics,
    );
    let node_ids = ids::id_set_from_array(object.get("nodes"));
    targets::validate_cameras(object.get("cameras"), &node_ids, diagnostics);
}

fn validate_required_id(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() => {}
        Some(_) => diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            format!("{path}.id"),
            "id must not be empty",
            "use a stable caller-owned id",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_id",
            "error",
            format!("{path}.id"),
            "entry must include an id string",
            "add a stable caller-owned id",
            None,
            false,
        )),
    }
}

fn validate_known_fields(
    path: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("field '{key}' is not accepted here"),
                "remove the field or wait for the feature slice that owns it",
                None,
                false,
            ));
        }
    }
}

fn finite_vec3(value: &Value) -> Option<[f32; 3]> {
    let array = value.as_array()?;
    let [x, y, z] = array.as_slice() else {
        return None;
    };
    let x = x.as_f64()? as f32;
    let y = y.as_f64()? as f32;
    let z = z.as_f64()? as f32;
    (x.is_finite() && y.is_finite() && z.is_finite()).then_some([x, y, z])
}
