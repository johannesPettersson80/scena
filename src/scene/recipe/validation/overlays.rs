use std::collections::BTreeSet;

use serde_json::Value;

use super::diagnostic;
use crate::scene::recipe::types::{
    SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1, SceneRecipeDiagnosticV1,
    SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1, SceneRecipeMeasurementV1,
    SceneRecipeSectionBoxV1,
};

pub(super) fn validate_section_box(
    section_box: Option<&Value>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(section_box) = section_box else {
        return;
    };
    match serde_json::from_value::<SceneRecipeSectionBoxV1>(section_box.clone()) {
        Ok(section_box) => {
            validate_import_reference(
                "$.section_box.import",
                &section_box.import,
                import_ids,
                diagnostics,
            );
            if !section_box.margin.is_finite() || section_box.margin < 0.0 {
                diagnostics.push(diagnostic(
                    "invalid_section_box",
                    "error",
                    "$.section_box.margin",
                    "section_box margin must be finite and non-negative",
                    "use a finite margin such as 0.01, or omit the field",
                    None,
                    false,
                ));
            }
        }
        Err(error) => diagnostics.push(diagnostic(
            "invalid_section_box",
            "error",
            "$.section_box",
            format!("section_box must match the supported recipe shape: {error}"),
            "emit section_box:{import,margin?,inverted?,helper_wireframe?}",
            None,
            false,
        )),
    }
}

pub(super) fn validate_measurements(
    measurements: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(measurements) = measurements else {
        return;
    };
    let Some(entries) = measurements.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_measurements",
            "error",
            "$.measurements",
            "measurements must be an array",
            "emit measurements as an array of distance overlay objects",
            None,
            false,
        ));
        return;
    };
    let mut ids = BTreeSet::new();
    for (index, value) in entries.iter().enumerate() {
        let path = format!("$.measurements[{index}]");
        match serde_json::from_value::<SceneRecipeMeasurementV1>(value.clone()) {
            Ok(measurement) => {
                validate_non_empty_id(
                    &format!("{path}.id"),
                    &measurement.id,
                    "measurement",
                    &mut ids,
                    diagnostics,
                );
                if measurement.kind != "distance" {
                    diagnostics.push(diagnostic(
                        "unsupported_measurement_kind",
                        "error",
                        format!("{path}.kind"),
                        format!(
                            "measurement kind '{}' is not supported in scene_recipe.v1",
                            measurement.kind
                        ),
                        "use kind:'distance' for recipe-authored measurement overlays",
                        None,
                        false,
                    ));
                }
                validate_vec3(&format!("{path}.start"), measurement.start, diagnostics);
                validate_vec3(&format!("{path}.end"), measurement.end, diagnostics);
                if measurement.start == measurement.end {
                    diagnostics.push(diagnostic(
                        "invalid_measurement",
                        "error",
                        format!("{path}.end"),
                        "distance measurement start and end must differ",
                        "provide two distinct world-space points",
                        None,
                        false,
                    ));
                }
            }
            Err(error) => diagnostics.push(diagnostic(
                "invalid_measurement",
                "error",
                path,
                format!("measurement must match the supported recipe shape: {error}"),
                "emit {id,kind:'distance',start,end,label?,unit?,precision?}",
                None,
                false,
            )),
        }
    }
}

pub(super) fn validate_callouts(
    callouts: Option<&Value>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(callouts) = callouts else {
        return;
    };
    let Some(entries) = callouts.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_callouts",
            "error",
            "$.callouts",
            "callouts must be an array",
            "emit callouts as an array of world or import-root label objects",
            None,
            false,
        ));
        return;
    };
    let mut ids = BTreeSet::new();
    for (index, value) in entries.iter().enumerate() {
        let path = format!("$.callouts[{index}]");
        match serde_json::from_value::<SceneRecipeCalloutV1>(value.clone()) {
            Ok(callout) => {
                validate_non_empty_id(
                    &format!("{path}.id"),
                    &callout.id,
                    "callout",
                    &mut ids,
                    diagnostics,
                );
                if callout.text.trim().is_empty() {
                    diagnostics.push(diagnostic(
                        "invalid_callout",
                        "error",
                        format!("{path}.text"),
                        "callout text must not be empty",
                        "provide visible label text",
                        None,
                        false,
                    ));
                }
                validate_vec3(
                    &format!("{path}.label_offset"),
                    callout.label_offset,
                    diagnostics,
                );
                match callout.target {
                    SceneRecipeCalloutTargetV1::ImportRoot {
                        import,
                        local_offset,
                    } => {
                        validate_import_reference(
                            &format!("{path}.target.import"),
                            &import,
                            import_ids,
                            diagnostics,
                        );
                        validate_vec3(
                            &format!("{path}.target.local_offset"),
                            local_offset,
                            diagnostics,
                        );
                    }
                    SceneRecipeCalloutTargetV1::World { position } => {
                        validate_vec3(&format!("{path}.target.position"), position, diagnostics);
                    }
                }
            }
            Err(error) => diagnostics.push(diagnostic(
                "invalid_callout",
                "error",
                path,
                format!("callout must match the supported recipe shape: {error}"),
                "emit {id,text,target,label_offset?} with target kind world or import_root",
                None,
                false,
            )),
        }
    }
}

