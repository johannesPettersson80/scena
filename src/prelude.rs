//! Curated imports for everyday scena application code.
//!
//! Stable scene, asset, material, interaction, and renderer types live here.
//! Machine contracts and versioned report types remain explicit root/module
//! imports so wildcard application imports do not pull the schema catalog into
//! scope.

pub use crate::{
    Aabb, AlphaMode, AntiAliasing, AssetError, AssetFetcher, AssetPath, Assets, Backend,
    Background, BuildError, Camera, CameraKey, Color, DirectionalLight, EnvironmentHandle,
    EnvironmentPreset, FramingOptions, FramingOutcome, GeometryDesc, GeometryError, GeometryHandle,
    GridFloorOptions, Hit, ImportError, ImportOptions, LookupError, MaterialDesc, MaterialHandle,
    NodeKey, OrbitControls, OrthographicCamera, PerspectiveCamera, PointLight, PrepareError,
    Profile, Quality, Quat, RenderError, RenderOutcome, Renderer, RendererOptions, Scene,
    SceneAsset, SceneImport, SourceCoordinateSystem, SourceUnits, SpotLight, TextureHandle,
    TextureMemoryDesc, TextureMemoryId, TextureMipPolicy, TextureSamplerDesc, TextureSlot,
    Transform, Vec3,
};

#[cfg(not(target_arch = "wasm32"))]
pub use crate::FileAssetFetcher;

#[cfg(target_arch = "wasm32")]
pub use crate::BrowserAssetFetcher;
