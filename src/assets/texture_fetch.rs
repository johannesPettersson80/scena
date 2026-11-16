use super::{AssetPath, Assets, TextureCacheKey, TextureHandle, TextureSourceFormat};

impl<F> Assets<F> {
    /// The texture [`Assets::load_texture`] can return without fetching: one
    /// that is cached and, where scena decodes its format, already decoded.
    ///
    /// The handle goes to the caller, so a hit is recorded as caller-owned
    /// and [`Assets::release_unreferenced`] keeps it.
    pub(super) fn cached_texture_if_decoded(
        &self,
        cache_key: &TextureCacheKey,
    ) -> Option<TextureHandle> {
        let mut storage = self.storage();
        let handle = *storage.texture_lookup.get(cache_key)?;
        let texture = storage.textures.get(handle)?;
        let ready = !texture_format_has_cpu_decoder(cache_key.source_format)
            || texture.has_decoded_pixels();
        ready.then(|| {
            storage.user_created_textures.insert(handle);
            handle
        })
    }
}

pub(super) fn warn_optional_texture_fetch_failed(path: &AssetPath, reason: &str) {
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

pub(super) const fn texture_format_has_cpu_decoder(source_format: TextureSourceFormat) -> bool {
    matches!(
        source_format,
        TextureSourceFormat::Png | TextureSourceFormat::Jpeg | TextureSourceFormat::Webp
    ) || (matches!(source_format, TextureSourceFormat::Ktx2Basisu) && cfg!(feature = "ktx2"))
        || matches!(
            source_format,
            TextureSourceFormat::MemoryRgba8 | TextureSourceFormat::MemoryRgba16Float
        )
}
