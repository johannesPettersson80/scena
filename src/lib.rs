//! `scena` is a Rust-native scene-graph renderer.
//!
//! The first implementation slice establishes the public scene/assets/renderer
//! vocabulary and the explicit prepare/render lifecycle.

pub mod animation;
pub mod assets;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub mod browser_probe;
pub mod controls;
pub mod diagnostics;
pub mod geometry;
pub mod material;
pub mod picking;
pub mod platform;
pub mod render;
pub mod scene;

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
    AssetFetcher, AssetLoadControl, AssetLoadProgress, AssetLoadReport, AssetPath, Assets,
    DefaultAssetFetcher, EnvironmentDerivative, EnvironmentDesc, EnvironmentHandle,
    EnvironmentSourceKind, GeometryHandle, GltfDecoderPolicy, GltfExtensionDiagnostic,
    GltfExtensionStatus, MaterialHandle, ModelHandle, RetainPolicy, SceneAsset, SceneAssetAnchor,
    SceneAssetClip, SceneAssetLight, SceneAssetMesh, SceneAssetNode, TextureDesc, TextureFilter,
    TextureHandle, TextureSamplerDesc, TextureSourceFormat, TextureWrap, WasmEnvironmentDelivery,
};
pub use controls::{
    OrbitControlAction, OrbitControls, PointerButton, PointerEvent, PointerEventKind, TouchEvent,
    TouchEventKind,
};
pub use diagnostics::{
    AlphaPipelineStatus, AnimationError, AssetError, Backend, BuildError, Capabilities,
    CapabilityStatus, ChangeKind, DebugOverlay, DevicePoll, Diagnostic, DiagnosticCode,
    DiagnosticSeverity, Error, HardwareTier, ImportDiagnosticOverlay, ImportDiagnosticOverlayKind,
    ImportError, InstantiateError, LookupError, NotPreparedReason, OutputStageStatus, PrepareError,
    RenderError, RenderOutcome, RendererStats,
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
pub use render::{
    OffscreenTarget, PixelReadback, Profile, Quality, RenderMode, Renderer, RendererOptions,
    Tonemapper,
};
pub use scene::{
    Angle, Camera, CameraKey, ClippingPlane, ClippingPlaneKey, ClippingPlaneSet, DepthRange,
    DirectionalLight, ImportAnchor, ImportAnchorDebugMetadata, ImportClip, ImportOptions,
    ImportPivot, Instance, InstanceCullingPolicy, InstanceId, InstanceSet, InstanceSetKey,
    LabelBillboard, LabelDesc, LabelKey, LabelRasterization, Light, LightBuilder, LightKey,
    MeshBuilder, MeshNode, ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera,
    PerspectiveCamera, PointLight, Quat, Scene, SceneDirtyState, SceneImport, SceneSkinBinding,
    SourceCoordinateSystem, SourceUnits, SpotLight, Transform, Vec3,
};
#[cfg(feature = "inspection")]
pub use scene::{SceneInspectionReport, SceneNodeInspection};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
