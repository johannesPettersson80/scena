mod states;

use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::scene::recipe::types::{
    SceneRecipeAnchorSourceV1, SceneRecipeAnchorV1, SceneRecipeBoundsSourceV1, SceneRecipeBoundsV1,
    SceneRecipeConnectorSourceV1, SceneRecipeConnectorV1, SceneRecipeDiagnosticV1,
    SceneRecipeNamedStateV1, SceneRecipeSpatialTargetV1, SceneRecipeTransformV1,
};

use super::diagnostic;
use states::validate_states;

pub(super) fn validate_spatial_state_sections(
    object: &Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let node_ids = target_node_ids(object);
    let import_ids = ids_from_array(object.get("imports"));
    let animated_nodes = animated_node_ids(object.get("animations"));

    if let Some(anchors) =
        decode_section::<SceneRecipeAnchorV1>("anchors", object.get("anchors"), diagnostics)
    {
        validate_anchors(&anchors, &node_ids, &import_ids, diagnostics);
    }
    if let Some(connectors) = decode_section::<SceneRecipeConnectorV1>(
        "connectors",
        object.get("connectors"),
        diagnostics,
    ) {
        validate_connectors(&connectors, &node_ids, &import_ids, diagnostics);
    }
    if let Some(bounds) =
        decode_section::<SceneRecipeBoundsV1>("bounds", object.get("bounds"), diagnostics)
    {
        validate_bounds(&bounds, &node_ids, &import_ids, diagnostics);
    }
    if let Some(states) = decode_section::<SceneRecipeNamedStateV1>(
        "named_states",
        object.get("named_states"),
        diagnostics,
    ) {
        validate_states(
            &states,
            &node_ids,
            &import_ids,
            &animated_nodes,
            diagnostics,
        );
    }
}

fn decode_section<T: DeserializeOwned>(
    name: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Vec<T>> {
    let value = value?;
    if !value.is_array() {
        diagnostics.push(error(
            "invalid_spatial_section",
            format!("$.{name}"),
            format!("{name} must be an array"),
            format!("emit {name} as an array of typed entries"),
        ));
        return None;
    }
    match serde_json::from_value::<Vec<T>>(value.clone()) {
        Ok(value) => Some(value),
        Err(parse_error) => {
            diagnostics.push(error(
                "invalid_spatial_section",
                format!("$.{name}"),
                format!("{name} does not match its v1 contract: {parse_error}"),
                "use `scena schema get scena.scene_recipe.v1` for the accepted fields",
            ));
            None
        }
    }
}

fn validate_anchors(
    anchors: &[SceneRecipeAnchorV1],
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, anchor) in anchors.iter().enumerate() {
        let path = format!("$.anchors[{index}]");
        match &anchor.source {
            SceneRecipeAnchorSourceV1::Authored { target, transform } => {
                validate_target(
                    &format!("{path}.source.target"),
                    target,
                    node_ids,
                    import_ids,
                    diagnostics,
                );
                if let Some(transform) = transform {
                    validate_local_transform(
                        &format!("{path}.source.transform"),
                        transform,
                        diagnostics,
                    );
                }
            }
            SceneRecipeAnchorSourceV1::Import { import, name } => {
                validate_import_alias(&path, import, name, import_ids, diagnostics);
            }
        }
        validate_non_empty_strings(&format!("{path}.tags"), &anchor.tags, diagnostics);
        if anchor.label.as_deref().is_some_and(str::is_empty) {
            diagnostics.push(error(
                "invalid_anchor",
                format!("{path}.label"),
                "anchor label must not be empty",
                "omit label or use a non-empty label",
            ));
        }
    }
}

