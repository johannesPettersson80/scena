//! `scena` is a Rust-native scene-graph renderer.
//!
//! The first implementation slice establishes the public scene/assets/renderer
//! vocabulary and the explicit prepare/render lifecycle.

pub mod animation;
pub mod assets;
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
    AssetFetcher, AssetPath, Assets, DefaultAssetFetcher, EnvironmentDerivative, EnvironmentDesc,
    EnvironmentHandle, EnvironmentSourceKind, GeometryHandle, MaterialHandle, ModelHandle,
    RetainPolicy, SceneAsset, SceneAssetAnchor, SceneAssetClip, SceneAssetLight, SceneAssetMesh,
    SceneAssetNode, TextureDesc, TextureHandle, WasmEnvironmentDelivery,
};
pub use diagnostics::{
    AlphaPipelineStatus, AnimationError, AssetError, Backend, BuildError, Capabilities,
    CapabilityStatus, ChangeKind, DevicePoll, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    Error, ImportDiagnosticOverlay, ImportDiagnosticOverlayKind, ImportError, InstantiateError,
    LookupError, NotPreparedReason, OutputStageStatus, PrepareError, RenderError, RenderOutcome,
    RendererStats,
};
pub use geometry::{
    Aabb, GeometryDesc, GeometryError, GeometryMorphTarget, GeometrySkin, GeometryTopology,
    GeometryVertex, Primitive, SkinningMatrix, Vertex,
};
pub use material::{
    AlphaMode, Color, ColorParseError, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
    DEFAULT_STROKE_WIDTH_PX, MaterialDesc, MaterialKind, TextureColorSpace, TextureTransform,
};
pub use picking::{CursorPosition, Hit, HitTarget, InteractionContext, InteractionStyle, Viewport};
#[cfg(not(target_arch = "wasm32"))]
pub use platform::NativeWindowHandle;
pub use platform::{PlatformSurface, SurfaceEvent, SurfaceKind, SurfaceSize};
pub use render::{OffscreenTarget, PixelReadback, Renderer, Tonemapper};
pub use scene::{
    Angle, Camera, CameraKey, ClippingPlane, ClippingPlaneKey, ClippingPlaneSet, DepthRange,
    DirectionalLight, ImportAnchor, ImportClip, ImportOptions, ImportPivot, Instance,
    InstanceCullingPolicy, InstanceId, InstanceSet, InstanceSetKey, LabelBillboard, LabelDesc,
    LabelKey, LabelRasterization, Light, LightBuilder, LightKey, MeshBuilder, MeshNode,
    ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera, PerspectiveCamera,
    PointLight, Quat, Scene, SceneImport, SceneSkinBinding, SourceCoordinateSystem, SourceUnits,
    SpotLight, Transform, Vec3,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
