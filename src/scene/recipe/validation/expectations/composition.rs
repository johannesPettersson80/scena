use serde_json::Value;

use super::validate_target;
use crate::scene::recipe::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

pub(super) const GROUNDED_FIELDS: &[&str] = &["id", "target", "plane_y", "tolerance"];
pub(super) const HELPER_OCCLUDED_FIELDS: &[&str] =
    &["id", "helper", "occluder", "tolerance_pixels"];
pub(super) const OCCLUSION_FIELDS: &[&str] = &["id", "front", "back", "tolerance_pixels"];
const BACKEND_FIELDS: &[&str] = &["backend", "gpu_device"];
const CLIPPING_FIELDS: &[&str] = &[
    "active_clipping_planes",
    "section_box_active",
    "section_box_inverted",
];
pub(super) const STATE_FIELDS: &[&str] = &["id", "import", "active_material_variant"];
pub(super) const TRANSFORM_FIELDS: &[&str] = &[
    "id",
    "target",
    "translation",
    "scale",
    "rotation_degrees",
    "translation_tolerance",
    "scale_tolerance",
    "rotation_tolerance_degrees",
];
pub(super) const SEPARATION_FIELDS: &[&str] = &["id", "a", "b", "min_gap", "tolerance"];
const BACKEND_VALUES: &[&str] = &[
    "headless",
    "headless_gpu",
    "surface_descriptor",
    "native_surface",
    "web_gpu",
    "web_gl2",
];

pub(super) fn validate_grounded_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.target"),
        object.get("target"),
        false,
        diagnostics,
    );
    if let Some(value) = object.get("plane_y") {
        match value.as_f64() {
            Some(value) if value.is_finite() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("{path}.plane_y"),
                "grounded expectation plane_y must be finite",
                "use a finite floor height such as 0.0",
                None,
                false,
            )),
        }
    }
    if let Some(value) = object.get("tolerance") {
        match value.as_f64() {
            Some(value) if value.is_finite() && value >= 0.0 => {}
            _ => diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("{path}.tolerance"),
                "grounded expectation tolerance must be finite and non-negative",
                "use a small world-space tolerance such as 0.01",
                None,
                false,
            )),
        }
    }
}

pub(super) fn validate_helper_occluded_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.helper"),
        object.get("helper"),
        false,
        diagnostics,
    );
    validate_target(
        &format!("{path}.occluder"),
        object.get("occluder"),
        false,
        diagnostics,
    );
    if let Some(value) = object.get("tolerance_pixels")
        && value
            .as_u64()
            .is_none_or(|value| value > u64::from(u32::MAX))
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.tolerance_pixels"),
            "helper occlusion tolerance_pixels must be a non-negative integer",
            "use 0 for a hard no-overdraw check or a small integer for antialiasing tolerance",
            None,
            false,
        ));
    }
}

pub(super) fn validate_occlusion_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.front"),
        object.get("front"),
        false,
        diagnostics,
    );
    validate_target(
        &format!("{path}.back"),
        object.get("back"),
        false,
        diagnostics,
    );
    if let Some(value) = object.get("tolerance_pixels")
        && value
            .as_u64()
            .is_none_or(|value| value > u64::from(u32::MAX))
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.tolerance_pixels"),
            "occlusion tolerance_pixels must be a non-negative integer",
            "use 0 for a hard depth-order check or a small integer for antialiasing tolerance",
            None,
            false,
        ));
    }
}

pub(super) fn validate_backend_expectation(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_backend",
            "expect_backend must be an object",
            "emit expect_backend:{backend?,gpu_device?}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !BACKEND_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("$.expect.expect_backend.{key}"),
                format!("field '{key}' is not accepted here"),
                "remove the field or use backend/gpu_device",
                None,
                false,
            ));
        }
    }
    if let Some(backend) = object.get("backend") {
        match backend.as_str() {
            Some(value) if BACKEND_VALUES.contains(&value) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                "$.expect.expect_backend.backend",
                "backend must be one of headless, headless_gpu, surface_descriptor, native_surface, web_gpu, or web_gl2",
                "set backend to the renderer backend the recipe must use",
                None,
                false,
            )),
        }
    }
    if let Some(gpu_device) = object.get("gpu_device")
        && !gpu_device.is_boolean()
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_backend.gpu_device",
            "gpu_device must be a boolean",
            "set gpu_device to true when GPU execution is required",
            None,
            false,
        ));
    }
}

