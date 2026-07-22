use std::collections::BTreeMap;

use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeAnchorSourceV1, SceneRecipeAnchorV1, SceneRecipeBoundsSourceV1, SceneRecipeBoundsV1,
    SceneRecipeBuildAnchorV1, SceneRecipeBuildBoundsV1, SceneRecipeBuildConnectionV1,
    SceneRecipeBuildConnectorV1, SceneRecipeBuildImportV1, SceneRecipeBuildNamedStateV1,
    SceneRecipeConnectorV1, SceneRecipeDiagnosticV1, SceneRecipeNamedStateV1,
    SceneRecipeSpatialTargetV1,
};
use crate::scene_host::SceneHostCore;
use crate::{Aabb, AnchorFrame, NodeKey, SourceCoordinateSystem, SourceUnits, Vec3};

use super::authoring::local_transform_from_recipe;
use super::{error_diagnostic, scene_host_error_diagnostic};

mod connectors;
mod states;

use connectors::build_connectors;
use states::build_named_states;

pub(super) struct SpatialStateManifest<'a> {
    pub anchors: &'a mut Vec<SceneRecipeBuildAnchorV1>,
    pub connectors: &'a mut Vec<SceneRecipeBuildConnectorV1>,
    pub connections: &'a mut Vec<SceneRecipeBuildConnectionV1>,
    pub bounds: &'a mut Vec<SceneRecipeBuildBoundsV1>,
    pub named_states: &'a mut Vec<SceneRecipeBuildNamedStateV1>,
}

pub(super) struct SpatialBuildInputs<'a> {
    pub node_keys: &'a BTreeMap<String, NodeKey>,
    pub imported_node_keys: &'a BTreeMap<String, NodeKey>,
    pub import_handles: &'a BTreeMap<String, u64>,
    pub imports: &'a [SceneRecipeBuildImportV1],
}

// Recipe collections and their output manifest slices remain explicit here so
// source IDs cannot be accidentally paired with the wrong artifact owner.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_spatial_state(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, crate::SceneRecipeColorV1>,
    anchors: &[SceneRecipeAnchorV1],
    connectors: &[SceneRecipeConnectorV1],
    bounds: &[SceneRecipeBoundsV1],
    states: &[SceneRecipeNamedStateV1],
    context: SpatialBuildInputs<'_>,
    manifest: SpatialStateManifest<'_>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    build_bounds(host, bounds, &context, manifest.bounds, diagnostics);
    build_anchors(host, anchors, &context, manifest.anchors, diagnostics);
    build_connectors(
        host,
        connectors,
        &context,
        manifest.connectors,
        manifest.connections,
        diagnostics,
    );
    build_named_states(
        host,
        colors,
        states,
        &context,
        manifest.named_states,
        diagnostics,
    );
}

