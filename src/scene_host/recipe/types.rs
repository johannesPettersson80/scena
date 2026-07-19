use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::SceneRecipeBuildV1;
use crate::scene_host::SceneHostCore;

use super::host::RecipeBackendPolicy;

#[derive(Debug)]
pub struct SceneHostRecipeBuild<F = DefaultAssetFetcher> {
    pub host: SceneHostCore<F>,
    pub manifest: SceneRecipeBuildV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecipeBuildMode {
    Host(RecipeBackendPolicy),
    ManifestOnly,
}
