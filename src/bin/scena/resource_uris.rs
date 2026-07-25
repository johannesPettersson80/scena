//! Rewrites a recipe's resource URIs relative to the recipe's own location.
//!
//! `place` reads a recipe from one directory and may emit a patch consumed from
//! another, so every relative `uri` has to be resolved against the recipe path
//! before the runtime loads it. Extracted from `place.rs` to keep that file
//! under the ARCH-KISS-SIZE cap; the logic is unchanged.

use super::super::scena_input::resolve_recipe_asset_uri;

pub(super) fn rebase_recipe_resource_uris(recipe_path: &str, recipe: &mut scena::SceneRecipeV1) {
    for import in &mut recipe.imports {
        rebase_resource_uri(recipe_path, &mut import.uri);
    }
    for font in &mut recipe.fonts {
        rebase_resource_uri(recipe_path, &mut font.uri);
    }
    if let Some(uri) = recipe
        .scene
        .as_mut()
        .and_then(|scene| scene.environment.as_mut())
        .and_then(|environment| environment.uri.as_mut())
    {
        rebase_resource_uri(recipe_path, uri);
    }
    for material in &mut recipe.materials {
        rebase_texture_slot(recipe_path, &mut material.base_color_texture);
        rebase_texture_slot(recipe_path, &mut material.normal_texture);
        rebase_texture_slot(recipe_path, &mut material.metallic_roughness_texture);
        rebase_texture_slot(recipe_path, &mut material.occlusion_texture);
        rebase_texture_slot(recipe_path, &mut material.emissive_texture);
        rebase_texture_slot(recipe_path, &mut material.clearcoat_texture);
        rebase_texture_slot(recipe_path, &mut material.clearcoat_roughness_texture);
        rebase_texture_slot(recipe_path, &mut material.clearcoat_normal_texture);
        rebase_texture_slot(recipe_path, &mut material.sheen_color_texture);
        rebase_texture_slot(recipe_path, &mut material.sheen_roughness_texture);
        rebase_texture_slot(recipe_path, &mut material.anisotropy_texture);
        rebase_texture_slot(recipe_path, &mut material.iridescence_texture);
        rebase_texture_slot(recipe_path, &mut material.iridescence_thickness_texture);
        rebase_texture_slot(recipe_path, &mut material.transmission_texture);
        rebase_texture_slot(recipe_path, &mut material.thickness_texture);
    }
}

fn rebase_texture_slot(recipe_path: &str, slot: &mut Option<scena::SceneRecipeTextureSlotV1>) {
    if let Some(slot) = slot {
        rebase_resource_uri(recipe_path, &mut slot.uri);
    }
}

fn rebase_resource_uri(recipe_path: &str, uri: &mut String) {
    let resolved = resolve_recipe_asset_uri(recipe_path, uri);
    *uri = if resolved.contains("://") || resolved.starts_with("data:") {
        resolved
    } else {
        std::fs::canonicalize(&resolved)
            .map(|path| path.display().to_string())
            .unwrap_or(resolved)
    };
}
