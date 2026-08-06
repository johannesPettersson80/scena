use std::collections::BTreeMap;

mod regular;
mod texture_slots;

use super::common::{DiagnosticPathExt, authored_color};
use crate::assets::{
    AssetPath, DefaultAssetFetcher, MaterialImperfectionDesc, PhotographicSurfaceDesc,
    PhotographicSurfaceKind,
};
use crate::material::{AlphaMode, MaterialDesc, MaterialKind, TextureColorSpace};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeAlphaModeV1, SceneRecipeBuildResourceV1, SceneRecipeColorV1,
    SceneRecipeDiagnosticV1, SceneRecipeMaterialV1, SceneRecipeTextureSlotV1,
};
use crate::scene_host::SceneHostCore;
use crate::{Color, MaterialHandle};

use self::texture_slots::load_texture_slot;
use super::super::error_diagnostic;
use super::super::policy::{RecipeBuildBudget, RecipeTextureBudget};
use regular::authored_material;

pub(in crate::scene_host::recipe) async fn build_authored_materials(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    recipes: &[SceneRecipeMaterialV1],
    resources: AuthoredMaterialResources<'_>,
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, MaterialHandle> {
    let mut handles = BTreeMap::new();
    let AuthoredMaterialResources {
        colors,
        build_budget,
        texture_budget,
    } = resources;
    if recipes.len() > policy.max_materials() {
        diagnostics.push(error_diagnostic(
            "$.materials",
            "policy_violation",
            format!(
                "recipe declares {} authored materials, exceeding RecipeBuildPolicy max_materials {}",
                recipes.len(),
                policy.max_materials()
            ),
            "reduce material count or raise the operator-owned max_materials policy",
        ));
        return handles;
    }
    if let Some(diagnostic) = build_budget.reserve_materials(policy, "$.materials", recipes.len()) {
        diagnostics.push(diagnostic);
        return handles;
    }
    let mut material_context = MaterialRecipeBuildRefs {
        colors,
        texture_budget,
    };
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.materials[{index}]");
        let base_color = match recipe.base_color.as_deref() {
            Some(base_color) => match authored_color(material_context.colors, base_color) {
                Ok(color) => Some(color),
                Err(diagnostic) => {
                    diagnostics.push((*diagnostic).with_path(format!("{path}.base_color")));
                    continue;
                }
            },
            None => None,
        };
        let (kind, handle) = if recipe.material_pack.is_some() {
            match authored_material_pack(
                policy,
                host,
                recipe_path,
                recipe,
                base_color,
                &path,
                &mut material_context,
            )
            .await
            {
                Ok(value) => value,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            }
        } else if recipe.photographic_surface.is_some() {
            match authored_photographic_surface(
                policy,
                host,
                recipe,
                base_color,
                &path,
                &mut material_context,
            ) {
                Ok(value) => value,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            }
        } else {
            let (kind, material) = match authored_material(
                policy,
                host,
                recipe_path,
                recipe,
                base_color,
                &path,
                &mut material_context,
            )
            .await
            {
                Ok(value) => value,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            };
            (kind, host.assets.create_material(material))
        };
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind: kind.to_owned(),
            vertex_count: None,
            index_count: None,
        });
    }
    handles
}

