use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color};
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::geometry::SkinningMatrix;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildTargetV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeNodeV1,
};
use crate::scene::{MeshLodLevel, SceneSkinBinding};
use crate::scene_host::SceneHostCore;
use crate::{GeometryHandle, MaterialHandle, NodeKey};

use super::super::error_diagnostic;
use super::super::policy::RecipeBuildBudget;

pub(in crate::scene_host::recipe) fn build_authored_nodes(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeNodeV1],
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
    if let Some(diagnostic) = resources
        .build_budget
        .reserve_nodes(policy, "$.nodes", recipes.len())
    {
        diagnostics.push(diagnostic);
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
        let Some(lod_levels) = resolve_lod_levels(recipe, resources.geometries, &path, diagnostics)
        else {
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
        let geometry_bounds = match host.assets.geometry(geometry) {
            Some(geometry) => Some(geometry.bounds()),
            None => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "geometry_bounds_missing",
                    format!("node '{}' geometry could not be resolved", recipe.id),
                    "declare a valid geometry before the node",
                ));
                continue;
            }
        };
        let mut transform_nodes = resources.imported_nodes.clone();
        transform_nodes.extend(node_keys.clone());
        let transform = match transform_from_recipe(
            recipe.transform.as_ref(),
            TransformResolutionInput {
                node_keys: &transform_nodes,
                imports: resources.imports,
                parent: Some(parent),
                current_bounds: geometry_bounds,
            },
            host,
        ) {
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
        if let Err(error) = host.scene.set_mesh_lods(node, lod_levels) {
            diagnostics.push(error_diagnostic(
                &path,
                "lod_create_failed",
                format!(
                    "failed to attach LOD levels to node '{}': {error}",
                    recipe.id
                ),
                "attach LOD levels only to authored mesh nodes",
            ));
            continue;
        }
        apply_node_attributes(host, recipe, node, resources.colors, &path, diagnostics);
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
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.nodes[{index}]");
        if let Some(node) = node_keys.get(&recipe.id).copied() {
            apply_node_deformations(
                host,
                recipe,
                node,
                resources.geometries,
                &node_keys,
                &path,
                diagnostics,
            );
        }
    }
    node_keys
}

fn resolve_lod_levels(
    recipe: &SceneRecipeNodeV1,
    geometries: &BTreeMap<String, GeometryHandle>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Vec<MeshLodLevel>> {
    let mut levels = Vec::new();
    for (index, lod) in recipe.lods.iter().enumerate() {
        let lod_path = format!("{path}.lods[{index}]");
        let Some(geometry) = geometries.get(&lod.geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &lod_path,
                "unknown_geometry_ref",
                format!(
                    "LOD level for node '{}' references missing geometry '{}'",
                    recipe.id, lod.geometry
                ),
                "declare the LOD geometry before the node",
            ));
            return None;
        };
        if !lod.max_screen_fraction.is_finite()
            || lod.max_screen_fraction <= 0.0
            || lod.max_screen_fraction > 1.0
        {
            diagnostics.push(error_diagnostic(
                format!("{lod_path}.max_screen_fraction"),
                "invalid_lod_threshold",
                "LOD max_screen_fraction must be finite and in (0, 1]",
                "use a fraction such as 0.15 for distant or small-on-screen geometry",
            ));
            return None;
        }
        levels.push(MeshLodLevel::new(lod.max_screen_fraction as f32, geometry));
    }
    Some(levels)
}

pub(in crate::scene_host::recipe) struct AuthoredNodeResources<'a> {
    pub(in crate::scene_host::recipe) colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    pub(in crate::scene_host::recipe) geometries: &'a BTreeMap<String, GeometryHandle>,
    pub(in crate::scene_host::recipe) materials: &'a BTreeMap<String, MaterialHandle>,
    pub(in crate::scene_host::recipe) imported_nodes: &'a BTreeMap<String, NodeKey>,
    pub(in crate::scene_host::recipe) imports: &'a BTreeMap<String, u64>,
    pub(in crate::scene_host::recipe) build_budget: &'a mut RecipeBuildBudget,
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

