use std::collections::BTreeMap;

use super::{SceneHostCore, error_diagnostic, scene_host_error_diagnostic};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildImportV1, SceneRecipeBuildTargetV1, SceneRecipeCalloutTargetV1,
    SceneRecipeDiagnosticV1, SceneRecipeExplodedViewModeV1, SceneRecipeTargetV1, SceneRecipeV1,
};
use crate::{Aabb, SceneHostEasing, SceneHostExplodedViewModeV1, SceneHostExplodedViewOptionsV1};

pub(super) fn apply_recipe_overlays(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeV1,
    imports: &[SceneRecipeBuildImportV1],
    nodes: &[SceneRecipeBuildTargetV1],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let targets = OverlayTargets::new(imports, nodes);
    apply_section_box(host, recipe, &targets, diagnostics);
    apply_measurements(host, recipe, diagnostics);
    apply_callouts(host, recipe, &targets, diagnostics);
    apply_exploded_view(host, recipe, &targets, diagnostics);
}

struct OverlayTargets {
    nodes: BTreeMap<String, u64>,
    import_roots: BTreeMap<String, Vec<u64>>,
}

impl OverlayTargets {
    fn new(imports: &[SceneRecipeBuildImportV1], nodes: &[SceneRecipeBuildTargetV1]) -> Self {
        let mut node_map = nodes
            .iter()
            .map(|node| (node.id.clone(), node.handle))
            .collect::<BTreeMap<_, _>>();
        let mut import_roots = BTreeMap::new();
        for import in imports {
            import_roots.insert(import.id.clone(), import.root_handles.clone());
            node_map.extend(
                import
                    .nodes_by_path
                    .iter()
                    .map(|(id, handle)| (id.clone(), *handle)),
            );
        }
        Self {
            nodes: node_map,
            import_roots,
        }
    }

    fn node(&self, id: &str) -> Option<u64> {
        self.nodes.get(id).copied()
    }

    fn import_roots(&self, id: &str) -> Option<&[u64]> {
        self.import_roots.get(id).map(Vec::as_slice)
    }
}

fn apply_section_box(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeV1,
    targets: &OverlayTargets,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(section_box) = &recipe.section_box else {
        return;
    };
    let bounds = match (&section_box.import, &section_box.target) {
        (Some(import), None) => import_bounds(host, targets, import, "$.section_box.import"),
        (None, Some(SceneRecipeTargetV1::Import { id })) => {
            import_bounds(host, targets, id, "$.section_box.target")
        }
        (None, Some(SceneRecipeTargetV1::Node { id })) => {
            node_bounds(host, targets, id, "$.section_box.target")
        }
        (None, Some(SceneRecipeTargetV1::World { .. })) => Err(error_diagnostic(
            "$.section_box.target",
            "invalid_section_box_target",
            "section_box target kind 'world' has no bounds",
            "target a node or import when computing a section box",
        )
        .into()),
        (Some(_), Some(_)) => Err(error_diagnostic(
            "$.section_box",
            "invalid_section_box",
            "section_box cannot set both import and target",
            "set exactly one of import or target",
        )
        .into()),
        (None, None) => Err(error_diagnostic(
            "$.section_box",
            "invalid_section_box",
            "section_box must set import or target",
            "target a node or import",
        )
        .into()),
    };
    let bounds = match bounds {
        Ok(bounds) => bounds,
        Err(diagnostic) => {
            diagnostics.push(*diagnostic);
            return;
        }
    };
    if let Err(error) = host.set_section_box_json(
        bounds,
        section_box.margin,
        section_box.inverted,
        section_box.helper_wireframe,
    ) {
        diagnostics.push(scene_host_error_diagnostic(
            "$.section_box",
            "section_box_failed",
            error,
        ));
    }
}

fn apply_measurements(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeV1,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, measurement) in recipe.measurements.iter().enumerate() {
        let path = format!("$.measurements[{index}]");
        if measurement.kind != "distance" {
            diagnostics.push(error_diagnostic(
                &path,
                "unsupported_feature",
                format!("measurement kind '{}' is not supported", measurement.kind),
                "use kind:\"distance\"",
            ));
            continue;
        }
        if let Err(error) = host.add_distance_measurement_json(
            &measurement.id,
            crate::Vec3::from_array(measurement.start),
            crate::Vec3::from_array(measurement.end),
            measurement.label.as_deref(),
            measurement.unit.as_deref().unwrap_or("unit"),
            measurement.precision.unwrap_or(2),
        ) {
            diagnostics.push(scene_host_error_diagnostic(
                path,
                "measurement_failed",
                error,
            ));
        }
    }
}

