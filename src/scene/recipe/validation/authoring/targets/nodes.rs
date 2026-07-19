use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::common::{
    TransformUse, validate_optional_i16, validate_optional_u64, validate_ref, validate_tags,
    validate_transform,
};
use crate::scene::recipe::validation::diagnostic;

mod lod;
mod skin;

use lod::validate_lods;
use skin::{SkinJointIndexLimit, validate_skin_binding};

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
    "lods",
    "morph_weights",
    "skin_binding",
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

pub(in crate::scene::recipe::validation::authoring) struct NodeValidationResources<'a> {
    pub(in crate::scene::recipe::validation::authoring) geometries: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) materials: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) morph_target_counts:
        &'a BTreeMap<String, usize>,
    pub(in crate::scene::recipe::validation::authoring) skinned_geometries: &'a BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) skin_max_joint_indices:
        &'a BTreeMap<String, SkinJointIndexLimit>,
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
        let material_id = object.get("material").and_then(Value::as_str);
        match (object.get("geometry"), object.get("material")) {
            (Some(geometry), Some(material)) => {
                validate_ref(
                    &format!("{path}.geometry"),
                    Some(geometry),
                    resources.geometries,
                    "geometry",
                    diagnostics,
                );
                validate_ref(
                    &format!("{path}.material"),
                    Some(material),
                    resources.materials,
                    "material",
                    diagnostics,
                );
            }
            (None, None) => {}
            _ => diagnostics.push(diagnostic(
                "incomplete_node_renderable",
                "error",
                &path,
                "node geometry and material must be provided together, or both omitted for a group node",
                "add both geometry and material, or omit both",
                None,
                false,
            )),
        }
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
        validate_lods(
            &format!("{path}.lods"),
            object.get("lods"),
            resources.geometries,
            diagnostics,
        );
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
            resources.skin_max_joint_indices,
            resources.all_node_ids,
            diagnostics,
        );
        if geometry_id.is_none()
            && (!object
                .get("lods")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
                || object.get("morph_weights").is_some()
                || object.get("skin_binding").is_some())
        {
            diagnostics.push(diagnostic(
                "group_node_deformation",
                "error",
                &path,
                "group nodes cannot declare LOD, morph, or skin data",
                "attach deformation data to a node with geometry and material",
                None,
                false,
            ));
        }
        let _ = material_id;
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
