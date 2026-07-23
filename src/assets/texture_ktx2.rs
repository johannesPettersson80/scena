use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
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

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
pub(in crate::assets) fn validate_ktx2_material_color_space(
    path: &AssetPath,
    bytes: &[u8],
    color_space: TextureColorSpace,
    material_slot: &str,
) -> Result<(), AssetError> {
    let reader = ktx2::Reader::new(bytes).map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("invalid KTX2 container: {error:?}"),
    })?;
    let actual_primaries = reader.color_primaries();
    let actual_transfer = reader.transfer_function();
    let (expected_primaries, expected_transfer, help) = match color_space {
        TextureColorSpace::Srgb => (
            Some(ktx2::ColorPrimaries::BT709),
            Some(ktx2::TransferFunction::SRGB),
            "Repair: encode color material textures with BT709 primaries and the sRGB transfer function.",
        ),
        TextureColorSpace::Linear => (
            None,
            Some(ktx2::TransferFunction::Linear),
            "Repair: encode non-color material textures with unspecified primaries and the linear transfer function.",
        ),
    };
    if actual_primaries == expected_primaries && actual_transfer == expected_transfer {
        return Ok(());
    }
    Err(AssetError::Ktx2ColorSpaceMismatch {
        path: path.as_str().to_string(),
        material_slot: material_slot.to_string(),
        dfd: Box::new(crate::diagnostics::Ktx2ColorSpaceDfd {
            expected_primaries: primaries_name(expected_primaries),
            expected_transfer: transfer_name(expected_transfer),
            actual_primaries: primaries_name(actual_primaries),
            actual_transfer: transfer_name(actual_transfer),
        }),
        help,
    })
}

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
fn primaries_name(value: Option<ktx2::ColorPrimaries>) -> &'static str {
    match value {
        None => "UNSPECIFIED",
        Some(ktx2::ColorPrimaries::BT709) => "BT709",
        Some(ktx2::ColorPrimaries::BT601EBU) => "BT601_EBU",
        Some(ktx2::ColorPrimaries::BT601SMPTE) => "BT601_SMPTE",
        Some(ktx2::ColorPrimaries::BT2020) => "BT2020",
        Some(ktx2::ColorPrimaries::CIEXYZ) => "CIEXYZ",
        Some(ktx2::ColorPrimaries::ACES) => "ACES",
        Some(ktx2::ColorPrimaries::ACESCC) => "ACESCC",
        Some(ktx2::ColorPrimaries::NTSC1953) => "NTSC1953",
        Some(ktx2::ColorPrimaries::PAL525) => "PAL525",
        Some(ktx2::ColorPrimaries::DISPLAYP3) => "DISPLAY_P3",
        Some(ktx2::ColorPrimaries::AdobeRGB) => "ADOBE_RGB",
        Some(_) => "UNKNOWN",
    }
}

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
fn transfer_name(value: Option<ktx2::TransferFunction>) -> &'static str {
    match value {
        None => "UNSPECIFIED",
        Some(ktx2::TransferFunction::Linear) => "LINEAR",
        Some(ktx2::TransferFunction::SRGB) => "SRGB",
        Some(ktx2::TransferFunction::ITU) => "ITU",
        Some(ktx2::TransferFunction::NTSC) => "NTSC",
        Some(ktx2::TransferFunction::SLOG) => "SLOG",
        Some(ktx2::TransferFunction::SLOG2) => "SLOG2",
        Some(ktx2::TransferFunction::BT1886) => "BT1886",
        Some(ktx2::TransferFunction::HLGOETF) => "HLG_OETF",
        Some(ktx2::TransferFunction::HLGEOTF) => "HLG_EOTF",
        Some(ktx2::TransferFunction::PQEOTF) => "PQ_EOTF",
        Some(ktx2::TransferFunction::PQOETF) => "PQ_OETF",
        Some(ktx2::TransferFunction::DCIP3) => "DCI_P3",
        Some(ktx2::TransferFunction::PALOETF) => "PAL_OETF",
        Some(ktx2::TransferFunction::PAL625EOTF) => "PAL625_EOTF",
        Some(ktx2::TransferFunction::ST240) => "ST240",
        Some(ktx2::TransferFunction::ACESCC) => "ACESCC",
        Some(ktx2::TransferFunction::ACESCCT) => "ACESCCT",
        Some(ktx2::TransferFunction::AdobeRGB) => "ADOBE_RGB",
        Some(_) => "UNKNOWN",
    }
}

#[cfg(all(
    feature = "ktx2",
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
fn decode_ktx2_basisu_rgba8_with_parser(
    path: &AssetPath,
    bytes: &[u8],
    color_space: TextureColorSpace,
) -> Result<TexturePixels, AssetError> {
    let _ = bytes;
    let _ = color_space;
    Err(AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "KTX2/Basis transcoding requires async Basis Universal initialization on wasm; \
             this sync texture decode path is fail-closed until the browser asset pipeline \
             can await transcoder initialization"
            .to_string(),
    })
}

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
fn decode_ktx2_basisu_rgba8_with_parser(
    path: &AssetPath,
    bytes: &[u8],
    color_space: TextureColorSpace,
) -> Result<TexturePixels, AssetError> {
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
    let declared_rgba8_bytes = u64::from(header.pixel_width)
        .checked_mul(u64::from(header.pixel_height))
        .and_then(|pixels| pixels.checked_mul(4))
        .unwrap_or(u64::MAX);
    if header.pixel_width == 0
        || header.pixel_height == 0
        || header.pixel_width > super::texture_limits::IMAGE_DECODE_MAX_DIMENSION
        || header.pixel_height > super::texture_limits::IMAGE_DECODE_MAX_DIMENSION
        || declared_rgba8_bytes > super::texture_limits::IMAGE_DECODE_MAX_ALLOC_BYTES
    {
        return Err(AssetError::TextureSizeLimit {
            path: path.as_str().to_string(),
            width: header.pixel_width,
            height: header.pixel_height,
            maximum_dimension: super::texture_limits::IMAGE_DECODE_MAX_DIMENSION,
            required_bytes: declared_rgba8_bytes,
            maximum_bytes: super::texture_limits::IMAGE_DECODE_MAX_ALLOC_BYTES,
        });
    }
    checked_rgba8_len(path, header.pixel_width, header.pixel_height)?;

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

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
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

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
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

#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
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
