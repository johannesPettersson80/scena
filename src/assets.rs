//! Asset fetchers, caches, glTF/GLB parsing, texture decoding, and asset handles.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use base64::Engine;
use slotmap::{SlotMap, new_key_type};

use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, StaticBatchReport};
use crate::material::{Color, MaterialDesc, TextureColorSpace};
use crate::scene::Transform;

mod environment;
mod environment_projection;
mod fetch;
mod gc;
mod gltf;
mod load;
#[cfg(feature = "obj")]
mod obj;
mod scene_loading;
mod texture;
pub use environment::{
    DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION, ENVIRONMENT_CUBEMAP_FACE_NORMALS,
    EnvironmentCubemapFaces, EnvironmentDerivative, EnvironmentDesc, EnvironmentSourceKind,
    WasmEnvironmentDelivery,
};
#[cfg(target_arch = "wasm32")]
pub use fetch::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use fetch::FileAssetFetcher;
pub use fetch::{AssetFetcher, DefaultAssetFetcher};
pub use gltf::{
    GltfDecoderPolicy, GltfExtensionDiagnostic, GltfExtensionStatus, MaterialVariantBinding,
    SceneAsset, SceneAssetAnchor, SceneAssetClip, SceneAssetLight, SceneAssetMesh, SceneAssetNode,
};
pub use load::{
    AssetLoadControl, AssetLoadOptions, AssetLoadProgress, AssetLoadReport, AssetLoadWarning,
};
pub use texture::{
    TextureDesc, TextureFilter, TextureSamplerDesc, TextureSourceFormat, TextureWrap,
};

use self::environment::{DEFAULT_ENVIRONMENT_SOURCE_PATH, is_equirectangular_hdr_path};
use self::texture::{TextureCacheKey, validate_texture_source_format};

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

/// Process-unique identifier for an [`Assets`] store. Each [`Assets::new`] /
/// [`Assets::with_fetcher`] call mints a fresh `AssetStoreId` from a
/// monotonically-increasing counter. The id stays stable for the lifetime of
/// the [`Assets`] instance and lets beginners distinguish "wrong Assets
/// store" from "stale handle in the same store" without parsing the
/// cargo-doc help text. Closes scena-api-ergonomics-reviewer Phase 6
/// finding F4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetStoreId(std::num::NonZeroU64);

impl AssetStoreId {
    fn next() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let raw = COUNTER.fetch_add(1, Ordering::Relaxed);
        let value = std::num::NonZeroU64::new(raw)
            .expect("AssetStoreId counter never returns zero before saturation");
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for AssetStoreId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assets#{}", self.0.get())
    }
}

/// Per-store eviction counts returned by [`Assets::release_unreferenced`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AssetEvictionStats {
    pub geometries_evicted: u32,
    pub materials_evicted: u32,
    pub textures_evicted: u32,
    pub environments_evicted: u32,
}