pub(super) fn validate_clipping_expectation(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_clipping",
            "expect_clipping must be an object",
            "emit expect_clipping:{active_clipping_planes?,section_box_active?,section_box_inverted?}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !CLIPPING_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("$.expect.expect_clipping.{key}"),
                format!("field '{key}' is not accepted here"),
                "remove the field or use active_clipping_planes/section_box_active/section_box_inverted",
                None,
                false,
            ));
        }
    }
    if let Some(value) = object.get("active_clipping_planes")
        && value
            .as_u64()
            .is_none_or(|value| value > u64::from(u32::MAX))
    {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            "$.expect.expect_clipping.active_clipping_planes",
            "active_clipping_planes must be a non-negative integer",
            "set the exact active user clipping-plane count expected after recipe build",
            None,
            false,
        ));
    }
    for field in ["section_box_active", "section_box_inverted"] {
        if let Some(value) = object.get(field)
            && !value.is_boolean()
        {
            diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("$.expect.expect_clipping.{field}"),
                format!("{field} must be a boolean"),
                "set true or false",
                None,
                false,
            ));
        }
    }
}

pub(super) fn validate_state_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match object.get("import").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            format!("{path}.import"),
            "state expectation import must be a non-empty string",
            "set import to an id from imports[]",
            None,
            false,
        )),
    }
    if let Some(value) = object.get("active_material_variant") {
        match value.as_str() {
            Some(value) if !value.trim().is_empty() => {}
            None if value.is_null() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_expect",
                "error",
                format!("{path}.active_material_variant"),
                "active_material_variant must be a non-empty string or null",
                "omit or set null for the default variant, or set a declared material variant name",
                None,
                false,
            )),
        }
    }
}

pub(super) fn validate_transform_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(
        &format!("{path}.target"),
        object.get("target"),
        false,
        diagnostics,
    );
    let mut has_component = false;
    for field in ["translation", "scale", "rotation_degrees"] {
        if object.contains_key(field) {
            has_component = true;
        }
        validate_vec3_optional(&format!("{path}.{field}"), object.get(field), diagnostics);
    }
    if !has_component {
        diagnostics.push(diagnostic(
            "invalid_expect",
            "error",
            path,
            "transform expectation must declare at least one of translation, scale, or rotation_degrees",
            "add a world-space translation, scale, or intrinsic X/Y/Z rotation_degrees expectation",
            None,
            false,
        ));
    }
    for field in [
        "translation_tolerance",
        "scale_tolerance",
        "rotation_tolerance_degrees",
    ] {
        if let Some(value) = object.get(field) {
            match value.as_f64() {
                Some(value) if value.is_finite() && value >= 0.0 => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_expect",
                    "error",
                    format!("{path}.{field}"),
                    format!("{field} must be finite and non-negative"),
                    "omit the tolerance or use a small non-negative numeric value",
                    None,
                    false,
                )),
            }
        }
    }
}

pub(super) fn validate_separation_expectation(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    validate_target(&format!("{path}.a"), object.get("a"), false, diagnostics);
    validate_target(&format!("{path}.b"), object.get("b"), false, diagnostics);
    for field in ["min_gap", "tolerance"] {
        if let Some(value) = object.get(field) {
            match value.as_f64() {
                Some(value) if value.is_finite() && value >= 0.0 => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_expect",
                    "error",
                    format!("{path}.{field}"),
                    format!("{field} must be finite and non-negative"),
                    "omit the field or use a non-negative world-space distance in meters",
                    None,
                    false,
                )),
            }
        }
    }
}

fn validate_vec3_optional(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(values) = value.as_array() else {
        push_vec3_error(path, diagnostics);
        return;
    };
    if values.len() != 3
        || !values
            .iter()
            .all(|value| value.as_f64().is_some_and(f64::is_finite))
    {
        push_vec3_error(path, diagnostics);
    }
}

fn push_vec3_error(path: &str, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    diagnostics.push(diagnostic(
        "invalid_expect",
        "error",
        path,
        "transform expectation vector must contain exactly three finite numbers",
        "emit [x,y,z] with finite numeric values",
        None,
        false,
    ));
}
