use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod authoring;
mod build_manifest;
mod expectations;
mod overlays;
mod photo;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
mod render_result;
mod setup;
mod spatial_state;
mod subject;
pub use authoring::{
    SceneRecipeAlphaModeV1, SceneRecipeAnimationChannelV1, SceneRecipeAnimationV1,
    SceneRecipeCameraFramingV1, SceneRecipeCameraV1, SceneRecipeClippingPlaneV1,
    SceneRecipeColorV1, SceneRecipeExpectedExtentV1, SceneRecipeFontV1, SceneRecipeGeometryV1,
    SceneRecipeImportEdgeEmphasisV1, SceneRecipeImportMaterialV1, SceneRecipeImportV1,
    SceneRecipeInstanceSetV1, SceneRecipeInstanceV1, SceneRecipeLabelV1, SceneRecipeLightV1,
    SceneRecipeLookAtTargetV1, SceneRecipeMaterialV1, SceneRecipeMeshV1, SceneRecipeMorphTargetV1,
    SceneRecipeMorphV1, SceneRecipeNodeLodV1, SceneRecipeNodeSkinBindingV1, SceneRecipeNodeV1,
    SceneRecipeParticleSetV1, SceneRecipeParticleV1, SceneRecipePrimitiveV1, SceneRecipeSkinV1,
    SceneRecipeTextureColorSpaceV1, SceneRecipeTextureSlotV1, SceneRecipeTransformConversionError,
    SceneRecipeTransformV1,
};
pub use build_manifest::{
    RECIPE_BUILD_RESULT_SCHEMA_V1, RecipeBuildExecutionV1, RecipeBuildResultV1,
    RecipeValidationModeV1, SceneRecipeBuildAnchorV1, SceneRecipeBuildAnimationV1,
    SceneRecipeBuildBoundsV1, SceneRecipeBuildConnectionV1, SceneRecipeBuildConnectorV1,
    SceneRecipeBuildImportV1, SceneRecipeBuildInstanceV1, SceneRecipeBuildNamedStateV1,
    SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1,
    SceneRecipeBuildV1, SceneRecipeDiagnosticResourceV1, SceneRecipeDiagnosticV1,
    SceneRecipeResourceResolutionV1, SceneRecipeResourceStatusV1, SceneRecipeValidationReportV1,
};
pub use expectations::{
    SceneRecipeBackendExpectationV1, SceneRecipeBboxFitExpectationV1,
    SceneRecipeClippingExpectationV1, SceneRecipeColorExpectationV1, SceneRecipeExpectV1,
    SceneRecipeGroundedExpectationV1, SceneRecipeHelperOcclusionExpectationV1,
    SceneRecipePickExpectationV1, SceneRecipeQualityAreaLightV1, SceneRecipeQualityContrastV1,
    SceneRecipeQualityDepthOfFieldV1, SceneRecipeQualityExpectationV1,
    SceneRecipeQualityExposureV1, SceneRecipeQualityGeometryV1, SceneRecipeQualityGroundingV1,
    SceneRecipeQualityLineV1, SceneRecipeQualityNoiseV1, SceneRecipeQualityReflectionV1,
    SceneRecipeQualityTextV1, SceneRecipeReferenceExpectationV1,
    SceneRecipeSeparationExpectationV1, SceneRecipeStateExpectationV1, SceneRecipeTargetBoundsV1,
    SceneRecipeTargetFitExpectationV1, SceneRecipeTargetRegionV1,
    SceneRecipeTransformExpectationV1, SceneRecipeVisibleExpectationV1,
};
pub use overlays::{
    SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1, SceneRecipeCaptureV1,
    SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1, SceneRecipeMeasurementV1,
    SceneRecipeSectionBoxV1, SceneRecipeTargetV1,
};
pub use photo::{
    SceneRecipePhotoCompositionV1, SceneRecipePhotoExposureV1, SceneRecipePhotoFocusV1,
    SceneRecipePhotoRangeV1, SceneRecipePhotoStagingV1, SceneRecipePhotoSubjectV1,
    SceneRecipePhotoV1,
};
#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub use render_result::{
    SCENE_COMPOSITION_SCHEMA_V1, SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1, SceneCompositionCheckV1,
    SceneCompositionRegionV1, SceneCompositionReportV1, SceneCompositionStatusV1,
    SceneCompositionSummaryV1, SceneRecipeRenderResultV1, SceneRecipeVerificationReasonV1,
    SceneRecipeVerificationReportV1, SceneRecipeVerificationSummaryV1,
};
pub use setup::{
    SceneRecipeAutoExposureV1, SceneRecipeBackgroundV1, SceneRecipeBloomV1,
    SceneRecipeDepthOfFieldFocusV1, SceneRecipeDepthOfFieldTargetV1, SceneRecipeDepthOfFieldV1,
    SceneRecipeEnvironmentV1, SceneRecipeGridReflectionV1, SceneRecipeGridV1,
    SceneRecipeMeteringRectV1, SceneRecipeMeteringTargetV1, SceneRecipeMeteringV1,
    SceneRecipeRenderV1, SceneRecipeSceneV1, SceneRecipeScreenSpaceReflectionsV1,
    SceneRecipeSsaoV1,
};
pub use spatial_state::{
    SceneRecipeAnchorSourceV1, SceneRecipeAnchorV1, SceneRecipeBoundsSourceV1, SceneRecipeBoundsV1,
    SceneRecipeConnectionParentingV1, SceneRecipeConnectionRollV1, SceneRecipeConnectorAlignmentV1,
    SceneRecipeConnectorMateV1, SceneRecipeConnectorPolarityV1, SceneRecipeConnectorRollPolicyV1,
    SceneRecipeConnectorSourceV1, SceneRecipeConnectorV1, SceneRecipeNamedStateV1,
    SceneRecipeSpatialTargetV1, SceneRecipeStateTintV1, SceneRecipeStateTransformV1,
    SceneRecipeStateVisibilityV1,
};
pub use subject::{
    SceneRecipeSubjectFallbackPolicyV1, SceneRecipeSubjectSpecV1, SceneRecipeSubjectV1,
};

