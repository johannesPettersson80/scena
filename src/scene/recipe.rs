mod build;
mod types;
mod validation;

pub use build::RecipeBuildPolicy;
#[cfg(feature = "scene-host")]
pub(crate) use build::build_diagnostic;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub use types::{
    SCENE_COMPOSITION_SCHEMA_V1, SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1, SceneCompositionCheckV1,
    SceneCompositionRegionV1, SceneCompositionReportV1, SceneCompositionStatusV1,
    SceneCompositionSummaryV1, SceneRecipeRenderResultV1, SceneRecipeVerificationReasonV1,
    SceneRecipeVerificationReportV1, SceneRecipeVerificationSummaryV1,
};
pub use types::{
    SCENE_RECIPE_BUILD_SCHEMA_V1, SCENE_RECIPE_SCHEMA_V1, SCENE_RECIPE_VALIDATION_SCHEMA_V1,
    SceneRecipeAlphaModeV1, SceneRecipeAnimationChannelV1, SceneRecipeAnimationV1,
    SceneRecipeBackgroundV1, SceneRecipeBboxFitExpectationV1, SceneRecipeBloomV1,
    SceneRecipeBuildAnimationV1, SceneRecipeBuildImportV1, SceneRecipeBuildResourceV1,
    SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1, SceneRecipeBuildV1,
    SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1, SceneRecipeCameraV1, SceneRecipeCaptureV1,
    SceneRecipeClippingPlaneV1, SceneRecipeColorExpectationV1, SceneRecipeColorV1,
    SceneRecipeDiagnosticV1, SceneRecipeEnvironmentV1, SceneRecipeExpectV1,
    SceneRecipeExpectedExtentV1, SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1,
    SceneRecipeFontV1, SceneRecipeGeometryV1, SceneRecipeGridV1, SceneRecipeImportV1,
    SceneRecipeInstanceSetV1, SceneRecipeInstanceV1, SceneRecipeLabelV1, SceneRecipeLightV1,
    SceneRecipeLookAtTargetV1, SceneRecipeMaterialV1, SceneRecipeMeasurementV1, SceneRecipeMeshV1,
    SceneRecipeMorphTargetV1, SceneRecipeMorphV1, SceneRecipeNodeSkinBindingV1, SceneRecipeNodeV1,
    SceneRecipeParticleSetV1, SceneRecipeParticleV1, SceneRecipePickExpectationV1,
    SceneRecipePrimitiveV1, SceneRecipeQualityContrastV1, SceneRecipeQualityExpectationV1,
    SceneRecipeQualityExposureV1, SceneRecipeQualityGeometryV1, SceneRecipeQualityLineV1,
    SceneRecipeQualityNoiseV1, SceneRecipeQualityTextV1, SceneRecipeReferenceExpectationV1,
    SceneRecipeRenderV1, SceneRecipeSceneV1, SceneRecipeSectionBoxV1, SceneRecipeSkinV1,
    SceneRecipeSsaoV1, SceneRecipeTargetV1, SceneRecipeTextureColorSpaceV1,
    SceneRecipeTextureSlotV1, SceneRecipeTransformV1, SceneRecipeV1, SceneRecipeValidationReportV1,
    SceneRecipeVisibleExpectationV1,
};
pub use validation::{
    parse_valid_scene_recipe_json, parse_valid_scene_recipe_json_with_policy,
    recipe_too_large_report, validate_scene_recipe_json, validate_scene_recipe_json_with_policy,
    validate_scene_recipe_value, validate_scene_recipe_value_with_policy,
};
