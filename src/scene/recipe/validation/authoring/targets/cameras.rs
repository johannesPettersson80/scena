use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::common::{TransformUse, validate_transform};
use crate::scene::recipe::validation::diagnostic;

const CAMERA_FIELDS: &[&str] = &["id", "kind", "fov_degrees", "active", "transform"];

pub(in crate::scene::recipe::validation::authoring) fn validate_cameras(
    value: Option<&Value>,
    nodes: &BTreeSet<String>,
    imports: &BTreeSet<String>,
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
                imports,
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
