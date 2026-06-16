use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color};
use super::transform::transform_from_recipe;
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildTargetV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeNodeV1,
};
use crate::scene_host::SceneHostCore;
use crate::{GeometryHandle, MaterialHandle, NodeKey};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_nodes(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeNodeV1],
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    resources: AuthoredNodeResources<'_>,
    manifest: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, NodeKey> {
    let mut node_keys = BTreeMap::new();
    if recipes.len() > policy.max_nodes() {
        diagnostics.push(error_diagnostic(
            "$.nodes",
            "policy_violation",
            format!(
                "recipe declares {} authored nodes, exceeding RecipeBuildPolicy max_nodes {}",
                recipes.len(),
                policy.max_nodes()
            ),
            "reduce node count or raise the operator-owned max_nodes policy",
        ));
        return node_keys;
    }
    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.nodes[{index}]");
        let Some(geometry) = resources.geometries.get(&recipe.geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_geometry_ref",
                format!(
                    "node '{}' references missing geometry '{}'",
                    recipe.id, recipe.geometry
                ),
                "declare the geometry before the node",
            ));
            continue;
        };
        let Some(material) = resources.materials.get(&recipe.material).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_material_ref",
                format!(
                    "node '{}' references missing material '{}'",
                    recipe.id, recipe.material
                ),
                "declare the material before the node",
            ));
            continue;
        };
        let parent = match &recipe.parent {
            Some(parent) => match node_keys.get(parent).copied() {
                Some(parent) => parent,
                None => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "unknown_node_ref",
                        format!(
                            "node '{}' references missing or forward parent '{}'",
                            recipe.id, parent
                        ),
                        "declare parent nodes before their children and avoid cycles",
                    ));
                    continue;
                }
            },
            None => root,
        };
        let transform =
            match transform_from_recipe(recipe.transform.as_ref(), &BTreeMap::new(), host) {
                Ok(transform) => transform,
                Err(diagnostic) => {
                    diagnostics.push((*diagnostic).with_path(format!("{path}.transform")));
                    continue;
                }
            };
        let node = match host
            .scene
            .mesh(geometry, material)
            .parent(parent)
            .transform(transform)
            .add()
        {
            Ok(node) => node,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "node_create_failed",
                    format!("failed to create node '{}': {error}", recipe.id),
                    "check the node parent, geometry, and material references",
                ));
                continue;
            }
        };
        apply_node_attributes(host, recipe, node, colors, &path, diagnostics);
        let handle = host.register_node(node);
        node_keys.insert(recipe.id.clone(), node);
        manifest.push(SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "node".to_owned(),
            parent: Some(
                host.node_handle_map
                    .get(&parent)
                    .copied()
                    .unwrap_or(root_handle),
            ),
            name: recipe.name.clone(),
            active: None,
        });
    }
    node_keys
}

pub(in crate::scene_host::recipe) struct AuthoredNodeResources<'a> {
    pub(in crate::scene_host::recipe) geometries: &'a BTreeMap<String, GeometryHandle>,
    pub(in crate::scene_host::recipe) materials: &'a BTreeMap<String, MaterialHandle>,
}

fn apply_node_attributes(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeNodeV1,
    node: NodeKey,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(visible) = recipe.visible
        && let Err(error) = host.scene.set_visible(node, visible)
    {
        diagnostics.push(error_diagnostic(
            path,
            "node_visible_failed",
            error.to_string(),
            "check the node reference",
        ));
    }
    for tag in &recipe.tags {
        if let Err(error) = host.scene.add_tag(node, tag.clone()) {
            diagnostics.push(error_diagnostic(
                path,
                "node_tag_failed",
                error.to_string(),
                "check the node reference",
            ));
        }
    }
    if let Some(mask) = recipe.layer_mask
        && let Err(error) = host.scene.set_layer_mask(node, mask)
    {
        diagnostics.push(error_diagnostic(
            path,
            "node_layer_mask_failed",
            error.to_string(),
            "check the node reference",
        ));
    }
    if let Some(group) = recipe.render_group
        && let Err(error) = host.scene.set_render_group(node, group)
    {
        diagnostics.push(error_diagnostic(
            path,
            "node_render_group_failed",
            error.to_string(),
            "check the node reference",
        ));
    }
    if let Some(tint) = &recipe.tint {
        match authored_color(colors, tint) {
            Ok(tint) => {
                if let Err(error) = host.scene.set_node_tint(node, Some(tint)) {
                    diagnostics.push(error_diagnostic(
                        path,
                        "node_tint_failed",
                        error.to_string(),
                        "check the node reference",
                    ));
                }
            }
            Err(diagnostic) => diagnostics.push((*diagnostic).with_path(format!("{path}.tint"))),
        }
    }
}
