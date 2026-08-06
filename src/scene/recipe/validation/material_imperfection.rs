use serde_json::Value;

use crate::assets::MaterialImperfectionProfileV1;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::diagnostic;

pub(super) fn validate_material_imperfection(
    path: &str,
    value: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_material_imperfection",
            "error",
            path,
            "material imperfection must be an object",
            "use imperfection:{profile:\"dust\",strength:0.30,physical_scale_m:0.003,seed:1}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !["profile", "strength", "physical_scale_m", "seed"].contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("material imperfection field '{key}' is not part of scena.scene_recipe.v1"),
                "use profile, strength, physical_scale_m, and seed",
                None,
                false,
            ));
        }
    }
    match object.get("profile").and_then(Value::as_str) {
        Some(profile) if MaterialImperfectionProfileV1::from_name(profile).is_some() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_material_imperfection_profile",
            "error",
            format!("{path}.profile"),
            "material imperfection profile must be one of the fixed supported profiles",
            format!(
                "use one of: {}",
                MaterialImperfectionProfileV1::NAMES.join(", ")
            ),
            None,
            false,
        )),
    }
    match object.get("strength").and_then(Value::as_f64) {
        Some(strength) if strength.is_finite() && (0.0..=1.0).contains(&strength) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_material_imperfection_strength",
            "error",
            format!("{path}.strength"),
            "material imperfection strength must be finite from 0 through 1",
            "start near dust 0.30, smudge 0.40, fine_scratches 0.30, or oil_film 0.65; glossy oil may use less",
            None,
            false,
        )),
    }
    match object.get("physical_scale_m").and_then(Value::as_f64) {
        Some(scale) if scale.is_finite() && scale > 0.0 => {}
        _ => diagnostics.push(diagnostic(
            "invalid_material_imperfection_scale",
            "error",
            format!("{path}.physical_scale_m"),
            "material imperfection physical_scale_m must be finite and positive",
            "use the physical size of one mark in metres",
            None,
            false,
        )),
    }
    if object.get("seed").and_then(Value::as_u64).is_none() {
        diagnostics.push(diagnostic(
            "invalid_material_imperfection_seed",
            "error",
            format!("{path}.seed"),
            "material imperfection seed must be an unsigned integer",
            "use a deterministic unsigned integer seed",
            None,
            false,
        ));
    }
}
