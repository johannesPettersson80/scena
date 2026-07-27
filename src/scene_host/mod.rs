//! Generic host facade over `Scene`, `Assets`, and `Renderer`.

mod animation;
mod annotations;
mod assets;
mod callouts;
mod camera;
mod capture;
mod composition;
mod connectors;
mod construction;
mod core;
mod core_handles;
#[cfg(test)]
mod core_tests;
mod error;
mod events;
mod exploded_view;
mod gizmo;
mod handles;
mod inputs;
mod inspection_tools;
mod instances;
mod interaction_verification;
mod introspection;
mod label_quality;
mod material_variants;
mod measurements;
mod photo;
mod photographic_lighting;
mod photographic_surface;
mod photographic_surroundings;
mod photographic_transport;
mod post;
mod presentation_timeline;
mod product;
mod product_options;
mod recipe;
mod reporting;
mod section_box;
mod semantic_aov;
mod subtree;
mod transforms;
mod transitions;
mod visual_patch;
mod visual_states;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_animation;
#[cfg(target_arch = "wasm32")]
mod wasm_assets;
#[cfg(target_arch = "wasm32")]
mod wasm_camera;
#[cfg(target_arch = "wasm32")]
mod wasm_capture;
#[cfg(target_arch = "wasm32")]
mod wasm_gizmo;
#[cfg(target_arch = "wasm32")]
mod wasm_introspection;
#[cfg(target_arch = "wasm32")]
mod wasm_measurements;
#[cfg(target_arch = "wasm32")]
mod wasm_post;
#[cfg(target_arch = "wasm32")]
mod wasm_presentation_timeline;
#[cfg(target_arch = "wasm32")]
mod wasm_product;
#[cfg(target_arch = "wasm32")]
mod wasm_readback;
#[cfg(target_arch = "wasm32")]
mod wasm_section_box;
#[cfg(target_arch = "wasm32")]
mod wasm_subtree;
#[cfg(target_arch = "wasm32")]
mod wasm_surface_events;
#[cfg(target_arch = "wasm32")]
mod wasm_transforms;
#[cfg(target_arch = "wasm32")]
mod wasm_transitions;
#[cfg(target_arch = "wasm32")]
mod wasm_visual_patch;