fn validate_connectors(
    connectors: &[SceneRecipeConnectorV1],
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let connector_ids = connectors
        .iter()
        .map(|connector| connector.id.as_str())
        .collect::<BTreeSet<_>>();
    for (index, connector) in connectors.iter().enumerate() {
        let path = format!("$.connectors[{index}]");
        match &connector.source {
            SceneRecipeConnectorSourceV1::Authored { target, transform } => {
                validate_target(
                    &format!("{path}.source.target"),
                    target,
                    node_ids,
                    import_ids,
                    diagnostics,
                );
                if let Some(transform) = transform {
                    validate_local_transform(
                        &format!("{path}.source.transform"),
                        transform,
                        diagnostics,
                    );
                }
            }
            SceneRecipeConnectorSourceV1::Import { import, name } => {
                validate_import_alias(&path, import, name, import_ids, diagnostics);
                if connector.connector_kind.is_some()
                    || !connector.allowed_mates.is_empty()
                    || !connector.tags.is_empty()
                    || connector.snap_tolerance.is_some()
                    || connector.clearance_hint.is_some()
                    || connector.roll_policy.is_some()
                    || connector.polarity.is_some()
                {
                    diagnostics.push(error(
                        "import_connector_override",
                        &path,
                        "import connector aliases preserve imported compatibility metadata",
                        "remove authored connector metadata from the import alias",
                    ));
                }
            }
        }
        validate_non_empty_optional(
            &format!("{path}.connector_kind"),
            connector.connector_kind.as_deref(),
            diagnostics,
        );
        validate_non_empty_strings(
            &format!("{path}.allowed_mates"),
            &connector.allowed_mates,
            diagnostics,
        );
        validate_non_empty_strings(&format!("{path}.tags"), &connector.tags, diagnostics);
        validate_non_negative(
            &format!("{path}.snap_tolerance"),
            connector.snap_tolerance,
            diagnostics,
        );
        validate_non_negative(
            &format!("{path}.clearance_hint"),
            connector.clearance_hint,
            diagnostics,
        );
        if let Some(mate) = &connector.mate {
            if !connector_ids.contains(mate.target.as_str()) || mate.target == connector.id {
                diagnostics.push(error(
                    "unknown_connector_mate",
                    format!("{path}.mate.target"),
                    format!(
                        "connector mate target '{}' is missing or self-referential",
                        mate.target
                    ),
                    "target another connector id declared in this recipe",
                ));
            }
            validate_non_negative(
                &format!("{path}.mate.axial_gap"),
                mate.axial_gap,
                diagnostics,
            );
            if let Some(roll) = mate.roll
                && !connection_roll_is_valid(roll)
            {
                diagnostics.push(error(
                    "invalid_connector_roll",
                    format!("{path}.mate.roll"),
                    "connector roll values must be finite and choose_nearest steps must be positive",
                    "use match_target, preserve_source, a positive choose_nearest step, or finite explicit degrees",
                ));
            }
        }
    }
}

fn validate_bounds(
    rows: &[SceneRecipeBoundsV1],
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, row) in rows.iter().enumerate() {
        let path = format!("$.bounds[{index}]");
        validate_target(
            &format!("{path}.target"),
            &row.target,
            node_ids,
            import_ids,
            diagnostics,
        );
        match row.source {
            SceneRecipeBoundsSourceV1::Authored => {
                let valid = row.min.zip(row.max).is_some_and(|(min, max)| {
                    min.into_iter().chain(max).all(f64::is_finite)
                        && (0..3).all(|axis| min[axis] <= max[axis])
                });
                if !valid {
                    diagnostics.push(error(
                        "invalid_authored_bounds",
                        &path,
                        "authored bounds require finite min/max with min <= max on every axis",
                        "emit finite scene-meter min and max vectors",
                    ));
                }
            }
            SceneRecipeBoundsSourceV1::Computed => {
                reject_explicit_bounds(row, &path, diagnostics);
            }
            SceneRecipeBoundsSourceV1::Imported => {
                reject_explicit_bounds(row, &path, diagnostics);
                if !matches!(
                    row.target,
                    SceneRecipeSpatialTargetV1::ImportRoot { .. }
                        | SceneRecipeSpatialTargetV1::ImportNode { .. }
                ) {
                    diagnostics.push(error(
                        "invalid_imported_bounds_target",
                        format!("{path}.target"),
                        "imported bounds require an import_root or import_node target",
                        "target the import that owns the converted source bounds",
                    ));
                }
            }
        }
    }
}

fn validate_target(
    path: &str,
    target: &SceneRecipeSpatialTargetV1,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let valid = match target {
        SceneRecipeSpatialTargetV1::Node { id } => node_ids.contains(id),
        SceneRecipeSpatialTargetV1::ImportRoot { id } => import_ids.contains(id),
        SceneRecipeSpatialTargetV1::ImportNode { import, path } => {
            import_ids.contains(import) && !path.trim().is_empty()
        }
    };
    if !valid {
        diagnostics.push(error(
            "unknown_spatial_target",
            path,
            "spatial target does not resolve to a declared persistent recipe id",
            "target a node/import id declared in this recipe and use an exact non-empty import path",
        ));
    }
}

