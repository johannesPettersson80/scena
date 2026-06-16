mod build;
mod types;
mod validation;

pub use build::RecipeBuildPolicy;
#[cfg(feature = "scene-host")]
pub(crate) use build::build_diagnostic;
pub use types::{
    SCENE_RECIPE_BUILD_SCHEMA_V1, SCENE_RECIPE_SCHEMA_V1, SCENE_RECIPE_VALIDATION_SCHEMA_V1,
    SceneRecipeBuildImportV1, SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1,
    SceneRecipeBuildTargetV1, SceneRecipeBuildV1, SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1,
    SceneRecipeCaptureV1, SceneRecipeDiagnosticV1, SceneRecipeExpectedExtentV1,
    SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1, SceneRecipeImportV1,
    SceneRecipeMeasurementV1, SceneRecipeSectionBoxV1, SceneRecipeV1,
    SceneRecipeValidationReportV1,
};
pub use validation::{
    parse_valid_scene_recipe_json, validate_scene_recipe_json, validate_scene_recipe_value,
};
