use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::common::{TransformUse, validate_transform};
use crate::scene::recipe::validation::diagnostic;

const CAMERA_FIELDS: &[&str] = &[
    "id",
    "kind",
    "fov_degrees",
    "lens",
    "framing",
    "active",
    "transform",
];
const FRAMING_FIELDS: &[&str] = &["mode", "preset", "fill", "margin_px", "target_region"];
const TARGET_REGION_FIELDS: &[&str] = &["bounds", "centroid"];
const TARGET_BOUNDS_FIELDS: &[&str] = &["min", "max"];

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
            Some("perspective" | "orthographic") => {}
            Some(kind) => diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("camera kind '{kind}' is not supported"),
                "use kind:\"perspective\" or kind:\"orthographic\"",
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
        if object.contains_key("fov_degrees") && object.contains_key("lens") {
            diagnostics.push(diagnostic(
                "invalid_camera_lens",
                "error",
                format!("{path}.lens"),
                "camera lens and fov_degrees are mutually exclusive",
                "use lens:\"standard\" for a named helper preset or fov_degrees for a raw escape hatch",
                None,
                false,
            ));
        }
        if let Some(lens) = object.get("lens") {
            match lens.as_str() {
                Some(name) if crate::PerspectiveCamera::from_lens_preset_name(name).is_some() => {}
                Some(name) => diagnostics.push(diagnostic(
                    "invalid_camera_lens",
                    "error",
                    format!("{path}.lens"),
                    format!("camera lens preset '{name}' is not supported"),
                    format!(
                        "use one of: {}",
                        crate::PerspectiveCamera::LENS_PRESET_NAMES.join(", ")
                    ),
                    None,
                    false,
                )),
                None => diagnostics.push(diagnostic(
                    "invalid_camera_lens",
                    "error",
                    format!("{path}.lens"),
                    "camera lens must be a string",
                    "use wide_angle, standard, portrait, or telephoto",
                    None,
                    false,
                )),
            }
        }
        validate_camera_framing(&path, object, diagnostics);
        let default_for_bounds_active = object
            .get("framing")
            .and_then(Value::as_object)
            .and_then(|framing| framing.get("mode"))
            .and_then(Value::as_str)
            == Some("default_for_bounds");
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
        } else if default_for_bounds_active {
            active_count += 1;
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

fn validate_camera_framing(
    path: &str,
    camera: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(framing) = camera.get("framing") else {
        return;
    };
    let framing_path = format!("{path}.framing");
    let Some(object) = framing.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            framing_path,
            "camera framing must be an object",
            "use framing:{preset:\"three_quarter_front_right\",fill:0.72}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(&framing_path, object, FRAMING_FIELDS, diagnostics);
    let mode = object
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("frame_bounds");
    if !matches!(
        mode,
        "frame_bounds" | "default_for_bounds" | "principal_face" | "target_region"
    ) {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            format!("{framing_path}.mode"),
            format!("camera framing mode '{mode}' is not supported"),
            "use frame_bounds, default_for_bounds, principal_face, or target_region",
            None,
            false,
        ));
    }
    if mode == "target_region" && !object.contains_key("target_region") {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            format!("{framing_path}.target_region"),
            "target_region framing requires target_region bounds and centroid",
            "emit framing.target_region:{bounds:{min,max},centroid}",
            None,
            false,
        ));
    }
    if object.contains_key("target_region") && mode != "target_region" {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            format!("{framing_path}.target_region"),
            "target_region is only accepted with framing.mode:\"target_region\"",
            "set framing.mode:\"target_region\" or remove target_region",
            None,
            false,
        ));
    }
    validate_target_region(
        &format!("{framing_path}.target_region"),
        object.get("target_region"),
        diagnostics,
    );
    if mode == "default_for_bounds" {
        for field in ["lens", "fov_degrees", "transform"] {
            if camera.contains_key(field) {
                diagnostics.push(diagnostic(
                    "invalid_camera_framing",
                    "error",
                    format!("{path}.{field}"),
                    "default_for_bounds framing uses Scene::add_perspective_camera_default_for and does not accept manual camera fields",
                    "remove lens/fov_degrees/transform or use framing.mode:\"frame_bounds\"",
                    None,
                    false,
                ));
            }
        }
        if camera.get("active").and_then(Value::as_bool) == Some(false) {
            diagnostics.push(diagnostic(
                "invalid_camera_framing",
                "error",
                format!("{path}.active"),
                "default_for_bounds creates the active camera",
                "omit active or set active:true",
                None,
                false,
            ));
        }
    }
    if let Some(preset) = object.get("preset") {
        match preset.as_str() {
            Some(name) if crate::FramingOptions::from_preset_name(name).is_some() => {}
            Some(name) => diagnostics.push(diagnostic(
                "invalid_camera_framing",
                "error",
                format!("{framing_path}.preset"),
                format!("camera framing preset '{name}' is not supported"),
                format!(
                    "use one of: {}",
                    crate::FramingOptions::PRESET_NAMES.join(", ")
                ),
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "invalid_camera_framing",
                "error",
                format!("{framing_path}.preset"),
                "camera framing preset must be a string",
                "use a named FramingOptions preset",
                None,
                false,
            )),
        }
    }
    if let Some(value) = object.get("fill") {
        match value.as_f64() {
            Some(value) if value.is_finite() && value > 0.0 => {}
            _ => diagnostics.push(diagnostic(
                "invalid_camera_framing",
                "error",
                format!("{framing_path}.fill"),
                "camera framing fill must be finite and positive",
                "use fill:0.72",
                None,
                false,
            )),
        }
    }
    if let Some(value) = object.get("margin_px") {
        match value.as_f64() {
            Some(value) if value.is_finite() && value >= 0.0 => {}
            _ => diagnostics.push(diagnostic(
                "invalid_camera_framing",
                "error",
                format!("{framing_path}.margin_px"),
                "camera framing margin_px must be finite and non-negative",
                "use margin_px:24.0",
                None,
                false,
            )),
        }
    }
}

