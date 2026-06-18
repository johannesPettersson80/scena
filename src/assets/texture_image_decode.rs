use std::io::Cursor;

use crate::assets::AssetPath;
use crate::diagnostics::AssetError;

use super::TexturePixels;
use super::texture_limits::{IMAGE_DECODE_MAX_ALLOC_BYTES, IMAGE_DECODE_MAX_DIMENSION};

pub(super) fn decode_png_rgba8(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<TexturePixels, AssetError> {
    decode_via_image_crate(path, bytes, image::ImageFormat::Png)
}

pub(super) fn decode_jpeg_rgba8(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<TexturePixels, AssetError> {
    decode_via_image_crate(path, bytes, image::ImageFormat::Jpeg)
}

/// Delegate PNG/JPEG decode to the `image` crate with decode limits so
/// malformed headers cannot be the first allocation guard.
fn decode_via_image_crate(
    path: &AssetPath,
    bytes: &[u8],
    format: image::ImageFormat,
) -> Result<TexturePixels, AssetError> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(IMAGE_DECODE_MAX_DIMENSION);
    limits.max_image_height = Some(IMAGE_DECODE_MAX_DIMENSION);
    limits.max_alloc = Some(IMAGE_DECODE_MAX_ALLOC_BYTES);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("invalid texture payload: {error}"),
    })?;
    let rgba = image.into_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    Ok(TexturePixels::single_level(width, height, rgba.into_raw()))
}
