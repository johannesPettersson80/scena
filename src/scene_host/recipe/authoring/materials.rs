use std::collections::BTreeMap;

mod texture_slots;

use super::common::{DiagnosticPathExt, authored_color};
use crate::assets::DefaultAssetFetcher;
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
        let handle = host.assets.create_material(material);
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

pub(in crate::scene_host::recipe) struct AuthoredMaterialResources<'a> {
    pub(in crate::scene_host::recipe) colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    pub(in crate::scene_host::recipe) build_budget: &'a mut RecipeBuildBudget,
    pub(in crate::scene_host::recipe) texture_budget: &'a mut RecipeTextureBudget,
}

struct MaterialRecipeBuildRefs<'a> {
    colors: &'a BTreeMap<String, SceneRecipeColorV1>,
    texture_budget: &'a mut RecipeTextureBudget,
}

async fn authored_material(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    recipe: &SceneRecipeMaterialV1,
    base_color: Option<Color>,
    path: &str,
    resources: &mut MaterialRecipeBuildRefs<'_>,
) -> Result<(String, MaterialDesc), Box<SceneRecipeDiagnosticV1>> {
    let (kind, mut material) = if let Some(preset) = recipe.preset.as_deref() {
        let material = MaterialDesc::from_preset_name(preset, base_color).ok_or_else(|| {
            Box::new(error_diagnostic(
                format!("{path}.preset"),
                "invalid_material_preset",
                format!("material preset '{preset}' is not supported"),
                format!("use one of: {}", MaterialDesc::PRESET_NAMES.join(", ")),
            ))
        })?;
        (material_kind_name(material.kind()), material)
    } else {
        let base_color = base_color.ok_or_else(|| {
            Box::new(error_diagnostic(
                format!("{path}.base_color"),
                "missing_base_color",
                "material base_color must be a color id, named Color constant, or #RRGGBB string",
                "provide base_color or use material.preset with an optional tint",
            ))
        })?;
        match recipe.kind.as_deref() {
            Some("unlit") => ("unlit", MaterialDesc::unlit(base_color)),
            Some("pbr_metallic_roughness") => (
                "pbr_metallic_roughness",
                MaterialDesc::pbr_metallic_roughness(
                    base_color,
                    recipe.metallic.unwrap_or(0.0) as f32,
                    recipe.roughness.unwrap_or(1.0) as f32,
                ),
            ),
            Some("line") => (
                "line",
                MaterialDesc::line(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32),
            ),
            Some("wireframe") => (
                "wireframe",
                MaterialDesc::wireframe(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32),
            ),
            Some("edge") => {
                let mut material =
                    MaterialDesc::edge(base_color, recipe.stroke_width_px.unwrap_or(1.0) as f32);
                if let Some(threshold) = recipe.edge_angle_threshold_degrees {
                    material = material.with_edge_angle_threshold_degrees(threshold as f32);
                }
                ("edge", material)
            }
            Some(kind) => {
                return Err(Box::new(error_diagnostic(
                    path,
                    "unsupported_feature",
                    format!("material kind '{kind}' is not implemented in this slice"),
                    "use kind:\"unlit\", \"pbr_metallic_roughness\", \"line\", \"wireframe\", or \"edge\"",
                )));
            }
            None => {
                return Err(Box::new(error_diagnostic(
                    format!("{path}.kind"),
                    "missing_material_kind",
                    "material must include either a preset string or a kind string",
                    "use preset:\"chrome\" or kind:\"pbr_metallic_roughness\" with base_color",
                )));
            }
        }
    };
    if let Some(metallic) = recipe.metallic {
        material = material.with_metallic_factor(metallic as f32);
    }
    if let Some(roughness) = recipe.roughness {
        material = material.with_roughness_factor(roughness as f32);
    }
    material = material.with_double_sided(recipe.double_sided);
    if let Some(emissive) = &recipe.emissive {
        material =
            material.with_emissive(authored_color(resources.colors, emissive).map_err(
                |diagnostic| Box::new((*diagnostic).with_path(format!("{path}.emissive"))),
            )?);
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
        resources.texture_budget,
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
        resources.texture_budget,
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
        resources.texture_budget,
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
        resources.texture_budget,
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
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_emissive_texture(texture);
    }
    if let Some(value) = recipe.clearcoat_factor {
        material = material.with_clearcoat_factor(value as f32);
    }
    if let Some(value) = recipe.clearcoat_roughness_factor {
        material = material.with_clearcoat_roughness_factor(value as f32);
    }
    if let Some(value) = recipe.clearcoat_normal_scale {
        material = material.with_clearcoat_normal_scale(value as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.clearcoat_texture.as_ref(),
        &format!("{path}.clearcoat_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_clearcoat_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.clearcoat_roughness_texture.as_ref(),
        &format!("{path}.clearcoat_roughness_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_clearcoat_roughness_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.clearcoat_normal_texture.as_ref(),
        &format!("{path}.clearcoat_normal_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_clearcoat_normal_texture(texture);
    }
    if let Some(color) = &recipe.sheen_color_factor {
        material = material.with_sheen_color_factor(
            authored_color(resources.colors, color).map_err(|diagnostic| {
                Box::new((*diagnostic).with_path(format!("{path}.sheen_color_factor")))
            })?,
        );
    }
    if let Some(value) = recipe.sheen_roughness_factor {
        material = material.with_sheen_roughness_factor(value as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.sheen_color_texture.as_ref(),
        &format!("{path}.sheen_color_texture"),
        TextureColorSpace::Srgb,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_sheen_color_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.sheen_roughness_texture.as_ref(),
        &format!("{path}.sheen_roughness_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_sheen_roughness_texture(texture);
    }
    if let Some(value) = recipe.anisotropy_strength_factor {
        material = material.with_anisotropy_strength_factor(value as f32);
    }
    if let Some(value) = recipe.anisotropy_rotation_radians {
        material = material.with_anisotropy_rotation_radians(value as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.anisotropy_texture.as_ref(),
        &format!("{path}.anisotropy_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_anisotropy_texture(texture);
    }
    if let Some(value) = recipe.iridescence_factor {
        material = material.with_iridescence_factor(value as f32);
    }
    if let Some(value) = recipe.iridescence_ior {
        material = material.with_iridescence_ior(value as f32);
    }
    if recipe.iridescence_thickness_minimum_nm.is_some()
        || recipe.iridescence_thickness_maximum_nm.is_some()
    {
        let minimum_nm = recipe
            .iridescence_thickness_minimum_nm
            .unwrap_or_else(|| f64::from(material.iridescence_thickness_minimum_nm()));
        let maximum_nm = recipe
            .iridescence_thickness_maximum_nm
            .unwrap_or_else(|| f64::from(material.iridescence_thickness_maximum_nm()));
        material =
            material.with_iridescence_thickness_range_nm(minimum_nm as f32, maximum_nm as f32);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.iridescence_texture.as_ref(),
        &format!("{path}.iridescence_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_iridescence_texture(texture);
    }
    if let Some(texture) = load_texture_slot(
        policy,
        host,
        recipe_path,
        recipe.iridescence_thickness_texture.as_ref(),
        &format!("{path}.iridescence_thickness_texture"),
        TextureColorSpace::Linear,
        resources.texture_budget,
    )
    .await?
    {
        material = material.with_iridescence_thickness_texture(texture);
    }
    if let Some(value) = recipe.dispersion_factor {
        material = material.with_dispersion_factor(value as f32);
    }
    if let Some(value) = recipe.transmission_factor {
        material = material.with_transmission_factor(value as f32);
    }
    if let Some(value) = recipe.ior {
        material = material.with_ior(value as f32);
    }
    if let Some(value) = recipe.thickness_factor {
        material = material.with_thickness_factor(value as f32);
    }
    if let Some(value) = recipe.attenuation_distance {
        material = material.with_attenuation_distance(value as f32);
    }
    if let Some(color) = &recipe.attenuation_color {
        material =
            material.with_attenuation_color(authored_color(resources.colors, color).map_err(
                |diagnostic| Box::new((*diagnostic).with_path(format!("{path}.attenuation_color"))),
            )?);
    }
    reject_gpu_unsupported_volume_texture(
        recipe.transmission_texture.as_ref(),
        path,
        "transmission_texture",
    )?;
    reject_gpu_unsupported_volume_texture(
        recipe.thickness_texture.as_ref(),
        path,
        "thickness_texture",
    )?;
    Ok((kind.to_owned(), material))
}

fn material_kind_name(kind: MaterialKind) -> &'static str {
    match kind {
        MaterialKind::Unlit => "unlit",
        MaterialKind::PbrMetallicRoughness => "pbr_metallic_roughness",
        MaterialKind::Line => "line",
        MaterialKind::Wireframe => "wireframe",
        MaterialKind::Edge => "edge",
    }
}

fn reject_gpu_unsupported_volume_texture(
    slot: Option<&SceneRecipeTextureSlotV1>,
    path: &str,
    field: &str,
) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
    if slot.is_none() {
        return Ok(());
    }
    Err(Box::new(error_diagnostic(
        format!("{path}.{field}"),
        "unsupported_feature",
        format!(
            "{field} is not exposed by scene_recipe.v1 until the GPU path supports it without exceeding the WebGL2 texture-unit floor"
        ),
        "remove this texture slot; transmission_factor remains supported for recipe-authored glass",
    )))
}

#[cfg(test)]
mod tests;
