//! `scena` is a Rust-native scene-graph renderer.
//!
//! The first implementation slice establishes the public scene/assets/renderer
//! vocabulary and the explicit prepare/render lifecycle.
//!
//! Shell-capable LLMs building viewers, CAD inspection scenes, digital twins,
//! configurators, dashboards, documentation renders, or interaction proofs
//! should start from the public [LLM app-builder guide] instead of guessing
//! recipe fields or private Rust APIs.
//!
//! [LLM app-builder guide]: https://github.com/johannesPettersson80/scena/blob/main/docs/guides/llm-app-builder.md

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
#[doc = include_str!("../docs/getting-started.md")]
pub mod onboarding_doctests {}

pub mod animation;
pub mod assets;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub mod browser_probe;
pub mod browser_proof;
pub mod capture;
pub mod contract_validation;
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
pub mod prelude;
pub mod reference_image;
pub mod render;
pub mod scene;
#[cfg(feature = "scene-host")]
pub mod scene_host;
pub mod schema_catalog;
pub mod viewer;
pub mod viewer_element;
pub mod vocabulary;

#[cfg(feature = "scene-host")]
pub(crate) use render::semantic_aov::{
    RawSemanticAovCapture, RawSemanticAovError, RawSemanticAovExclusions,
};

pub use animation::{
    AnimationChannel, AnimationClip, AnimationClipKey, AnimationInterpolation, AnimationLoopMode,
    AnimationMixer, AnimationMixerKey, AnimationOutput, AnimationPlaybackState,
    AnimationSourceChannel, AnimationSourceClip, AnimationTarget, AnimationUpdateMetrics,
};
#[cfg(target_arch = "wasm32")]
pub use assets::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use assets::FileAssetFetcher;
pub use assets::{
    ASSET_CATALOG_SCHEMA_V1, ASSET_CONVERSION_SCHEMA_V1, ASSET_DOCTOR_REPORT_SCHEMA_V1,
    ASSET_GEOMETRY_SUMMARY_SCHEMA_V1, ASSET_LOAD_REPORT_SCHEMA_V1,
    ASSET_READINESS_REPORT_SCHEMA_V1, AssetCatalogAssetV1, AssetCatalogExpectedBoundsV1,
    AssetCatalogFeatureRequirementV1, AssetCatalogMaterialRequirementsV1, AssetCatalogPreviewV1,
    AssetCatalogV1, AssetConversionDiagnosticSeverityV1, AssetConversionDiagnosticStreamV1,
    AssetConversionDiagnosticV1, AssetConversionReportV1, AssetConversionStatusV1, AssetDerivative,
    AssetDoctorFindingV1, AssetDoctorReportV1, AssetDoctorSeverityV1, AssetDoctorSummaryV1,
    AssetEvictionStats, AssetExternalResource, AssetExternalResourceKind,
    AssetExternalResourceStatus, AssetExternalResourceV1, AssetFetcher, AssetLoadControl,
    AssetLoadOptions, AssetLoadProgress, AssetLoadProgressV1, AssetLoadReport, AssetLoadReportV1,
    AssetLoadWarning, AssetLoadWarningV1, AssetMaterialFallback, AssetMaterialFallbackKind,
    AssetMaterialFallbackV1, AssetMaterialSource, AssetMaterialSourceKind, AssetPath,
    AssetProvenance, AssetReadinessAssetReportV1, AssetReadinessFindingV1, AssetReadinessPreviewV1,
    AssetReadinessReportV1, AssetReadinessSeverityV1, AssetReadinessSummaryV1, AssetReloadError,
    AssetStoreId, Assets, DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION, DefaultAssetFetcher,
    EnvironmentDerivative, EnvironmentDesc, EnvironmentHandle, EnvironmentPrefilterSidecar,
    EnvironmentPreset, EnvironmentPresetMetadata, EnvironmentSidecarHeader,
    EnvironmentSidecarProfile, EnvironmentSourceKind, GeometryHandle, GltfDecoderPolicy,
    GltfExtensionDiagnostic, GltfExtensionStatus, GltfSceneSelection,
    MATERIAL_LIBRARY_CATALOG_SCHEMA_V1, MATERIAL_LIBRARY_CATALOG_SCHEMA_V2, MaterialHandle,
    MaterialPresetAssets, MaterialPresetProvenance, MaterialVariantBinding, ModelHandle,
    PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES, PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1,
    PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2, PhotographicMaterialArchiveVariantV2,
    PhotographicMaterialCatalogEntryV1, PhotographicMaterialCatalogEntryV2,
    PhotographicMaterialCatalogV1, PhotographicMaterialCatalogV2, PhotographicMaterialCategoryV1,
    PhotographicMaterialMapKindV1, PhotographicMaterialPackAssets,
    PhotographicMaterialPackMapRoleV1, PhotographicMaterialPackMapV1,
    PhotographicMaterialPackSourceV1, PhotographicMaterialPackV1, PhotographicMaterialPackV2,
    PhotographicMaterialResolutionV1, PhotographicSurfaceAssets, PhotographicSurfaceDesc,
    PhotographicSurfaceKind, RetainPolicy, SIDECAR_FILE_SUFFIX, SceneAsset, SceneAssetAnchor,
    SceneAssetClip, SceneAssetGeometrySummary, SceneAssetLight, SceneAssetMesh, SceneAssetNode,
    SelectedGltfScene, TextureDesc, TextureFilter, TextureHandle, TextureMemoryDesc,
    TextureMemoryId, TextureMipPolicy, TexturePixelFormat, TextureSamplerDesc, TextureSlot,
    TextureSourceFormat, TextureWrap, WasmEnvironmentDelivery, parse_sidecar_header,
    photographic_material_catalog_v1, photographic_material_catalog_v2,
    select_photographic_material_resolution, source_backed_material_preset_provenance,
    validate_scene_recipe_json_with_assets, validate_scene_recipe_json_with_assets_and_policy,
};
#[cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]
pub use assets::{AssetHotReloadError, AssetHotReloadWatcher};
#[cfg(feature = "khronos-samples")]
pub use assets::{KhronosSample, KhronosSampleMetadata, KhronosSamples};
#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
pub use assets::{
    PhotographicMaterialPackError, compile_photographic_material_archive,
    compile_photographic_material_archive_at_resolution,
};
pub use browser_proof::{BROWSER_PROOF_RUN_SCHEMA_V1, BrowserProofRunV1};
pub use capture::{
    CAPTURE_BASELINE_SCHEMA_V1, CAPTURE_SCHEMA_V1, CaptureAutoFrame, CaptureAutoFrameViewport,
    CaptureBaselineDiff, CaptureBaselineError, CaptureBaselineReport, CaptureBaselineTolerance,
    CaptureCamera, CaptureContactSheet, CaptureContactSheetError, CaptureContactSheetTile,
    CaptureDescriptor, CaptureError, CaptureFrameProvenance, CaptureOptions, CapturePayload,
    CapturePayloadKind, CapturePixelBounds, CapturePixelSummary, CapturePngError, CapturePoint2,
    CaptureProjectedPoint, CaptureProjection, CaptureRevisions, CaptureRgba8, CaptureScreenRect,
    CaptureScreenRegion, CaptureViewport, auto_frame_metadata, capture_contact_sheet_rgba8,
    capture_rgba8, capture_rgba8_from_pixels, capture_unverified_rgba8_from_pixels,
    compare_captures_with_tolerance, fnv1a64_hex, project_aabb_from_capture,
    project_world_point_from_capture, sample_rgba8, screen_region_from_center_size,
    screen_region_from_points, screen_region_from_rect, summarize_pixel_readback, summarize_rgba8,
    transform_point_for_projection,
};
pub use contract_validation::{
    CONTRACT_VALIDATION_SCHEMA_V1, ContractValidationDiagnosticV1, ContractValidationReportV1,
    JSON_SCHEMA_EXPORT_SCHEMA_V1, JsonSchemaExportV1, contract_json_schema_export_v1,
    validate_contract_json_v1,
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
    CAPABILITY_REPORT_SCHEMA_V1, Capabilities, CapabilityConstraintProbeV1,
    CapabilityConstraintStatusV1, CapabilityProbeModeV1, CapabilityProbeStatusV1,
    CapabilityProbeUnavailableV1, CapabilityProbeV1, CapabilityReport, CapabilityReportV1,
    CapabilityStatus, CapabilityTargetProbeV1, ChangeKind, DevicePoll, DevicePollStatus,
    Diagnostic, DiagnosticCode, DiagnosticContext, DiagnosticSeverity, Error, ErrorDiagnostic,
    GpuAdapterReport, GpuDeviceReport, HardwareTier, ImportDiagnosticOverlay,
    ImportDiagnosticOverlayKind, ImportError, InstantiateError, Ktx2ColorSpaceDfd, LookupError,
    MissingTextureDetails, NotPreparedReason, OutputColorSpace, OutputStageStatus,
    PostProcessingDepthSourceV1, PostProcessingPassV1, PostProcessingReportV1, PrepareError,
    RenderError, RenderOutcome, RendererStats, nearest_name_candidates,
};
pub use geometry::{
    Aabb, GeometryDesc, GeometryError, GeometryMorphTarget, GeometrySkin, GeometryTopology,
    GeometryVertex, Primitive, SkinningMatrix, StaticBatchReport, Vertex,
};
pub use material::{
    AlphaMode, Color, ColorParseError, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
    DEFAULT_STROKE_WIDTH_PX, MaterialDesc, MaterialKind, PhotographicMicroSurface,
    TextureColorSpace, TextureTransform,
};
pub use picking::{CursorPosition, Hit, HitTarget, InteractionContext, PickingMetrics, Viewport};
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
    RenderIntrospectionTimingsV1,
};
#[cfg(feature = "inspection")]
pub use render::quality::{
    DepthOfFieldQualityInput, RENDER_QUALITY_SCHEMA_V1, ReferenceQualityMetrics,
    RenderQualityAreaLightMetrics, RenderQualityCheckV1, RenderQualityDepthOfFieldMetrics,
    RenderQualityFrameMetrics, RenderQualityGeometryEdgeMetrics, RenderQualityGridLineMetrics,
    RenderQualityLabelBackgroundMetrics, RenderQualityLabelMetrics, RenderQualityLineMetrics,
    RenderQualityProfile, RenderQualityRegion, RenderQualityRegionV1, RenderQualityReportV1,
    RenderQualityRgba8Input, RenderQualityStatusV1, RenderQualitySummaryV1,
    area_light_shadow_metrics, depth_of_field_metrics, evaluate_area_light_region_quality,
    evaluate_depth_of_field_region_quality, evaluate_geometry_region_quality,
    evaluate_grid_line_region_quality, evaluate_grounding_region_quality,
    evaluate_label_region_quality, evaluate_label_region_quality_with_background,
    evaluate_line_region_quality, evaluate_reflection_region_quality, evaluate_render_quality,
    evaluate_render_quality_rgba8, evaluate_render_quality_rgba8_region, frame_metrics,
    geometry_edge_metrics, grid_line_metrics, label_background_metrics, label_metrics,
    line_metrics, reference_quality_metrics, ssim_grayscale,
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
    AntiAliasing, AutoExposureConfig, AutoExposureMeteringDomain, AutoExposureResult,
    AutoExposureStatus, AutoExposureSubjectRect, Background, BakedAmbientOcclusionConfig,
    DepthOfFieldConfig, HeadlessBackendSelectionReport, MeteringMode, OffscreenTarget,
    OrderIndependentTransparencyConfig, PixelReadback, PostBloomConfig, PrepareWorkMetrics,
    Profile, Quality, ReconstructionFilter, RenderMode, RenderReadbackMode, RenderWorkMetrics,
    Renderer, RendererOptions, ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig,
    Tonemapper, WhiteBalance, estimate_auto_exposure_from_linear_colors,
    estimate_auto_exposure_from_linear_colors_with_subject_rect, estimate_auto_exposure_from_srgb8,
};
#[cfg(feature = "inspection")]
pub use render::{
    EXPOSURE_REPORT_SCHEMA_V1, ExposureReportAutoV1, ExposureReportFrameKeyV1,
    ExposureReportSubjectV1, ExposureReportV1, FOCUS_REPORT_SCHEMA_V1, FocusReportFrameKeyV1,
    FocusReportResolvedV1, FocusReportTargetV1, FocusReportV1, SUBJECT_OBSERVATION_SCHEMA_V1,
    SubjectObservationBoundsV1, SubjectObservationDepthV1, SubjectObservationFallbackV1,
    SubjectObservationFrameKeyV1, SubjectObservationMetricsV1, SubjectObservationPixelQualityV1,
    SubjectObservationTargetV1, SubjectObservationV1,
};
pub use scene::recipe::{
    FIELD_MODEL_SCHEMA_V1, RECIPE_BUILD_RESULT_SCHEMA_V1, RECIPE_POLICY_SCHEMA_V1,
    RecipeBuildExecutionV1, RecipeBuildPolicy, RecipeBuildPolicyBoolV1, RecipeBuildPolicyLimitV1,
    RecipeBuildPolicyReportV1, RecipeBuildPolicyRootV1, RecipeBuildPolicyStringV1,
    RecipeBuildResultV1, RecipeValidationModeV1, SCENE_RECIPE_BUILD_SCHEMA_V1,
    SCENE_RECIPE_DIFF_SCHEMA_V1, SCENE_RECIPE_SCHEMA_V1, SCENE_RECIPE_VALIDATION_SCHEMA_V1,
    SceneRecipeAlphaModeV1, SceneRecipeAnchorSourceV1, SceneRecipeAnchorV1,
    SceneRecipeAnimationChannelV1, SceneRecipeAnimationV1, SceneRecipeBackendExpectationV1,
    SceneRecipeBboxFitExpectationV1, SceneRecipeBoundsSourceV1, SceneRecipeBoundsV1,
    SceneRecipeBuildAnchorV1, SceneRecipeBuildAnimationV1, SceneRecipeBuildBoundsV1,
    SceneRecipeBuildConnectionV1, SceneRecipeBuildConnectorV1, SceneRecipeBuildImportV1,
    SceneRecipeBuildInstanceV1, SceneRecipeBuildNamedStateV1, SceneRecipeBuildResourceV1,
    SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1, SceneRecipeBuildV1,
    SceneRecipeCalloutTargetV1, SceneRecipeCalloutV1, SceneRecipeCameraV1, SceneRecipeCaptureV1,
    SceneRecipeClippingExpectationV1, SceneRecipeClippingPlaneV1, SceneRecipeColorExpectationV1,
    SceneRecipeColorV1, SceneRecipeConnectionParentingV1, SceneRecipeConnectionRollV1,
    SceneRecipeConnectorAlignmentV1, SceneRecipeConnectorMateV1, SceneRecipeConnectorPolarityV1,
    SceneRecipeConnectorRollPolicyV1, SceneRecipeConnectorSourceV1, SceneRecipeConnectorV1,
    SceneRecipeDepthOfFieldTargetV1, SceneRecipeDiagnosticResourceV1, SceneRecipeDiagnosticV1,
    SceneRecipeDiffChangeKindV1, SceneRecipeDiffChangeV1, SceneRecipeDiffOptions,
    SceneRecipeDiffReportV1, SceneRecipeDiffScopeV1, SceneRecipeExpectV1,
    SceneRecipeExpectedExtentV1, SceneRecipeExplodedViewModeV1, SceneRecipeExplodedViewV1,
    SceneRecipeFontV1, SceneRecipeGeometryV1, SceneRecipeGroundedExpectationV1,
    SceneRecipeHelperOcclusionExpectationV1, SceneRecipeImportEdgeRoundingReportV1,
    SceneRecipeImportEdgeRoundingV1, SceneRecipeImportMaterialBindingV1, SceneRecipeImportV1,
    SceneRecipeInstanceSetV1, SceneRecipeInstanceV1, SceneRecipeLabelV1, SceneRecipeLightV1,
    SceneRecipeLookAtTargetV1, SceneRecipeMaterialPackV1, SceneRecipeMaterialV1,
    SceneRecipeMeasurementV1, SceneRecipeMeshV1, SceneRecipeMeteringRectV1,
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
    SceneRecipeResourceResolutionV1, SceneRecipeResourceStatusV1, SceneRecipeSectionBoxV1,
    SceneRecipeSeparationExpectationV1, SceneRecipeSkinV1, SceneRecipeSourceMaterialSelectorV1,
    SceneRecipeSpatialTargetV1, SceneRecipeStateExpectationV1, SceneRecipeStateTintV1,
    SceneRecipeStateTransformV1, SceneRecipeStateVisibilityV1, SceneRecipeSubjectFallbackPolicyV1,
    SceneRecipeSubjectSpecV1, SceneRecipeSubjectV1, SceneRecipeTargetBoundsV1,
    SceneRecipeTargetFitExpectationV1, SceneRecipeTargetRegionV1, SceneRecipeTargetResolutionError,
    SceneRecipeTargetResolutionErrorKind, SceneRecipeTargetResolutionMode, SceneRecipeTargetV1,
    SceneRecipeTextureColorSpaceV1, SceneRecipeTextureSlotV1, SceneRecipeTransformConversionError,
    SceneRecipeTransformExpectationV1, SceneRecipeTransformV1, SceneRecipeV1,
    SceneRecipeValidationReportV1, SceneRecipeVisibleExpectationV1, SchemaFieldModelV1,
    SchemaFieldV1, diff_scene_recipes, parse_valid_scene_recipe_json,
    parse_valid_scene_recipe_json_with_policy, recipe_too_large_report,
    resolve_scene_recipe_target_handles, scene_recipe_field_model_v1,
    scene_recipe_json_schema_paths_v1, scene_recipe_json_schema_v1, validate_scene_recipe_json,
    validate_scene_recipe_json_syntax_with_policy, validate_scene_recipe_json_with_policy,
    validate_scene_recipe_value, validate_scene_recipe_value_with_policy,
};
#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub use scene::recipe::{
    SCENE_COMPOSITION_SCHEMA_V1, SCENE_RECIPE_RENDER_RESULT_SCHEMA_V1, SceneCompositionCheckV1,
    SceneCompositionRegionV1, SceneCompositionReportV1, SceneCompositionStatusV1,
    SceneCompositionSummaryV1, SceneRecipeRenderResultV1, SceneRecipeVerificationReasonV1,
    SceneRecipeVerificationReportV1, SceneRecipeVerificationSummaryV1,
};
pub use scene::{
    AnchorFrame, AnchorKey, Angle, AnnotationAnchor, AnnotationAnchorTarget,
    AnnotationProjectionReportV1, AnnotationProjectionV1, AreaLight, AreaLightShape, Callout,
    CalloutAnchor, CalloutAnchorKind, CalloutReport, Camera, CameraKey, ClippingPlane,
    ClippingPlaneKey, ClippingPlaneSet, ConnectOptions, ConnectionAlignment, ConnectionError,
    ConnectionLineOverlay, ConnectionMagnetPreview, ConnectionMagnetVisualCue, ConnectionParenting,
    ConnectionPreview, ConnectionRequest, ConnectionRoll, ConnectionWarning, ConnectorFrame,
    ConnectorKey, ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy,
    DEFAULT_REFLECTION_PROBE_RESOLUTION, DepthRange, DirectionalLight, ExplodedTransformUpdate,
    ExplodedView, ExplodedViewPlan, FramingOptions, FramingOutcome, GridFloorHandles,
    GridFloorOptions, ImportAnchor, ImportAnchorDebugMetadata, ImportClip, ImportConnector,
    ImportOptions, ImportPivot, InspectionHelperKind, InspectionHelperReport,
    InspectionToolkitReport, Instance, InstanceId, InstanceSet, InstanceSetKey, LabelBillboard,
    LabelDesc, LabelFontError, LabelFontFace, LabelKey, LabelMetrics, Light, LightBuilder,
    LightKey, MAX_REFLECTION_PROBES, MeasurementAxis, MeasurementKind, MeasurementOverlay,
    MeasurementOverlayReport, MeasurementReport, MeshBuilder, MeshNode, ModelBuilder, ModelNode,
    Node, NodeKey, NodeKind, OrthographicCamera, PLACEMENT_VERBS, Particle, ParticleSet,
    ParticleSetError, ParticleSetKey, PerspectiveCamera, PointLight, ProjectedPoint, Quat,
    ReflectionProbe, ReflectionProbeError, ReflectionProbeKey,
    SCENE_ANNOTATION_PROJECTION_SCHEMA_V1, SCENE_PLACEMENT_RESULT_SCHEMA_V1,
    SCENE_RECIPE_PATCH_SCHEMA_V1, Scene, SceneDirtyState, SceneImport, ScenePlacementDiagnosticV1,
    ScenePlacementResultV1, ScenePlacementTargetV1, SceneRecipePatchResultV1,
    SceneRecipePatchSuccessInputV1, SceneRecipeSemanticChangeV1, SceneSkinBinding,
    SceneTintSnapshot, SceneTintSnapshotEntry, SceneVisibilityRestoreReport,
    SceneVisibilitySnapshot, SceneVisibilitySnapshotEntry, ScreenRect, SectionBox,
    SourceCoordinateSystem, SourceUnits, SpotLight, StudioLightingHandles, Transform, UnitFormat,
    Vec3, placement_align_to_feature_transform, placement_center_transform,
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
    InteractionVerificationSummaryV1, InteractionViewportV1, PHOTO_CANDIDATE_PLAN_SCHEMA_V1,
    PHOTO_PLAN_SCHEMA_V1, PHOTO_QUALITY_ANALYSIS_SCHEMA_V1, PHOTO_QUALITY_EXECUTION_SCHEMA_V1,
    PHOTO_REPORT_SCHEMA_V1, PHOTO_SHADED_CANDIDATE_SELECTION_SCHEMA_V1,
    PHOTO_SUBJECT_REGION_SCHEMA_V1, PHOTOGRAPHIC_MATERIAL_RESOLUTION_SELECTION_SCHEMA_V1,
    PRESENTATION_TIMELINE_SCHEMA_V1, PRODUCT_OPTIONS_SCHEMA_V1, PhotoCandidateConstraintsV1,
    PhotoCandidateFillRangeV1, PhotoCandidateObservation, PhotoCandidatePlanV1,
    PhotoCandidateRequest, PhotoCandidateScore, PhotoCandidateScoringReport,
    PhotoCandidateStagingV1, PhotoCompositionCandidateV1, PhotoContourQualityMetricsV1,
    PhotoGroundingQualityMetricsV1, PhotoMaterialQualityMetricsV1, PhotoPhysicalCameraV1,
    PhotoPlanArtifactsV1, PhotoPlanSourceV1, PhotoPlanSubjectV1, PhotoPlanTargetV1, PhotoPlanV1,
    PhotoProjectedTextureDensityV1, PhotoQualityAnalysisInputV1, PhotoQualityAnalysisReportV1,
    PhotoReportV1, PhotoSubjectRegionV1, PhotographicAssetIssueClassV1, PhotographicAssetIssueV1,
    PhotographicMaterialResolutionSelectionReportV1, PhotographicMaterialResolutionSelectionV1,
    PhotographicSurfaceReportV1, PhotographicSurroundingsReportV1,
    PresentationTimelineActionKindV1, PresentationTimelineActionV1,
    PresentationTimelineCameraBookmarkV1, PresentationTimelineV1, ProductOptionGroupV1,
    ProductOptionV1, ProductOptionsV1, SCENE_HOST_ANIMATION_INVENTORY_SCHEMA_V1,
    SCENE_HOST_ASSET_IMPORT_SCHEMA_V1, SCENE_HOST_CLIPPING_PLANES_SCHEMA_V1,
    SCENE_HOST_GIZMO_DRAG_SCHEMA_V1, SCENE_HOST_GROUNDING_SCHEMA_V1,
    SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1, SCENE_HOST_SECTION_BOX_SCHEMA_V1,
    SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1, SCENE_HOST_SUBTREE_SCHEMA_V1,
    SCENE_HOST_VISUAL_STATE_SCHEMA_V1, SCENE_HOST_VISUAL_STATES_SCHEMA_V1,
    SceneHostAnimationClipV1, SceneHostAnimationInventoryV1, SceneHostAnimationLoopMode,
    SceneHostAnimationPlayOptions, SceneHostAssetImportReportV1, SceneHostCalloutReportV1,
    SceneHostCameraProjection, SceneHostCameraState, SceneHostClippingPlaneV1,
    SceneHostClippingPlanesV1, SceneHostCore, SceneHostEasing, SceneHostError, SceneHostErrorCode,
    SceneHostExplodedViewModeV1, SceneHostExplodedViewOptionsV1, SceneHostGizmoAxisV1,
    SceneHostGizmoConstraintV1, SceneHostGizmoDragV1, SceneHostGizmoModeV1, SceneHostGizmoRayV1,
    SceneHostGizmoSpaceV1, SceneHostGroundingFallbackV1, SceneHostGroundingPathV1,
    SceneHostGroundingReportV1, SceneHostMeasurementAuthorityV1,
    SceneHostMeasurementLabelProjectionV1, SceneHostMeasurementOverlayReportV1,
    SceneHostRecipeBuild, SceneHostSectionBoxReportV1, SceneHostSemanticAovCaptureV1,
    SceneHostSemanticAovExclusionsV1, SceneHostSemanticAovLegendEntryV1, SceneHostSubtreeNodeV1,
    SceneHostSubtreeReportV1, SceneHostVisualStateSummaryV1, SceneHostVisualStateV1,
    SceneHostVisualStatesReportV1, SceneSetupPreset, VISUAL_PATCH_SCHEMA_V1,
    VisualPatchAnimationTimeModeV1, VisualPatchAnimationTimeV1, VisualPatchAppliedCountsV1,
    VisualPatchCameraEasedV1, VisualPatchEntryErrorV1, VisualPatchHoverV1,
    VisualPatchLabelTargetV1, VisualPatchLabelV1, VisualPatchMaterialVariantV1,
    VisualPatchResultV1, VisualPatchRevisionDeltaV1, VisualPatchSectionBoxV1,
    VisualPatchSelectionV1, VisualPatchTintEasedV1, VisualPatchTintV1, VisualPatchTransformEasedV1,
    VisualPatchTransformV1, VisualPatchV1, VisualPatchVisibilityV1, analyze_photo_quality,
    camera_behavior_candidate_plan, host_event_kind_name, physical_px, product_hero_candidate_plan,
    score_camera_behavior_candidates, score_product_hero_candidates,
};
pub use schema_catalog::{
    AGENT_GUIDE_SCHEMA_V1, AGENT_SMOKE_TEMPLATE_SCHEMA_V1, AGENT_TEMPLATE_CATALOG_SCHEMA_V1,
    AgentGuideV1, AgentSmokeTemplateCommandV1, AgentSmokeTemplateFileV1, AgentSmokeTemplateV1,
    AgentTemplateCatalogEntryV1, AgentTemplateCatalogV1, SCHEMA_CATALOG_SCHEMA_V1,
    SCHEMA_ENTRY_SCHEMA_V1, SchemaCatalogEntryV1, SchemaCatalogV1, SchemaEntryReportV1,
    agent_guide_v1, nearest_schema_name, schema_catalog_entry, schema_catalog_v1,
    schema_entry_report_v1,
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
pub use vocabulary::{
    VOCABULARY_SCHEMA_V1, VocabularyReportV1, VocabularyV1, VocabularyValueV1,
    validate_vocabulary_report_v1, vocabulary_report_v1, vocabulary_v1,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
