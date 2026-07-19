use std::collections::BTreeMap;

use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildConnectionV1, SceneRecipeBuildConnectorV1, SceneRecipeConnectionParentingV1,
    SceneRecipeConnectionRollV1, SceneRecipeConnectorAlignmentV1, SceneRecipeConnectorPolarityV1,
    SceneRecipeConnectorRollPolicyV1, SceneRecipeConnectorSourceV1, SceneRecipeConnectorV1,
    SceneRecipeDiagnosticV1, SceneRecipeSpatialTargetV1,
};
use crate::scene_host::SceneHostCore;
use crate::{
    ConnectOptions, ConnectionAlignment, ConnectorFrame, ConnectorKey, ConnectorPolarity,
    ConnectorRollPolicy,
};

use super::{
    DiagnosticPathExt, SpatialBuildInputs, coordinate_system_name, resolve_target,
    source_units_name,
};
use crate::scene_host::recipe::authoring::local_transform_from_recipe;
use crate::scene_host::recipe::{error_diagnostic, scene_host_error_diagnostic};

pub(super) fn build_connectors(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeConnectorV1],
    context: &SpatialBuildInputs<'_>,
    manifest: &mut Vec<SceneRecipeBuildConnectorV1>,
    connections: &mut Vec<SceneRecipeBuildConnectionV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut keys = BTreeMap::<String, ConnectorKey>::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.connectors[{index}]");
        let (mut frame, target, source) = match &recipe.source {
            SceneRecipeConnectorSourceV1::Authored { target, transform } => {
                let node = match resolve_target(host, target, context) {
                    Ok(node) => node,
                    Err(message) => {
                        diagnostics.push(error_diagnostic(
                            format!("{path}.source.target"),
                            "unknown_spatial_target",
                            message,
                            "target a persistent object emitted by this recipe build",
                        ));
                        continue;
                    }
                };
                let transform = match local_transform_from_recipe(transform.as_ref()) {
                    Ok(transform) => transform,
                    Err(diagnostic) => {
                        diagnostics
                            .push((*diagnostic).with_path(format!("{path}.source.transform")));
                        continue;
                    }
                };
                (
                    ConnectorFrame::new(node, transform).named(recipe.id.clone()),
                    target.clone(),
                    "authored",
                )
            }
            SceneRecipeConnectorSourceV1::Import { import, name } => {
                let Some(import_handle) = context.import_handles.get(import).copied() else {
                    diagnostics.push(error_diagnostic(
                        format!("{path}.source.import"),
                        "unknown_import_feature",
                        format!(
                            "connector '{}' references unavailable import '{import}'",
                            recipe.id
                        ),
                        "use a required import that completed successfully",
                    ));
                    continue;
                };
                let imported = match host.resolve_import(import_handle) {
                    Ok(scene_import) => match scene_import.connector(name).cloned() {
                        Ok(connector) => connector,
                        Err(error) => {
                            diagnostics.push(error_diagnostic(
                                format!("{path}.source.name"),
                                "unknown_import_feature",
                                error.to_string(),
                                "use an exact unique imported connector name",
                            ));
                            continue;
                        }
                    },
                    Err(error) => {
                        diagnostics.push(scene_host_error_diagnostic(
                            format!("{path}.source.name"),
                            "unknown_import_feature",
                            error,
                        ));
                        continue;
                    }
                };
                (
                    ConnectorFrame::from_import_connector(&imported).named(recipe.id.clone()),
                    SceneRecipeSpatialTargetV1::ImportRoot { id: import.clone() },
                    "import",
                )
            }
        };
        if source == "authored" {
            if let Some(kind) = &recipe.connector_kind {
                frame = frame.with_kind(kind.clone());
            }
            for allowed in &recipe.allowed_mates {
                frame = frame.with_allowed_mate(allowed.clone());
            }
            for tag in &recipe.tags {
                frame = frame.with_tag(tag.clone());
            }
            if let Some(value) = recipe.snap_tolerance {
                frame = frame.with_snap_tolerance(value as f32);
            }
            if let Some(value) = recipe.clearance_hint {
                frame = frame.with_clearance_hint(value as f32);
            }
            if let Some(policy) = recipe.roll_policy {
                frame = frame.with_roll_policy(match policy {
                    SceneRecipeConnectorRollPolicyV1::Preserve => ConnectorRollPolicy::Preserve,
                    SceneRecipeConnectorRollPolicyV1::ChooseNearest => {
                        ConnectorRollPolicy::ChooseNearest
                    }
                });
            }
            if let Some(polarity) = recipe.polarity {
                frame = frame.with_polarity(match polarity {
                    SceneRecipeConnectorPolarityV1::Plug => ConnectorPolarity::Plug,
                    SceneRecipeConnectorPolarityV1::Socket => ConnectorPolarity::Socket,
                    SceneRecipeConnectorPolarityV1::Neutral => ConnectorPolarity::Neutral,
                });
            }
        }
        let node = frame.node();
        let units = source_units_name(frame.source_units()).to_owned();
        let coordinate_system = coordinate_system_name(frame.source_coordinate_system()).to_owned();
        let key = match host.scene.add_connector(frame) {
            Ok(key) => key,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "connector_create_failed",
                    error.to_string(),
                    "check connector target, frame, and imported feature lifetime",
                ));
                continue;
            }
        };
        keys.insert(recipe.id.clone(), key);
        manifest.push(SceneRecipeBuildConnectorV1 {
            id: recipe.id.clone(),
            identity_scope: "persistent_recipe_id".to_owned(),
            source: source.to_owned(),
            target,
            node_handle: host.register_node(node),
            source_units: units,
            source_coordinate_system: coordinate_system,
            status: "resolved".to_owned(),
        });
    }

    for (index, recipe) in recipes.iter().enumerate() {
        let Some(mate) = &recipe.mate else {
            continue;
        };
        let path = format!("$.connectors[{index}].mate");
        let Some(source) = keys.get(&recipe.id).copied() else {
            continue;
        };
        let Some(target) = keys.get(&mate.target).copied() else {
            diagnostics.push(error_diagnostic(
                format!("{path}.target"),
                "connector_mate_failed",
                format!("connector mate target '{}' was not resolved", mate.target),
                "fix the target connector diagnostic first",
            ));
            continue;
        };
        match host
            .scene
            .connect_by_key(source, target, connect_options(mate))
        {
            Ok(preview) => connections.push(SceneRecipeBuildConnectionV1 {
                source: recipe.id.clone(),
                target: mate.target.clone(),
                status: "applied".to_owned(),
                snap_distance_scene_meters: preview.snap_distance(),
            }),
            Err(error) => diagnostics.push(error_diagnostic(
                &path,
                "connector_mate_failed",
                format!(
                    "connector '{}' could not mate to '{}': {error}",
                    recipe.id, mate.target
                ),
                "check kind compatibility, source metadata, snap tolerance, scale, and frame handedness",
            )),
        }
    }
}

