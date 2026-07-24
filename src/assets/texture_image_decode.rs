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

pub(super) fn decode_webp_rgba8(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<TexturePixels, AssetError> {
    decode_via_image_crate(path, bytes, image::ImageFormat::WebP)
}

/// Delegate maintained image codecs to the `image` crate with decode limits so
/// malformed headers cannot be the first allocation guard.
fn decode_via_image_crate(
    path: &AssetPath,
    bytes: &[u8],
    format: image::ImageFormat,
) -> Result<TexturePixels, AssetError> {
    let (width, height) = image::ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid texture payload: {error}"),
        })?;
    let required_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .unwrap_or(u64::MAX);
    if width == 0
        || height == 0
        || width > IMAGE_DECODE_MAX_DIMENSION
        || height > IMAGE_DECODE_MAX_DIMENSION
        || required_bytes > IMAGE_DECODE_MAX_ALLOC_BYTES
    {
        return Err(AssetError::TextureSizeLimit {
            path: path.as_str().to_string(),
            width,
            height,
            maximum_dimension: IMAGE_DECODE_MAX_DIMENSION,
            required_bytes,
            maximum_bytes: IMAGE_DECODE_MAX_ALLOC_BYTES,
        });
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(IMAGE_DECODE_MAX_DIMENSION);
    limits.max_image_height = Some(IMAGE_DECODE_MAX_DIMENSION);
    limits.max_alloc = Some(IMAGE_DECODE_MAX_ALLOC_BYTES);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| match error {
        image::ImageError::Limits(_) => AssetError::TextureSizeLimit {
            path: path.as_str().to_string(),
            width,
            height,
            maximum_dimension: IMAGE_DECODE_MAX_DIMENSION,
            required_bytes,
            maximum_bytes: IMAGE_DECODE_MAX_ALLOC_BYTES,
        },
        error => AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid texture payload: {error}"),
        },
    })?;
    let rgba = image.into_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    Ok(TexturePixels::single_level(width, height, rgba.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a14_oversized_native_texture_reports_dedicated_size_error() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, IMAGE_DECODE_MAX_DIMENSION + 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .expect("oversized header fixture is syntactically valid");
            writer
                .write_image_data(&vec![0; (IMAGE_DECODE_MAX_DIMENSION as usize + 1) * 4])
                .expect("oversized fixture data writes");
            writer.finish().expect("fixture finishes");
        }
        let error = decode_png_rgba8(&AssetPath::from("oversized.png"), &bytes)
            .expect_err("oversized native texture must fail closed");
        assert!(
            matches!(
                &error,
                AssetError::TextureSizeLimit {
                    width,
                    height: 1,
                    maximum_dimension: IMAGE_DECODE_MAX_DIMENSION,
                    ..
                } if *width == IMAGE_DECODE_MAX_DIMENSION + 1
            ),
            "unexpected oversized-texture error: {error:?}"
        );
    }
}
