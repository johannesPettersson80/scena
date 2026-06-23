use crate::assets::DefaultAssetFetcher;
use crate::material::TextureColorSpace;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeDiagnosticV1, SceneRecipeTextureColorSpaceV1,
    SceneRecipeTextureSlotV1,
};
use crate::scene_host::SceneHostCore;
use crate::{AssetPath, TextureHandle};

use super::super::super::error_diagnostic;
use super::super::super::policy::RecipeTextureBudget;

pub(super) async fn load_texture_slot(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    slot: Option<&SceneRecipeTextureSlotV1>,
    path: &str,
    default_color_space: TextureColorSpace,
    texture_budget: &mut RecipeTextureBudget,
) -> Result<Option<TextureHandle>, Box<SceneRecipeDiagnosticV1>> {
    let Some(slot) = slot else {
        return Ok(None);
    };
    let color_space = match slot.color_space {
        Some(SceneRecipeTextureColorSpaceV1::Srgb) => TextureColorSpace::Srgb,
        Some(SceneRecipeTextureColorSpaceV1::Linear) => TextureColorSpace::Linear,
        None => default_color_space,
    };
    let resolved =
        texture_budget.reserve_texture_uri(policy, recipe_path, &slot.uri, path.to_owned())?;
    match host
        .assets
        .load_texture(AssetPath::from(resolved.as_str()), color_space)
        .await
    {
        Ok(texture) => Ok(Some(texture)),
        Err(_) if slot.optional => Ok(None),
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
