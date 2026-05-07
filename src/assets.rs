//! Asset fetchers, caches, glTF/GLB parsing, texture decoding, and asset handles.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use slotmap::{SlotMap, new_key_type};

use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, StaticBatchReport};
use crate::material::{MaterialDesc, TextureColorSpace};
use crate::scene::Transform;

mod environment;
mod fetch;
mod gltf;
mod load;
#[cfg(feature = "obj")]
mod obj;
pub use environment::{
    EnvironmentDerivative, EnvironmentDesc, EnvironmentSourceKind, WasmEnvironmentDelivery,
};
#[cfg(target_arch = "wasm32")]
pub use fetch::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use fetch::FileAssetFetcher;
pub use fetch::{AssetFetcher, DefaultAssetFetcher};
pub use gltf::{
    GltfDecoderPolicy, GltfExtensionDiagnostic, GltfExtensionStatus, SceneAsset, SceneAssetAnchor,
    SceneAssetClip, SceneAssetLight, SceneAssetMesh, SceneAssetNode,
};
pub use load::{AssetLoadControl, AssetLoadProgress, AssetLoadReport};

use self::environment::{DEFAULT_ENVIRONMENT_SOURCE_PATH, is_equirectangular_hdr_path};
use self::load::{AssetLoadTelemetry, check_cancelled};

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
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureSourceFormat {
    Png,
    Jpeg,
    Webp,
    Ktx2Basisu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureFilter {
    Nearest,
    Linear,
    NearestMipmapNearest,
    LinearMipmapNearest,
    NearestMipmapLinear,
    LinearMipmapLinear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureWrap {
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureSamplerDesc {
    mag_filter: Option<TextureFilter>,
    min_filter: Option<TextureFilter>,
    wrap_s: TextureWrap,
    wrap_t: TextureWrap,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TextureCacheKey {
    path: AssetPath,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
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

    pub fn create_static_batch(
        &self,
        source: &GeometryDesc,
        transforms: impl IntoIterator<Item = Transform>,
    ) -> GeometryHandle {
        self.create_geometry(GeometryDesc::static_batch(source, transforms))
    }

    pub fn create_static_batch_with_report(
        &self,
        source: &GeometryDesc,
        transforms: impl IntoIterator<Item = Transform>,
    ) -> (GeometryHandle, StaticBatchReport) {
        let transforms = transforms.into_iter().collect::<Vec<_>>();
        let report = GeometryDesc::static_batch_report(source, transforms.len());
        let handle = self.create_geometry(GeometryDesc::static_batch(source, transforms));
        (handle, report)
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

    /// Returns a cloned geometry descriptor for a typed geometry handle.
    ///
    /// ```compile_fail
    /// # use scena::{Assets, MaterialHandle};
    /// # let assets = Assets::new();
    /// # let material: MaterialHandle = todo!();
    /// let _ = assets.geometry(material);
    /// ```
    pub fn geometry(&self, handle: GeometryHandle) -> Option<GeometryDesc> {
        self.storage().geometries.get(handle).cloned()
    }

    pub async fn load_texture(
        &self,
        path: impl Into<AssetPath>,
        color_space: TextureColorSpace,
    ) -> Result<TextureHandle, AssetError> {
        let path = path.into();
        let source_format = validate_texture_source_format(&path)?;
        let cache_key = TextureCacheKey {
            path: path.clone(),
            color_space,
            sampler: TextureSamplerDesc::default(),
            source_format,
        };
        let mut storage = self.storage();
        if let Some(handle) = storage.texture_lookup.get(&cache_key) {
            return Ok(*handle);
        }

        // M1 stores the split cache entries. The contract-required warning for the same
        // path under another color space will use diagnostics once that surface exists.
        let handle = storage.textures.insert(TextureDesc {
            path,
            color_space,
            sampler: cache_key.sampler,
            source_format,
        });
        storage.texture_lookup.insert(cache_key, handle);
        Ok(handle)
    }

    /// Returns a cloned texture descriptor for a typed texture handle.
    ///
    /// ```compile_fail
    /// # use scena::{Assets, MaterialHandle};
    /// # let assets = Assets::new();
    /// # let material: MaterialHandle = todo!();
    /// let _ = assets.texture(material);
    /// ```
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
        let cache_key = environment.source_path().clone();
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
        Ok(self.load_scene_with_report(path).await?.into_asset())
    }

    pub async fn load_scene_with_report(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        self.load_scene_report_inner(path.into(), None, None).await
    }

    pub async fn load_scene_with_progress<P>(
        &self,
        path: impl Into<AssetPath>,
        mut progress: P,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError>
    where
        P: FnMut(AssetLoadProgress),
    {
        self.load_scene_report_inner(path.into(), None, Some(&mut progress))
            .await
    }

    pub async fn load_scene_controlled(
        &self,
        path: impl Into<AssetPath>,
        control: &AssetLoadControl,
    ) -> Result<SceneAsset, AssetError> {
        Ok(self
            .load_scene_report_inner(path.into(), Some(control), None)
            .await?
            .into_asset())
    }

    pub async fn reload_scene(&self, scene: &SceneAsset) -> Result<SceneAsset, AssetError> {
        let path = scene.path().clone();
        if self.retain_policy != RetainPolicy::Always {
            return Err(AssetError::ReloadRequiresRetain {
                path: path.as_str().to_string(),
                help: "set RetainPolicy::Always before reloading scene assets",
            });
        }

        let mut progress_events = Vec::new();
        let mut progress = None;
        let (scene, _telemetry) = self
            .parse_scene_uncached(path.clone(), None, &mut progress_events, &mut progress)
            .await?;
        self.storage().scene_lookup.insert(path, scene.clone());
        Ok(scene)
    }

    async fn load_scene_report_inner(
        &self,
        path: AssetPath,
        control: Option<&AssetLoadControl>,
        mut progress: Option<&mut dyn FnMut(AssetLoadProgress)>,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        let mut progress_events = Vec::new();
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::LoadStarted { path: path.clone() },
        );
        check_cancelled(&path, control)?;
        if let Some(scene) = self.storage().scene_lookup.get(&path).cloned() {
            load::emit_progress(
                &mut progress_events,
                &mut progress,
                AssetLoadProgress::CacheHit { path: path.clone() },
            );
            return Ok(AssetLoadReport {
                asset: scene,
                path,
                cache_hit: true,
                fetched_bytes: 0,
                external_buffers: 0,
                progress_events,
            });
        }

        let (scene, telemetry) = self
            .parse_scene_uncached(path.clone(), control, &mut progress_events, &mut progress)
            .await?;
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Parsed {
                path: path.clone(),
                nodes: scene.node_count(),
                meshes: scene.mesh_count(),
            },
        );
        check_cancelled(&path, control)?;
        self.storage()
            .scene_lookup
            .insert(path.clone(), scene.clone());
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Cached { path: path.clone() },
        );
        Ok(AssetLoadReport {
            asset: scene,
            path,
            cache_hit: false,
            fetched_bytes: telemetry.fetched_bytes,
            external_buffers: telemetry.external_buffers,
            progress_events,
        })
    }

    async fn parse_scene_uncached(
        &self,
        path: AssetPath,
        control: Option<&AssetLoadControl>,
        progress_events: &mut Vec<AssetLoadProgress>,
        progress: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
    ) -> Result<(SceneAsset, AssetLoadTelemetry), AssetError> {
        check_cancelled(&path, control)?;
        let bytes = self.fetcher.fetch(&path).await?;
        load::emit_progress(
            progress_events,
            progress,
            AssetLoadProgress::AssetFetched {
                path: path.clone(),
                bytes: bytes.len(),
            },
        );
        check_cancelled(&path, control)?;
        let external_paths = SceneAsset::external_buffer_paths(&path, &bytes)?;
        let mut external_buffers = BTreeMap::new();
        let mut telemetry = AssetLoadTelemetry {
            fetched_bytes: bytes.len(),
            external_buffers: 0,
        };
        for (index, external_path) in external_paths {
            check_cancelled(&path, control)?;
            let bytes = self.fetcher.fetch(&external_path).await?;
            load::emit_progress(
                progress_events,
                progress,
                AssetLoadProgress::ExternalBufferFetched {
                    path: external_path.clone(),
                    index,
                    bytes: bytes.len(),
                },
            );
            telemetry.fetched_bytes = telemetry.fetched_bytes.saturating_add(bytes.len());
            telemetry.external_buffers = telemetry.external_buffers.saturating_add(1);
            external_buffers.insert(index, bytes);
        }
        check_cancelled(&path, control)?;
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
        Ok((scene, telemetry))
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

    pub const fn sampler(&self) -> TextureSamplerDesc {
        self.sampler
    }

    pub const fn source_format(&self) -> TextureSourceFormat {
        self.source_format
    }
}

impl TextureSamplerDesc {
    pub const fn new(
        mag_filter: Option<TextureFilter>,
        min_filter: Option<TextureFilter>,
        wrap_s: TextureWrap,
        wrap_t: TextureWrap,
    ) -> Self {
        Self {
            mag_filter,
            min_filter,
            wrap_s,
            wrap_t,
        }
    }

    pub const fn mag_filter(self) -> Option<TextureFilter> {
        self.mag_filter
    }

    pub const fn min_filter(self) -> Option<TextureFilter> {
        self.min_filter
    }

    pub const fn wrap_s(self) -> TextureWrap {
        self.wrap_s
    }

    pub const fn wrap_t(self) -> TextureWrap {
        self.wrap_t
    }
}

impl Default for TextureSamplerDesc {
    fn default() -> Self {
        Self {
            mag_filter: None,
            min_filter: None,
            wrap_s: TextureWrap::Repeat,
            wrap_t: TextureWrap::Repeat,
        }
    }
}

pub(crate) fn validate_texture_source_format(
    path: &AssetPath,
) -> Result<TextureSourceFormat, AssetError> {
    let lower = path.as_str().to_ascii_lowercase();
    if lower.ends_with(".png") || lower.starts_with("data:image/png") {
        return Ok(TextureSourceFormat::Png);
    }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.starts_with("data:image/jpeg") {
        return Ok(TextureSourceFormat::Jpeg);
    }
    if lower.ends_with(".webp") || lower.starts_with("data:image/webp") {
        return Ok(TextureSourceFormat::Webp);
    }
    #[cfg(feature = "ktx2")]
    {
        if lower.ends_with(".ktx2") || lower.starts_with("data:image/ktx2") {
            return Ok(TextureSourceFormat::Ktx2Basisu);
        }
    }
    Err(AssetError::UnsupportedTextureFormat {
        path: path.as_str().to_string(),
        help: "supported texture format set is PNG, JPEG, and WebP; compressed texture decoders need an explicit feature/policy",
    })
}