fn build_anchors(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeAnchorV1],
    context: &SpatialBuildInputs<'_>,
    manifest: &mut Vec<SceneRecipeBuildAnchorV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.anchors[{index}]");
        let result = match &recipe.source {
            SceneRecipeAnchorSourceV1::Authored { target, transform } => {
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
                let mut frame = AnchorFrame::new(node, transform).named(recipe.id.clone());
                for tag in &recipe.tags {
                    frame = frame.with_tag(tag.clone());
                }
                if let Some(label) = &recipe.label {
                    frame = frame.with_label(label.clone());
                }
                (frame, target.clone(), "authored")
            }
            SceneRecipeAnchorSourceV1::Import { import, name } => {
                let Some(import_handle) = context.import_handles.get(import).copied() else {
                    diagnostics.push(error_diagnostic(
                        format!("{path}.source.import"),
                        "unknown_import_feature",
                        format!(
                            "anchor '{}' references unavailable import '{import}'",
                            recipe.id
                        ),
                        "use a required import that completed successfully",
                    ));
                    continue;
                };
                let anchor = match host.resolve_import(import_handle) {
                    Ok(scene_import) => match scene_import.anchor(name).cloned() {
                        Ok(anchor) => anchor,
                        Err(error) => {
                            diagnostics.push(error_diagnostic(
                                format!("{path}.source.name"),
                                "unknown_import_feature",
                                error.to_string(),
                                "use an exact unique imported anchor name",
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
                let frame = AnchorFrame::from_import_anchor(&anchor).named(recipe.id.clone());
                (
                    frame,
                    SceneRecipeSpatialTargetV1::ImportRoot { id: import.clone() },
                    "import",
                )
            }
        };
        let (frame, target, source) = result;
        let node = frame.node();
        let units = source_units_name(frame.source_units()).to_owned();
        let coordinate_system = coordinate_system_name(frame.source_coordinate_system()).to_owned();
        if let Err(error) = host.scene.add_anchor(frame) {
            diagnostics.push(error_diagnostic(
                &path,
                "anchor_create_failed",
                error.to_string(),
                "check the anchor target and imported feature lifetime",
            ));
            continue;
        }
        manifest.push(SceneRecipeBuildAnchorV1 {
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
}

fn build_bounds(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeBoundsV1],
    context: &SpatialBuildInputs<'_>,
    manifest: &mut Vec<SceneRecipeBuildBoundsV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.bounds[{index}]");
        let node = match resolve_target(host, &recipe.target, context) {
            Ok(node) => node,
            Err(message) => {
                diagnostics.push(error_diagnostic(
                    format!("{path}.target"),
                    "unknown_spatial_target",
                    message,
                    "target a persistent object emitted by this recipe build",
                ));
                continue;
            }
        };
        let (bounds, source, space) = match recipe.source {
            SceneRecipeBoundsSourceV1::Authored => {
                let (Some(min), Some(max)) = (recipe.min, recipe.max) else {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "invalid_authored_bounds",
                        "authored bounds require min and max",
                        "emit finite scene-meter min and max vectors",
                    ));
                    continue;
                };
                let bounds = Aabb::new(vec3(min), vec3(max));
                if let Err(error) = host.scene.set_authored_node_bounds(node, bounds) {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "authored_bounds_override",
                        error.to_string(),
                        "target an empty group node without geometry- or asset-owned bounds",
                    ));
                    continue;
                }
                (bounds, "authored", "local")
            }
            SceneRecipeBoundsSourceV1::Computed | SceneRecipeBoundsSourceV1::Imported => {
                let bounds = match host.scene.node_world_bounds(node, &host.assets) {
                    Ok(Some(bounds)) => bounds,
                    Ok(None) => {
                        diagnostics.push(error_diagnostic(
                            &path,
                            "bounds_missing",
                            format!("bounds '{}' target has no resolvable bounds", recipe.id),
                            "target a bounded renderer or imported object",
                        ));
                        continue;
                    }
                    Err(error) => {
                        diagnostics.push(error_diagnostic(
                            &path,
                            "bounds_resolve_failed",
                            error.to_string(),
                            "fix the target's scene graph or asset bounds",
                        ));
                        continue;
                    }
                };
                let source = match recipe.source {
                    SceneRecipeBoundsSourceV1::Computed => "computed",
                    SceneRecipeBoundsSourceV1::Imported => "imported",
                    SceneRecipeBoundsSourceV1::Authored => unreachable!(),
                };
                (bounds, source, "world")
            }
        };
        manifest.push(SceneRecipeBuildBoundsV1 {
            id: recipe.id.clone(),
            identity_scope: "persistent_recipe_id".to_owned(),
            target: recipe.target.clone(),
            source: source.to_owned(),
            space: space.to_owned(),
            units: "scene_meters".to_owned(),
            min: bounds.min.to_array(),
            max: bounds.max.to_array(),
        });
    }
}

fn resolve_target(
    host: &SceneHostCore<DefaultAssetFetcher>,
    target: &SceneRecipeSpatialTargetV1,
    context: &SpatialBuildInputs<'_>,
) -> Result<NodeKey, String> {
    match target {
        SceneRecipeSpatialTargetV1::Node { id } => context
            .node_keys
            .get(id)
            .copied()
            .ok_or_else(|| format!("node target '{id}' was not resolved")),
        SceneRecipeSpatialTargetV1::ImportRoot { id } => {
            let import = context
                .imports
                .iter()
                .find(|entry| entry.id == *id)
                .ok_or_else(|| format!("import root '{id}' was not resolved"))?;
            let handle = import
                .primary_root
                .ok_or_else(|| format!("import '{id}' has no primary root"))?;
            host.resolve_node(handle)
                .map_err(|error| format!("import '{id}' primary root is unavailable: {error}"))
        }
        SceneRecipeSpatialTargetV1::ImportNode { import, path } => context
            .imported_node_keys
            .get(&format!("{import}:{path}"))
            .copied()
            .ok_or_else(|| format!("import node '{import}:{path}' was not resolved")),
    }
}

fn vec3(value: [f64; 3]) -> Vec3 {
    Vec3::new(value[0] as f32, value[1] as f32, value[2] as f32)
}

fn source_units_name(units: SourceUnits) -> &'static str {
    match units {
        SourceUnits::Meters => "meters",
        SourceUnits::Centimeters => "centimeters",
        SourceUnits::Millimeters => "millimeters",
        SourceUnits::Inches => "inches",
        SourceUnits::Feet => "feet",
    }
}

fn coordinate_system_name(system: SourceCoordinateSystem) -> &'static str {
    match system {
        SourceCoordinateSystem::GltfYUpRightHanded => "gltf_y_up_right_handed",
        SourceCoordinateSystem::YUpLeftHanded => "y_up_left_handed",
        SourceCoordinateSystem::ZUpRightHanded => "z_up_right_handed",
        SourceCoordinateSystem::ZUpLeftHanded => "z_up_left_handed",
    }
}

trait DiagnosticPathExt {
    fn with_path(self, path: String) -> Self;
}

impl DiagnosticPathExt for SceneRecipeDiagnosticV1 {
    fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }
}