pub const SCENE_RECIPE_SCHEMA_V1: &str = "scena.scene_recipe.v1";
pub const SCENE_RECIPE_VALIDATION_SCHEMA_V1: &str = "scena.scene_recipe_validation.v1";
pub const SCENE_RECIPE_BUILD_SCHEMA_V1: &str = "scena.scene_recipe_build.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SceneRecipeV1 {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<SceneRecipeImportV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, SceneRecipeColorV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometries: Vec<SceneRecipeGeometryV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub morphs: Vec<SceneRecipeMorphV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skins: Vec<SceneRecipeSkinV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<SceneRecipeMaterialV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<SceneRecipeNodeV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<SceneRecipeAnchorV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<SceneRecipeConnectorV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<SceneRecipeBoundsV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub named_states: Vec<SceneRecipeNamedStateV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_sets: Vec<SceneRecipeInstanceSetV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub particles: Vec<SceneRecipeParticleSetV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<SceneRecipeFontV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<SceneRecipeLabelV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clipping_planes: Vec<SceneRecipeClippingPlaneV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<SceneRecipeAnimationV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cameras: Vec<SceneRecipeCameraV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lights: Vec<SceneRecipeLightV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<SceneRecipeSceneV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<SceneRecipeRenderV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<SceneRecipePhotoV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<SceneRecipeExpectV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_box: Option<SceneRecipeSectionBoxV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measurements: Vec<SceneRecipeMeasurementV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callouts: Vec<SceneRecipeCalloutV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploded_view: Option<SceneRecipeExplodedViewV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<SceneRecipeCaptureV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

pub(super) fn default_transform_scale() -> [f64; 3] {
    [1.0, 1.0, 1.0]
}

pub(super) fn default_transform_up() -> [f64; 3] {
    [0.0, 1.0, 0.0]
}

pub(super) fn is_zero_vec3(value: &[f64; 3]) -> bool {
    *value == [0.0, 0.0, 0.0]
}

pub(super) fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}

pub(super) fn is_default_scale(value: &[f64; 3]) -> bool {
    *value == default_transform_scale()
}

pub(super) fn is_default_up(value: &[f64; 3]) -> bool {
    *value == default_transform_up()
}

pub(super) fn is_false(value: &bool) -> bool {
    !*value
}

pub(super) fn is_true(value: &bool) -> bool {
    *value
}

pub(super) fn default_true() -> bool {
    true
}