fn connect_options(mate: &crate::SceneRecipeConnectorMateV1) -> ConnectOptions {
    let mut options = ConnectOptions::default();
    if let Some(alignment) = mate.alignment {
        options = options.with_alignment(match alignment {
            SceneRecipeConnectorAlignmentV1::ForwardToForward => {
                ConnectionAlignment::ForwardToForward
            }
            SceneRecipeConnectorAlignmentV1::ForwardToBack => ConnectionAlignment::ForwardToBack,
        });
    }
    if let Some(roll) = mate.roll {
        options = match roll {
            SceneRecipeConnectionRollV1::MatchTarget => options.match_target_roll(),
            SceneRecipeConnectionRollV1::PreserveSource => options.preserve_roll(),
            SceneRecipeConnectionRollV1::ChooseNearest { step_degrees } => {
                options.choose_nearest_roll_degrees(step_degrees as f32)
            }
            SceneRecipeConnectionRollV1::Explicit { degrees } => {
                options.with_explicit_roll_degrees(degrees as f32)
            }
        };
    }
    if let Some(parenting) = mate.parenting {
        options = match parenting {
            SceneRecipeConnectionParentingV1::PreserveSourceParent => {
                options.preserve_source_parent()
            }
            SceneRecipeConnectionParentingV1::ReparentSourceToTargetParent => {
                options.reparent_source_to_target_parent()
            }
        };
    }
    if let Some(gap) = mate.axial_gap {
        options = options.with_axial_gap(gap as f32);
    }
    options
}
