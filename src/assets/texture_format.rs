use super::{AssetError, AssetPath, TextureSourceFormat, TextureWrap};

pub(crate) fn validate_texture_source_format(
    path: &AssetPath,
) -> Result<TextureSourceFormat, AssetError> {
    let lower = path.as_str().to_ascii_lowercase();
    if lower.ends_with(".png") || lower.starts_with("data:image/png") {
        return Ok(TextureSourceFormat::Png);
    }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.starts_with("data:image/jpeg") {
        return Ok(TextureSourceFormat::Jpeg);
    }
    if lower.ends_with(".webp") || lower.starts_with("data:image/webp") {
        return Ok(TextureSourceFormat::Webp);
    }
    #[cfg(feature = "ktx2")]
    if lower.ends_with(".ktx2") || lower.starts_with("data:image/ktx2") {
        return Ok(TextureSourceFormat::Ktx2Basisu);
    }
    Err(AssetError::UnsupportedTextureFormat {
        path: path.as_str().to_string(),
        help: "supported texture format set is PNG, JPEG, and WebP; compressed texture decoders need an explicit feature/policy",
    })
}

pub(super) fn wrap_texture_coordinate(value: f32, wrap: TextureWrap) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    match wrap {
        TextureWrap::Repeat => value.rem_euclid(1.0),
        TextureWrap::ClampToEdge => value.clamp(0.0, 1.0),
        TextureWrap::MirroredRepeat => {
            let wrapped = value.rem_euclid(2.0);
            if wrapped <= 1.0 {
                wrapped
            } else {
                2.0 - wrapped
            }
        }
    }
}
