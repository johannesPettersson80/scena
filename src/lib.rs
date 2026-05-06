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
    AssetPath, Assets, EnvironmentHandle, GeometryHandle, MaterialHandle, ModelHandle,
    RetainPolicy, SceneAsset, TextureDesc, TextureHandle,
};
pub use diagnostics::{
    AssetError, Backend, BuildError, Capabilities, ChangeKind, Error, LookupError,
    NotPreparedReason, PrepareError, RenderError, RenderOutcome, RendererStats,
};
pub use geometry::{Primitive, Vertex};
pub use material::{
    AlphaMode, Color, ColorParseError, MaterialDesc, MaterialKind, TextureColorSpace,
};
#[cfg(not(target_arch = "wasm32"))]
pub use platform::NativeWindowHandle;
pub use platform::{PlatformSurface, SurfaceEvent, SurfaceKind, SurfaceSize};
pub use render::Renderer;
pub use scene::{
    Angle, Camera, CameraKey, Node, NodeKey, OrthographicCamera, PerspectiveCamera, Quat, Scene,
    Transform, Vec3,
};

/// Crate-level result type for APIs that can return any structured `scena` error.
pub type Result<T> = std::result::Result<T, Error>;
