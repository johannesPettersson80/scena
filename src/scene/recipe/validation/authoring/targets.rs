use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::diagnostic;
use super::{finite_vec3, validate_known_fields, validate_required_id};

const NODE_FIELDS: &[&str] = &["id", "geometry", "material", "name", "transform"];
const CAMERA_FIELDS: &[&str] = &["id", "kind", "fov_degrees", "active", "transform"];
const RAW_TRANSFORM_FIELDS: &[&str] = &["kind", "translation", "rotation", "scale"];
const TRS_TRANSFORM_FIELDS: &[&str] = &["kind", "translation", "rotation_degrees", "scale"];
const LOOK_AT_TRANSFORM_FIELDS: &[&str] = &["kind", "eye", "target", "up"];

pub(super) fn has_authored_renderable_nodes(object: &Map<String, Value>) -> bool {
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

pub(super) fn validate_nodes(
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

pub(super) fn validate_cameras(
    value: Option<&Value>,
    nodes: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(cameras) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_cameras",
            "error",
            "$.cameras",
            "cameras must be an array",
            "emit cameras:[{id,kind,active,transform}]",
            None,
            false,
        ));
        return;
    };
    let mut active_count = 0usize;
    for (index, camera) in cameras.iter().enumerate() {
        let path = format!("$.cameras[{index}]");
        let Some(object) = camera.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_camera",
                "error",
                &path,
                "camera entry must be an object",
                "emit camera entries as {id, kind, transform}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, CAMERA_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        match object.get("kind").and_then(Value::as_str) {
            Some("perspective") => {}
            Some(kind) => diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("camera kind '{kind}' is not implemented in this slice"),
                "use kind:\"perspective\"",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_camera_kind",
                "error",
                format!("{path}.kind"),
                "camera must include a kind string",
                "use kind:\"perspective\"",
                None,
                false,
            )),
        }
        if let Some(fov) = object.get("fov_degrees") {
            match fov.as_f64() {
                Some(value) if value.is_finite() && value > 0.0 && value < 180.0 => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_fov",
                    "error",
                    format!("{path}.fov_degrees"),
                    "perspective camera fov_degrees must be finite and in (0, 180)",
                    "use a field of view such as 40.0",
                    None,
                    false,
                )),
            }
        }
        if let Some(active) = object.get("active") {
            if active.as_bool() == Some(true) {
                active_count += 1;
            } else if !active.is_boolean() {
                diagnostics.push(diagnostic(
                    "invalid_active",
                    "error",
                    format!("{path}.active"),
                    "camera active must be a boolean",
                    "set active:true on at most one camera",
                    None,
                    false,
                ));
            }
        }
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{path}.transform"),
                transform,
                TransformUse::Camera,
                nodes,
                diagnostics,
            );
        }
    }
    if active_count > 1 {
        diagnostics.push(diagnostic(
            "duplicate_active_camera",
            "error",
            "$.cameras",
            "at most one authored camera may be active",
            "mark exactly one camera active or omit active to keep the default camera",
            None,
            false,
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformUse {
    Node,
    Camera,
}

fn validate_transform(
    path: &str,
    value: &Value,
    usage: TransformUse,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_transform",
            "error",
            path,
            "transform must be an object with a kind",
            "emit a raw, trs, or camera look_at transform",
            None,
            false,
        ));
        return;
    };
    match object.get("kind").and_then(Value::as_str) {
        Some("raw") => {
            validate_known_fields(path, object, RAW_TRANSFORM_FIELDS, diagnostics);
            validate_vec3_optional(
                &format!("{path}.translation"),
                object.get("translation"),
                diagnostics,
            );
            validate_vec3_optional(&format!("{path}.scale"), object.get("scale"), diagnostics);
            validate_quat(
                &format!("{path}.rotation"),
                object.get("rotation"),
                diagnostics,
            );
        }
        Some("trs") => {
            validate_known_fields(path, object, TRS_TRANSFORM_FIELDS, diagnostics);
            validate_vec3_optional(
                &format!("{path}.translation"),
                object.get("translation"),
                diagnostics,
            );
            validate_vec3_optional(
                &format!("{path}.rotation_degrees"),
                object.get("rotation_degrees"),
                diagnostics,
            );
            validate_vec3_optional(&format!("{path}.scale"), object.get("scale"), diagnostics);
        }
        Some("look_at") if usage == TransformUse::Camera => {
            validate_known_fields(path, object, LOOK_AT_TRANSFORM_FIELDS, diagnostics);
            validate_vec3(&format!("{path}.eye"), object.get("eye"), diagnostics);
            validate_look_at_target(
                &format!("{path}.target"),
                object.get("target"),
                node_ids,
                diagnostics,
            );
            validate_vec3_optional(&format!("{path}.up"), object.get("up"), diagnostics);
        }
        Some("look_at") => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            format!("{path}.kind"),
            "look_at transforms are only implemented for cameras in this slice",
            "use raw or trs for authored nodes until the placement slice lands",
            None,
            false,
        )),
        Some(kind) => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            format!("{path}.kind"),
            format!("transform kind '{kind}' is not implemented in this slice"),
            "use raw, trs, or a camera look_at transform",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_transform_kind",
            "error",
            format!("{path}.kind"),
            "transform must include a kind string",
            "use kind:\"trs\" or kind:\"raw\"",
            None,
            false,
        )),
    }
}

