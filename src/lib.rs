//! `scena` is a Rust-native scene-graph renderer.
//!
//! The first implementation slice establishes the public scene/assets/renderer
//! vocabulary and the explicit prepare/render lifecycle.

pub mod animation;
pub mod assets;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub mod browser_probe;
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
    ASSET_GEOMETRY_SUMMARY_SCHEMA_V1, ASSET_LOAD_REPORT_SCHEMA_V1, AssetDerivative,
    AssetEvictionStats, AssetFetcher, AssetLoadControl, AssetLoadOptions, AssetLoadProgress,
    AssetLoadProgressV1, AssetLoadReport, AssetLoadReportV1, AssetLoadWarning, AssetLoadWarningV1,
    AssetPath, AssetProvenance, AssetStoreId, Assets, DefaultAssetFetcher, EnvironmentDerivative,
    EnvironmentDesc, EnvironmentHandle, EnvironmentPrefilterSidecar, EnvironmentPreset,
    EnvironmentPresetMetadata, EnvironmentSidecarHeader, EnvironmentSidecarProfile,
    EnvironmentSourceKind, GeometryHandle, GltfDecoderPolicy, GltfExtensionDiagnostic,
    GltfExtensionStatus, MaterialHandle, MaterialPresetAssets, MaterialPresetProvenance,
    MaterialVariantBinding, ModelHandle, RetainPolicy, SIDECAR_FILE_SUFFIX, SceneAsset,
    SceneAssetAnchor, SceneAssetClip, SceneAssetGeometrySummary, SceneAssetLight, SceneAssetMesh,
    SceneAssetNode, TextureDesc, TextureFilter, TextureHandle, TextureSamplerDesc,
    TextureSourceFormat, TextureWrap, WasmEnvironmentDelivery, parse_sidecar_header,
    source_backed_material_preset_provenance,
};
#[cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]
pub use assets::{AssetHotReloadError, AssetHotReloadWatcher};
#[cfg(feature = "khronos-samples")]
pub use assets::{KhronosSample, KhronosSampleMetadata, KhronosSamples};
pub use capture::{
    CAPTURE_SCHEMA_V1, CaptureAutoFrame, CaptureAutoFrameViewport, CaptureCamera,
    CaptureDescriptor, CaptureError, CaptureOptions, CapturePayload, CapturePayloadKind,
    CapturePixelBounds, CapturePixelSummary, CapturePoint2, CaptureProjection, CaptureRevisions,
    CaptureRgba8, CaptureScreenRect, CaptureViewport, auto_frame_metadata, capture_rgba8,
    capture_rgba8_from_pixels, fnv1a64_hex, sample_rgba8, summarize_pixel_readback,
    summarize_rgba8,
};
pub use controls::{
    CameraOrbitUrlState, CameraOrbitUrlStateError, FlyControls, FollowControls, OrbitControlAction,
    OrbitControls, PointerButton, PointerEvent, PointerEventKind, TouchEvent, TouchEventKind,
};
pub use diagnostics::{
    AdapterLimitsReport, AlphaPipelineStatus, AnimationError, AssetError, Backend, BuildError,
    CAPABILITY_REPORT_SCHEMA_V1, Capabilities, CapabilityReport, CapabilityReportV1,
    CapabilityStatus, ChangeKind, DebugOverlay, DevicePoll, Diagnostic, DiagnosticCode,
    DiagnosticSeverity, Error, GpuAdapterReport, HardwareTier, ImportDiagnosticOverlay,
    ImportDiagnosticOverlayKind, ImportError, InstantiateError, LookupError, NotPreparedReason,
    OutputColorSpace, OutputStageStatus, PrepareError, RenderError, RenderOutcome, RendererStats,
};
pub use geometry::{
    Aabb, GeometryDesc, GeometryError, GeometryMorphTarget, GeometrySkin, GeometryTopology,
    GeometryVertex, Primitive, SkinningMatrix, StaticBatchReport, Vertex,
};
pub use material::{
    AlphaMode, Color, ColorParseError, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
    DEFAULT_STROKE_WIDTH_PX, MaterialDesc, MaterialKind, TextureColorSpace, TextureTransform,
};
pub use picking::{CursorPosition, Hit, HitTarget, InteractionContext, InteractionStyle, Viewport};
#[cfg(not(target_arch = "wasm32"))]
pub use platform::NativeWindowHandle;
pub use platform::{PlatformSurface, SurfaceEvent, SurfaceKind, SurfaceSize, SurfaceViewport};
pub use reference_image::{
    ReferenceImage, ReferenceImageError, ReferenceImageReport, ReferenceImageTolerance, regress,
    regress_with_tolerance,
};
pub use render::{
    AntiAliasing, AutoExposureConfig, AutoExposureResult, Background, OffscreenTarget,
    OrderIndependentTransparencyConfig, PixelReadback, PostBloomConfig, Profile, Quality,
    RenderMode, Renderer, RendererOptions, ScreenSpaceAmbientOcclusionConfig, Tonemapper,
    estimate_auto_exposure_from_linear_colors, estimate_auto_exposure_from_srgb8,
};
pub use scene::{
    AnchorFrame, AnchorKey, Angle, AnnotationAnchor, AnnotationAnchorTarget,
    AnnotationProjectionReportV1, AnnotationProjectionV1, Camera, CameraKey, ClippingPlane,
    ClippingPlaneKey, ClippingPlaneSet, ConnectOptions, ConnectionAlignment, ConnectionError,
    ConnectionLineOverlay, ConnectionMagnetPreview, ConnectionMagnetVisualCue, ConnectionParenting,
    ConnectionPreview, ConnectionRequest, ConnectionRoll, ConnectionWarning, ConnectorFrame,
    ConnectorKey, ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, DepthRange,
    DirectionalLight, FramingOptions, FramingOutcome, GridFloorHandles, GridFloorOptions,
    ImportAnchor, ImportAnchorDebugMetadata, ImportClip, ImportConnector, ImportOptions,
    ImportPivot, Instance, InstanceCullingPolicy, InstanceId, InstanceSet, InstanceSetKey,
    LabelBillboard, LabelDesc, LabelKey, LabelRasterization, Light, LightBuilder, LightKey,
    MeshBuilder, MeshNode, ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera,
    PerspectiveCamera, PointLight, ProjectedPoint, Quat, SCENE_ANNOTATION_PROJECTION_SCHEMA_V1,
    Scene, SceneDirtyState, SceneImport, SceneSkinBinding, ScreenRect, SourceCoordinateSystem,
    SourceUnits, SpotLight, StudioLightingHandles, Transform, Vec3,
};
#[cfg(feature = "inspection")]
pub use scene::{
    SCENE_INSPECTION_SCHEMA_V1, SceneCameraFrustumInspection, SceneCameraFrustumInspectionV1,
    SceneDrawInspection, SceneDrawInspectionV1, SceneInspectionCountsV1, SceneInspectionReport,
    SceneInspectionReportV1, SceneInspectionRevisionsV1, SceneMaterialInspection,
    SceneNodeInspection, SceneNodeInspectionV1, SceneNormalInspection, SceneNormalInspectionV1,
    SceneTextureInspection,
};
#[cfg(feature = "scene-host")]
pub use scene_host::{
    SCENE_HOST_ASSET_IMPORT_SCHEMA_V1, SceneHostAssetImportReportV1, SceneHostCameraState,
    SceneHostCore, SceneHostError, SceneHostErrorCode,
};
pub use viewer::{
    FirstRender, HeadlessGltfViewer, HeadlessGltfViewerBuilder, InteractiveGltfViewer,
    InteractiveGltfViewerBuilder, ViewerCaptureError, ViewerPngError, first_render_gltf_headless,
    headless_gltf_viewer, interactive_gltf_viewer,
};
#[cfg(all(target_arch = "wasm32", feature = "viewer-element"))]
pub use viewer_element::define_scena_viewer;
pub use viewer_element::{
    SCENA_VIEWER_TAG, ScenaViewerAccessibilityDefaults, ScenaViewerAnnotationAnchor,
    ScenaViewerAnnotationError, ScenaViewerAttributes, ScenaViewerDropDecision,
    ScenaViewerDropKind, ScenaViewerDroppedFile, ScenaViewerGestureAction,
    ScenaViewerInspectorDiagnostic, ScenaViewerInspectorSnapshot, ScenaViewerKeyboardAction,
    ScenaViewerProgress, ScenaViewerProgressPhase, ScenaViewerVariantOption,
    ScenaViewerVariantSelection,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
