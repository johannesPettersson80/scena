use super::{PlaceCommandArgs, PlacementRuntime, PlacementRuntimeImport};

#[derive(Clone, Copy)]
enum AuthoredFeatureKind {
    Anchor,
    Connector,
}

struct AuthoredFeatureRef<'a> {
    kind: AuthoredFeatureKind,
    name: &'a str,
}

pub(super) fn place_authored_feature(
    runtime: &PlacementRuntime,
    args: &PlaceCommandArgs,
    source_index: usize,
    verb: String,
    align_orientation: bool,
) -> scena::ScenePlacementResultV1 {
    let source_import = &runtime.imports[source_index];
    let target_import_id = args
        .target_import_id
        .as_deref()
        .unwrap_or(&source_import.id);
    let Some(target_index) = runtime.import_index(target_import_id) else {
        return scena::ScenePlacementResultV1::failure(
            source_import.id.clone(),
            verb,
            scena::ScenePlacementDiagnosticV1::new(
                "unknown_import",
                "$.target_import",
                format!("target import '{target_import_id}' was not found"),
                "pass --target-import with one of the recipe import ids",
            ),
        );
    };
    let target_import = &runtime.imports[target_index];
    let source_spec = match feature_spec(
        args.source_anchor.as_deref(),
        args.source_connector.as_deref(),
        "source",
    ) {
        Ok(spec) => spec,
        Err(diagnostic) => {
            return scena::ScenePlacementResultV1::failure(
                source_import.id.clone(),
                verb,
                *diagnostic,
            );
        }
    };
    let target_spec = match feature_spec(
        args.target_anchor.as_deref(),
        args.target_connector.as_deref(),
        "target",
    ) {
        Ok(spec) => spec,
        Err(diagnostic) => {
            return scena::ScenePlacementResultV1::failure(
                source_import.id.clone(),
                verb,
                *diagnostic,
            );
        }
    };
    let source_frame = match authored_feature_frame(source_import, source_spec, "source") {
        Ok(frame) => frame,
        Err(diagnostic) => {
            return scena::ScenePlacementResultV1::failure(
                source_import.id.clone(),
                verb,
                *diagnostic,
            );
        }
    };
    let target_frame = match authored_feature_frame(target_import, target_spec, "target") {
        Ok(frame) => frame,
        Err(diagnostic) => {
            return scena::ScenePlacementResultV1::failure(
                source_import.id.clone(),
                verb,
                *diagnostic,
            );
        }
    };
    let source_current = runtime
        .scene
        .world_transform(source_frame.node())
        .unwrap_or(source_import.transform);
    let target_feature = match feature_world_transform(runtime, &target_frame, "target") {
        Ok(transform) => transform,
        Err(diagnostic) => {
            return scena::ScenePlacementResultV1::failure(
                source_import.id.clone(),
                verb,
                *diagnostic,
            );
        }
    };
    let transform = if align_orientation {
        scena::placement_align_to_feature_transform(
            source_current,
            source_frame.local_transform(),
            target_feature,
        )
    } else {
        scena::placement_place_on_feature_transform(
            source_current,
            source_frame.local_transform(),
            target_feature,
        )
    };
    match transform {
        Ok(transform) => {
            scena::ScenePlacementResultV1::success(source_import.id.clone(), verb, transform)
        }
        Err(error) => {
            scena::ScenePlacementResultV1::failure(source_import.id.clone(), verb, *error)
        }
    }
}

fn feature_spec<'a>(
    anchor: Option<&'a str>,
    connector: Option<&'a str>,
    role: &str,
) -> Result<AuthoredFeatureRef<'a>, Box<scena::ScenePlacementDiagnosticV1>> {
    match (anchor, connector) {
        (Some(name), None) => Ok(AuthoredFeatureRef {
            kind: AuthoredFeatureKind::Anchor,
            name,
        }),
        (None, Some(name)) => Ok(AuthoredFeatureRef {
            kind: AuthoredFeatureKind::Connector,
            name,
        }),
        (None, None) => Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "missing_authored_feature",
            format!("$.{role}"),
            format!("{role} feature requires an anchor or connector name"),
            format!("pass --{role}-anchor <name> or --{role}-connector <name>"),
        ))),
        (Some(_), Some(_)) => Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "ambiguous_authored_feature",
            format!("$.{role}"),
            format!("{role} feature must not specify both anchor and connector"),
            format!("pass only one of --{role}-anchor or --{role}-connector"),
        ))),
    }
}

