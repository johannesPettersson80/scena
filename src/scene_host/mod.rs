//! Generic host facade over `Scene`, `Assets`, and `Renderer`.

mod animation;
mod annotations;
mod assets;
mod camera;
mod capture;
mod core;
mod error;
mod events;
mod handles;
mod inputs;
mod inspection_tools;
mod instances;
mod interaction_verification;
mod material_variants;
mod post;
mod product;
mod reporting;
mod subtree;
mod transforms;
mod transitions;
mod visual_patch;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_animation;
#[cfg(target_arch = "wasm32")]
mod wasm_assets;
#[cfg(target_arch = "wasm32")]
mod wasm_camera;
#[cfg(target_arch = "wasm32")]
mod wasm_post;
#[cfg(target_arch = "wasm32")]
mod wasm_product;
#[cfg(target_arch = "wasm32")]
mod wasm_readback;
#[cfg(target_arch = "wasm32")]
mod wasm_subtree;
#[cfg(target_arch = "wasm32")]
mod wasm_transforms;
#[cfg(target_arch = "wasm32")]
mod wasm_transitions;
#[cfg(target_arch = "wasm32")]
mod wasm_visual_patch;

pub type SceneHostCameraState = crate::controls::CameraState;
pub type SceneHostEasing = crate::controls::TransitionEasing;
pub use animation::{SceneHostAnimationLoopMode, SceneHostAnimationPlayOptions};
pub use core::SceneHostCore;
pub use error::{SceneHostError, SceneHostErrorCode};
pub use events::{
    HOST_EVENT_SCHEMA_V1, HostEventBatchV1, HostEventButtonV1, HostEventHitV1,
    HostEventHoverPhaseV1, HostEventModifiersV1, HostEventTargetKindV1, HostEventV1,
};
pub use interaction_verification::{
    INTERACTION_EXPECTATION_SCHEMA_V1, INTERACTION_VERIFICATION_SCHEMA_V1,
    InteractionCoordinateSpaceV1, InteractionCoordinatesV1, InteractionExpectationV1,
    InteractionStepExpectationV1, InteractionStepExpectedV1, InteractionStepObservedV1,
    InteractionStepReportV1, InteractionVerificationArtifactsV1, InteractionVerificationFixV1,
    InteractionVerificationReasonV1, InteractionVerificationReportV1,
    InteractionVerificationSummaryV1, InteractionViewportV1, host_event_kind_name, physical_px,
};
pub use reporting::{
    SCENE_HOST_ANIMATION_INVENTORY_SCHEMA_V1, SCENE_HOST_ASSET_IMPORT_SCHEMA_V1,
    SCENE_HOST_SUBTREE_SCHEMA_V1, SceneHostAnimationClipV1, SceneHostAnimationInventoryV1,
    SceneHostAssetImportReportV1, SceneHostSubtreeNodeV1, SceneHostSubtreeReportV1,
};
pub use visual_patch::{
    VISUAL_PATCH_SCHEMA_V1, VisualPatchAnimationTimeModeV1, VisualPatchAnimationTimeV1,
    VisualPatchAppliedCountsV1, VisualPatchCameraEasedV1, VisualPatchEntryErrorV1,
    VisualPatchHoverV1, VisualPatchLabelTargetV1, VisualPatchLabelV1, VisualPatchMaterialVariantV1,
    VisualPatchResultV1, VisualPatchRevisionDeltaV1, VisualPatchSelectionV1,
    VisualPatchTintEasedV1, VisualPatchTintV1, VisualPatchTransformEasedV1, VisualPatchTransformV1,
    VisualPatchV1, VisualPatchVisibilityV1,
};

#[cfg(target_arch = "wasm32")]
pub use wasm::SceneHost;
