//! Asset fetchers, caches, glTF/GLB parsing, texture decoding, and asset handles.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use slotmap::{SlotMap, new_key_type};

use crate::diagnostics::AssetError;
use crate::geometry::GeometryDesc;
use crate::material::{MaterialDesc, TextureColorSpace};

mod fetch;
mod gltf;
#[cfg(feature = "obj")]
mod obj;
#[cfg(target_arch = "wasm32")]
pub use fetch::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use fetch::FileAssetFetcher;
pub use fetch::{AssetFetcher, DefaultAssetFetcher};
pub use gltf::{
    SceneAsset, SceneAssetAnchor, SceneAssetClip, SceneAssetLight, SceneAssetMesh, SceneAssetNode,
};

const DEFAULT_ENVIRONMENT_NAME: &str = "neutral-studio";
const DEFAULT_ENVIRONMENT_SOURCE_PATH: &str =
    "tests/assets/environment/neutral-studio.placeholder.hdr";
const DEFAULT_ENVIRONMENT_SOURCE_SHA256: &str =
    "b95916ffe38d8825bbf701fd2a6efe56983e1f7d241856426440869138e3973e";
const DEFAULT_ENVIRONMENT_LICENSE: &str = "CC0-1.0";
const DEFAULT_ENVIRONMENT_GENERATOR: &str =
    "xtask generate-default-env --input tests/assets/environment/neutral-studio.placeholder.hdr";
const DEFAULT_ENVIRONMENT_CUBEMAP_PATH: &str =
    "tests/assets/environment/generated/neutral-studio-cubemap.ktx2";
const DEFAULT_ENVIRONMENT_CUBEMAP_SHA256: &str =
    "e6c9093c4dc8efd2fa9f46be2a41d5bc97e977240dd81eccbc8cbc50e5181f24";
const DEFAULT_ENVIRONMENT_BRDF_LUT_PATH: &str =
    "tests/assets/environment/generated/brdf-lut-256.rgba16f";
const DEFAULT_ENVIRONMENT_BRDF_LUT_SHA256: &str =
    "08a2a2c32fe45ccf0d799db947a729269aaf58ec0c933c3e6e8dd99784789ef7";

new_key_type! {
    pub struct ModelHandle;
    pub struct GeometryHandle;
    pub struct MaterialHandle;
    pub struct TextureHandle;
    pub struct EnvironmentHandle;
}

