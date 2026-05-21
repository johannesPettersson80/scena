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

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const BROWSER_TEXTURE_MAX_DIMENSION_2D: u32 = 2048;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn browser_texture_resize_dimensions(
    width: u32,
    height: u32,
    max_dimension: u32,
) -> Option<(u32, u32)> {
    if width == 0 || height == 0 || max_dimension == 0 {
        return None;
    }
    let source_max = width.max(height);
    if source_max <= max_dimension {
        return None;
    }
    let source_max = u64::from(source_max);
    let max_dimension = u64::from(max_dimension);
    let resize_axis = |value: u32| -> u32 {
        ((u64::from(value) * max_dimension).div_ceil(source_max)).clamp(1, max_dimension) as u32
    };
    Some((resize_axis(width), resize_axis(height)))
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
    let image = await_browser_image_bitmap(
        path,
        window
            .create_image_bitmap_with_blob(&blob)
            .map_err(|error| browser_image_bitmap_error(path, "createImageBitmap", error))?,
        "createImageBitmap",
    )
    .await?;

    if let Some((width, height)) = browser_texture_resize_dimensions(
        image.width(),
        image.height(),
        BROWSER_TEXTURE_MAX_DIMENSION_2D,
    ) {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena resized browser texture '{}' from {}x{} to {}x{} to fit the WebGL2-safe {}px texture limit",
            path.as_str(),
            image.width(),
            image.height(),
            width,
            height,
            BROWSER_TEXTURE_MAX_DIMENSION_2D
        )));
        let options = web_sys::ImageBitmapOptions::new();
        options.set_resize_width(width);
        options.set_resize_height(height);
        return await_browser_image_bitmap(
            path,
            window
                .create_image_bitmap_with_blob_and_image_bitmap_options(&blob, &options)
                .map_err(|error| {
                    browser_image_bitmap_error(path, "createImageBitmap resize", error)
                })?,
            "createImageBitmap resize",
        )
        .await;
    }

    Ok(image)
}

#[cfg(target_arch = "wasm32")]
async fn await_browser_image_bitmap(
    path: &AssetPath,
    promise: js_sys::Promise,
    operation: &str,
) -> Result<web_sys::ImageBitmap, AssetError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    JsFuture::from(promise)
        .await
        .map_err(|error| AssetError::Io {
            path: path.as_str().to_string(),
            reason: error
                .as_string()
                .unwrap_or_else(|| format!("{operation} await failed: {error:?}")),
        })?
        .dyn_into::<web_sys::ImageBitmap>()
        .map_err(|error| AssetError::Io {
            path: path.as_str().to_string(),
            reason: error
                .as_string()
                .unwrap_or_else(|| format!("{operation} returned wrong type: {error:?}")),
        })
}

#[cfg(target_arch = "wasm32")]
fn browser_image_bitmap_error(
    path: &AssetPath,
    operation: &str,
    error: wasm_bindgen::JsValue,
) -> AssetError {
    AssetError::Io {
        path: path.as_str().to_string(),
        reason: error
            .as_string()
            .unwrap_or_else(|| format!("{operation} failed: {error:?}")),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_texture_resize_dimensions_clamps_square_to_max() {
        assert_eq!(
            browser_texture_resize_dimensions(4096, 4096, BROWSER_TEXTURE_MAX_DIMENSION_2D),
            Some((2048, 2048))
        );
    }

    #[test]
    fn browser_texture_resize_dimensions_preserves_aspect_ratio() {
        assert_eq!(
            browser_texture_resize_dimensions(4096, 2048, BROWSER_TEXTURE_MAX_DIMENSION_2D),
            Some((2048, 1024))
        );
        assert_eq!(
            browser_texture_resize_dimensions(2048, 4096, BROWSER_TEXTURE_MAX_DIMENSION_2D),
            Some((1024, 2048))
        );
    }

    #[test]
    fn browser_texture_resize_dimensions_leaves_small_images_unscaled() {
        assert_eq!(
            browser_texture_resize_dimensions(1024, 1024, BROWSER_TEXTURE_MAX_DIMENSION_2D),
            None
        );
        assert_eq!(
            browser_texture_resize_dimensions(2048, 1024, BROWSER_TEXTURE_MAX_DIMENSION_2D),
            None
        );
        assert_eq!(
            browser_texture_resize_dimensions(1024, 2048, BROWSER_TEXTURE_MAX_DIMENSION_2D),
            None
        );
    }
}
