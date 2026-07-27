use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color, vec3};
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeClippingPlaneV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeInstanceSetV1, SceneRecipeLabelV1,
};
use crate::scene_host::SceneHostCore;
use crate::{
    ClippingPlane, ClippingPlaneSet, GeometryHandle, LabelDesc, LabelFontFace, MaterialHandle,
    NodeKey,
};

use super::super::error_diagnostic;
use super::super::policy::RecipeBuildBudget;

pub(in crate::scene_host::recipe) fn build_authored_instance_sets(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeInstanceSetV1],
    resources: InstanceSetResources<'_>,
    manifest: &mut Vec<crate::SceneRecipeBuildTargetV1>,
    instance_manifest: &mut Vec<crate::SceneRecipeBuildInstanceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, NodeKey> {
    let mut node_keys = BTreeMap::new();
    let requested_instances = recipes
        .iter()
        .map(|recipe| recipe.instances.len())
        .sum::<usize>();
    if requested_instances > policy.max_instances() {
        diagnostics.push(error_diagnostic(
            "$.instance_sets",
            "policy_violation",
            format!(
                "recipe declares {requested_instances} instances, exceeding RecipeBuildPolicy max_instances {}",
                policy.max_instances()
            ),
            "reduce instance count or raise the operator-owned max_instances policy",
        ));
        return node_keys;
    }
    if let Some(diagnostic) =
        resources
            .build_budget
            .reserve_instances(policy, "$.instance_sets", requested_instances)
    {
        diagnostics.push(diagnostic);
        return node_keys;
    }

    let root = host.scene.root();
    let root_handle = host.root_handle();
    let mut transform_nodes = resources.imported_nodes.clone();
    transform_nodes.extend(resources.nodes.clone());
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.instance_sets[{index}]");
        let Some(geometry) = resources.geometries.get(&recipe.geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_geometry_ref",
                format!(
                    "instance set '{}' references missing geometry '{}'",
                    recipe.id, recipe.geometry
                ),
                "declare the geometry before the instance set",
            ));
            continue;
        };
        let Some(material) = resources.materials.get(&recipe.material).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_material_ref",
                format!(
                    "instance set '{}' references missing material '{}'",
                    recipe.id, recipe.material
                ),
                "declare the material before the instance set",
            ));
            continue;
        };
        let parent = match &recipe.parent {
            Some(parent) => match transform_nodes.get(parent).copied() {
                Some(parent) => parent,
                None => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "unknown_node_ref",
                        format!(
                            "instance set '{}' references missing parent '{}'",
                            recipe.id, parent
                        ),
                        "parent instance sets under authored nodes or imported node paths declared earlier",
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
                    format!(
                        "instance set '{}' geometry could not be resolved",
                        recipe.id
                    ),
                    "declare a valid geometry before the instance set",
                ));
                continue;
            }
        };
        let root_transform = match transform_from_recipe(
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
        let (node, set) =
            match host
                .scene
                .add_instance_set_node(parent, geometry, material, root_transform)
            {
                Ok(value) => value,
                Err(error) => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "instance_set_create_failed",
                        format!("failed to create instance set '{}': {error}", recipe.id),
                        "check the parent, geometry, and material references",
                    ));
                    continue;
                }
            };
        let handle = host.register_node(node);
        for (instance_index, instance) in recipe.instances.iter().enumerate() {
            let instance_path = format!("{path}.instances[{instance_index}]");
            let transform = match transform_from_recipe(
                instance.transform.as_ref(),
                TransformResolutionInput {
                    node_keys: &transform_nodes,
                    imports: resources.imports,
                    parent: Some(node),
                    current_bounds: geometry_bounds,
                },
                host,
            ) {
                Ok(transform) => transform,
                Err(diagnostic) => {
                    diagnostics.push((*diagnostic).with_path(format!("{instance_path}.transform")));
                    continue;
                }
            };
            let instance_id = match host.scene.push_instance(set, transform) {
                Ok(instance) => instance,
                Err(error) => {
                    diagnostics.push(error_diagnostic(
                        &instance_path,
                        "instance_create_failed",
                        format!("failed to create instance '{}': {error}", instance.id),
                        "check the instance set and transform",
                    ));
                    continue;
                }
            };
            instance_manifest.push(crate::SceneRecipeBuildInstanceV1 {
                set_id: recipe.id.clone(),
                id: instance.id.clone(),
                set_handle: handle,
                instance_id: instance_id.as_u64(),
                identity_scope: "runtime_scoped".to_owned(),
            });
            if let Some(visible) = instance.visible
                && let Err(error) = host.scene.set_instance_visible(set, instance_id, visible)
            {
                diagnostics.push(error_diagnostic(
                    &instance_path,
                    "instance_visible_failed",
                    error.to_string(),
                    "check the instance reference",
                ));
            }
            if let Some(tint) = &instance.tint {
                match authored_color(resources.colors, tint) {
                    Ok(tint) => {
                        if let Err(error) =
                            host.scene.set_instance_tint(set, instance_id, Some(tint))
                        {
                            diagnostics.push(error_diagnostic(
                                &instance_path,
                                "instance_tint_failed",
                                error.to_string(),
                                "check the instance reference",
                            ));
                        }
                    }
                    Err(diagnostic) => {
                        diagnostics.push((*diagnostic).with_path(format!("{instance_path}.tint")));
                    }
                }
            }
        }
        node_keys.insert(recipe.id.clone(), node);
        manifest.push(crate::SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "instance_set".to_owned(),
            parent: Some(
                host.node_handle_map
                    .get(&parent)
                    .copied()
                    .unwrap_or(root_handle),
            ),
            name: None,
            visible: Some(true),
            active: None,
        });
    }
    node_keys
}