pub type SceneHostCameraState = crate::controls::CameraState;
pub type SceneHostEasing = crate::controls::TransitionEasing;
pub use animation::{SceneHostAnimationLoopMode, SceneHostAnimationPlayOptions};
pub use callouts::SceneHostCalloutReportV1;
pub use composition::CompositionOverlaySegmentV1;
pub use connectors::{
    CONNECTOR_BROWSER_SCHEMA_V1, ConnectorBrowserCandidateV1, ConnectorBrowserConnectorV1,
    ConnectorBrowserReportV1, ConnectorBrowserScopeV1, ConnectorBrowserSummaryV1,
    ConnectorBrowserVisualCueV1, ConnectorLineV1, ConnectorTransformV1,
};
pub use core::SceneHostCore;
pub use error::{SceneHostError, SceneHostErrorCode};
pub use events::{
    HOST_EVENT_SCHEMA_V1, HostEventBatchV1, HostEventHitV1, HostEventHoverPhaseV1,
    HostEventTargetKindV1, HostEventV1,
};
pub use exploded_view::{SceneHostExplodedViewModeV1, SceneHostExplodedViewOptionsV1};
pub use gizmo::{
    SCENE_HOST_GIZMO_DRAG_SCHEMA_V1, SceneHostGizmoAxisV1, SceneHostGizmoConstraintV1,
    SceneHostGizmoDragV1, SceneHostGizmoModeV1, SceneHostGizmoRayV1, SceneHostGizmoSpaceV1,
};
pub use interaction_verification::{
    INTERACTION_EXPECTATION_SCHEMA_V1, INTERACTION_VERIFICATION_SCHEMA_V1,
    InteractionCoordinateSpaceV1, InteractionCoordinatesV1, InteractionExpectationV1,
    InteractionStepExpectationV1, InteractionStepExpectedV1, InteractionStepObservedV1,
    InteractionStepReportV1, InteractionVerificationArtifactsV1, InteractionVerificationFixV1,
    InteractionVerificationReasonV1, InteractionVerificationReportV1,
    InteractionVerificationSummaryV1, InteractionViewportV1, host_event_kind_name, physical_px,
};
pub use measurements::{
    SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1, SceneHostMeasurementAuthorityV1,
    SceneHostMeasurementLabelProjectionV1, SceneHostMeasurementOverlayReportV1,
};
pub use photo::{
    PHOTO_CANDIDATE_PLAN_SCHEMA_V1, PHOTO_PLAN_SCHEMA_V1, PHOTO_REPORT_SCHEMA_V1,
    PHOTO_SHADED_CANDIDATE_SELECTION_SCHEMA_V1, PHOTO_SUBJECT_REGION_SCHEMA_V1,
    PhotoCandidateConstraintsV1, PhotoCandidateFillRangeV1, PhotoCandidateObservation,
    PhotoCandidatePlanV1, PhotoCandidateRequest, PhotoCandidateScore, PhotoCandidateScoringReport,
    PhotoCandidateStagingV1, PhotoCompositionCandidateV1, PhotoPhysicalCameraV1,
    PhotoPlanArtifactsV1, PhotoPlanSourceV1, PhotoPlanSubjectV1, PhotoPlanTargetV1, PhotoPlanV1,
    PhotoReportV1, PhotoSubjectRegionV1, camera_behavior_candidate_plan,
    product_hero_candidate_plan, score_camera_behavior_candidates, score_product_hero_candidates,
};
pub use photographic_lighting::{
    PHOTOGRAPHIC_LIGHTING_REPORT_SCHEMA_V1, PhotographicEnvironmentProfileV1,
    PhotographicGeometryProfileV1, PhotographicLightV1, PhotographicLightingAdjustmentV1,
    PhotographicLightingReportV1, PhotographicMaterialProfileV1, PhotographicWhiteBalanceV1,
};
pub use photographic_surface::{
    PHOTOGRAPHIC_SURFACE_REPORT_SCHEMA_V1, PhotographicAssetIssueClassV1, PhotographicAssetIssueV1,
    PhotographicSurfaceRejectedMeshV1, PhotographicSurfaceReportV1,
};
pub use photographic_surroundings::{
    PHOTOGRAPHIC_SURROUNDINGS_REPORT_SCHEMA_V1, PhotographicSurroundingsReportV1,
};
pub use photographic_transport::{
    PHOTOGRAPHIC_TRANSPORT_REPORT_SCHEMA_V1, PhotographicTransportQuality,
    PhotographicTransportReportV1,
};
pub use presentation_timeline::{
    PRESENTATION_TIMELINE_SCHEMA_V1, PresentationTimelineActionKindV1,
    PresentationTimelineActionV1, PresentationTimelineCameraBookmarkV1, PresentationTimelineV1,
};
pub use product::{
    SCENE_HOST_GROUNDING_SCHEMA_V1, SceneHostGroundingFallbackV1, SceneHostGroundingPathV1,
    SceneHostGroundingReportV1, SceneSetupPreset,
};
pub use product_options::{
    PRODUCT_OPTIONS_SCHEMA_V1, ProductOptionGroupV1, ProductOptionV1, ProductOptionsV1,
};
pub use recipe::SceneHostRecipeBuild;
pub use reporting::{
    SCENE_HOST_ANIMATION_INVENTORY_SCHEMA_V1, SCENE_HOST_ASSET_IMPORT_SCHEMA_V1,
    SCENE_HOST_SUBTREE_SCHEMA_V1, SceneHostAnimationClipV1, SceneHostAnimationInventoryV1,
    SceneHostAssetImportReportV1, SceneHostSubtreeNodeV1, SceneHostSubtreeReportV1,
};
pub use section_box::{
    SCENE_HOST_SECTION_BOX_SCHEMA_V1, SceneHostClippingPlaneV1, SceneHostSectionBoxReportV1,
};
pub use semantic_aov::{
    SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1, SceneHostSemanticAovCaptureV1,
    SceneHostSemanticAovExclusionsV1, SceneHostSemanticAovLegendEntryV1, palette_rgba8,
};
pub use visual_patch::{
    VISUAL_PATCH_SCHEMA_V1, VisualPatchAnimationTimeModeV1, VisualPatchAnimationTimeV1,
    VisualPatchAppliedCountsV1, VisualPatchCameraEasedV1, VisualPatchEntryErrorV1,
    VisualPatchHoverV1, VisualPatchLabelTargetV1, VisualPatchLabelV1, VisualPatchMaterialVariantV1,
    VisualPatchResultV1, VisualPatchRevisionDeltaV1, VisualPatchSectionBoxV1,
    VisualPatchSelectionV1, VisualPatchTintEasedV1, VisualPatchTintV1, VisualPatchTransformEasedV1,
    VisualPatchTransformV1, VisualPatchV1, VisualPatchVisibilityV1,
};
pub use visual_states::{
    SCENE_HOST_VISUAL_STATE_SCHEMA_V1, SCENE_HOST_VISUAL_STATES_SCHEMA_V1,
    SceneHostVisualStateSummaryV1, SceneHostVisualStateV1, SceneHostVisualStatesReportV1,
};

#[cfg(target_arch = "wasm32")]
pub use wasm::SceneHost;
