//! Asset fetchers, caches, glTF/GLB parsing, texture decoding, and asset handles.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use slotmap::{SlotMap, new_key_type};

use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, StaticBatchReport};
use crate::material::{Color, MaterialDesc, TextureColorSpace};
use crate::scene::Transform;

mod asset_path;
mod builtin;
mod catalog;
mod conversion;
mod doctor;
mod environment;
mod environment_hdr;
mod environment_loading;
mod environment_preset;
mod environment_projection;
#[doc(hidden)]
pub mod environment_sidecar;
mod external_resources;
mod fetch;
mod gc;
mod gltf;
#[cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]
mod hot_reload;
#[cfg(feature = "khronos-samples")]
mod khronos;
mod load;
mod material_imperfection;
#[cfg(test)]
mod material_imperfection_tests;
mod material_library;
mod material_presets;
mod material_source;
mod memory_textures;
#[cfg(feature = "obj")]
mod obj;
mod photographic_surface;
#[cfg(test)]
mod photographic_surface_tests;
mod provenance;
mod recipe_validation;
mod scene_cache;
mod scene_loading;
mod store_id;
mod texture;
mod texture_fetch;
pub use asset_path::AssetPath;
pub(crate) use builtin::{bundled_scene_bytes, is_bundled_scene_uri};
pub use catalog::{
    ASSET_CATALOG_SCHEMA_V1, ASSET_READINESS_REPORT_SCHEMA_V1, AssetCatalogAssetV1,
    AssetCatalogExpectedBoundsV1, AssetCatalogFeatureRequirementV1,
    AssetCatalogMaterialRequirementsV1, AssetCatalogPreviewV1, AssetCatalogV1,
    AssetReadinessAssetReportV1, AssetReadinessFindingV1, AssetReadinessPreviewV1,
    AssetReadinessReportV1, AssetReadinessSeverityV1, AssetReadinessSummaryV1,
};
pub use conversion::{
    ASSET_CONVERSION_SCHEMA_V1, AssetConversionDiagnosticSeverityV1,
    AssetConversionDiagnosticStreamV1, AssetConversionDiagnosticV1, AssetConversionReportV1,
    AssetConversionStatusV1,
};
pub use doctor::{
    ASSET_DOCTOR_REPORT_SCHEMA_V1, AssetDoctorFindingV1, AssetDoctorReportV1,
    AssetDoctorSeverityV1, AssetDoctorSummaryV1,
};
pub use environment::{
    DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION, ENVIRONMENT_CUBEMAP_FACE_NORMALS,
    EnvironmentCubemapFaces, EnvironmentDerivative, EnvironmentDesc, EnvironmentSourceKind,
    WasmEnvironmentDelivery,
};
pub use environment_preset::{EnvironmentPreset, EnvironmentPresetMetadata};
#[doc(hidden)]
pub use environment_sidecar::{
    EnvironmentPrefilterSidecar, EnvironmentSidecarHeader, EnvironmentSidecarProfile,
    SIDECAR_FILE_SUFFIX, parse_sidecar_header,
};
#[cfg(target_arch = "wasm32")]
pub use fetch::BrowserAssetFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use fetch::FileAssetFetcher;
pub use fetch::{AssetFetcher, DefaultAssetFetcher};
pub use gltf::{
    ASSET_GEOMETRY_SUMMARY_SCHEMA_V1, GltfDecoderPolicy, GltfExtensionDiagnostic,
    GltfExtensionStatus, MaterialVariantBinding, SceneAsset, SceneAssetAnchor, SceneAssetClip,
    SceneAssetGeometrySummary, SceneAssetLight, SceneAssetMesh, SceneAssetNode, SelectedGltfScene,
};
#[cfg(all(feature = "hot-reload", not(target_arch = "wasm32")))]
pub use hot_reload::{AssetHotReloadError, AssetHotReloadWatcher};
#[cfg(feature = "khronos-samples")]
pub use khronos::{KhronosSample, KhronosSampleMetadata, KhronosSamples};
pub use load::{
    ASSET_LOAD_REPORT_SCHEMA_V1, AssetExternalResource, AssetExternalResourceKind,
    AssetExternalResourceStatus, AssetExternalResourceV1, AssetLoadControl, AssetLoadOptions,
    AssetLoadProgress, AssetLoadProgressV1, AssetLoadReport, AssetLoadReportV1, AssetLoadWarning,
    AssetLoadWarningV1, AssetMaterialFallback, AssetMaterialFallbackKind, AssetMaterialFallbackV1,
    AssetReloadError, GltfSceneSelection,
};
pub use material_imperfection::*;
pub use material_library::{
    MATERIAL_LIBRARY_CATALOG_SCHEMA_V1, MATERIAL_LIBRARY_CATALOG_SCHEMA_V2,
    PHOTOGRAPHIC_MATERIAL_ARCHIVE_MAX_BYTES, PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1,
    PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2, PhotographicMaterialArchiveVariantV2,
    PhotographicMaterialCatalogEntryV1, PhotographicMaterialCatalogEntryV2,
    PhotographicMaterialCatalogV1, PhotographicMaterialCatalogV2, PhotographicMaterialCategoryV1,
    PhotographicMaterialMapKindV1, PhotographicMaterialPackAssets,
    PhotographicMaterialPackMapRoleV1, PhotographicMaterialPackMapV1,
    PhotographicMaterialPackSourceV1, PhotographicMaterialPackV1, PhotographicMaterialPackV2,
    PhotographicMaterialResolutionV1, photographic_material_catalog_v1,
    photographic_material_catalog_v2, select_photographic_material_resolution,
};
#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
pub use material_library::{
    PhotographicMaterialPackError, compile_photographic_material_archive,
    compile_photographic_material_archive_at_resolution,
};
pub use material_presets::{
    MaterialPresetAssets, MaterialPresetProvenance, source_backed_material_preset_provenance,
};
pub use material_source::{AssetMaterialSource, AssetMaterialSourceKind};
pub use photographic_surface::*;
pub use provenance::{AssetDerivative, AssetProvenance};
pub use recipe_validation::{
    validate_scene_recipe_json_with_assets, validate_scene_recipe_json_with_assets_and_policy,
};
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub(crate) use texture::BROWSER_TEXTURE_MAX_DIMENSION_2D;
pub use texture::{
    TextureDesc, TextureFilter, TextureMemoryDesc, TextureMemoryId, TextureMipPolicy,
    TexturePixelFormat, TextureSamplerDesc, TextureSlot, TextureSourceFormat, TextureWrap,
};
use texture_fetch::{texture_format_has_cpu_decoder, warn_optional_texture_fetch_failed};

