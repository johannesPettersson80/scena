use std::collections::BTreeSet;

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
];

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

pub(in crate::scene::recipe::validation::authoring) fn validate_nodes(
    value: Option<&Value>,
    geometries: &BTreeSet<String>,
    materials: &BTreeSet<String>,
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
        validate_ref(
            &format!("{path}.geometry"),
            object.get("geometry"),
            geometries,
            "geometry",
            diagnostics,
        );
        validate_ref(
            &format!("{path}.material"),
            object.get("material"),
            materials,
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
                &BTreeSet::new(),
                diagnostics,
            );
        }
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