fn validate_local_transform(
    path: &str,
    transform: &SceneRecipeTransformV1,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let valid = match transform {
        SceneRecipeTransformV1::Raw {
            translation,
            rotation,
            scale,
        } => {
            translation
                .iter()
                .chain(rotation)
                .chain(scale)
                .all(|v| v.is_finite())
                && rotation.iter().map(|v| v * v).sum::<f64>() > f64::EPSILON
                && scale.iter().all(|value| value.abs() > f64::EPSILON)
        }
        SceneRecipeTransformV1::Trs {
            translation,
            rotation_degrees,
            scale,
        } => {
            translation
                .iter()
                .chain(rotation_degrees)
                .chain(scale)
                .all(|v| v.is_finite())
                && scale.iter().all(|value| value.abs() > f64::EPSILON)
        }
        _ => false,
    };
    if !valid {
        diagnostics.push(error(
            "invalid_spatial_transform",
            path,
            "spatial frames and named-state transforms require finite raw or trs local transforms with non-zero scale",
            "use kind:raw or kind:trs in local scene-meter coordinates",
        ));
    }
}

fn reject_explicit_bounds(
    row: &SceneRecipeBoundsV1,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if row.min.is_some() || row.max.is_some() {
        diagnostics.push(error(
            "invalid_bounds_source",
            path,
            "computed/imported bounds do not accept authored min or max values",
            "remove min/max or use source:authored",
        ));
    }
}

fn validate_import_alias(
    path: &str,
    import: &str,
    name: &str,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if !import_ids.contains(import) || name.trim().is_empty() {
        diagnostics.push(error(
            "unknown_import_feature",
            format!("{path}.source"),
            "import feature aliases require a declared import id and non-empty exact name",
            "reference an imported anchor or connector by exact name",
        ));
    }
}

fn validate_non_negative(
    path: &str,
    value: Option<f64>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        diagnostics.push(error(
            "invalid_scene_distance",
            path,
            "scene-meter distance must be finite and non-negative",
            "use a finite value greater than or equal to zero",
        ));
    }
}

fn validate_non_empty_strings(
    path: &str,
    values: &[String],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if values.iter().any(|value| value.trim().is_empty()) {
        diagnostics.push(error(
            "invalid_spatial_string",
            path,
            "string arrays must contain only non-empty values",
            "remove empty values",
        ));
    }
}

fn validate_non_empty_optional(
    path: &str,
    value: Option<&str>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if value.is_some_and(|value| value.trim().is_empty()) {
        diagnostics.push(error(
            "invalid_spatial_string",
            path,
            "value must not be empty",
            "use a non-empty value or omit the field",
        ));
    }
}

fn connection_roll_is_valid(roll: crate::SceneRecipeConnectionRollV1) -> bool {
    match roll {
        crate::SceneRecipeConnectionRollV1::ChooseNearest { step_degrees } => {
            step_degrees.is_finite() && step_degrees > 0.0
        }
        crate::SceneRecipeConnectionRollV1::Explicit { degrees } => degrees.is_finite(),
        crate::SceneRecipeConnectionRollV1::MatchTarget
        | crate::SceneRecipeConnectionRollV1::PreserveSource => true,
    }
}

fn target_node_ids(object: &Map<String, Value>) -> BTreeSet<String> {
    ["nodes", "instance_sets", "particles", "labels"]
        .into_iter()
        .flat_map(|section| ids_from_array(object.get(section)))
        .collect()
}

fn ids_from_array(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn animated_node_ids(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|animation| {
            animation
                .get("channels")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|channel| {
            let target = channel.get("target")?;
            (target.get("kind").and_then(Value::as_str) == Some("node"))
                .then(|| target.get("id").and_then(Value::as_str))
                .flatten()
        })
        .map(str::to_owned)
        .collect()
}

fn error(
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
) -> SceneRecipeDiagnosticV1 {
    diagnostic(code, "error", path, message, help, None, false)
}
