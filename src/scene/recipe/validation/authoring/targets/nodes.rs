use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::common::{
    TransformUse, validate_optional_i16, validate_optional_u64, validate_ref, validate_tags,
    validate_transform,
};
use crate::scene::recipe::validation::diagnostic;

const NODE_FIELDS: &[&str] = &[
    "id",
    "geometry",
    "material",
    "parent",
    "name",
    "tags",
    "visible",
    "layer_mask",
    "render_group",
    "tint",
    "transform",
    "morph_weights",
    "skin_binding",
];
const SKIN_BINDING_FIELDS: &[&str] = &["joints", "inverse_bind_matrices"];

pub(in crate::scene::recipe::validation::authoring) fn has_authored_renderable_nodes(
    object: &Map<String, Value>,
) -> bool {
    object
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|node| {
            node.as_object().is_some_and(|node| {
                node.get("geometry").is_some_and(Value::is_string)
                    && node.get("material").is_some_and(Value::is_string)
            })
        })
}

pub(in crate::scene::recipe::validation::authoring) struct NodeValidationResources<'a> {
    pub(in crate::scene::recipe::validation::authoring) geometries: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) materials: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) morph_target_counts:
        &'a BTreeMap<String, usize>,
    pub(in crate::scene::recipe::validation::authoring) skinned_geometries: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) all_node_ids: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) imports: &'a BTreeSet<String>,
}

pub(in crate::scene::recipe::validation::authoring) fn validate_nodes(
    value: Option<&Value>,
    resources: NodeValidationResources<'_>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(nodes) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_nodes",
            "error",
            "$.nodes",
            "nodes must be an array",
            "emit nodes:[{id,geometry,material}]",
            None,
            false,
        ));
        return;
    };
    for (index, node) in nodes.iter().enumerate() {
        let path = format!("$.nodes[{index}]");
        let Some(object) = node.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_node",
                "error",
                &path,
                "node entry must be an object",
                "emit node entries as {id, geometry, material}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, NODE_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let geometry_id = object.get("geometry").and_then(Value::as_str);
        validate_ref(
            &format!("{path}.geometry"),
            object.get("geometry"),
            resources.geometries,
            "geometry",
            diagnostics,
        );
        validate_ref(
            &format!("{path}.material"),
            object.get("material"),
            resources.materials,
            "material",
            diagnostics,
        );
        if let Some(parent) = object.get("parent") {
            validate_ref(
                &format!("{path}.parent"),
                Some(parent),
                &node_ids_before(nodes, index),
                "node",
                diagnostics,
            );
        }
        if object
            .get("name")
            .is_some_and(|name| !name.is_string() && !name.is_null())
        {
            diagnostics.push(diagnostic(
                "invalid_name",
                "error",
                format!("{path}.name"),
                "node name must be a string when present",
                "use a human-readable node name or omit name",
                None,
                false,
            ));
        }
        validate_tags(&format!("{path}.tags"), object.get("tags"), diagnostics);
        if object
            .get("visible")
            .is_some_and(|value| !value.is_boolean())
        {
            diagnostics.push(diagnostic(
                "invalid_visible",
                "error",
                format!("{path}.visible"),
                "visible must be a boolean",
                "use true or false",
                None,
                false,
            ));
        }
        validate_optional_u64(
            &format!("{path}.layer_mask"),
            object.get("layer_mask"),
            diagnostics,
        );
        validate_optional_i16(
            &format!("{path}.render_group"),
            object.get("render_group"),
            diagnostics,
        );
        if object
            .get("tint")
            .is_some_and(|tint| !tint.is_string() && !tint.is_null())
        {
            diagnostics.push(diagnostic(
                "invalid_tint",
                "error",
                format!("{path}.tint"),
                "tint must be a color id or #RRGGBB string",
                "reference a declared color or omit tint",
                None,
                false,
            ));
        }
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{path}.transform"),
                transform,
                TransformUse::Node,
                &node_ids_before(nodes, index),
                resources.imports,
                diagnostics,
            );
        }
        validate_morph_weights(
            &format!("{path}.morph_weights"),
            object.get("morph_weights"),
            geometry_id.and_then(|geometry| resources.morph_target_counts.get(geometry).copied()),
            diagnostics,
        );
        validate_skin_binding(
            &format!("{path}.skin_binding"),
            object.get("skin_binding"),
            geometry_id,
            resources.skinned_geometries,
            resources.all_node_ids,
            diagnostics,
        );
    }
}

fn node_ids_before(nodes: &[Value], exclusive_index: usize) -> BTreeSet<String> {
    nodes
        .iter()
        .take(exclusive_index)
        .filter_map(|node| node.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn validate_morph_weights(
    path: &str,
    value: Option<&Value>,
    target_count: Option<usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(weights) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_morph",
            "error",
            path,
            "morph_weights must be an array of finite numbers",
            "emit one weight per authored morph target",
            None,
            false,
        ));
        return;
    };
    let Some(target_count) = target_count else {
        diagnostics.push(diagnostic(
            "invalid_morph",
            "error",
            path,
            "morph_weights require a geometry with authored morph targets",
            "remove morph_weights or use a morph-derived geometry",
            None,
            false,
        ));
        return;
    };
    if weights.len() != target_count {
        diagnostics.push(diagnostic(
            "invalid_morph",
            "error",
            path,
            format!(
                "morph_weights has {} entries but geometry has {target_count} morph targets",
                weights.len()
            ),
            "emit exactly one weight per authored morph target",
            None,
            false,
        ));
    }
    for (index, weight) in weights.iter().enumerate() {
        match weight.as_f64() {
            Some(weight) if weight.is_finite() && weight.abs() <= f64::from(f32::MAX) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_morph",
                "error",
                format!("{path}[{index}]"),
                "morph weight must be finite and f32-compatible",
                "emit finite numeric weights",
                None,
                false,
            )),
        }
    }
}

fn validate_skin_binding(
    path: &str,
    value: Option<&Value>,
    geometry_id: Option<&str>,
    skinned_geometries: &BTreeSet<String>,
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
