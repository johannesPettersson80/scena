use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

use super::super::super::validate_known_fields;
use super::super::common::validate_ref;

const LOD_FIELDS: &[&str] = &["geometry", "max_screen_fraction"];

pub(in crate::scene::recipe::validation::authoring) fn validate_lods(
    path: &str,
    value: Option<&Value>,
    geometries: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(levels) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_lods",
            "error",
            path,
            "lods must be an array",
            "emit lods:[{geometry,max_screen_fraction}] or omit lods",
            None,
            false,
        ));
        return;
    };
    for (index, level) in levels.iter().enumerate() {
        let level_path = format!("{path}[{index}]");
        let Some(object) = level.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_lod",
                "error",
                &level_path,
                "LOD entry must be an object",
                "emit {geometry,max_screen_fraction}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&level_path, object, LOD_FIELDS, diagnostics);
        validate_ref(
            &format!("{level_path}.geometry"),
            object.get("geometry"),
            geometries,
            "geometry",
            diagnostics,
        );
        let Some(threshold) = object.get("max_screen_fraction").and_then(Value::as_f64) else {
            diagnostics.push(diagnostic(
                "invalid_lod_threshold",
                "error",
                format!("{level_path}.max_screen_fraction"),
                "LOD max_screen_fraction must be a finite number in (0, 1]",
                "use a fraction such as 0.15 for distant or small-on-screen geometry",
                None,
                false,
            ));
            continue;
        };
        if !threshold.is_finite() || threshold <= 0.0 || threshold > 1.0 {
            diagnostics.push(diagnostic(
                "invalid_lod_threshold",
                "error",
                format!("{level_path}.max_screen_fraction"),
                "LOD max_screen_fraction must be in (0, 1]",
                "use a fraction such as 0.15 for distant or small-on-screen geometry",
                None,
                false,
            ));
        }
    }
}
