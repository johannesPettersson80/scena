use crate::assets::AssetPath;
use crate::diagnostics::AssetError;

use super::TextureSourceFormat;
use super::texture_ktx2::ktx2_descriptor_only_error;

pub(super) fn resolve_texture_source_bytes(
    path: &AssetPath,
    source_format: TextureSourceFormat,
    source_bytes: Option<&[u8]>,
) -> Result<Option<Vec<u8>>, AssetError> {
    if let Some(bytes) = source_bytes {
        return Ok(Some(bytes.to_vec()));
    }
    if path.as_str().starts_with("data:") {
        return decode_data_uri(path).map(Some);
    }
    match source_format {
        TextureSourceFormat::Ktx2Basisu => Err(ktx2_descriptor_only_error(path)),
        TextureSourceFormat::Png | TextureSourceFormat::Jpeg | TextureSourceFormat::Webp => {
            Ok(None)
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn browser_native_decode_format(source_format: TextureSourceFormat) -> bool {
    matches!(
        source_format,
        TextureSourceFormat::Png | TextureSourceFormat::Jpeg | TextureSourceFormat::Webp
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn decode_browser_image_bitmap(
    path: &AssetPath,
    bytes: std::sync::Arc<[u8]>,
) -> Result<web_sys::ImageBitmap, AssetError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| AssetError::Io {
        path: path.as_str().to_string(),
        reason: "browser image decode requires a Window".to_string(),
    })?;
    let array = js_sys::Uint8Array::from(bytes.as_ref());
    let parts = js_sys::Array::of1(&array.into());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts.into()).map_err(|error| {
        AssetError::Io {
            path: path.as_str().to_string(),
            reason: error
                .as_string()
                .unwrap_or_else(|| format!("Blob construction failed: {error:?}")),
        }
    })?;
    let promise = window
        .create_image_bitmap_with_blob(&blob)
        .map_err(|error| AssetError::Io {
            path: path.as_str().to_string(),
            reason: error
                .as_string()
                .unwrap_or_else(|| format!("createImageBitmap failed: {error:?}")),
        })?;
    JsFuture::from(promise)
        .await
        .map_err(|error| AssetError::Io {
            path: path.as_str().to_string(),
            reason: error
                .as_string()
                .unwrap_or_else(|| format!("createImageBitmap await failed: {error:?}")),
        })?
        .dyn_into::<web_sys::ImageBitmap>()
        .map_err(|error| AssetError::Io {
            path: path.as_str().to_string(),
            reason: error
                .as_string()
                .unwrap_or_else(|| format!("createImageBitmap returned wrong type: {error:?}")),
        })
}

fn decode_data_uri(path: &AssetPath) -> Result<Vec<u8>, AssetError> {
    let Some((_, encoded)) = path.as_str().split_once(";base64,") else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "only base64 texture data URIs are supported for embedded texture decoding"
                .to_string(),
        });
    };
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid embedded texture base64: {error}"),
        })
}
