use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{finite_vec3, validate_known_fields};
use crate::scene::recipe::validation::diagnostic;

const RAW_TRANSFORM_FIELDS: &[&str] = &["kind", "translation", "rotation", "scale"];
const TRS_TRANSFORM_FIELDS: &[&str] = &["kind", "translation", "rotation_degrees", "scale"];
const LOOK_AT_TRANSFORM_FIELDS: &[&str] = &["kind", "eye", "target", "up"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scene::recipe::validation::authoring) enum TransformUse {
    Node,
    Camera,
}

pub(in crate::scene::recipe::validation::authoring) fn validate_transform(
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

pub(in crate::scene::recipe::validation::authoring) fn validate_look_at_target(
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

pub(in crate::scene::recipe::validation::authoring) fn validate_ref(
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

pub(in crate::scene::recipe::validation::authoring) fn validate_tags(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(tags) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_tags",
            "error",
            path,
            "tags must be an array of strings",
            "use tags:[\"cad\",\"authored\"]",
            None,
            false,
        ));
        return;
    };
    for (index, tag) in tags.iter().enumerate() {
        match tag.as_str() {
            Some(tag) if !tag.trim().is_empty() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_tag",
                "error",
                format!("{path}[{index}]"),
                "tag must be a non-empty string",
                "use a stable tag string",
                None,
                false,
            )),
        }
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_u64(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    if value.as_u64().is_none() {
        diagnostics.push(diagnostic(
            "invalid_integer",
            "error",
            path,
            "field must be an unsigned integer",
            "use an integer value",
            None,
            false,
        ));
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_i16(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_i64() {
        Some(value) if (i64::from(i16::MIN)..=i64::from(i16::MAX)).contains(&value) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_integer",
            "error",
            path,
            "field must fit in i16",
            "use a small signed integer render group",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_non_negative(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(value) if value.is_finite() && value >= 0.0 => {}
        _ => diagnostics.push(diagnostic(
            "invalid_number",
            "error",
            path,
            "field must be finite and non-negative",
            "use a value greater than or equal to zero",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_optional_angle(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_f64() {
        Some(value) if value.is_finite() && (0.0..=180.0).contains(&value) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_angle",
            "error",
            path,
            "angle must be finite and in [0, 180]",
            "use degrees in the accepted range",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_vec3(
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

pub(in crate::scene::recipe::validation::authoring) fn validate_vec3_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.is_some() {
        validate_vec3(path, value, diagnostics);
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_quat(
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
