use std::sync::Arc;

use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

use super::{AssetFetcher, AssetPath, Assets, TextureHandle, TextureMemoryDesc, TextureSlot};

impl<F> Assets<F> {
    /// Creates or reuses an immutable application-generated texture.
    ///
    /// The explicit [`super::TextureMemoryId`] is the cache identity: reusing
    /// it with byte-identical pixels and options returns the existing handle,
    /// while reusing it with changed content fails closed.
    pub fn create_texture(
        &self,
        descriptor: TextureMemoryDesc,
    ) -> Result<TextureHandle, AssetError> {
        let identity = descriptor.identity().clone();
        let descriptor = descriptor.into_texture_desc()?;
        let mut storage = self.storage();
        if let Some(handle) = storage.memory_texture_lookup.get(&identity).copied() {
            let Some(existing) = storage.textures.get(handle) else {
                return Err(AssetError::TextureIdentityCollision {
                    identity: identity.as_str().to_string(),
                });
            };
            if existing.as_ref() == &descriptor {
                return Ok(handle);
            }
            return Err(AssetError::TextureIdentityCollision {
                identity: identity.as_str().to_string(),
            });
        }
        let handle = storage.textures.insert(Arc::new(descriptor));
        storage.memory_texture_lookup.insert(identity, handle);
        storage.user_created_textures.insert(handle);
        Ok(handle)
    }

    /// Creates an application-generated texture after validating the material
    /// slot's required sRGB/linear interpretation.
    pub fn create_texture_for_slot(
        &self,
        descriptor: TextureMemoryDesc,
        slot: TextureSlot,
    ) -> Result<TextureHandle, AssetError> {
        let expected = slot.color_space();
        let actual = descriptor.color_space();
        if actual != expected {
            return Err(AssetError::TextureColorSpaceMismatch {
                identity: descriptor.identity().as_str().to_string(),
                slot: slot.as_str().to_string(),
                expected: texture_color_space_name(expected).to_string(),
                actual: texture_color_space_name(actual).to_string(),
            });
        }
        self.create_texture(descriptor)
    }

    /// Loads a path-backed texture using the color-space contract of a typed
    /// material slot.
    pub async fn load_texture_for_slot(
        &self,
        path: impl Into<AssetPath>,
        slot: TextureSlot,
    ) -> Result<TextureHandle, AssetError>
    where
        F: AssetFetcher,
    {
        self.load_texture(path, slot.color_space()).await
    }
}

const fn texture_color_space_name(color_space: TextureColorSpace) -> &'static str {
    match color_space {
        TextureColorSpace::Srgb => "srgb",
        TextureColorSpace::Linear => "linear",
    }
}
