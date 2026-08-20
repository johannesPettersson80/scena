use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use slotmap::SlotMap;

use super::*;

/// CPU-side retention behavior for asset data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainPolicy {
    Never,
    OnContextLossOnly,
    Always,
}

/// Process-unique identifier for an [`Assets`] store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetStoreId(pub(super) std::num::NonZeroU64);

/// Per-store eviction counts returned by [`Assets::release_unreferenced`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AssetEvictionStats {
    pub geometries_evicted: u32,
    pub materials_evicted: u32,
    pub textures_evicted: u32,
    pub environments_evicted: u32,
}

#[derive(Debug, Clone)]
pub(super) struct AssetStorage {
    pub(super) geometries: SlotMap<GeometryHandle, Arc<GeometryDesc>>,
    pub(super) materials: SlotMap<MaterialHandle, Arc<MaterialDesc>>,
    pub(super) material_sources: BTreeMap<MaterialHandle, AssetMaterialSource>,
    #[cfg(feature = "scene-host")]
    pub(super) photographic_material_pack_bindings:
        BTreeMap<MaterialHandle, material_library::PhotographicMaterialPackBinding>,
    pub(super) textures: SlotMap<TextureHandle, Arc<TextureDesc>>,
    pub(super) environments: SlotMap<EnvironmentHandle, Arc<EnvironmentDesc>>,
    pub(super) scene_lookup: BTreeMap<scene_cache::SceneCacheKey, SceneAsset>,
    pub(super) scene_load_telemetry: BTreeMap<scene_cache::SceneCacheKey, load::AssetLoadTelemetry>,
    pub(super) texture_lookup: BTreeMap<TextureCacheKey, TextureHandle>,
    pub(super) memory_texture_lookup: BTreeMap<TextureMemoryId, TextureHandle>,
    pub(super) photographic_surface_lookup: photographic_surface::GeneratedFinishStore,
    pub(super) texture_warnings: Vec<AssetLoadWarning>,
    pub(super) texture_cache_update_policy: TextureCacheUpdatePolicy,
    pub(super) environment_lookup: BTreeMap<AssetPath, EnvironmentHandle>,
    pub(super) user_created_geometries: BTreeSet<GeometryHandle>,
    pub(super) user_created_materials: BTreeSet<MaterialHandle>,
    pub(super) user_created_textures: BTreeSet<TextureHandle>,
    pub(super) user_created_environments: BTreeSet<EnvironmentHandle>,
}