fn validate_look_at_target(
    path: &str,
    value: Option<&Value>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value {
        Some(Value::String(target)) if node_ids.contains(target) => {}
        Some(Value::String(target)) => diagnostics.push(diagnostic(
            "unknown_node_ref",
            "error",
            path,
            format!("look_at target references unknown node '{target}'"),
            "target an authored node id or provide a [x,y,z] position",
            None,
            false,
        )),
        Some(value) if finite_vec3(value).is_some() => {}
        Some(_) => diagnostics.push(diagnostic(
            "invalid_look_at_target",
            "error",
            path,
            "look_at target must be an authored node id or a finite [x,y,z] position",
            "target an authored node id or provide a [x,y,z] position",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_look_at_target",
            "error",
            path,
            "look_at transform requires a target",
            "target an authored node id or provide a [x,y,z] position",
            None,
            false,
        )),
    }
}

fn validate_ref(
    path: &str,
    value: Option<&Value>,
    ids: &BTreeSet<String>,
    kind: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(value) if ids.contains(value) => {}
        Some(value) => diagnostics.push(diagnostic(
            format!("unknown_{kind}_ref"),
            "error",
            path,
            format!("{kind} reference '{value}' does not name a declared {kind}"),
            format!("declare a {kind} with this id before referencing it"),
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            format!("missing_{kind}_ref"),
            "error",
            path,
            format!("node must include a {kind} reference"),
            format!("set {kind} to a declared {kind} id"),
            None,
            false,
        )),
    }
}

fn validate_vec3(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.and_then(finite_vec3).is_none() {
        diagnostics.push(diagnostic(
            "invalid_vector",
            "error",
            path,
            "field must be a finite [x,y,z] array",
            "use three finite numbers",
            None,
            false,
        ));
    }
}

fn validate_vec3_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.is_some() {
        validate_vec3(path, value, diagnostics);
    }
}

fn validate_quat(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "missing_rotation",
            "error",
            path,
            "raw transform rotation must be a finite non-zero [x,y,z,w] quaternion",
            "use [0,0,0,1] for identity",
            None,
            false,
        ));
        return;
    };
    let Some(array) = value.as_array().filter(|array| array.len() == 4) else {
        diagnostics.push(diagnostic(
            "invalid_rotation",
            "error",
            path,
            "raw transform rotation must be a finite non-zero [x,y,z,w] quaternion",
            "use [0,0,0,1] for identity",
            None,
            false,
        ));
        return;
    };
    let values = array
        .iter()
        .filter_map(Value::as_f64)
        .map(|value| value as f32)
        .collect::<Vec<_>>();
    let length_sq = values.iter().map(|value| value * value).sum::<f32>();
    if values.len() != 4 || !length_sq.is_finite() || length_sq <= f32::EPSILON {
        diagnostics.push(diagnostic(
            "invalid_rotation",
            "error",
            path,
            "raw transform rotation must be a finite non-zero [x,y,z,w] quaternion",
            "use [0,0,0,1] for identity",
            None,
            false,
        ));
    }
}
