use super::{AssetPath, TextureSourceFormat};

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
