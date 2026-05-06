//! Asset fetchers, caches, glTF/GLB parsing, texture decoding, and asset handles.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use slotmap::{SlotMap, new_key_type};

use crate::diagnostics::AssetError;
use crate::geometry::GeometryDesc;
use crate::material::{MaterialDesc, TextureColorSpace};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAsset {
    _private: (),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetPath(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDesc {
    path: AssetPath,
    color_space: TextureColorSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TextureCacheKey {
    path: AssetPath,
    color_space: TextureColorSpace,
}

/// Asset source and cache owner.
#[derive(Debug, Clone)]
pub struct Assets<F = ()> {
    fetcher: F,
    retain_policy: RetainPolicy,
    storage: Arc<Mutex<AssetStorage>>,
}

#[derive(Debug)]
struct AssetStorage {
    geometries: SlotMap<GeometryHandle, GeometryDesc>,
    materials: SlotMap<MaterialHandle, MaterialDesc>,
    textures: SlotMap<TextureHandle, TextureDesc>,
    texture_lookup: BTreeMap<TextureCacheKey, TextureHandle>,
}

impl Assets<()> {
    pub fn new() -> Self {
        Self::with_fetcher(())
    }
}

impl Default for Assets<()> {
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
                texture_lookup: BTreeMap::new(),
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

    fn storage(&self) -> MutexGuard<'_, AssetStorage> {
        self.storage
            .lock()
            .expect("asset storage mutex should not be poisoned")
    }
}

impl SceneAsset {
    pub const fn empty() -> Self {
        Self { _private: () }
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
