mod build;
mod diff;
mod field_model;
mod target_resolution;
mod types;
mod validation;

#[cfg(feature = "scene-host")]
pub(crate) use build::RecipeResourcePlan;
#[cfg(feature = "scene-host")]
pub(crate) use build::build_diagnostic;
pub(crate) use build::{PlannedRecipeResource, RecipeResourceRole};
pub use build::{
    RECIPE_POLICY_SCHEMA_V1, RecipeBuildPolicy, RecipeBuildPolicyBoolV1, RecipeBuildPolicyLimitV1,
    RecipeBuildPolicyReportV1, RecipeBuildPolicyRootV1, RecipeBuildPolicyStringV1,
};
pub use diff::{
    SCENE_RECIPE_DIFF_SCHEMA_V1, SceneRecipeDiffChangeKindV1, SceneRecipeDiffChangeV1,
    SceneRecipeDiffOptions, SceneRecipeDiffReportV1, SceneRecipeDiffScopeV1, diff_scene_recipes,
};
pub(crate) use field_model::{
    AREA_LIGHT_PRESETS, DIRECTIONAL_LIGHT_PRESETS, POINT_LIGHT_PRESETS, RENDER_PROFILES,
    RENDER_QUALITIES, SCENE_PRESETS, STUDIO_LIGHT_PRESETS, TONEMAPPERS,
};
pub use field_model::{
    FIELD_MODEL_SCHEMA_V1, SchemaFieldModelV1, SchemaFieldV1, scene_recipe_field_model_v1,
    scene_recipe_json_schema_paths_v1, scene_recipe_json_schema_v1,
};
pub use target_resolution::{
    SceneRecipeTargetResolutionError, SceneRecipeTargetResolutionErrorKind,
    SceneRecipeTargetResolutionMode, resolve_scene_recipe_target_handles,
};
pub use types::{
    RECIPE_BUILD_RESULT_SCHEMA_V1, RecipeBuildExecutionV1, RecipeBuildResultV1,
    RecipeValidationModeV1, SCENE_RECIPE_BUILD_SCHEMA_V1, SCENE_RECIPE_SCHEMA_V1,
    SCENE_RECIPE_VALIDATION_SCHEMA_V1, SceneRecipeAlphaModeV1, SceneRecipeAnchorSourceV1,
    SceneRecipeAnchorV1, SceneRecipeAnimationChannelV1, SceneRecipeAnimationV1,
    SceneRecipeAutoExposureV1, SceneRecipeBackendExpectationV1, SceneRecipeBackgroundV1,
    SceneRecipeBboxFitExpectationV1, SceneRecipeBloomV1, SceneRecipeBoundsSourceV1,
    SceneRecipeBoundsV1, SceneRecipeBuildAnchorV1, SceneRecipeBuildAnimationV1,
    SceneRecipeBuildBoundsV1, SceneRecipeBuildConnectionV1, SceneRecipeBuildConnectorV1,
    SceneRecipeBuildImportV1, SceneRecipeBuildInstanceV1, SceneRecipeBuildNamedStateV1,
    SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1,
    SceneRecipeBuildV1, SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1,
    SceneRecipeCameraFramingV1, SceneRecipeCameraV1, SceneRecipeCaptureV1,
    SceneRecipeClippingExpectationV1, SceneRecipeClippingPlaneV1, SceneRecipeColorExpectationV1,
    SceneRecipeColorV1, SceneRecipeConnectionParentingV1, SceneRecipeConnectionRollV1,
    SceneRecipeConnectorAlignmentV1, SceneRecipeConnectorMateV1, SceneRecipeConnectorPolarityV1,
    SceneRecipeConnectorRollPolicyV1, SceneRecipeConnectorSourceV1, SceneRecipeConnectorV1,
    SceneRecipeDepthOfFieldFocusV1, SceneRecipeDepthOfFieldTargetV1, SceneRecipeDepthOfFieldV1,
    SceneRecipeDiagnosticResourceV1, SceneRecipeDiagnosticV1, SceneRecipeEnvironmentV1,
    SceneRecipeExpectV1, SceneRecipeExpectedExtentV1, SceneRecipeExplodedViewModeV1,
    SceneRecipeExplodedViewV1, SceneRecipeFontV1, SceneRecipeGeometryV1,
    SceneRecipeGridReflectionV1, SceneRecipeGridV1, SceneRecipeGroundedExpectationV1,
    SceneRecipeHelperOcclusionExpectationV1, SceneRecipeImportEdgeEmphasisV1,
    SceneRecipeImportEdgeRoundingReportV1, SceneRecipeImportEdgeRoundingV1,
    SceneRecipeImportMaterialBindingV1, SceneRecipeImportMaterialV1, SceneRecipeImportV1,
    SceneRecipeInstanceSetV1, SceneRecipeInstanceV1, SceneRecipeLabelV1, SceneRecipeLightV1,
    SceneRecipeLookAtTargetV1, SceneRecipeMaterialImperfectionV1, SceneRecipeMaterialPackV1,
    SceneRecipeMaterialV1, SceneRecipeMeasurementV1, SceneRecipeMeshV1, SceneRecipeMeteringRectV1,
    SceneRecipeMeteringTargetV1, SceneRecipeMeteringV1, SceneRecipeMorphTargetV1,
    SceneRecipeMorphV1, SceneRecipeNamedStateV1, SceneRecipeNodeLodV1,
    SceneRecipeNodeSkinBindingV1, SceneRecipeNodeV1, SceneRecipeParticleSetV1,
    SceneRecipeParticleV1, SceneRecipePhotoCompositionV1, SceneRecipePhotoExposureV1,
    SceneRecipePhotoFocusV1, SceneRecipePhotoQualityV1, SceneRecipePhotoRangeV1,
    SceneRecipePhotoStagingV1, SceneRecipePhotoSubjectV1, SceneRecipePhotoV1,
    SceneRecipePhotographicSurfaceV1, SceneRecipePickExpectationV1, SceneRecipePrimitiveV1,
    SceneRecipeQualityAreaLightV1, SceneRecipeQualityContrastV1, SceneRecipeQualityDepthOfFieldV1,
    SceneRecipeQualityExpectationV1, SceneRecipeQualityExposureV1, SceneRecipeQualityGeometryV1,
    SceneRecipeQualityGroundingV1, SceneRecipeQualityLineV1, SceneRecipeQualityNoiseV1,
    SceneRecipeQualityReflectionV1, SceneRecipeQualityTextV1, SceneRecipeReferenceExpectationV1,
    SceneRecipeRenderV1, SceneRecipeResourceResolutionV1, SceneRecipeResourceStatusV1,
    SceneRecipeSceneV1, SceneRecipeScreenSpaceReflectionsV1, SceneRecipeSectionBoxV1,
    SceneRecipeSeparationExpectationV1, SceneRecipeSkinV1, SceneRecipeSourceMaterialSelectorV1,
    SceneRecipeSpatialTargetV1, SceneRecipeSsaoV1, SceneRecipeStateExpectationV1,
    SceneRecipeStateTintV1, SceneRecipeStateTransformV1, SceneRecipeStateVisibilityV1,
    SceneRecipeSubjectFallbackPolicyV1, SceneRecipeSubjectSpecV1, SceneRecipeSubjectV1,
    SceneRecipeTargetBoundsV1, SceneRecipeTargetFitExpectationV1, SceneRecipeTargetRegionV1,
    SceneRecipeTargetV1, SceneRecipeTextureColorSpaceV1, SceneRecipeTextureSlotV1,
    SceneRecipeTransformConversionError, SceneRecipeTransformExpectationV1, SceneRecipeTransformV1,
    SceneRecipeV1, SceneRecipeValidationReportV1, SceneRecipeVisibleExpectationV1,
};
#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub use types::{
    SCENE_COMPOSITION_SCHEMA_V1, SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1, SceneCompositionCheckV1,
    SceneCompositionRegionV1, SceneCompositionReportV1, SceneCompositionStatusV1,
    SceneCompositionSummaryV1, SceneRecipeRenderResultV1, SceneRecipeVerificationReasonV1,
    SceneRecipeVerificationReportV1, SceneRecipeVerificationSummaryV1,
};
pub use validation::{
    parse_valid_scene_recipe_json, parse_valid_scene_recipe_json_with_policy,
    recipe_too_large_report, validate_scene_recipe_json,
    validate_scene_recipe_json_syntax_with_policy, validate_scene_recipe_json_with_policy,
    validate_scene_recipe_value, validate_scene_recipe_value_with_policy,
};
