use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::scene::recipe::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

pub(in crate::scene::recipe::validation::authoring) use super::super::super::resources::SkinJointIndexLimit;
use super::super::super::validate_known_fields;

const SKIN_BINDING_FIELDS: &[&str] = &["joints", "inverse_bind_matrices"];

pub(in crate::scene::recipe::validation::authoring) fn validate_skin_binding(
    path: &str,
    value: Option<&Value>,
    geometry_id: Option<&str>,
    skinned_geometries: &BTreeSet<String>,
    skin_max_joint_indices: &BTreeMap<String, SkinJointIndexLimit>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let uses_skinned_geometry =
        geometry_id.is_some_and(|geometry| skinned_geometries.contains(geometry));
    match (value, uses_skinned_geometry) {
        (None, false) => return,
        (None, true) => {
            diagnostics.push(diagnostic(
                "missing_skin_binding",
                "error",
                path,
                "node uses a skinned geometry but has no skin_binding",
                "bind the node to authored joint ids and inverse bind matrices",
                None,
                false,
            ));
            return;
        }
        (Some(_), false) => {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                path,
                "skin_binding requires a skin-derived geometry",
                "remove skin_binding or use a geometry from skins[]",
                None,
                false,
            ));
        }
        (Some(_), true) => {}
    }
    let Some(object) = value.and_then(Value::as_object) else {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            "skin_binding must be an object",
            "emit {joints:[...],inverse_bind_matrices:[[...]]}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, SKIN_BINDING_FIELDS, diagnostics);
    let joint_count = validate_skin_binding_joints(
        &format!("{path}.joints"),
        object.get("joints"),
        node_ids,
        diagnostics,
    );
    if let (Some(geometry), Some(joint_count)) = (geometry_id, joint_count)
        && let Some(limit) = skin_max_joint_indices.get(geometry)
        && limit.index >= joint_count
    {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            &limit.path,
            format!(
                "skin geometry '{geometry}' references joint index {} but its node binding has only {joint_count} joint(s)",
                limit.index
            ),
            "add enough skin_binding joints or lower the authored skin joint indices",
            None,
            false,
        ));
    }
    validate_inverse_bind_matrices(
        &format!("{path}.inverse_bind_matrices"),
        object.get("inverse_bind_matrices"),
        joint_count,
        diagnostics,
    );
}

fn validate_skin_binding_joints(
    path: &str,
    value: Option<&Value>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<usize> {
    let Some(joints) = value
        .and_then(Value::as_array)
        .filter(|joints| !joints.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            "skin_binding joints must be a non-empty string array",
            "emit joint node ids",
            None,
            false,
        ));
        return None;
    };
    for (index, joint) in joints.iter().enumerate() {
        match joint.as_str() {
            Some(id) if node_ids.contains(id) => {}
            Some(id) => diagnostics.push(diagnostic(
                "unknown_node_ref",
                "error",
                format!("{path}[{index}]"),
                format!("skin_binding joint references unknown node '{id}'"),
                "target an authored node id from nodes[]",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{index}]"),
                "skin_binding joint must be a node id string",
                "emit a stable node id",
                None,
                false,
            )),
        }
    }
    Some(joints.len())
}

fn validate_inverse_bind_matrices(
    path: &str,
    value: Option<&Value>,
    joint_count: Option<usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(matrices) = value
        .and_then(Value::as_array)
        .filter(|matrices| !matrices.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            "skin_binding inverse_bind_matrices must be a non-empty array",
            "emit one 16-number matrix per joint",
            None,
            false,
        ));
        return;
    };
    if let Some(expected) = joint_count
        && matrices.len() != expected
    {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            format!(
                "skin_binding has {} inverse bind matrices but {expected} joints",
                matrices.len()
            ),
            "emit exactly one inverse bind matrix per joint",
            None,
            false,
        ));
    }
    for (matrix_index, matrix) in matrices.iter().enumerate() {
        let Some(values) = matrix.as_array() else {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{matrix_index}]"),
                "inverse bind matrix must be a 16-number array",
                "emit glTF column-major matrix values",
                None,
                false,
            ));
            continue;
        };
        if values.len() != 16 {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{matrix_index}]"),
                "inverse bind matrix must contain exactly 16 values",
                "emit glTF column-major matrix values",
                None,
                false,
            ));
        }
        for (value_index, value) in values.iter().enumerate() {
            match value.as_f64() {
                Some(value) if value.is_finite() && value.abs() <= f64::from(f32::MAX) => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_skin",
                    "error",
                    format!("{path}[{matrix_index}][{value_index}]"),
                    "inverse bind matrix values must be finite f32-compatible numbers",
                    "emit finite numeric matrix values",
                    None,
                    false,
                )),
            }
        }
    }
}
