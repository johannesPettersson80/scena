//! `scena` is a Rust-native scene-graph renderer.
//!
//! The first implementation slice establishes the public scene/assets/renderer
//! vocabulary and the explicit prepare/render lifecycle.

pub mod animation;
pub mod assets;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub mod browser_probe;
pub mod controls;
#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
pub mod demo_page;
pub mod diagnostics;
pub mod geometry;
pub mod material;
pub mod picking;
pub mod platform;
pub mod reference_image;
pub mod render;
pub mod scene;
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
    AssetEvictionStats, AssetFetcher, AssetLoadControl, AssetLoadOptions, AssetLoadProgress,
    AssetLoadReport, AssetLoadWarning, AssetPath, AssetStoreId, Assets, DefaultAssetFetcher,
    EnvironmentDerivative, EnvironmentDesc, EnvironmentHandle, EnvironmentPreset,
    EnvironmentPresetMetadata, EnvironmentSourceKind, GeometryHandle, GltfDecoderPolicy,
    GltfExtensionDiagnostic, GltfExtensionStatus, MaterialHandle, MaterialVariantBinding,
    ModelHandle, RetainPolicy, SceneAsset, SceneAssetAnchor, SceneAssetClip, SceneAssetLight,
    SceneAssetMesh, SceneAssetNode, TextureDesc, TextureFilter, TextureHandle, TextureSamplerDesc,
    TextureSourceFormat, TextureWrap, WasmEnvironmentDelivery,
};
#[cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]
pub use assets::{AssetHotReloadError, AssetHotReloadWatcher};
#[cfg(feature = "khronos-samples")]
pub use assets::{KhronosSample, KhronosSampleMetadata, KhronosSamples};
pub use controls::{
    CameraOrbitUrlState, CameraOrbitUrlStateError, FlyControls, FollowControls, OrbitControlAction,
    OrbitControls, PointerButton, PointerEvent, PointerEventKind, TouchEvent, TouchEventKind,
};
pub use diagnostics::{
    AdapterLimitsReport, AlphaPipelineStatus, AnimationError, AssetError, Backend, BuildError,
    Capabilities, CapabilityReport, CapabilityStatus, ChangeKind, DebugOverlay, DevicePoll,
    Diagnostic, DiagnosticCode, DiagnosticSeverity, Error, GpuAdapterReport, HardwareTier,
    ImportDiagnosticOverlay, ImportDiagnosticOverlayKind, ImportError, InstantiateError,
    LookupError, NotPreparedReason, OutputStageStatus, PrepareError, RenderError, RenderOutcome,
    RendererStats,
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
    AutoExposureConfig, AutoExposureResult, Background, OffscreenTarget, PixelReadback,
    PostBloomConfig, Profile, Quality, RenderMode, Renderer, RendererOptions,
    ScreenSpaceAmbientOcclusionConfig, Tonemapper, estimate_auto_exposure_from_linear_colors,
    estimate_auto_exposure_from_srgb8,
};
pub use scene::{
    AnchorFrame, AnchorKey, Angle, Camera, CameraKey, ClippingPlane, ClippingPlaneKey,
    ClippingPlaneSet, ConnectOptions, ConnectionAlignment, ConnectionError, ConnectionLineOverlay,
    ConnectionMagnetPreview, ConnectionMagnetVisualCue, ConnectionParenting, ConnectionPreview,
    ConnectionRequest, ConnectionRoll, ConnectionWarning, ConnectorFrame, ConnectorKey,
    ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, DepthRange, DirectionalLight,
    FramingOptions, FramingOutcome, GridFloorHandles, GridFloorOptions, ImportAnchor,
    ImportAnchorDebugMetadata, ImportClip, ImportConnector, ImportOptions, ImportPivot, Instance,
    InstanceCullingPolicy, InstanceId, InstanceSet, InstanceSetKey, LabelBillboard, LabelDesc,
    LabelKey, LabelRasterization, Light, LightBuilder, LightKey, MeshBuilder, MeshNode,
    ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera, PerspectiveCamera,
    PointLight, ProjectedPoint, Quat, Scene, SceneDirtyState, SceneImport, SceneSkinBinding,
    ScreenRect, SourceCoordinateSystem, SourceUnits, SpotLight, StudioLightingHandles, Transform,
    Vec3,
};
#[cfg(feature = "inspection")]
pub use scene::{
    SceneCameraFrustumInspection, SceneDrawInspection, SceneInspectionReport,
    SceneMaterialInspection, SceneNodeInspection, SceneNormalInspection, SceneTextureInspection,
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
