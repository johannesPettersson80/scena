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
mod fetch;
mod gltf;
mod load;
#[cfg(feature = "obj")]
mod obj;
mod texture;
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
pub use texture::{
    TextureDesc, TextureFilter, TextureSamplerDesc, TextureSourceFormat, TextureWrap,
};

use self::environment::{DEFAULT_ENVIRONMENT_SOURCE_PATH, is_equirectangular_hdr_path};
use self::load::{AssetLoadTelemetry, check_cancelled};
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

    /// Frees `GeometryDesc` / `MaterialDesc` / `TextureDesc` /
    /// `EnvironmentDesc` slotmap entries that no cached `SceneAsset`,
    /// material descriptor, or environment lookup still references.
    ///
    /// Long-running hot-reload sessions accumulate dead handles in the
    /// asset slotmaps because [`Assets::reload_scene`] inserts fresh
    /// entries for the replacement scene without evicting the prior
    /// scene's geometry/material/texture entries; only the latest
    /// `SceneAsset` per path is retained in `scene_lookup`. This helper
    /// computes the transitive closure of handles still reachable from
    /// `scene_lookup`, the materials those scenes' meshes reference, the
    /// textures those materials reference, and the cached environment
    /// lookup, then drops every other entry. Returns a per-store eviction
    /// count.
    ///
    /// Closes scena-gltf-animation-reviewer Phase 6 finding F4.
    pub fn release_unreferenced(&self) -> AssetEvictionStats {
        let mut storage = self.storage();
        let mut referenced_geometries: std::collections::BTreeSet<GeometryHandle> =
            std::collections::BTreeSet::new();
        let mut referenced_materials: std::collections::BTreeSet<MaterialHandle> =
            std::collections::BTreeSet::new();
        let mut referenced_textures: std::collections::BTreeSet<TextureHandle> =
            std::collections::BTreeSet::new();
        let mut referenced_environments: std::collections::BTreeSet<EnvironmentHandle> =
            std::collections::BTreeSet::new();

        for scene in storage.scene_lookup.values() {
            for node in scene.nodes() {
                for mesh in node.meshes() {
                    referenced_geometries.insert(mesh.geometry());
                    referenced_materials.insert(mesh.material());
                }
            }
        }
        for environment in storage.environment_lookup.values().copied() {
            referenced_environments.insert(environment);
        }
        for material_handle in referenced_materials.iter().copied() {
            if let Some(material) = storage.materials.get(material_handle) {
                for handle in [
                    material.base_color_texture(),
                    material.normal_texture(),
                    material.metallic_roughness_texture(),
                    material.occlusion_texture(),
                    material.emissive_texture(),
                ]
                .into_iter()
                .flatten()
                {
                    referenced_textures.insert(handle);
                }
            }
        }

        let mut stats = AssetEvictionStats::default();

        let geometry_keys: Vec<GeometryHandle> = storage.geometries.keys().collect();
        for handle in geometry_keys {
            if !referenced_geometries.contains(&handle) {
                storage.geometries.remove(handle);
                stats.geometries_evicted += 1;
            }
        }
        let material_keys: Vec<MaterialHandle> = storage.materials.keys().collect();
        for handle in material_keys {
            if !referenced_materials.contains(&handle) {
                storage.materials.remove(handle);
                stats.materials_evicted += 1;
            }
        }
        let texture_keys: Vec<TextureHandle> = storage.textures.keys().collect();
        for handle in texture_keys {
            if !referenced_textures.contains(&handle) {
                storage.textures.remove(handle);
                stats.textures_evicted += 1;
            }
        }
        // Drop texture_lookup entries that pointed at evicted textures so
        // a stable retained-reload identity does not resurrect a dead handle.
        let live_textures: std::collections::BTreeSet<TextureHandle> =
            storage.textures.keys().collect();
        storage
            .texture_lookup
            .retain(|_, handle| live_textures.contains(handle));
        let environment_keys: Vec<EnvironmentHandle> = storage.environments.keys().collect();
        for handle in environment_keys {
            if !referenced_environments.contains(&handle) {
                storage.environments.remove(handle);
                stats.environments_evicted += 1;
            }
        }
        stats
    }

    pub fn create_material(&self, material: impl Into<MaterialDesc>) -> MaterialHandle {
        self.storage().materials.insert(material.into())
    }

    #[cfg(test)]
    pub(crate) fn create_texture_for_test(
        &self,
        path: impl Into<AssetPath>,
        color_space: TextureColorSpace,
        source_format: TextureSourceFormat,
        source_bytes: Option<&[u8]>,
    ) -> Result<TextureHandle, AssetError> {
        Ok(self.storage().textures.insert(TextureDesc::new_with_bytes(
            path.into(),
            color_space,
            TextureSamplerDesc::default(),
            source_format,
            source_bytes,
        )?))
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
            return Ok(handle);
        }
        let handle = storage.textures.insert(TextureDesc::new_with_bytes(
            path,
            color_space,
            cache_key.sampler,
            source_format,
            source_bytes.as_deref(),
        )?);
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

    pub fn try_texture(&self, handle: TextureHandle) -> Result<TextureDesc, AssetError> {
        self.texture(handle)
            .ok_or(AssetError::TextureHandleNotFound { texture: handle })
    }

    pub(crate) fn sample_texture(&self, handle: TextureHandle, uv: [f32; 2]) -> Option<Color> {
        self.storage()
            .textures
            .get(handle)
            .and_then(|texture| texture.sample_nearest(uv))
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
            Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => Ok(None),
            Err(error) => Err(error),
        }
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
    )
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
        let reloaded = match self
            .parse_scene_uncached(path.clone(), None, &mut progress_events, &mut progress)
            .await
        {
            Ok((scene, _telemetry)) => scene,
            Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => {
                let Some(bytes) = scene.retained_source_bytes() else {
                    return Err(AssetError::ReloadRequiresRetain {
                        path: path.as_str().to_string(),
                        help: "retained source bytes are unavailable; reload needs the original source to be fetchable",
                    });
                };
                let mut storage = self.storage();
                SceneAsset::from_gltf_bytes(path.clone(), bytes, &mut storage)?
                    .with_retained_source_bytes(bytes)
            }
            Err(error) => return Err(error),
        };
        self.storage().scene_lookup.insert(path, reloaded.clone());
        Ok(reloaded)
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
        let external_image_paths = SceneAsset::external_image_paths(&path, &bytes)?;
        let mut external_buffers = BTreeMap::new();
        let mut external_images = BTreeMap::new();
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
        for external_path in external_image_paths {
            if external_images.contains_key(&external_path) {
                continue;
            }
            if validate_texture_source_format(&external_path).is_err() {
                continue;
            }
            check_cancelled(&path, control)?;
            let bytes = match self.fetcher.fetch(&external_path).await {
                Ok(bytes) => bytes,
                Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => continue,
                Err(error) => return Err(error),
            };
            telemetry.fetched_bytes = telemetry.fetched_bytes.saturating_add(bytes.len());
            external_images.insert(external_path, bytes);
        }
        check_cancelled(&path, control)?;
        let mut storage = self.storage();
        let mut scene = if external_buffers.is_empty() && external_images.is_empty() {
            SceneAsset::from_gltf_bytes(path.clone(), &bytes, &mut storage)?
        } else {
            SceneAsset::from_gltf_bytes_with_external_resources(
                path.clone(),
                &bytes,
                &external_buffers,
                &external_images,
                &mut storage,
            )?
        };
        if self.retain_policy == RetainPolicy::Always {
            scene = scene.with_retained_source_bytes(&bytes);
        }
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