pub(in crate::scene_host::recipe) struct InstanceSetResources<'a> {
    pub(in crate::scene_host::recipe) colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    pub(in crate::scene_host::recipe) geometries: &'a BTreeMap<String, GeometryHandle>,
    pub(in crate::scene_host::recipe) materials: &'a BTreeMap<String, MaterialHandle>,
    pub(in crate::scene_host::recipe) nodes: &'a BTreeMap<String, NodeKey>,
    pub(in crate::scene_host::recipe) imported_nodes: &'a BTreeMap<String, NodeKey>,
    pub(in crate::scene_host::recipe) imports: &'a BTreeMap<String, u64>,
    pub(in crate::scene_host::recipe) build_budget: &'a mut RecipeBuildBudget,
}

pub(in crate::scene_host::recipe) fn build_authored_labels(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeLabelV1],
    resources: LabelResources<'_>,
    manifest: &mut Vec<crate::SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, NodeKey> {
    let mut label_nodes = BTreeMap::new();
    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.labels[{index}]");
        let parent = match &recipe.parent {
            Some(parent) => match resources.nodes.get(parent).copied() {
                Some(parent) => parent,
                None => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "unknown_node_ref",
                        format!(
                            "label '{}' references missing parent '{}'",
                            recipe.id, parent
                        ),
                        "parent labels under authored nodes or instance sets declared earlier",
                    ));
                    continue;
                }
            },
            None => root,
        };
        let transform = match transform_from_recipe(
            recipe.transform.as_ref(),
            TransformResolutionInput {
                node_keys: resources.nodes,
                imports: resources.imports,
                parent: Some(parent),
                current_bounds: None,
            },
            host,
        ) {
            Ok(transform) => transform,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.transform")));
                continue;
            }
        };
        let label = match label_desc_from_recipe(recipe, resources.colors, resources.fonts, &path) {
            Ok(label) => label,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                continue;
            }
        };
        let (_label, node) = match host.scene.add_label_node(parent, label, transform) {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "label_create_failed",
                    format!("failed to create label '{}': {error}", recipe.id),
                    "check the label parent and transform",
                ));
                continue;
            }
        };
        let handle = host.register_node(node);
        label_nodes.insert(recipe.id.clone(), node);
        manifest.push(crate::SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "label".to_owned(),
            parent: Some(
                host.node_handle_map
                    .get(&parent)
                    .copied()
                    .unwrap_or(root_handle),
            ),
            name: Some(recipe.text.clone()),
            visible: Some(true),
            active: None,
        });
    }
    label_nodes
}

