mod build;
mod types;
mod validation;

pub use build::RecipeBuildPolicy;
#[cfg(feature = "scene-host")]
pub(crate) use build::build_diagnostic;
pub use types::{
    SCENE_RECIPE_BUILD_SCHEMA_V1, SCENE_RECIPE_SCHEMA_V1, SCENE_RECIPE_VALIDATION_SCHEMA_V1,
    SceneRecipeAlphaModeV1, SceneRecipeBackgroundV1, SceneRecipeBloomV1, SceneRecipeBuildImportV1,
    SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1,
    SceneRecipeBuildV1, SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1, SceneRecipeCameraV1,
    SceneRecipeCaptureV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeEnvironmentV1,
    SceneRecipeExpectedExtentV1, SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1,
    SceneRecipeGeometryV1, SceneRecipeGridV1, SceneRecipeImportV1, SceneRecipeLightV1,
    SceneRecipeLookAtTargetV1, SceneRecipeMaterialV1, SceneRecipeMeasurementV1, SceneRecipeMeshV1,
    SceneRecipeNodeV1, SceneRecipePrimitiveV1, SceneRecipeRenderV1, SceneRecipeSceneV1,
    SceneRecipeSectionBoxV1, SceneRecipeSsaoV1, SceneRecipeTargetV1,
    SceneRecipeTextureColorSpaceV1, SceneRecipeTextureSlotV1, SceneRecipeTransformV1,
    SceneRecipeV1, SceneRecipeValidationReportV1,
};
pub use validation::{
    parse_valid_scene_recipe_json, validate_scene_recipe_json, validate_scene_recipe_value,
};