pub(super) fn validate_exploded_view(
    exploded_view: Option<&Value>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(exploded_view) = exploded_view else {
        return;
    };
    match serde_json::from_value::<SceneRecipeExplodedViewV1>(exploded_view.clone()) {
        Ok(exploded_view) => {
            validate_import_reference(
                "$.exploded_view.import",
                &exploded_view.import,
                import_ids,
                diagnostics,
            );
            if !exploded_view.factor.is_finite() || !(0.0..=1.0).contains(&exploded_view.factor) {
                diagnostics.push(diagnostic(
                    "invalid_exploded_view",
                    "error",
                    "$.exploded_view.factor",
                    "exploded_view factor must be finite and between 0 and 1",
                    "use a presentation factor such as 0.0, 0.5, or 1.0",
                    None,
                    false,
                ));
            }
            if !exploded_view.distance.is_finite() || exploded_view.distance < 0.0 {
                diagnostics.push(diagnostic(
                    "invalid_exploded_view",
                    "error",
                    "$.exploded_view.distance",
                    "exploded_view distance must be finite and non-negative",
                    "use a non-negative presentation offset distance",
                    None,
                    false,
                ));
            }
            if matches!(exploded_view.mode, SceneRecipeExplodedViewModeV1::Axis) {
                match exploded_view.axis {
                    Some(axis)
                        if axis.iter().all(|value| value.is_finite()) && axis != [0.0; 3] => {}
                    _ => diagnostics.push(diagnostic(
                        "invalid_exploded_view",
                        "error",
                        "$.exploded_view.axis",
                        "axis exploded views require a finite non-zero axis",
                        "provide axis such as [1,0,0]",
                        None,
                        false,
                    )),
                }
            }
        }
        Err(error) => diagnostics.push(diagnostic(
            "invalid_exploded_view",
            "error",
            "$.exploded_view",
            format!("exploded_view must match the supported recipe shape: {error}"),
            "emit exploded_view:{import,mode?,axis?,factor?,distance?}",
            None,
            false,
        )),
    }
}

fn validate_import_reference(
    path: &str,
    import: &str,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if import.trim().is_empty() {
        diagnostics.push(diagnostic(
            "invalid_import_reference",
            "error",
            path,
            "recipe overlay import reference must not be empty",
            "reference one of the ids in imports[]",
            None,
            false,
        ));
    } else if !import_ids.contains(import) {
        diagnostics.push(diagnostic(
            "unknown_import_reference",
            "error",
            path,
            format!("recipe overlay references unknown import '{import}'"),
            "reference one of the ids in imports[]",
            None,
            false,
        ));
    }
}

fn validate_non_empty_id(
    path: &str,
    id: &str,
    label: &str,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if id.trim().is_empty() {
        diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            path,
            format!("{label} id must not be empty"),
            "use a stable caller-owned id",
            None,
            false,
        ));
    } else if !ids.insert(id.to_owned()) {
        diagnostics.push(diagnostic(
            "duplicate_id",
            "error",
            path,
            format!("{label} id '{id}' is used more than once"),
            "make overlay ids unique",
            None,
            false,
        ));
    }
}

fn validate_vec3(path: &str, value: [f32; 3], diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    if value.iter().any(|component| !component.is_finite()) {
        diagnostics.push(diagnostic(
            "invalid_vec3",
            "error",
            path,
            "vector components must be finite",
            "emit three finite numbers",
            None,
            false,
        ));
    }
}