fn apply_callouts(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeV1,
    targets: &OverlayTargets,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, callout) in recipe.callouts.iter().enumerate() {
        let path = format!("$.callouts[{index}]");
        let result = match &callout.target {
            SceneRecipeCalloutTargetV1::ImportRoot {
                import,
                local_offset,
            } => match targets
                .import_roots(import)
                .and_then(|roots| roots.first().copied())
            {
                Some(root) => host.add_node_callout(
                    &callout.id,
                    root,
                    *local_offset,
                    callout.label_offset,
                    &callout.text,
                ),
                None => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "unknown_import_ref",
                        format!("callout references unknown import '{import}'"),
                        "target an import id from the build manifest",
                    ));
                    continue;
                }
            },
            SceneRecipeCalloutTargetV1::Node { id, local_offset } => match targets.node(id) {
                Some(node) => host.add_node_callout(
                    &callout.id,
                    node,
                    *local_offset,
                    callout.label_offset,
                    &callout.text,
                ),
                None => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "unknown_node_ref",
                        format!("callout references unknown node '{id}'"),
                        "target an authored or imported node id from the build manifest",
                    ));
                    continue;
                }
            },
            SceneRecipeCalloutTargetV1::World { position } => {
                host.add_world_callout(&callout.id, *position, callout.label_offset, &callout.text)
            }
        };
        if let Err(error) = result {
            diagnostics.push(scene_host_error_diagnostic(path, "callout_failed", error));
        }
    }
}

fn apply_exploded_view(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeV1,
    targets: &OverlayTargets,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(exploded) = &recipe.exploded_view else {
        return;
    };
    let Some(root) = targets
        .import_roots(&exploded.import)
        .and_then(|roots| roots.first().copied())
    else {
        diagnostics.push(error_diagnostic(
            "$.exploded_view.import",
            "unknown_import_ref",
            format!(
                "exploded_view references unknown import '{}'",
                exploded.import
            ),
            "target an import id from the build manifest",
        ));
        return;
    };
    let mode = match exploded.mode {
        SceneRecipeExplodedViewModeV1::DirectChildren => {
            SceneHostExplodedViewModeV1::DirectChildren
        }
        SceneRecipeExplodedViewModeV1::HierarchyDepth => {
            SceneHostExplodedViewModeV1::HierarchyDepth
        }
        SceneRecipeExplodedViewModeV1::Axis => SceneHostExplodedViewModeV1::Axis,
    };
    let patch = match host.exploded_view_patch(
        root,
        SceneHostExplodedViewOptionsV1 {
            mode,
            axis: exploded.axis,
            factor: exploded.factor,
            distance: exploded.distance,
            duration_seconds: None,
            easing: SceneHostEasing::Linear,
        },
    ) {
        Ok(patch) => patch,
        Err(error) => {
            diagnostics.push(scene_host_error_diagnostic(
                "$.exploded_view",
                "exploded_view_failed",
                error,
            ));
            return;
        }
    };
    if let Err(error) = host.apply_patch(&patch) {
        diagnostics.push(scene_host_error_diagnostic(
            "$.exploded_view",
            "exploded_view_patch_failed",
            error,
        ));
    }
}

fn node_bounds(
    host: &SceneHostCore<DefaultAssetFetcher>,
    targets: &OverlayTargets,
    id: &str,
    path: &str,
) -> Result<Aabb, Box<SceneRecipeDiagnosticV1>> {
    let Some(handle) = targets.node(id) else {
        return Err(Box::new(error_diagnostic(
            path,
            "unknown_node_ref",
            format!("target references unknown node '{id}'"),
            "target an authored or imported node id from the build manifest",
        )));
    };
    match host.node_world_bounds(handle) {
        Ok(Some(bounds)) => Ok(bounds),
        Ok(None) => Err(Box::new(error_diagnostic(
            path,
            "node_bounds_missing",
            format!("target node '{id}' has no renderable bounds"),
            "target a renderable node or import subtree",
        ))),
        Err(error) => Err(Box::new(scene_host_error_diagnostic(
            path,
            "node_bounds_failed",
            error,
        ))),
    }
}

fn import_bounds(
    host: &SceneHostCore<DefaultAssetFetcher>,
    targets: &OverlayTargets,
    id: &str,
    path: &str,
) -> Result<Aabb, Box<SceneRecipeDiagnosticV1>> {
    let Some(roots) = targets.import_roots(id) else {
        return Err(Box::new(error_diagnostic(
            path,
            "unknown_import_ref",
            format!("target references unknown import '{id}'"),
            "target an import id from the build manifest",
        )));
    };
    let mut combined = None;
    for root in roots {
        match host.node_world_bounds(*root) {
            Ok(Some(bounds)) => {
                combined = Some(combined.map_or(bounds, |current: Aabb| current.union(bounds)));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(Box::new(scene_host_error_diagnostic(
                    path,
                    "import_bounds_failed",
                    error,
                )));
            }
        }
    }
    combined.ok_or_else(|| {
        Box::new(error_diagnostic(
            path,
            "import_bounds_missing",
            format!("import '{id}' has no renderable bounds"),
            "target a renderable import or node",
        ))
    })
}
