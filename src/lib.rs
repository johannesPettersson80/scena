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

pub use assets::{
    AssetPath, Assets, EnvironmentDerivative, EnvironmentDesc, EnvironmentHandle,
    EnvironmentSourceKind, GeometryHandle, MaterialHandle, ModelHandle, RetainPolicy, SceneAsset,
    TextureDesc, TextureHandle, WasmEnvironmentDelivery,
};
pub use diagnostics::{
    AlphaPipelineStatus, AssetError, Backend, BuildError, Capabilities, ChangeKind, DevicePoll,
    Diagnostic, DiagnosticCode, DiagnosticSeverity, Error, LookupError, NotPreparedReason,
    OutputStageStatus, PrepareError, RenderError, RenderOutcome, RendererStats,
};
pub use geometry::{
    Aabb, GeometryDesc, GeometryError, GeometryTopology, GeometryVertex, Primitive, Vertex,
};
pub use material::{
    AlphaMode, Color, ColorParseError, DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
    DEFAULT_STROKE_WIDTH_PX, MaterialDesc, MaterialKind, TextureColorSpace,
};
#[cfg(not(target_arch = "wasm32"))]
pub use platform::NativeWindowHandle;
pub use platform::{PlatformSurface, SurfaceEvent, SurfaceKind, SurfaceSize};
pub use render::{Renderer, Tonemapper};
pub use scene::{
    Angle, Camera, CameraKey, DepthRange, DirectionalLight, Light, LightBuilder, LightKey,
    MeshBuilder, MeshNode, ModelBuilder, ModelNode, Node, NodeKey, NodeKind, OrthographicCamera,
    PerspectiveCamera, PointLight, Quat, Scene, SpotLight, Transform, Vec3,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