fn validate_target_region(
    path: &str,
    target_region: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(target_region) = target_region else {
        return;
    };
    let Some(object) = target_region.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            path,
            "target_region must be an object",
            "emit target_region:{bounds:{min:[x,y,z],max:[x,y,z]},centroid:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, TARGET_REGION_FIELDS, diagnostics);
    validate_target_bounds(&format!("{path}.bounds"), object.get("bounds"), diagnostics);
    validate_vec3(
        &format!("{path}.centroid"),
        object.get("centroid"),
        "target_region centroid",
        diagnostics,
    );
}

fn validate_target_bounds(
    path: &str,
    bounds: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(bounds) = bounds else {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            path,
            "target_region bounds are required",
            "emit bounds:{min:[x,y,z],max:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = bounds.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            path,
            "target_region bounds must be an object",
            "emit bounds:{min:[x,y,z],max:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, TARGET_BOUNDS_FIELDS, diagnostics);
    let min = validate_vec3(
        &format!("{path}.min"),
        object.get("min"),
        "target_region bounds min",
        diagnostics,
    );
    let max = validate_vec3(
        &format!("{path}.max"),
        object.get("max"),
        "target_region bounds max",
        diagnostics,
    );
    if let (Some(min), Some(max)) = (min, max)
        && min.iter().zip(max).any(|(min, max)| *min > max)
    {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            path,
            "target_region bounds min must be <= max on every axis",
            "emit finite axis-aligned bounds around the target region",
            None,
            false,
        ));
    }
}

fn validate_vec3(
    path: &str,
    value: Option<&Value>,
    label: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<[f64; 3]> {
    let Some(values) = value.and_then(Value::as_array) else {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            path,
            format!("{label} must be a three-number array"),
            "emit finite [x,y,z] coordinates",
            None,
            false,
        ));
        return None;
    };
    if values.len() != 3 {
        diagnostics.push(diagnostic(
            "invalid_camera_framing",
            "error",
            path,
            format!("{label} must contain exactly three numbers"),
            "emit finite [x,y,z] coordinates",
            None,
            false,
        ));
        return None;
    }
    let mut parsed = [0.0; 3];
    for (index, value) in values.iter().enumerate() {
        match value.as_f64() {
            Some(number) if number.is_finite() => parsed[index] = number,
            _ => {
                diagnostics.push(diagnostic(
                    "invalid_camera_framing",
                    "error",
                    format!("{path}[{index}]"),
                    format!("{label} coordinates must be finite numbers"),
                    "emit finite [x,y,z] coordinates",
                    None,
                    false,
                ));
                return None;
            }
        }
    }
    Some(parsed)
}