/// Asset source and cache owner.
#[derive(Debug, Clone)]
pub struct Assets<F = DefaultAssetFetcher> {
    fetcher: F,
    retain_policy: RetainPolicy,
    storage: Arc<Mutex<AssetStorage>>,
    store_id: AssetStoreId,
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
    // Tracks descriptors minted directly by `Assets::create_<kind>` (user-created)
    // rather than by glTF parsing or environment loading. `release_unreferenced`
    // ALWAYS retains these so a procedural-scene caller cannot lose handles they
    // still hold. Closes scena-api-ergonomics-reviewer 4b0e621 finding N2.
    user_created_geometries: std::collections::BTreeSet<GeometryHandle>,
    user_created_materials: std::collections::BTreeSet<MaterialHandle>,
    user_created_textures: std::collections::BTreeSet<TextureHandle>,
    user_created_environments: std::collections::BTreeSet<EnvironmentHandle>,
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
                user_created_geometries: std::collections::BTreeSet::new(),
                user_created_materials: std::collections::BTreeSet::new(),
                user_created_textures: std::collections::BTreeSet::new(),
                user_created_environments: std::collections::BTreeSet::new(),
            })),
            store_id: AssetStoreId::next(),
        }
    }

    /// Returns the unique [`AssetStoreId`] minted at construction. Two
    /// [`Assets`] instances created independently always carry distinct ids;
    /// a [`Clone`] of an existing instance shares the storage and therefore
    /// returns the same id, so the store id reliably tracks "which Assets
    /// store owns this handle?". Combine with `Assets::contains_<kind>` to
    /// distinguish "wrong Assets store" from "stale handle in the same
    /// store" before consuming the typed `*HandleNotFound` error variants.
    /// Closes scena-api-ergonomics-reviewer F4.
    pub fn store_id(&self) -> AssetStoreId {
        self.store_id
    }

    /// Returns true when `handle` resolves to a live geometry descriptor in
    /// this [`Assets`] store, mirroring the predicate the
    /// [`AssetError::GeometryHandleNotFound`] variant guards. Useful when
    /// callers want to programmatically distinguish "wrong store" from
    /// "stale handle" without parsing the diagnostic display text.
    pub fn contains_geometry(&self, handle: GeometryHandle) -> bool {
        self.storage().geometries.contains_key(handle)
    }

    /// Returns true when `handle` resolves to a live material descriptor.
    pub fn contains_material(&self, handle: MaterialHandle) -> bool {
        self.storage().materials.contains_key(handle)
    }

    /// Returns true when `handle` resolves to a live texture descriptor.
    pub fn contains_texture(&self, handle: TextureHandle) -> bool {
        self.storage().textures.contains_key(handle)
    }

    /// Returns true when `handle` resolves to a live environment descriptor.
    pub fn contains_environment(&self, handle: EnvironmentHandle) -> bool {
        self.storage().environments.contains_key(handle)
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
        let mut storage = self.storage();
        let handle = storage.materials.insert(material.into());
        storage.user_created_materials.insert(handle);
        handle
    }

    #[cfg(test)]
    pub(crate) fn create_texture_for_test(
        &self,
        path: impl Into<AssetPath>,
        color_space: TextureColorSpace,
        source_format: TextureSourceFormat,
        source_bytes: Option<&[u8]>,
    ) -> Result<TextureHandle, AssetError> {
        let mut storage = self.storage();
        let handle = storage.textures.insert(TextureDesc::new_with_bytes(
            path.into(),
            color_space,
            TextureSamplerDesc::default(),
            source_format,
            source_bytes,
        )?);
        storage.user_created_textures.insert(handle);
        Ok(handle)
    }

    pub fn create_geometry(&self, geometry: GeometryDesc) -> GeometryHandle {
        let mut storage = self.storage();
        let handle = storage.geometries.insert(geometry);
        storage.user_created_geometries.insert(handle);
        handle
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

    pub fn try_material(&self, handle: MaterialHandle) -> Result<MaterialDesc, AssetError> {
        self.material(handle)
            .ok_or(AssetError::MaterialHandleNotFound { material: handle })
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

    pub fn try_geometry(&self, handle: GeometryHandle) -> Result<GeometryDesc, AssetError> {
        self.geometry(handle)
            .ok_or(AssetError::GeometryHandleNotFound { geometry: handle })
    }

    pub async fn load_texture(
        &self,
        path: impl Into<AssetPath>,
        color_space: TextureColorSpace,
    ) -> Result<TextureHandle, AssetError>
    where
        F: AssetFetcher,
    {
        let path = path.into();
        let source_format = validate_texture_source_format(&path)?;
        let cache_key = TextureCacheKey {
            path: path.clone(),
            color_space,
            sampler: TextureSamplerDesc::default(),
            source_format,
        };
        if let Some(handle) = self.cached_texture_if_decoded(&cache_key) {
            return Ok(handle);
        }
        let source_bytes = self
            .fetch_optional_texture_bytes(&path, source_format)
            .await?;

        let handle = {
            let mut storage = self.storage();
            if let Some(handle) = storage.texture_lookup.get(&cache_key).copied() {
                if source_bytes.is_some() {
                    storage
                        .textures
                        .get_mut(handle)
                        .ok_or_else(|| AssetError::Parse {
                            path: path.as_str().to_string(),
                            reason: "texture cache lookup pointed at a missing texture descriptor"
                                .to_string(),
                        })?
                        .decode_missing_pixels_from_bytes(source_bytes.as_deref())?;
                }
                handle
            } else {
                let handle = storage.textures.insert(TextureDesc::new_with_bytes(
                    path,
                    color_space,
                    cache_key.sampler,
                    source_format,
                    source_bytes.as_deref(),
                )?);
                storage.texture_lookup.insert(cache_key, handle);
                handle
            }
        };
        #[cfg(target_arch = "wasm32")]
        self.decode_browser_texture_image(handle).await?;
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

    pub fn try_texture(&self, handle: TextureHandle) -> Result<TextureDesc, AssetError> {
        self.texture(handle)
            .ok_or(AssetError::TextureHandleNotFound { texture: handle })
    }

    pub(crate) fn sample_texture(&self, handle: TextureHandle, uv: [f32; 2]) -> Option<Color> {
        self.storage()
            .textures
            .get(handle)
            .and_then(|texture| texture.sample_bilinear(uv))
    }

    pub fn default_environment(&self) -> EnvironmentHandle {
        self.insert_environment(EnvironmentDesc::neutral_studio())
    }

    pub async fn load_environment(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<EnvironmentHandle, AssetError>
    where
        F: AssetFetcher,
    {
        let path = path.into();
        if let Some(handle) = self.storage().environment_lookup.get(&path).copied() {
            return Ok(handle);
        }
        let environment = if path.as_str() == DEFAULT_ENVIRONMENT_SOURCE_PATH {
            EnvironmentDesc::neutral_studio()
        } else if is_equirectangular_hdr_path(&path) {
            if let Some(source_bytes) = embedded_environment_bytes(&path)? {
                EnvironmentDesc::from_equirectangular_hdr_bytes(path.clone(), &source_bytes)?
            } else {
                match self.fetcher.fetch(&path).await {
                    Ok(source_bytes) => EnvironmentDesc::from_equirectangular_hdr_bytes(
                        path.clone(),
                        &source_bytes,
                    )?,
                    Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => {
                        EnvironmentDesc::from_equirectangular_hdr_path(path.clone())
                    }
                    Err(error) => return Err(error),
                }
            }
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

    pub fn try_environment(
        &self,
        handle: EnvironmentHandle,
    ) -> Result<EnvironmentDesc, AssetError> {
        self.environment(handle)
            .ok_or(AssetError::EnvironmentHandleNotFound {
                environment: handle,
            })
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

    fn cached_texture_if_decoded(&self, cache_key: &TextureCacheKey) -> Option<TextureHandle> {
        let storage = self.storage();
        let handle = *storage.texture_lookup.get(cache_key)?;
        let texture = storage.textures.get(handle)?;
        (!texture_format_has_cpu_decoder(cache_key.source_format) || texture.has_decoded_pixels())
            .then_some(handle)
    }

    async fn fetch_optional_texture_bytes(
        &self,
        path: &AssetPath,
        source_format: TextureSourceFormat,
    ) -> Result<Option<Vec<u8>>, AssetError>
    where
        F: AssetFetcher,
    {
        if !texture_format_has_cpu_decoder(source_format) || path.as_str().starts_with("data:") {
            return Ok(None);
        }
        match self.fetcher.fetch(path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(AssetError::NotFound { .. }) => {
                warn_optional_texture_fetch_failed(path, "not found");
                Ok(None)
            }
            Err(AssetError::Io { reason, .. }) => {
                warn_optional_texture_fetch_failed(path, &reason);
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn decode_browser_texture_image(&self, handle: TextureHandle) -> Result<(), AssetError> {
        let Some((path, bytes)) = ({
            let storage = self.storage();
            storage.textures.get(handle).and_then(|texture| {
                texture
                    .browser_decode_source()
                    .map(|bytes| (texture.path().clone(), bytes))
            })
        }) else {
            return Ok(());
        };

        let image = self::texture::decode_browser_image_bitmap(&path, bytes).await?;
        if let Some(texture) = self.storage().textures.get_mut(handle) {
            texture.set_browser_image(image);
        }
        Ok(())
    }
}

fn warn_optional_texture_fetch_failed(path: &AssetPath, reason: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena asset warning: optional texture fetch failed for '{}': {}",
            path.as_str(),
            reason
        )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (path, reason);
    }
}

fn embedded_environment_bytes(path: &AssetPath) -> Result<Option<Vec<u8>>, AssetError> {
    if !path.as_str().starts_with("data:") {
        return Ok(None);
    }
    let Some((_, encoded)) = path.as_str().split_once(";base64,") else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason:
                "only base64 Radiance HDR data URIs are supported for embedded environment decoding"
                    .to_string(),
        });
    };
    let encoded = encoded
        .split_once('#')
        .map_or(encoded, |(payload, _fragment)| payload);
    let encoded = encoded
        .split_once('?')
        .map_or(encoded, |(payload, _query)| payload);
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map(Some)
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid embedded environment base64: {error}"),
        })
}

const fn texture_format_has_cpu_decoder(source_format: TextureSourceFormat) -> bool {
    matches!(
        source_format,
        TextureSourceFormat::Png | TextureSourceFormat::Jpeg
    ) || (matches!(source_format, TextureSourceFormat::Ktx2Basisu) && cfg!(feature = "ktx2"))
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
