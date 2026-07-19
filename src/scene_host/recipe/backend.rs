use super::{RecipeBackendPolicy, SceneHostRecipeBuild};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{RecipeBuildPolicy, SceneRecipeBuildV1};
use crate::scene_host::SceneHostCore;

impl SceneHostCore<DefaultAssetFetcher> {
    pub async fn build_recipe_json(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
    ) -> Result<SceneHostRecipeBuild<DefaultAssetFetcher>, SceneRecipeBuildV1> {
        Self::build_recipe_json_with_backend(recipe_path, text, policy, RecipeBackendPolicy::Cpu)
            .await
    }

    pub async fn build_recipe_json_gpu(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
    ) -> Result<SceneHostRecipeBuild<DefaultAssetFetcher>, SceneRecipeBuildV1> {
        Self::build_recipe_json_with_backend(
            recipe_path,
            text,
            policy,
            RecipeBackendPolicy::StrictGpu,
        )
        .await
    }

    pub async fn build_recipe_json_prefer_gpu(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
    ) -> Result<SceneHostRecipeBuild<DefaultAssetFetcher>, SceneRecipeBuildV1> {
        Self::build_recipe_json_with_backend(
            recipe_path,
            text,
            policy,
            RecipeBackendPolicy::PreferGpu,
        )
        .await
    }
}