/// CPU-side retention behavior for asset data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainPolicy {
    Never,
    OnContextLossOnly,
    Always,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetPath(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDesc {
    path: AssetPath,
    color_space: TextureColorSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmEnvironmentDelivery {
    Bundled,
    SeparateFetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentSourceKind {
    EquirectangularHdr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDerivative {
    path: AssetPath,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDesc {
    name: String,
    source_path: AssetPath,
    source_kind: EnvironmentSourceKind,
    source_dimensions: Option<(u32, u32)>,
    source_sha256: Option<String>,
    license: Option<String>,
    generator: Option<String>,
    cubemap_resolution: u32,
    brdf_lut_size: u32,
    wasm_delivery: WasmEnvironmentDelivery,
    derivatives: Vec<EnvironmentDerivative>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TextureCacheKey {
    path: AssetPath,
    color_space: TextureColorSpace,
}

/// Asset source and cache owner.
#[derive(Debug, Clone)]
pub struct Assets<F = DefaultAssetFetcher> {
    fetcher: F,
    retain_policy: RetainPolicy,
    storage: Arc<Mutex<AssetStorage>>,
}

#[derive(Debug)]
struct AssetStorage {
    geometries: SlotMap<GeometryHandle, GeometryDesc>,
    materials: SlotMap<MaterialHandle, MaterialDesc>,
    textures: SlotMap<TextureHandle, TextureDesc>,
    environments: SlotMap<EnvironmentHandle, EnvironmentDesc>,
    scene_lookup: BTreeMap<AssetPath, SceneAsset>,
    texture_lookup: BTreeMap<TextureCacheKey, TextureHandle>,
    environment_lookup: BTreeMap<AssetPath, EnvironmentHandle>,
}

impl Assets<DefaultAssetFetcher> {
    pub fn new() -> Self {
        Self::with_fetcher(DefaultAssetFetcher::default())
    }
}

impl Default for Assets<DefaultAssetFetcher> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> Assets<F> {
    pub fn with_fetcher(fetcher: F) -> Self {
        Self {
            fetcher,
            retain_policy: RetainPolicy::OnContextLossOnly,
            storage: Arc::new(Mutex::new(AssetStorage {
                geometries: SlotMap::with_key(),
                materials: SlotMap::with_key(),
                textures: SlotMap::with_key(),
                environments: SlotMap::with_key(),
                scene_lookup: BTreeMap::new(),
                texture_lookup: BTreeMap::new(),
                environment_lookup: BTreeMap::new(),
            })),
        }
    }

    pub fn fetcher(&self) -> &F {
        &self.fetcher
    }

    pub fn retain_policy(&self) -> RetainPolicy {
        self.retain_policy
    }

    pub fn set_retain_policy(&mut self, policy: RetainPolicy) {
        self.retain_policy = policy;
    }

    pub fn create_material(&self, material: impl Into<MaterialDesc>) -> MaterialHandle {
        self.storage().materials.insert(material.into())
    }

    pub fn create_geometry(&self, geometry: GeometryDesc) -> GeometryHandle {
        self.storage().geometries.insert(geometry)
    }

    pub fn geometry(&self, handle: GeometryHandle) -> Option<GeometryDesc> {
        self.storage().geometries.get(handle).cloned()
    }

    /// Returns a cloned material descriptor for a typed material handle.
    ///
    /// ```compile_fail
    /// # use scena::{Assets, TextureHandle};
    /// # let assets = Assets::new();
    /// # let texture: TextureHandle = todo!();
    /// let _ = assets.material(texture);
    /// ```
    pub fn material(&self, handle: MaterialHandle) -> Option<MaterialDesc> {
        self.storage().materials.get(handle).cloned()
    }

    pub async fn load_texture(
        &self,
        path: impl Into<AssetPath>,
        color_space: TextureColorSpace,
    ) -> Result<TextureHandle, AssetError> {
        let path = path.into();
        let cache_key = TextureCacheKey {
            path: path.clone(),
            color_space,
        };
        let mut storage = self.storage();
        if let Some(handle) = storage.texture_lookup.get(&cache_key) {
            return Ok(*handle);
        }

        // M1 stores the split cache entries. The contract-required warning for the same
        // path under another color space will use diagnostics once that surface exists.
        let handle = storage.textures.insert(TextureDesc { path, color_space });
        storage.texture_lookup.insert(cache_key, handle);
        Ok(handle)
    }

    pub fn texture(&self, handle: TextureHandle) -> Option<TextureDesc> {
        self.storage().textures.get(handle).cloned()
    }

    pub fn default_environment(&self) -> EnvironmentHandle {
        self.insert_environment(EnvironmentDesc::neutral_studio())
    }

    pub async fn load_environment(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<EnvironmentHandle, AssetError> {
        let path = path.into();
        let environment = if path.as_str() == DEFAULT_ENVIRONMENT_SOURCE_PATH {
            EnvironmentDesc::neutral_studio()
        } else if is_equirectangular_hdr_path(&path) {
            EnvironmentDesc::from_equirectangular_hdr_path(path)
        } else {
            return Err(AssetError::UnsupportedEnvironmentFormat {
                path: path.as_str().to_string(),
                help: "use Radiance .hdr equirectangular input for the M2 environment path",
            });
        };
        Ok(self.insert_environment(environment))
    }

    pub fn environment(&self, handle: EnvironmentHandle) -> Option<EnvironmentDesc> {
        self.storage().environments.get(handle).cloned()
    }

    fn insert_environment(&self, environment: EnvironmentDesc) -> EnvironmentHandle {
        let cache_key = environment.source_path.clone();
        let mut storage = self.storage();
        if let Some(handle) = storage.environment_lookup.get(&cache_key) {
            return *handle;
        }
        let handle = storage.environments.insert(environment);
        storage.environment_lookup.insert(cache_key, handle);
        handle
    }

    fn storage(&self) -> MutexGuard<'_, AssetStorage> {
        self.storage
            .lock()
            .expect("asset storage mutex should not be poisoned")
    }
}

impl<F: AssetFetcher> Assets<F> {
    pub async fn load_scene(&self, path: impl Into<AssetPath>) -> Result<SceneAsset, AssetError> {
        let path = path.into();
        if let Some(scene) = self.storage().scene_lookup.get(&path).cloned() {
            return Ok(scene);
        }

        let bytes = self.fetcher.fetch(&path).await?;
        let external_paths = SceneAsset::external_buffer_paths(&path, &bytes)?;
        let mut external_buffers = BTreeMap::new();
        for (index, external_path) in external_paths {
            let bytes = self.fetcher.fetch(&external_path).await?;
            external_buffers.insert(index, bytes);
        }
        let mut storage = self.storage();
        let scene = if external_buffers.is_empty() {
            SceneAsset::from_gltf_bytes(path.clone(), &bytes, &mut storage)?
        } else {
            SceneAsset::from_gltf_bytes_with_external_buffers(
                path.clone(),
                &bytes,
                &external_buffers,
                &mut storage,
            )?
        };
        storage.scene_lookup.insert(path, scene.clone());
        Ok(scene)
    }
}

impl AssetPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AssetPath {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for AssetPath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl TextureDesc {
    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub const fn color_space(&self) -> TextureColorSpace {
        self.color_space
    }
}

impl EnvironmentDesc {
    pub fn neutral_studio() -> Self {
        Self {
            name: DEFAULT_ENVIRONMENT_NAME.to_string(),
            source_path: AssetPath::from(DEFAULT_ENVIRONMENT_SOURCE_PATH),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions: None,
            source_sha256: Some(DEFAULT_ENVIRONMENT_SOURCE_SHA256.to_string()),
            license: Some(DEFAULT_ENVIRONMENT_LICENSE.to_string()),
            generator: Some(DEFAULT_ENVIRONMENT_GENERATOR.to_string()),
            cubemap_resolution: 256,
            brdf_lut_size: 256,
            wasm_delivery: WasmEnvironmentDelivery::Bundled,
            derivatives: vec![
                EnvironmentDerivative {
                    path: AssetPath::from(DEFAULT_ENVIRONMENT_CUBEMAP_PATH),
                    sha256: DEFAULT_ENVIRONMENT_CUBEMAP_SHA256.to_string(),
                },
                EnvironmentDerivative {
                    path: AssetPath::from(DEFAULT_ENVIRONMENT_BRDF_LUT_PATH),
                    sha256: DEFAULT_ENVIRONMENT_BRDF_LUT_SHA256.to_string(),
                },
            ],
        }
    }

    pub fn from_equirectangular_hdr_path(path: impl Into<AssetPath>) -> Self {
        let path = path.into();
        let source_dimensions = parse_equirectangular_hdr_dimensions(&path);
        Self {
            name: environment_name_from_path(&path).to_string(),
            source_path: path,
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions,
            source_sha256: None,
            license: None,
            generator: None,
            cubemap_resolution: 0,
            brdf_lut_size: 0,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source_path(&self) -> &AssetPath {
        &self.source_path
    }

    pub const fn source_kind(&self) -> EnvironmentSourceKind {
        self.source_kind
    }

    pub const fn source_dimensions(&self) -> Option<(u32, u32)> {
        self.source_dimensions
    }

    pub const fn is_equirectangular_hdr(&self) -> bool {
        matches!(self.source_kind, EnvironmentSourceKind::EquirectangularHdr)
    }

    pub fn source_sha256(&self) -> Option<&str> {
        self.source_sha256.as_deref()
    }

    pub fn license(&self) -> Option<&str> {
        self.license.as_deref()
    }

    pub fn generator(&self) -> Option<&str> {
        self.generator.as_deref()
    }

    pub const fn cubemap_resolution(&self) -> u32 {
        self.cubemap_resolution
    }

    pub const fn brdf_lut_size(&self) -> u32 {
        self.brdf_lut_size
    }

    pub const fn wasm_delivery(&self) -> WasmEnvironmentDelivery {
        self.wasm_delivery
    }

    pub fn derivatives(&self) -> &[EnvironmentDerivative] {
        &self.derivatives
    }
}

impl EnvironmentDerivative {
    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

fn environment_name_from_path(path: &AssetPath) -> &str {
    path.as_str()
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path.as_str())
}

fn is_equirectangular_hdr_path(path: &AssetPath) -> bool {
    path.as_str().to_ascii_lowercase().ends_with(".hdr")
}

fn parse_equirectangular_hdr_dimensions(path: &AssetPath) -> Option<(u32, u32)> {
    let stem = path
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or(path.as_str())
        .strip_suffix(".hdr")?;
    let dimensions = stem.rsplit('_').next()?;
    let (width, height) = dimensions.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}