pub(in crate::scene_host::recipe) struct LabelResources<'a> {
    pub(in crate::scene_host::recipe) colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    pub(in crate::scene_host::recipe) fonts: &'a BTreeMap<String, LabelFontFace>,
    pub(in crate::scene_host::recipe) nodes: &'a BTreeMap<String, NodeKey>,
    pub(in crate::scene_host::recipe) imports: &'a BTreeMap<String, u64>,
}

fn label_desc_from_recipe(
    recipe: &SceneRecipeLabelV1,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    fonts: &BTreeMap<String, LabelFontFace>,
    path: &str,
) -> Result<LabelDesc, Box<SceneRecipeDiagnosticV1>> {
    let mut label = if let Some(font_id) = &recipe.font {
        let Some(font) = fonts.get(font_id).cloned() else {
            return Err(Box::new(error_diagnostic(
                format!("{path}.font"),
                "unknown_font_ref",
                format!(
                    "label '{}' references missing font '{}'",
                    recipe.id, font_id
                ),
                "declare and successfully load the font before referencing it",
            )));
        };
        LabelDesc::truetype_face(recipe.text.clone(), font).map_err(|error| {
            Box::new(error_diagnostic(
                format!("{path}.text"),
                "unsupported_feature",
                error.to_string(),
                "use basic Latin text for font-backed labels",
            ))
        })?
    } else {
        LabelDesc::new(recipe.text.clone())
    };
    if let Some(size) = recipe.size_px {
        if !size.is_finite() || size <= 0.0 {
            return Err(Box::new(error_diagnostic(
                format!("{path}.size_px"),
                "invalid_label",
                "label size_px must be finite and positive",
                "use a readable pixel size",
            )));
        }
        label = label.with_size(size as f32);
    }
    if let Some(color) = &recipe.color {
        label = label
            .with_color(authored_color(colors, color).map_err(|diagnostic| {
                Box::new((*diagnostic).with_path(format!("{path}.color")))
            })?);
    }
    if let Some(background) = &recipe.background {
        label =
            label.with_background(authored_color(colors, background).map_err(|diagnostic| {
                Box::new((*diagnostic).with_path(format!("{path}.background")))
            })?);
    }
    if let Some(halo) = &recipe.halo {
        label = label.with_halo(
            authored_color(colors, halo)
                .map_err(|diagnostic| Box::new((*diagnostic).with_path(format!("{path}.halo"))))?,
        );
    }
    Ok(label)
}

pub(in crate::scene_host::recipe) fn build_authored_clipping_planes(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeClippingPlaneV1],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let active_count = recipes.iter().filter(|plane| plane.active).count();
    let max_planes = host.recipe_max_clipping_planes();
    if active_count > max_planes {
        diagnostics.push(error_diagnostic(
            "$.clipping_planes",
            "policy_violation",
            format!(
                "recipe activates {active_count} clipping planes, exceeding renderer max_clipping_planes {max_planes}"
            ),
            "reduce active clipping planes or target a backend with a higher clipping-plane limit",
        ));
        return;
    }

    let mut set = ClippingPlaneSet::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.clipping_planes[{index}]");
        let normal = vec3(recipe.normal);
        let length_sq = normal.length_squared();
        if !length_sq.is_finite() || length_sq <= f32::EPSILON || !recipe.distance.is_finite() {
            diagnostics.push(error_diagnostic(
                &path,
                "invalid_clipping_plane",
                format!(
                    "clipping plane '{}' has invalid normal or distance",
                    recipe.id
                ),
                "use a finite non-zero normal and finite distance",
            ));
            continue;
        }
        let plane = host.scene.add_clipping_plane(ClippingPlane::new(
            normal.normalize(),
            recipe.distance as f32,
        ));
        if recipe.active {
            set = set.with_plane(plane);
        }
    }
    if !set.planes().is_empty()
        && let Err(error) = host.scene.set_clipping_planes(set)
    {
        diagnostics.push(error_diagnostic(
            "$.clipping_planes",
            "clipping_planes_failed",
            error.to_string(),
            "check clipping plane handles",
        ));
    }
}
