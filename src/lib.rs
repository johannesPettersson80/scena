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

#[cfg(target_arch = "wasm32")]
pub use assets::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use assets::FileAssetFetcher;
pub use assets::{
    AssetFetcher, AssetPath, Assets, DefaultAssetFetcher, EnvironmentDerivative, EnvironmentDesc,
    EnvironmentHandle, EnvironmentSourceKind, GeometryHandle, MaterialHandle, ModelHandle,
    RetainPolicy, SceneAsset, SceneAssetNode, TextureDesc, TextureHandle, WasmEnvironmentDelivery,
};
pub use diagnostics::{
    AlphaPipelineStatus, AssetError, Backend, BuildError, Capabilities, CapabilityStatus,
    ChangeKind, DevicePoll, Diagnostic, DiagnosticCode, DiagnosticSeverity, Error, ImportError,
    InstantiateError, LookupError, NotPreparedReason, OutputStageStatus, PrepareError, RenderError,
    RenderOutcome, RendererStats,
};
pub use geometry::{
    Aabb, GeometryDesc, GeometryError, GeometryTopology, GeometryVertex, Primitive, Vertex,
};
pub use material::{
    AlphaMode, Color, ColorParseError, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
    DEFAULT_STROKE_WIDTH_PX, MaterialDesc, MaterialKind, TextureColorSpace,
};
pub use picking::{CursorPosition, Hit, HitTarget, Viewport};
#[cfg(not(target_arch = "wasm32"))]
pub use platform::NativeWindowHandle;
pub use platform::{PlatformSurface, SurfaceEvent, SurfaceKind, SurfaceSize};
pub use render::{Renderer, Tonemapper};
pub use scene::{
    Angle, Camera, CameraKey, ClippingPlane, ClippingPlaneKey, ClippingPlaneSet, DepthRange,
    DirectionalLight, ImportOptions, Light, LightBuilder, LightKey, MeshBuilder, MeshNode,
    ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera, PerspectiveCamera,
    PointLight, Quat, Scene, SceneImport, SourceCoordinateSystem, SourceUnits, SpotLight,
    Transform, Vec3,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