async fn authored_material_pack(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    recipe: &SceneRecipeMaterialV1,
    base_color: Option<Color>,
    path: &str,
    resources: &mut MaterialRecipeBuildRefs<'_>,
) -> Result<(String, MaterialHandle), Box<SceneRecipeDiagnosticV1>> {
    let pack_ref = recipe
        .material_pack
        .as_ref()
        .expect("caller checks material_pack");
    let resolved = policy.resolve_import_uri(
        recipe_path,
        &pack_ref.uri,
        format!("{path}.material_pack.uri"),
    )?;
    let loaded = host
        .assets
        .load_photographic_material_pack(AssetPath::from(resolved.as_str()))
        .await
        .map_err(|error| {
            Box::new(error_diagnostic(
                format!("{path}.material_pack.uri"),
                "material_pack_load_failed",
                format!("scena could not load the photographic material pack: {error}"),
                "compile or fetch the pack with `scena materials`, then reference its scena-material-pack.json",
            ))
        })?;
    if let Some(expected) = pack_ref.expected_archive_sha256.as_deref()
        && !expected.eq_ignore_ascii_case(&loaded.pack().source.archive_sha256)
    {
        return Err(Box::new(error_diagnostic(
            format!("{path}.material_pack.expected_archive_sha256"),
            "material_pack_source_sha256_mismatch",
            format!(
                "material pack archive SHA-256 is {}, but the recipe pins {expected}",
                loaded.pack().source.archive_sha256
            ),
            "review the source change, then update the recipe lock intentionally",
        )));
    }
    let texture_resources = loaded.pack().maps.iter().map(|map| {
        (
            loaded.texture_resource_identity(map),
            usize::try_from(map.width)
                .unwrap_or(usize::MAX)
                .saturating_mul(usize::try_from(map.height).unwrap_or(usize::MAX))
                .saturating_mul(4),
        )
    });
    if let Some(diagnostic) = resources.texture_budget.reserve_loaded_texture_resources(
        policy,
        &format!("{path}.material_pack"),
        texture_resources,
    ) {
        return Err(Box::new(diagnostic));
    }
    let mut material = host
        .assets
        .try_material(loaded.material())
        .map_err(|error| {
            Box::new(error_diagnostic(
                format!("{path}.material_pack"),
                "material_pack_load_failed",
                format!("loaded material pack did not resolve its material: {error}"),
                "recompile the material pack with the current scena version",
            ))
        })?;
    if let Some(color) = base_color {
        material = material.with_base_color(color);
    }
    if let Some(tile_size_m) = pack_ref.tile_size_m {
        material = material.with_photographic_surface_tile_size_m(tile_size_m as f32);
    }
    material = material.with_double_sided(recipe.double_sided);
    if let Some(imperfection) = recipe.imperfection.as_ref() {
        let decoded_bytes = [
            imperfection
                .profile
                .replaces_normal_texture()
                .then_some(loaded.normal_texture()),
            Some(loaded.metallic_roughness_texture()),
        ]
        .into_iter()
        .flatten()
        .filter_map(|texture| host.assets.texture(texture)?.decoded_dimensions())
        .map(|(width, height)| {
            usize::try_from(width)
                .unwrap_or(usize::MAX)
                .saturating_mul(usize::try_from(height).unwrap_or(usize::MAX))
                .saturating_mul(4)
        })
        .sum();
        if let Some(diagnostic) = resources.texture_budget.reserve_loaded_textures(
            policy,
            &format!("{path}.imperfection"),
            imperfection.profile.replacement_texture_count(),
            decoded_bytes,
        ) {
            return Err(Box::new(diagnostic));
        }
        material = host
            .assets
            .composite_material_imperfection(
                material,
                MaterialImperfectionDesc::new(imperfection.profile)
                    .with_strength(imperfection.strength as f32)
                    .with_physical_scale_m(imperfection.physical_scale_m as f32)
                    .with_seed(imperfection.seed),
            )
            .map_err(|error| {
                Box::new(error_diagnostic(
                    format!("{path}.imperfection"),
                    "material_imperfection_generation_failed",
                    format!("failed to composite material imperfection: {error}"),
                    "use a decoded material pack with normal and roughness maps",
                ))
            })?;
    }
    let material = host
        .assets
        .create_photographic_material_pack_derivative(loaded.material(), material)
        .map_err(|error| {
            Box::new(error_diagnostic(
                format!("{path}.material_pack"),
                "material_pack_load_failed",
                format!("failed to preserve material-pack resolution identity: {error}"),
                "recompile the material pack with the current scena version",
            ))
        })?;
    Ok(("pbr_metallic_roughness".to_string(), material))
}

pub(in crate::scene_host::recipe) struct AuthoredMaterialResources<'a> {
    pub(in crate::scene_host::recipe) colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    pub(in crate::scene_host::recipe) build_budget: &'a mut RecipeBuildBudget,
    pub(in crate::scene_host::recipe) texture_budget: &'a mut RecipeTextureBudget,
}

struct MaterialRecipeBuildRefs<'a> {
    colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    texture_budget: &'a mut RecipeTextureBudget,
}

