use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color};
use crate::assets::DefaultAssetFetcher;
use crate::material::{AlphaMode, MaterialDesc, TextureColorSpace};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeAlphaModeV1, SceneRecipeBuildResourceV1, SceneRecipeColorV1,
    SceneRecipeDiagnosticV1, SceneRecipeMaterialV1, SceneRecipeTextureColorSpaceV1,
    SceneRecipeTextureSlotV1,
};
use crate::scene_host::SceneHostCore;
use crate::{AssetPath, Color, MaterialHandle};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) async fn build_authored_materials(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipes: &[SceneRecipeMaterialV1],
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, MaterialHandle> {
    let mut handles = BTreeMap::new();
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
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.materials[{index}]");
        let base_color = match authored_color(colors, &recipe.base_color) {
            Ok(color) => color,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.base_color")));
                continue;
            }
        };
        let (kind, material) =
            match authored_material(policy, host, recipe_path, colors, recipe, base_color, &path)
                .await
            {
                Ok(value) => value,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            };
        let handle = host.assets.create_material(material);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind,
            vertex_count: None,
            index_count: None,
        });
    }
    handles
}

async fn authored_material(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipe: &SceneRecipeMaterialV1,
    base_color: Color,
    path: &str,
) -> Result<(String, MaterialDesc), Box<SceneRecipeDiagnosticV1>> {
    let (kind, mut material) = match recipe.kind.as_str() {
        "unlit" => ("unlit", MaterialDesc::unlit(base_color)),
        "pbr_metallic_roughness" => (
            "pbr_metallic_roughness",
            MaterialDesc::pbr_metallic_roughness(
                base_color,
                recipe.metallic.unwrap_or(0.0) as f32,
                recipe.roughness.unwrap_or(1.0) as f32,
            ),
        ),
        "line" => (
            "line",
            MaterialDesc::line(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32),
        ),
        "wireframe" => (
            "wireframe",
            MaterialDesc::wireframe(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32),
        ),
        "edge" => {
            let mut material =
                MaterialDesc::edge(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32);
            if let Some(threshold) = recipe.edge_angle_threshold_degrees {
                material = material.with_edge_angle_threshold_degrees(threshold as f32);
            }
            ("edge", material)
        }
        kind => {
            return Err(Box::new(error_diagnostic(
                path,
                "unsupported_feature",
                format!("material kind '{kind}' is not implemented in this slice"),
                "use kind:\"unlit\", \"pbr_metallic_roughness\", \"line\", \"wireframe\", or \"edge\"",
            )));
        }
    };
    material = material.with_double_sided(recipe.double_sided);
    if let Some(emissive) = &recipe.emissive {
        material =
            material.with_emissive(authored_color(colors, emissive).map_err(|diagnostic| {
                Box::new((*diagnostic).with_path(format!("{path}.emissive")))
            })?);
    }
    if let Some(strength) = recipe.emissive_strength {
        material = material.with_emissive_strength(strength as f32);
    }
    if let Some(alpha_mode) = &recipe.alpha_mode {
        material = material.with_alpha_mode(match alpha_mode {
            SceneRecipeAlphaModeV1::Opaque => AlphaMode::Opaque,
            SceneRecipeAlphaModeV1::Mask { cutoff } => AlphaMode::Mask {
                cutoff: *cutoff as f32,
            },
            SceneRecipeAlphaModeV1::Blend => AlphaMode::Blend,
        });
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.base_color_texture.as_ref(),
        &format!("{path}.base_color_texture"),
        TextureColorSpace::Srgb,
    )
    .await?
    {
        material = material.with_base_color_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.normal_texture.as_ref(),
        &format!("{path}.normal_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_normal_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.metallic_roughness_texture.as_ref(),
        &format!("{path}.metallic_roughness_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_metallic_roughness_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.occlusion_texture.as_ref(),
        &format!("{path}.occlusion_texture"),
        TextureColorSpace::Linear,
    )
    .await?
    {
        material = material.with_occlusion_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.emissive_texture.as_ref(),
        &format!("{path}.emissive_texture"),
        TextureColorSpace::Srgb,
    )
    .await?
    {
        material = material.with_emissive_texture(texture);
    }
    Ok((kind.to_owned(), material))
}

async fn load_texture_slot(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    slot: Option<&SceneRecipeTextureSlotV1>,
    path: &str,
    default_color_space: TextureColorSpace,
) -> Result<Option<crate::TextureHandle>, Box<SceneRecipeDiagnosticV1>> {
    let Some(slot) = slot else {
        return Ok(None);
    };
    let color_space = match slot.color_space {
        Some(SceneRecipeTextureColorSpaceV1::Srgb) => TextureColorSpace::Srgb,
        Some(SceneRecipeTextureColorSpaceV1::Linear) => TextureColorSpace::Linear,
        None => default_color_space,
    };
    let resolved = policy.resolve_import_uri(recipe_path, &slot.uri, format!("{path}.uri"))?;
    match host
        .assets
        .load_texture(AssetPath::from(resolved.as_str()), color_space)
        .await
    {
        Ok(texture) => Ok(Some(texture)),
        Err(error) if slot.optional => Ok(None),
        Err(error) => Err(Box::new(error_diagnostic(
            path,
            "texture_load_failed",
            format!(
                "required texture '{}' could not be loaded: {error}",
                slot.uri
            ),
            "fix the texture uri or mark the texture slot optional only if the fallback is acceptable",
        ))),
    }
}