fn authored_feature_frame(
    import: &PlacementRuntimeImport,
    spec: AuthoredFeatureRef<'_>,
    role: &str,
) -> Result<scena::ConnectorFrame, Box<scena::ScenePlacementDiagnosticV1>> {
    match spec.kind {
        AuthoredFeatureKind::Anchor => import
            .import
            .anchor(spec.name)
            .map(scena::ConnectorFrame::from_import_anchor)
            .map_err(|error| Box::new(feature_lookup_diagnostic(import, role, spec, error))),
        AuthoredFeatureKind::Connector => import
            .import
            .connector(spec.name)
            .map(scena::ConnectorFrame::from_import_connector)
            .map_err(|error| Box::new(feature_lookup_diagnostic(import, role, spec, error))),
    }
}

fn feature_world_transform(
    runtime: &PlacementRuntime,
    frame: &scena::ConnectorFrame,
    role: &str,
) -> Result<scena::Transform, Box<scena::ScenePlacementDiagnosticV1>> {
    runtime
        .scene
        .world_transform(frame.node())
        .map(|node_world| scena::Transform::compose(node_world, frame.local_transform()))
        .ok_or_else(|| {
            Box::new(scena::ScenePlacementDiagnosticV1::new(
                "authored_feature_node_missing",
                format!("$.{role}"),
                "authored feature node no longer exists in the instantiated scene",
                "reload the asset and retry placement",
            ))
        })
}

fn feature_lookup_diagnostic(
    import: &PlacementRuntimeImport,
    role: &str,
    spec: AuthoredFeatureRef<'_>,
    error: scena::LookupError,
) -> scena::ScenePlacementDiagnosticV1 {
    let (feature_kind, flag_name) = match spec.kind {
        AuthoredFeatureKind::Anchor => ("anchor", format!("--{role}-anchor")),
        AuthoredFeatureKind::Connector => ("connector", format!("--{role}-connector")),
    };
    let mut diagnostic = match error {
        scena::LookupError::AnchorNotFound { .. }
        | scena::LookupError::ConnectorNotFound { .. } => scena::ScenePlacementDiagnosticV1::new(
            "authored_feature_not_found",
            format!("$.imports[{}].{feature_kind}s", import.recipe_index),
            format!("{role} {feature_kind} '{}' was not found", spec.name),
            format!("pass {flag_name} with an authored {feature_kind} name from the asset"),
        ),
        scena::LookupError::AmbiguousAnchorName { .. }
        | scena::LookupError::AmbiguousConnectorName { .. } => {
            scena::ScenePlacementDiagnosticV1::new(
                "ambiguous_authored_feature",
                format!("$.imports[{}].{feature_kind}s", import.recipe_index),
                format!(
                    "{role} {feature_kind} '{}' matched multiple authored features",
                    spec.name
                ),
                format!("use an asset with unique {feature_kind} names for placement"),
            )
        }
        other => scena::ScenePlacementDiagnosticV1::new(
            "invalid_authored_feature",
            format!("$.imports[{}].{feature_kind}s", import.recipe_index),
            format!(
                "failed to resolve {role} {feature_kind} '{}': {other}",
                spec.name
            ),
            "fix the authored feature metadata before placement",
        ),
    };
    if let Some(suggestion) = feature_name_suggestion(import, spec.kind) {
        diagnostic = diagnostic.with_suggestion(suggestion);
    }
    diagnostic
}

fn feature_name_suggestion(
    import: &PlacementRuntimeImport,
    kind: AuthoredFeatureKind,
) -> Option<String> {
    match kind {
        AuthoredFeatureKind::Anchor => import
            .import
            .anchors()
            .ok()
            .and_then(|anchors| anchors.first())
            .map(|anchor| anchor.name().to_owned()),
        AuthoredFeatureKind::Connector => import
            .import
            .connectors()
            .ok()
            .and_then(|connectors| connectors.first())
            .map(|connector| connector.name().to_owned()),
    }
}
