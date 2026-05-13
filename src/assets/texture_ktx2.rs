use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

#[cfg(feature = "ktx2")]
use super::TextureMipLevel;
use super::TexturePixels;

pub(super) fn ktx2_descriptor_only_error(path: &AssetPath) -> AssetError {
    AssetError::UnsupportedOptionalExtensionUsed {
        path: path.as_str().to_string(),
        extension: "KHR_texture_basisu".to_string(),
        help: "enable a decoder-backed ktx2 path and provide decodable KTX2/Basis bytes; \
               descriptor-only KTX2 textures are not supported"
            .to_string(),
    }
}

pub(super) fn decode_ktx2_basisu_rgba8(
    path: &AssetPath,
    bytes: &[u8],
    color_space: TextureColorSpace,
) -> Result<TexturePixels, AssetError> {
    #[cfg(feature = "ktx2")]
    {
        decode_ktx2_basisu_rgba8_with_parser(path, bytes, color_space)
    }
    #[cfg(not(feature = "ktx2"))]
    {
        let _ = bytes;
        let _ = color_space;
        Err(ktx2_descriptor_only_error(path))
    }
}

#[cfg(feature = "ktx2")]
fn decode_ktx2_basisu_rgba8_with_parser(
    path: &AssetPath,
    bytes: &[u8],
    color_space: TextureColorSpace,
) -> Result<TexturePixels, AssetError> {
    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))]
    {
        let _ = bytes;
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason:
                "KTX2/Basis transcoding requires async Basis Universal initialization on wasm; \
                     this sync texture decode path is fail-closed until the browser asset pipeline \
                     can await transcoder initialization"
                    .to_string(),
        });
    }

    let reader = ktx2::Reader::new(bytes).map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("invalid KTX2 container: {error:?}"),
    })?;
    let header = reader.header();
    if header.pixel_depth > 0 || header.face_count > 1 || header.layer_count > 1 {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "only 2D, single-layer KTX2/Basis textures can be decoded into TexturePixels"
                .to_string(),
        });
    }

    #[cfg(not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    )))]
    {
        use basisu_c_sys::TranscodeTargetFormat;
        use basisu_c_sys::extra::{
            BasisuTranscoder, ChannelType, SupportedTextureCompression, basisu_transcoder_init,
        };

        pollster::block_on(basisu_transcoder_init());
        let transcoder = BasisuTranscoder::new(
            bytes,
            SupportedTextureCompression::empty(),
            ChannelType::Rgba,
        )
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("failed to initialize KTX2/Basis transcoder: {error}"),
        })?;
        let info = transcoder.get_info();
        let encoded_color_space = if info.is_srgb {
            TextureColorSpace::Srgb
        } else {
            TextureColorSpace::Linear
        };
        if encoded_color_space != color_space {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "KTX2/Basis color-space mismatch: texture is authored as {encoded_color_space:?} but was requested as {color_space:?}"
                ),
            });
        }
        if info.faces != 1 || info.layers > 1 {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "KTX2/Basis texture is not a single 2D image: faces={}, layers={}",
                    info.faces, info.layers
                ),
            });
        }
        let image = transcoder
            .transcode(Some(TranscodeTargetFormat::RGBA32), Some(info.is_srgb))
            .map_err(|error| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("failed to transcode KTX2/Basis texture to RGBA8: {error}"),
            })?;
        if !format!("{:?}", image.format).starts_with("Rgba8Unorm") {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "KTX2/Basis transcoder returned unsupported CPU texture format {:?}",
                    image.format
                ),
            });
        }
        let width = info.width.max(1);
        let height = info.height.max(1);
        let base_level_len = checked_rgba8_len(path, width, height)?;
        if image.data.len() < base_level_len {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "KTX2/Basis transcoder returned {} byte(s), expected at least {base_level_len}",
                    image.data.len()
                ),
            });
        }
        TexturePixels::from_mip_levels(
            path,
            decoded_ktx2_rgba8_mip_levels(path, width, height, info.levels, &image.data)?,
        )
    }
}

#[cfg(feature = "ktx2")]
fn decoded_ktx2_rgba8_mip_levels(
    path: &AssetPath,
    width: u32,
    height: u32,
    level_count: u32,
    data: &[u8],
) -> Result<Vec<TextureMipLevel>, AssetError> {
    if width == 0 || height == 0 {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("KTX2/Basis texture has invalid base dimensions {width}x{height}"),
        });
    }
    if level_count == 0 {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "KTX2/Basis texture has zero mip levels".to_string(),
        });
    }
    let mut levels = Vec::with_capacity(level_count as usize);
    let mut offset = 0usize;
    for level_index in 0..level_count {
        let level_width = (width >> level_index).max(1);
        let level_height = (height >> level_index).max(1);
        let level_len = checked_rgba8_len(path, level_width, level_height)?;
        let end = offset
            .checked_add(level_len)
            .ok_or_else(|| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: "KTX2/Basis decoded mip byte offsets overflowed".to_string(),
            })?;
        let Some(level_bytes) = data.get(offset..end) else {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "KTX2/Basis transcoder returned truncated mip level {level_index}: \
                     need bytes {offset}..{end}, got {}",
                    data.len()
                ),
            });
        };
        levels.push(TextureMipLevel {
            width: level_width,
            height: level_height,
            rgba8: level_bytes.to_vec(),
        });
        offset = end;
    }
    Ok(levels)
}

#[cfg(feature = "ktx2")]
fn checked_rgba8_len(path: &AssetPath, width: u32, height: u32) -> Result<usize, AssetError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("texture dimensions {width}x{height} overflow RGBA8 byte length"),
        })?;
    usize::try_from(pixels).map_err(|_| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("texture dimensions {width}x{height} exceed platform address space"),
    })
}

#[cfg(feature = "ktx2")]
pub(super) fn validate_rgba8_payload_len(
    path: &AssetPath,
    width: u32,
    height: u32,
    actual_len: usize,
) -> Result<(), AssetError> {
    if width == 0 || height == 0 {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("texture level has invalid dimensions {width}x{height}"),
        });
    }
    let expected_len = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("texture dimensions {width}x{height} overflow RGBA8 byte length"),
        })?;
    if u64::try_from(actual_len).ok() != Some(expected_len) {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!(
                "texture RGBA8 payload length mismatch for {width}x{height}: \
                 got {actual_len}, expected {expected_len}"
            ),
        });
    }
    Ok(())
}