use self::fetch::TrackedAssetFetcher;
use self::texture::{TextureCacheKey, TextureCacheUpdatePolicy, validate_texture_source_format};

new_key_type! {
    pub struct ModelHandle;
    pub struct GeometryHandle;
    pub struct MaterialHandle;
    pub struct TextureHandle;
    pub struct EnvironmentHandle;
}

#[cfg(feature = "scene-host")]
mod material_summary;
#[cfg(feature = "scene-host")]
use material_summary::{EffectiveMaterialPbr, metallic_roughness_texture_means};

mod store;
use store::AssetStorage;
pub use store::{AssetEvictionStats, AssetStoreId, RetainPolicy};

/// Asset source and cache owner.
#[derive(Debug, Clone)]
pub struct Assets<F = DefaultAssetFetcher> {
    fetcher: F,
    retain_policy: RetainPolicy,
    storage: Arc<Mutex<AssetStorage>>,
    storage_lock_acquisitions: Arc<AtomicU64>,
    fetch_attempts: Arc<AtomicU64>,
    store_id: AssetStoreId,
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
                material_sources: BTreeMap::new(),
                #[cfg(feature = "scene-host")]
                photographic_material_pack_bindings: BTreeMap::new(),
                textures: SlotMap::with_key(),
                environments: SlotMap::with_key(),
                scene_lookup: BTreeMap::new(),
                scene_load_telemetry: BTreeMap::new(),
                texture_lookup: BTreeMap::new(),
                memory_texture_lookup: BTreeMap::new(),
                photographic_surface_lookup: BTreeMap::new(),
                texture_warnings: Vec::new(),
                texture_cache_update_policy: TextureCacheUpdatePolicy::Immutable,
                environment_lookup: BTreeMap::new(),
                user_created_geometries: std::collections::BTreeSet::new(),
                user_created_materials: std::collections::BTreeSet::new(),
                user_created_textures: std::collections::BTreeSet::new(),
                user_created_environments: std::collections::BTreeSet::new(),
            })),
            storage_lock_acquisitions: Arc::new(AtomicU64::new(0)),
            fetch_attempts: Arc::new(AtomicU64::new(0)),
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

    /// Returns the cumulative number of times this store's mutex was acquired.
    ///
    /// This monotonic counter is intended for deterministic profiling. Take a
    /// snapshot before and after an operation instead of resetting it; cloned
    /// [`Assets`] values share both storage and this counter.
    #[doc(hidden)]
    pub fn storage_lock_acquisitions(&self) -> u64 {
        self.storage_lock_acquisitions.load(Ordering::Relaxed)
    }

    /// Returns the cumulative number of source-byte fetch attempts made by
    /// this asset store. Failed requests and glTF external resources count;
    /// cache hits and embedded data do not.
    #[doc(hidden)]
    pub fn fetch_attempts(&self) -> u64 {
        self.fetch_attempts.load(Ordering::Relaxed)
    }

    fn tracked_fetcher(&self) -> TrackedAssetFetcher<'_, F> {
        TrackedAssetFetcher::new(&self.fetcher, &self.fetch_attempts)
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

    /// Returns structured warnings emitted while decoding path-backed
    /// textures, including browser downscaling decisions.
    pub fn texture_warnings(&self) -> Vec<AssetLoadWarning> {
        self.storage().texture_warnings.clone()
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
        let handle = storage.materials.insert(Arc::new(material.into()));
        storage
            .material_sources
            .insert(handle, AssetMaterialSource::user_created());
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
        let handle = storage
            .textures
            .insert(Arc::new(TextureDesc::new_with_bytes(
                path.into(),
                color_space,
                TextureSamplerDesc::default(),
                source_format,
                source_bytes,
            )?));
        storage.user_created_textures.insert(handle);
        Ok(handle)
    }

    pub fn create_geometry(&self, geometry: GeometryDesc) -> GeometryHandle {
        let mut storage = self.storage();
        let handle = storage.geometries.insert(Arc::new(geometry));
        storage.user_created_geometries.insert(handle);
        handle
    }

    pub fn create_environment(&self, environment: EnvironmentDesc) -> EnvironmentHandle {
        let mut storage = self.storage();
        let handle = storage.environments.insert(Arc::new(environment));
        storage.user_created_environments.insert(handle);
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
        self.material_snapshot(handle)
            .map(|snapshot| snapshot.as_ref().clone())
    }

    /// Returns an immutable shared snapshot of a material descriptor.
    pub fn material_snapshot(&self, handle: MaterialHandle) -> Option<Arc<MaterialDesc>> {
        self.storage().materials.get(handle).cloned()
    }

    pub fn material_source(&self, handle: MaterialHandle) -> Option<AssetMaterialSource> {
        self.storage().material_sources.get(&handle).cloned()
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
        self.geometry_snapshot(handle)
            .map(|snapshot| snapshot.as_ref().clone())
    }

    /// Returns an immutable shared snapshot of a geometry descriptor.
    pub fn geometry_snapshot(&self, handle: GeometryHandle) -> Option<Arc<GeometryDesc>> {
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
                    let texture =
                        storage
                            .textures
                            .get_mut(handle)
                            .ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason:
                                    "texture cache lookup pointed at a missing texture descriptor"
                                        .to_string(),
                            })?;
                    Arc::make_mut(texture)
                        .decode_missing_pixels_from_bytes(source_bytes.as_deref())?;
                }
                handle
            } else {
                let handle = storage
                    .textures
                    .insert(Arc::new(TextureDesc::new_with_bytes(
                        path,
                        color_space,
                        cache_key.sampler,
                        source_format,
                        source_bytes.as_deref(),
                    )?));
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
        self.texture_snapshot(handle)
            .map(|snapshot| snapshot.as_ref().clone())
    }

    /// Returns an immutable shared snapshot of a texture descriptor.
    pub fn texture_snapshot(&self, handle: TextureHandle) -> Option<Arc<TextureDesc>> {
        self.storage().textures.get(handle).cloned()
    }

    pub(crate) fn texture_snapshots(
        &self,
        handles: impl IntoIterator<Item = TextureHandle>,
    ) -> BTreeMap<TextureHandle, Arc<TextureDesc>> {
        let storage = self.storage();
        handles
            .into_iter()
            .filter_map(|handle| {
                storage
                    .textures
                    .get(handle)
                    .cloned()
                    .map(|texture| (handle, texture))
            })
            .collect()
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

    #[cfg(feature = "scene-host")]
    pub(crate) fn effective_material_pbr(&self, material: &MaterialDesc) -> EffectiveMaterialPbr {
        let sampled = material
            .metallic_roughness_texture()
            .and_then(|handle| self.texture_snapshot(handle))
            .and_then(|texture| metallic_roughness_texture_means(&texture));
        let (roughness_texture_mean, metallic_texture_mean) = sampled.unwrap_or((1.0, 1.0));
        EffectiveMaterialPbr {
            metallic_mean: (material.metallic_factor() * metallic_texture_mean).clamp(0.0, 1.0),
            roughness_mean: (material.roughness_factor() * roughness_texture_mean).clamp(0.0, 1.0),
        }
    }

    pub fn default_environment(&self) -> EnvironmentHandle {
        self.insert_environment(EnvironmentDesc::neutral_studio())
    }

    /// The bundled studio HDRI, decoded from bytes compiled into the binary.
    ///
    /// [`Self::default_environment`] returns a preview fixture whose own source
    /// declares `not HDR input and not IBL proof`: six constant radiance values,
    /// one per cube face. A metal surface reflecting that has no structure to
    /// reflect, which is why derived product renders looked like clay. This is a
    /// real 128x64 equirectangular capture at 27 KiB, so it needs no filesystem,
    /// no fetcher, and no async, and it works on wasm.
    ///
    /// The cubemap is deliberately small: the source has 8,192 pixels, so the
    /// 256-face default would prefilter far more texels than the capture can
    /// justify.
    pub fn bundled_studio_environment(&self) -> Result<EnvironmentHandle, AssetError> {
        let desc = EnvironmentDesc::from_equirectangular_hdr_bytes(
            crate::assets::environment_preset::BUNDLED_STUDIO_URI,
            crate::assets::environment_preset::BUNDLED_STUDIO_BYTES,
        )?
        .with_cubemap_resolution(64);
        Ok(self.insert_environment(desc))
    }

    fn storage(&self) -> MutexGuard<'_, AssetStorage> {
        self.storage_lock_acquisitions
            .fetch_add(1, Ordering::Relaxed);
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
        match self.tracked_fetcher().fetch(path).await {
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

        let (image, warning) = self::texture::decode_browser_image_bitmap(&path, bytes).await?;
        if let Some(texture) = self.storage().textures.get_mut(handle) {
            Arc::make_mut(texture).set_browser_image(image);
        }
        if let Some(warning) = warning {
            self.storage().texture_warnings.push(warning);
        }
        Ok(())
    }
}

#[cfg(test)]
mod snapshot_tests;