fn apply_node_deformations(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeNodeV1,
    node: NodeKey,
    geometries: &BTreeMap<String, GeometryHandle>,
    node_keys: &BTreeMap<String, NodeKey>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if !recipe.morph_weights.is_empty() {
        match validate_morph_weight_count(host, recipe, geometries) {
            Ok(()) => {
                let weights = recipe
                    .morph_weights
                    .iter()
                    .map(|weight| *weight as f32)
                    .collect::<Vec<_>>();
                if let Err(error) = host.scene.set_morph_weights(node, weights) {
                    diagnostics.push(error_diagnostic(
                        path,
                        "morph_weights_failed",
                        error.to_string(),
                        "check the node and morph target references",
                    ));
                }
            }
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.morph_weights")))
            }
        }
    }
    if let Some(binding) = &recipe.skin_binding {
        match scene_skin_binding(host, recipe, binding, geometries, node_keys) {
            Ok(binding) => {
                if let Err(error) = host.scene.set_skin_binding(node, binding) {
                    diagnostics.push(error_diagnostic(
                        path,
                        "skin_binding_failed",
                        error.to_string(),
                        "check the node and skin binding references",
                    ));
                }
            }
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.skin_binding")))
            }
        }
    }
}

fn validate_morph_weight_count(
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeNodeV1,
    geometries: &BTreeMap<String, GeometryHandle>,
) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
    let Some(handle) = geometries.get(&recipe.geometry).copied() else {
        return Err(Box::new(error_diagnostic(
            "$",
            "unknown_geometry_ref",
            format!(
                "node '{}' references missing geometry '{}'",
                recipe.id, recipe.geometry
            ),
            "declare the geometry before the node",
        )));
    };
    let Some(geometry) = host.assets.geometry(handle) else {
        return Err(Box::new(error_diagnostic(
            "$",
            "geometry_missing",
            format!(
                "node '{}' geometry '{}' could not be resolved",
                recipe.id, recipe.geometry
            ),
            "declare a valid geometry before the node",
        )));
    };
    let target_count = geometry.morph_targets().len();
    if target_count == 0 || recipe.morph_weights.len() != target_count {
        return Err(Box::new(error_diagnostic(
            "$",
            "invalid_morph",
            format!(
                "node '{}' has {} morph weights but geometry '{}' has {target_count} morph targets",
                recipe.id,
                recipe.morph_weights.len(),
                recipe.geometry
            ),
            "emit exactly one morph weight per target",
        )));
    }
    Ok(())
}

fn scene_skin_binding(
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeNodeV1,
    binding: &crate::SceneRecipeNodeSkinBindingV1,
    geometries: &BTreeMap<String, GeometryHandle>,
    node_keys: &BTreeMap<String, NodeKey>,
) -> Result<SceneSkinBinding, Box<SceneRecipeDiagnosticV1>> {
    let Some(handle) = geometries.get(&recipe.geometry).copied() else {
        return Err(Box::new(error_diagnostic(
            "$",
            "unknown_geometry_ref",
            format!(
                "node '{}' references missing geometry '{}'",
                recipe.id, recipe.geometry
            ),
            "declare the geometry before the node",
        )));
    };
    let Some(geometry) = host.assets.geometry(handle) else {
        return Err(Box::new(error_diagnostic(
            "$",
            "geometry_missing",
            format!(
                "node '{}' geometry '{}' could not be resolved",
                recipe.id, recipe.geometry
            ),
            "declare a valid geometry before the node",
        )));
    };
    let Some(skin) = geometry.skin() else {
        return Err(Box::new(error_diagnostic(
            "$",
            "invalid_skin",
            format!(
                "node '{}' declares skin_binding for non-skinned geometry '{}'",
                recipe.id, recipe.geometry
            ),
            "remove skin_binding or use a skin-derived geometry",
        )));
    };
    let binding_nodes = binding
        .binding_nodes()
        .iter()
        .map(|node_id| {
            node_keys.get(node_id).copied().ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "unknown_node_ref",
                    format!("skin_binding references unknown node '{node_id}'"),
                    "target an authored node id",
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if skin
        .influence_indices()
        .iter()
        .flatten()
        .any(|influence| *influence >= binding_nodes.len())
    {
        return Err(Box::new(error_diagnostic(
            "$",
            "invalid_skin",
            format!(
                "node '{}' skin geometry '{}' references an influence index outside its {}-node binding",
                recipe.id,
                recipe.geometry,
                binding_nodes.len()
            ),
            "make skin influence indices reference entries in skin_binding",
        )));
    }
    let matrices = binding
        .inverse_bind_matrices
        .iter()
        .map(|values| SkinningMatrix::from_gltf_column_major(values.map(|value| value as f32)))
        .collect();
    Ok(SceneSkinBinding::new(binding_nodes, matrices))
}