fn authored_photographic_surface(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe: &SceneRecipeMaterialV1,
    base_color: Option<Color>,
    path: &str,
    resources: &mut MaterialRecipeBuildRefs<'_>,
) -> Result<(String, MaterialHandle), Box<SceneRecipeDiagnosticV1>> {
    let surface = recipe
        .photographic_surface
        .as_ref()
        .expect("caller checks photographic_surface");
    let base_color = base_color.ok_or_else(|| {
        Box::new(error_diagnostic(
            format!("{path}.base_color"),
            "missing_base_color",
            "photographic surface base_color must be a color id, named Color constant, or #RRGGBB string",
            "provide base_color for the generated surface",
        ))
    })?;
    let kind = PhotographicSurfaceKind::from_name(&surface.kind).ok_or_else(|| {
        Box::new(error_diagnostic(
            format!("{path}.photographic_surface.kind"),
            "invalid_photographic_surface_kind",
            format!(
                "photographic surface kind '{}' is not supported",
                surface.kind
            ),
            format!("use one of: {}", PhotographicSurfaceKind::NAMES.join(", ")),
        ))
    })?;
    let resolution = surface.resolution.unwrap_or(256);
    let decoded_bytes = usize::try_from(resolution)
        .unwrap_or(usize::MAX)
        .saturating_mul(usize::try_from(resolution).unwrap_or(usize::MAX))
        .saturating_mul(4)
        .saturating_mul(3);
    if let Some(diagnostic) = resources.texture_budget.reserve_loaded_textures(
        policy,
        &format!("{path}.photographic_surface"),
        3,
        decoded_bytes,
    ) {
        return Err(Box::new(diagnostic));
    }

    let mut descriptor = PhotographicSurfaceDesc::new(kind, base_color)
        .with_seed(surface.seed.unwrap_or(0))
        .with_resolution(resolution);
    if let Some(value) = surface.tile_size_m {
        descriptor = descriptor.with_tile_size_m(value as f32);
    }
    if let Some(value) = surface.feature_scale_m {
        descriptor = descriptor.with_feature_scale_m(value as f32);
    }
    if let Some(value) = surface.metallic {
        descriptor = descriptor.with_metallic(value as f32);
    }
    if let Some(value) = surface.roughness {
        descriptor = descriptor.with_roughness(value as f32);
    }
    if let Some(value) = surface.variation {
        descriptor = descriptor.with_variation(value as f32);
    }
    if let Some(value) = surface.wear {
        descriptor = descriptor.with_wear(value as f32);
    }
    let generated = host
        .assets
        .create_photographic_surface(descriptor)
        .map_err(|error| {
            Box::new(error_diagnostic(
                format!("{path}.photographic_surface"),
                "photographic_surface_generation_failed",
                format!("scena could not generate the photographic surface: {error}"),
                "reduce resolution or correct the physical surface descriptor",
            ))
        })?;
    if recipe.double_sided || recipe.imperfection.is_some() {
        let mut material = host
            .assets
            .material(generated.material())
            .expect("generated material handle resolves")
            .with_double_sided(recipe.double_sided);
        if let Some(imperfection) = recipe.imperfection.as_ref() {
            let derived_bytes = usize::try_from(resolution)
                .unwrap_or(usize::MAX)
                .saturating_mul(usize::try_from(resolution).unwrap_or(usize::MAX))
                .saturating_mul(4)
                .saturating_mul(imperfection.profile.replacement_texture_count());
            if let Some(diagnostic) = resources.texture_budget.reserve_loaded_textures(
                policy,
                &format!("{path}.imperfection"),
                imperfection.profile.replacement_texture_count(),
                derived_bytes,
            ) {
                return Err(Box::new(diagnostic));
            }
            material = host
                .assets
                .composite_material_imperfection(
                    material,
                    MaterialImperfectionDesc::new(imperfection.profile)
                        .with_strength(imperfection.strength as f32)
                        .with_physical_scale_m(imperfection.physical_scale_m as f32)
                        .with_seed(imperfection.seed),
                )
                .map_err(|error| {
                    Box::new(error_diagnostic(
                        format!("{path}.imperfection"),
                        "material_imperfection_generation_failed",
                        format!("failed to composite material imperfection: {error}"),
                        "use a generated photographic surface with normal and roughness maps",
                    ))
                })?;
        }
        return Ok((
            "pbr_metallic_roughness".to_string(),
            host.assets.create_material(material),
        ));
    }
    Ok(("pbr_metallic_roughness".to_string(), generated.material()))
}

#[cfg(test)]
mod tests;
