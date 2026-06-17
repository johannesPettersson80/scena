//! `scena` is a Rust-native scene-graph renderer.
//!
//! The first implementation slice establishes the public scene/assets/renderer
//! vocabulary and the explicit prepare/render lifecycle.

pub mod animation;
pub mod assets;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub mod browser_probe;
pub mod browser_proof;
pub mod capture;
pub mod controls;
#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
pub mod demo_page;
pub mod diagnostics;
pub mod geometry;
pub mod material;
#[doc(hidden)]
pub mod material_showcase;
pub mod picking;
pub mod platform;
pub mod reference_image;
pub mod render;
pub mod scene;
#[cfg(feature = "scene-host")]
pub mod scene_host;
pub mod schema_catalog;
pub mod viewer;
pub mod viewer_element;

pub use animation::{
    AnimationChannel, AnimationClip, AnimationClipKey, AnimationInterpolation, AnimationLoopMode,
    AnimationMixer, AnimationMixerKey, AnimationOutput, AnimationPlaybackState,
    AnimationSourceChannel, AnimationSourceClip, AnimationTarget,
};
#[cfg(target_arch = "wasm32")]
pub use assets::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use assets::FileAssetFetcher;
pub use assets::{
    ASSET_CATALOG_SCHEMA_V1, ASSET_DOCTOR_REPORT_SCHEMA_V1, ASSET_GEOMETRY_SUMMARY_SCHEMA_V1,
    ASSET_LOAD_REPORT_SCHEMA_V1, ASSET_READINESS_REPORT_SCHEMA_V1, AssetCatalogAssetV1,
    AssetCatalogExpectedBoundsV1, AssetCatalogFeatureRequirementV1,
    AssetCatalogMaterialRequirementsV1, AssetCatalogPreviewV1, AssetCatalogV1, AssetDerivative,
    AssetDoctorFindingV1, AssetDoctorReportV1, AssetDoctorSeverityV1, AssetDoctorSummaryV1,
    AssetEvictionStats, AssetExternalResource, AssetExternalResourceKind,
    AssetExternalResourceStatus, AssetExternalResourceV1, AssetFetcher, AssetLoadControl,
    AssetLoadOptions, AssetLoadProgress, AssetLoadProgressV1, AssetLoadReport, AssetLoadReportV1,
    AssetLoadWarning, AssetLoadWarningV1, AssetMaterialFallback, AssetMaterialFallbackKind,
    AssetMaterialFallbackV1, AssetMaterialSource, AssetMaterialSourceKind, AssetPath,
    AssetProvenance, AssetReadinessAssetReportV1, AssetReadinessFindingV1, AssetReadinessPreviewV1,
    AssetReadinessReportV1, AssetReadinessSeverityV1, AssetReadinessSummaryV1, AssetStoreId,
    Assets, DefaultAssetFetcher, EnvironmentDerivative, EnvironmentDesc, EnvironmentHandle,
    EnvironmentPrefilterSidecar, EnvironmentPreset, EnvironmentPresetMetadata,
    EnvironmentSidecarHeader, EnvironmentSidecarProfile, EnvironmentSourceKind, GeometryHandle,
    GltfDecoderPolicy, GltfExtensionDiagnostic, GltfExtensionStatus, MaterialHandle,
    MaterialPresetAssets, MaterialPresetProvenance, MaterialVariantBinding, ModelHandle,
    RetainPolicy, SIDECAR_FILE_SUFFIX, SceneAsset, SceneAssetAnchor, SceneAssetClip,
    SceneAssetGeometrySummary, SceneAssetLight, SceneAssetMesh, SceneAssetNode, TextureDesc,
    TextureFilter, TextureHandle, TextureSamplerDesc, TextureSourceFormat, TextureWrap,
    WasmEnvironmentDelivery, parse_sidecar_header, source_backed_material_preset_provenance,
    validate_scene_recipe_json_with_assets,
};
#[cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]
pub use assets::{AssetHotReloadError, AssetHotReloadWatcher};
#[cfg(feature = "khronos-samples")]
pub use assets::{KhronosSample, KhronosSampleMetadata, KhronosSamples};
pub use browser_proof::{BROWSER_PROOF_RUN_SCHEMA_V1, BrowserProofRunV1};
pub use capture::{
    CAPTURE_BASELINE_SCHEMA_V1, CAPTURE_SCHEMA_V1, CaptureAutoFrame, CaptureAutoFrameViewport,
    CaptureBaselineDiff, CaptureBaselineError, CaptureBaselineReport, CaptureBaselineTolerance,
    CaptureCamera, CaptureContactSheet, CaptureContactSheetError, CaptureContactSheetTile,
    CaptureDescriptor, CaptureError, CaptureOptions, CapturePayload, CapturePayloadKind,
    CapturePixelBounds, CapturePixelSummary, CapturePngError, CapturePoint2, CaptureProjection,
    CaptureRevisions, CaptureRgba8, CaptureScreenRect, CaptureViewport, auto_frame_metadata,
    capture_contact_sheet_rgba8, capture_rgba8, capture_rgba8_from_pixels,
    compare_captures_with_tolerance, fnv1a64_hex, sample_rgba8, summarize_pixel_readback,
    summarize_rgba8,
};
pub use controls::{
    CameraBookmark, CameraFlyTo, CameraOrbitUrlState, CameraOrbitUrlStateError, CameraState,
    CameraTransitionError, FlyControls, FollowControls, GizmoAxis, GizmoConstraint, GizmoMode,
    GizmoRay, GizmoSpace, OrbitControlAction, OrbitControls, PointerButton, PointerEvent,
    PointerEventKind, TouchEvent, TouchEventKind, TransformGizmo, TransformGizmoHelpers,
    TransitionEasing,
};
pub use diagnostics::{
    AdapterLimitsReport, AlphaPipelineStatus, AnimationError, AssetError, Backend, BuildError,
    CAPABILITY_REPORT_SCHEMA_V1, Capabilities, CapabilityReport, CapabilityReportV1,
    CapabilityStatus, ChangeKind, DevicePoll, Diagnostic, DiagnosticCode, DiagnosticContext,
    DiagnosticSeverity, Error, GpuAdapterReport, HardwareTier, ImportDiagnosticOverlay,
    ImportDiagnosticOverlayKind, ImportError, InstantiateError, LookupError, NotPreparedReason,
    OutputColorSpace, OutputStageStatus, PostProcessingDepthSourceV1, PostProcessingPassV1,
    PostProcessingReportV1, PrepareError, RenderError, RenderOutcome, RendererStats,
};
pub use geometry::{
    Aabb, GeometryDesc, GeometryError, GeometryMorphTarget, GeometrySkin, GeometryTopology,
    GeometryVertex, Primitive, SkinningMatrix, StaticBatchReport, Vertex,
};
pub use material::{
    AlphaMode, Color, ColorParseError, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
    DEFAULT_STROKE_WIDTH_PX, MaterialDesc, MaterialKind, TextureColorSpace, TextureTransform,
};
pub use picking::{CursorPosition, Hit, HitTarget, InteractionContext, Viewport};
#[cfg(not(target_arch = "wasm32"))]
pub use platform::NativeWindowHandle;
pub use platform::{PlatformSurface, SurfaceEvent, SurfaceKind, SurfaceSize, SurfaceViewport};
pub use reference_image::{
    ReferenceImage, ReferenceImageError, ReferenceImageReport, ReferenceImageTolerance, regress,
    regress_with_tolerance,
};
#[cfg(feature = "inspection")]
pub use render::animation_introspection::{
    ANIMATION_INTROSPECTION_SCHEMA_V1, AnimationChannelChangeCounts, AnimationClipIntrospectionV1,
    AnimationIntrospectionArtifactsV1, AnimationIntrospectionFixV1, AnimationIntrospectionReasonV1,
    AnimationIntrospectionReportV1, AnimationIntrospectionSummaryV1, AnimationObservedValueV1,
    AnimationSampleV1, animation_channel_change_counts, transform_differs, transform_is_finite,
};
#[cfg(feature = "inspection")]
pub use render::appearance::{
    APPEARANCE_EXPECTATION_SCHEMA_V1, APPEARANCE_INTROSPECTION_SCHEMA_V1, AppearanceAlphaSummaryV1,
    AppearanceArtifactsV1, AppearanceCaptureSummaryV1, AppearanceExpectationV1, AppearanceFixV1,
    AppearanceIntrospectionOptions, AppearanceIntrospectionReportV1, AppearanceReasonV1,
    AppearanceRectV1, AppearanceSampleRegionV1, AppearanceSummaryV1, AppearanceTargetExpectationV1,
    AppearanceTargetReportV1,
};
#[cfg(feature = "inspection")]
pub use render::introspection::{
    RENDER_INTROSPECTION_SCHEMA_V1, RenderIntrospectionArtifactsV1,
    RenderIntrospectionCapabilitiesV1, RenderIntrospectionCaptureSummaryV1,
    RenderIntrospectionFixV1, RenderIntrospectionFramingV1, RenderIntrospectionLuminanceV1,
    RenderIntrospectionNodeDetailV1, RenderIntrospectionNodesSummaryV1, RenderIntrospectionOptions,
    RenderIntrospectionReasonV1, RenderIntrospectionRectV1, RenderIntrospectionReportV1,
};
#[cfg(feature = "inspection")]
pub use render::visibility_diagnosis::{
    VISIBILITY_DIAGNOSIS_SCHEMA_V1, VisibilityDiagnosisEvidenceV1, VisibilityDiagnosisFixV1,
    VisibilityDiagnosisOptions, VisibilityDiagnosisReasonV1, VisibilityDiagnosisReportV1,
    VisibilityDiagnosisSummaryV1, VisibilityDiagnosisTargetV1,
};
#[cfg(feature = "inspection")]
pub use render::visual_repair::{
    AGENT_LOOP_RESULT_SCHEMA_V1, AgentLoopResultV1, VISUAL_REPAIR_PLAN_SCHEMA_V1,
    VisualRepairActionV1, VisualRepairPlanV1, VisualRepairRemainingReasonV1,
    VisualRepairSkippedActionV1,
};
pub use render::{
    AntiAliasing, AutoExposureConfig, AutoExposureResult, Background, OffscreenTarget,
    OrderIndependentTransparencyConfig, PixelReadback, PostBloomConfig, Profile, Quality,
    RenderMode, Renderer, RendererOptions, ScreenSpaceAmbientOcclusionConfig, Tonemapper,
    estimate_auto_exposure_from_linear_colors, estimate_auto_exposure_from_srgb8,
};
pub use scene::recipe::{
    RecipeBuildPolicy, SCENE_RECIPE_BUILD_SCHEMA_V1, SCENE_RECIPE_SCHEMA_V1,
    SCENE_RECIPE_VALIDATION_SCHEMA_V1, SceneRecipeAlphaModeV1, SceneRecipeBboxFitExpectationV1,
    SceneRecipeBuildImportV1, SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1,
    SceneRecipeBuildTargetV1, SceneRecipeBuildV1, SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1,
    SceneRecipeCameraV1, SceneRecipeCaptureV1, SceneRecipeColorExpectationV1, SceneRecipeColorV1,
    SceneRecipeDiagnosticV1, SceneRecipeExpectV1, SceneRecipeExpectedExtentV1,
    SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1, SceneRecipeGeometryV1,
    SceneRecipeImportV1, SceneRecipeLightV1, SceneRecipeLookAtTargetV1, SceneRecipeMaterialV1,
    SceneRecipeMeasurementV1, SceneRecipeMeshV1, SceneRecipeNodeV1, SceneRecipePickExpectationV1,
    SceneRecipePrimitiveV1, SceneRecipeSectionBoxV1, SceneRecipeTargetV1,
    SceneRecipeTextureColorSpaceV1, SceneRecipeTextureSlotV1, SceneRecipeTransformV1,
    SceneRecipeV1, SceneRecipeValidationReportV1, SceneRecipeVisibleExpectationV1,
    parse_valid_scene_recipe_json, validate_scene_recipe_json, validate_scene_recipe_value,
};
#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub use scene::recipe::{
    SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1, SceneRecipeRenderResultV1,
    SceneRecipeVerificationReasonV1, SceneRecipeVerificationReportV1,
    SceneRecipeVerificationSummaryV1,
};
pub use scene::{
    AnchorFrame, AnchorKey, Angle, AnnotationAnchor, AnnotationAnchorTarget,
    AnnotationProjectionReportV1, AnnotationProjectionV1, Callout, CalloutAnchor,
    CalloutAnchorKind, CalloutReport, Camera, CameraKey, ClippingPlane, ClippingPlaneKey,
    ClippingPlaneSet, ConnectOptions, ConnectionAlignment, ConnectionError, ConnectionLineOverlay,
    ConnectionMagnetPreview, ConnectionMagnetVisualCue, ConnectionParenting, ConnectionPreview,
    ConnectionRequest, ConnectionRoll, ConnectionWarning, ConnectorFrame, ConnectorKey,
    ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, DepthRange, DirectionalLight,
    ExplodedTransformUpdate, ExplodedView, ExplodedViewPlan, FramingOptions, FramingOutcome,
    GridFloorHandles, GridFloorOptions, ImportAnchor, ImportAnchorDebugMetadata, ImportClip,
    ImportConnector, ImportOptions, ImportPivot, InspectionHelperKind, InspectionHelperReport,
    InspectionToolkitReport, Instance, InstanceId, InstanceSet, InstanceSetKey, LabelBillboard,
    LabelDesc, LabelKey, LabelMetrics, Light, LightBuilder, LightKey, MeasurementAxis,
    MeasurementKind, MeasurementOverlay, MeasurementOverlayReport, MeasurementReport, MeshBuilder,
    MeshNode, ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera,
    PerspectiveCamera, PointLight, ProjectedPoint, Quat, SCENE_ANNOTATION_PROJECTION_SCHEMA_V1,
    SCENE_PLACEMENT_RESULT_SCHEMA_V1, Scene, SceneDirtyState, SceneImport,
    ScenePlacementDiagnosticV1, ScenePlacementResultV1, SceneSkinBinding, SceneTintSnapshot,
    SceneTintSnapshotEntry, SceneVisibilitySnapshot, SceneVisibilitySnapshotEntry, ScreenRect,
    SectionBox, SourceCoordinateSystem, SourceUnits, SpotLight, StudioLightingHandles, Transform,
    UnitFormat, Vec3, placement_align_to_feature_transform, placement_center_transform,
    placement_fit_to_size_transform, placement_ground_transform, placement_look_at_transform,
    placement_place_on_feature_transform,
};
#[cfg(feature = "inspection")]
pub use scene::{
    SCENE_INSPECTION_SCHEMA_V1, SceneCameraFrustumInspection, SceneCameraFrustumInspectionV1,
    SceneDrawInspection, SceneDrawInspectionV1, SceneHostInstanceEntryInspectionV1,
    SceneHostInstanceSetInspectionV1, SceneImportInspectionV1, SceneInspectionCountsV1,
    SceneInspectionReport, SceneInspectionReportV1, SceneInspectionRevisionsV1,
    SceneMaterialInspection, SceneMaterialInspectionV1, SceneMaterialSlotInspectionV1,
    SceneMaterialSourceInspectionV1, SceneNodeInspection, SceneNodeInspectionV1,
    SceneNormalInspection, SceneNormalInspectionV1, SceneTextureInspection,
};
#[cfg(feature = "scene-host")]
pub use scene_host::{
    CONNECTOR_BROWSER_SCHEMA_V1, ConnectorBrowserCandidateV1, ConnectorBrowserConnectorV1,
    ConnectorBrowserReportV1, ConnectorBrowserScopeV1, ConnectorBrowserSummaryV1,
    ConnectorBrowserVisualCueV1, ConnectorLineV1, ConnectorTransformV1, HOST_EVENT_SCHEMA_V1,
    HostEventBatchV1, HostEventHitV1, HostEventHoverPhaseV1, HostEventTargetKindV1, HostEventV1,
    INTERACTION_EXPECTATION_SCHEMA_V1, INTERACTION_VERIFICATION_SCHEMA_V1,
    InteractionCoordinateSpaceV1, InteractionCoordinatesV1, InteractionExpectationV1,
    InteractionStepExpectationV1, InteractionStepExpectedV1, InteractionStepObservedV1,
    InteractionStepReportV1, InteractionVerificationArtifactsV1, InteractionVerificationFixV1,
    InteractionVerificationReasonV1, InteractionVerificationReportV1,
    InteractionVerificationSummaryV1, InteractionViewportV1, PRESENTATION_TIMELINE_SCHEMA_V1,
    PRODUCT_OPTIONS_SCHEMA_V1, PresentationTimelineActionKindV1, PresentationTimelineActionV1,
    PresentationTimelineCameraBookmarkV1, PresentationTimelineV1, ProductOptionGroupV1,
    ProductOptionV1, ProductOptionsV1, SCENE_HOST_ANIMATION_INVENTORY_SCHEMA_V1,
    SCENE_HOST_ASSET_IMPORT_SCHEMA_V1, SCENE_HOST_GIZMO_DRAG_SCHEMA_V1,
    SCENE_HOST_GROUNDING_SCHEMA_V1, SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1,
    SCENE_HOST_SECTION_BOX_SCHEMA_V1, SCENE_HOST_SUBTREE_SCHEMA_V1,
    SCENE_HOST_VISUAL_STATE_SCHEMA_V1, SCENE_HOST_VISUAL_STATES_SCHEMA_V1,
    SceneHostAnimationClipV1, SceneHostAnimationInventoryV1, SceneHostAnimationLoopMode,
    SceneHostAnimationPlayOptions, SceneHostAssetImportReportV1, SceneHostCalloutReportV1,
    SceneHostCameraState, SceneHostClippingPlaneV1, SceneHostCore, SceneHostEasing, SceneHostError,
    SceneHostErrorCode, SceneHostExplodedViewModeV1, SceneHostExplodedViewOptionsV1,
    SceneHostGizmoAxisV1, SceneHostGizmoConstraintV1, SceneHostGizmoDragV1, SceneHostGizmoModeV1,
    SceneHostGizmoRayV1, SceneHostGizmoSpaceV1, SceneHostGroundingFallbackV1,
    SceneHostGroundingPathV1, SceneHostGroundingReportV1, SceneHostMeasurementLabelProjectionV1,
    SceneHostMeasurementOverlayReportV1, SceneHostRecipeBuild, SceneHostSectionBoxReportV1,
    SceneHostSubtreeNodeV1, SceneHostSubtreeReportV1, SceneHostVisualStateSummaryV1,
    SceneHostVisualStateV1, SceneHostVisualStatesReportV1, VISUAL_PATCH_SCHEMA_V1,
    VisualPatchAnimationTimeModeV1, VisualPatchAnimationTimeV1, VisualPatchAppliedCountsV1,
    VisualPatchCameraEasedV1, VisualPatchEntryErrorV1, VisualPatchHoverV1,
    VisualPatchLabelTargetV1, VisualPatchLabelV1, VisualPatchMaterialVariantV1,
    VisualPatchResultV1, VisualPatchRevisionDeltaV1, VisualPatchSectionBoxV1,
    VisualPatchSelectionV1, VisualPatchTintEasedV1, VisualPatchTintV1, VisualPatchTransformEasedV1,
    VisualPatchTransformV1, VisualPatchV1, VisualPatchVisibilityV1, host_event_kind_name,
    physical_px,
};
pub use schema_catalog::{
    AGENT_SMOKE_TEMPLATE_SCHEMA_V1, AgentSmokeTemplateCommandV1, AgentSmokeTemplateFileV1,
    AgentSmokeTemplateV1, SCHEMA_CATALOG_SCHEMA_V1, SCHEMA_ENTRY_SCHEMA_V1, SchemaCatalogEntryV1,
    SchemaCatalogV1, SchemaEntryReportV1, nearest_schema_name, schema_catalog_entry,
    schema_catalog_v1, schema_entry_report_v1,
};
pub use viewer::{
    AssetCatalogPreviewError, AssetCatalogPreviewPng, FirstRender, HeadlessGltfViewer,
    HeadlessGltfViewerBuilder, InteractiveGltfViewer, InteractiveGltfViewerBuilder,
    VIEWER_PROFILE_NAMES, ViewerCaptureError, ViewerPngError, ViewerProfile, ViewerProfileLighting,
    first_render_gltf_headless, headless_gltf_viewer, interactive_gltf_viewer,
    render_asset_catalog_preview_png,
};
#[cfg(all(target_arch = "wasm32", feature = "viewer-element"))]
pub use viewer_element::define_scena_viewer;
pub use viewer_element::{
    SCENA_VIEWER_TAG, ScenaViewerAccessibilityDefaults, ScenaViewerAnnotationAnchor,
    ScenaViewerAnnotationError, ScenaViewerAnnotationLayoutEntry, ScenaViewerAnnotationLayoutInput,
    ScenaViewerAnnotationLayoutOptions, ScenaViewerAnnotationLayoutReport, ScenaViewerAttributes,
    ScenaViewerDropDecision, ScenaViewerDropKind, ScenaViewerDroppedFile, ScenaViewerGestureAction,
    ScenaViewerInspectorDiagnostic, ScenaViewerInspectorSnapshot, ScenaViewerKeyboardAction,
    ScenaViewerProgress, ScenaViewerProgressPhase, ScenaViewerVariantOption,
    ScenaViewerVariantSelection, layout_scena_viewer_annotations,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
