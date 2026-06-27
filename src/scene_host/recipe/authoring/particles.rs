use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color, vec3};
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeParticleSetV1,
};
use crate::scene_host::SceneHostCore;
use crate::{NodeKey, Particle, ParticleSet};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_particle_sets(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeParticleSetV1],
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    resources: ParticleSetResources<'_>,
    manifest: &mut Vec<crate::SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, NodeKey> {
    let mut node_keys = BTreeMap::new();
    let requested_particles = recipes
        .iter()
        .map(|recipe| recipe.particles.len())
        .sum::<usize>();
    if requested_particles > policy.max_particles() {
        diagnostics.push(error_diagnostic(
            "$.particles",
            "policy_violation",
            format!(
                "recipe declares {requested_particles} particles, exceeding RecipeBuildPolicy max_particles {}",
                policy.max_particles()
            ),
            "reduce particle count or raise the operator-owned max_particles policy",
        ));
        return node_keys;
    }

    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.particles[{index}]");
        let particle_set = match particle_set_from_recipe(recipe, colors, &path) {
            Ok(particles) => particles,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                continue;
            }
        };
        let parent = match &recipe.parent {
            Some(parent) => match resources.nodes.get(parent).copied() {
                Some(parent) => parent,
                None => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "unknown_node_ref",
                        format!(
                            "particle set '{}' references missing parent '{}'",
                            recipe.id, parent
                        ),
                        "parent particle sets under authored nodes declared earlier",
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
                current_bounds: Some(particle_set.bounds()),
            },
            host,
        ) {
            Ok(transform) => transform,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.transform")));
                continue;
            }
        };
        let (node, _set) = match host
            .scene
            .add_particle_set_node(parent, particle_set, transform)
        {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "particle_set_create_failed",
                    format!("failed to create particle set '{}': {error}", recipe.id),
                    "check the parent and particle buffer",
                ));
                continue;
            }
        };
        apply_particle_set_attributes(host, recipe, node, &path, diagnostics);
        let handle = host.register_node(node);
        node_keys.insert(recipe.id.clone(), node);
        manifest.push(crate::SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "particle_set".to_owned(),
            parent: Some(
                host.node_handle_map
                    .get(&parent)
                    .copied()
                    .unwrap_or(root_handle),
            ),
            name: None,
            active: None,
        });
    }
    node_keys
}

pub(in crate::scene_host::recipe) struct ParticleSetResources<'a> {
    pub(in crate::scene_host::recipe) nodes: &'a BTreeMap<String, NodeKey>,
    pub(in crate::scene_host::recipe) imports: &'a BTreeMap<String, u64>,
}

fn particle_set_from_recipe(
    recipe: &SceneRecipeParticleSetV1,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    path: &str,
) -> Result<ParticleSet, Box<SceneRecipeDiagnosticV1>> {
    let mut particles = Vec::with_capacity(recipe.particles.len());
    for (index, particle) in recipe.particles.iter().enumerate() {
        let particle_path = format!("{path}.particles[{index}]");
        let color = authored_color(colors, &particle.color).map_err(|diagnostic| {
            Box::new((*diagnostic).with_path(format!("{particle_path}.color")))
        })?;
        let mut authored = Particle::new(vec3(particle.position), color, particle.size_px as f32);
        if let Some(rotation_degrees) = particle.rotation_degrees {
            authored = authored.with_rotation_radians((rotation_degrees as f32).to_radians());
        }
        particles.push(authored);
    }
    ParticleSet::try_new(particles).map_err(|error| {
        Box::new(error_diagnostic(
            path,
            "invalid_particle",
            format!("particle set '{}' is invalid: {error}", recipe.id),
            "emit finite positions, finite colors, positive size_px, and finite rotations",
        ))
    })
}

fn apply_particle_set_attributes(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeParticleSetV1,
    node: NodeKey,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(visible) = recipe.visible
        && let Err(error) = host.scene.set_visible(node, visible)
    {
        diagnostics.push(error_diagnostic(
            path,
            "particle_set_visible_failed",
            error.to_string(),
            "check the particle set node reference",
        ));
    }
    if let Some(mask) = recipe.layer_mask
        && let Err(error) = host.scene.set_layer_mask(node, mask)
    {
        diagnostics.push(error_diagnostic(
            path,
            "particle_set_layer_mask_failed",
            error.to_string(),
            "check the particle set node reference",
        ));
    }
    if let Some(group) = recipe.render_group
        && let Err(error) = host.scene.set_render_group(node, group)
    {
        diagnostics.push(error_diagnostic(
            path,
            "particle_set_render_group_failed",
            error.to_string(),
            "check the particle set node reference",
        ));
    }
}
