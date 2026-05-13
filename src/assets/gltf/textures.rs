//! Stage C2: glTF texture/image/sampler parsing now uses the `gltf`
//! crate's typed accessors. Embedded data-URI / bufferView images are
//! still resolved by scena since the gltf crate's `import` path needs a
//! filesystem; here we keep our own external-image fetcher and feed
//! decoded bytes through scena's `TextureDesc`.

use std::collections::BTreeMap;

use ::gltf::Document;
use ::gltf::image::{Image, Source as ImageSource};
use ::gltf::texture::{MagFilter, MinFilter, Texture, WrappingMode};

use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

use super::super::{
    AssetPath, AssetStorage, TextureCacheKey, TextureDesc, TextureFilter, TextureHandle,
    TextureSamplerDesc, TextureSourceFormat, TextureWrap, validate_texture_source_format,
};
use super::buffers::ResolvedGltfBuffers;
use super::external::resolve_relative_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::assets::gltf) struct GltfTexture {
    path: AssetPath,
    sampler: TextureSamplerDesc,
    uses_basisu: bool,
    source_bytes: Option<Vec<u8>>,
}

pub(in crate::assets::gltf) fn parse_textures(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    external_images: &BTreeMap<AssetPath, Vec<u8>>,
    _storage: &mut AssetStorage,
) -> Vec<GltfTexture> {
    document
        .textures()
        .filter_map(|texture| {
            let basisu_image = texture
                .extension_value("KHR_texture_basisu")
                .and_then(|value| value.get("source"))
                .and_then(|value| value.as_u64())
                .and_then(|value| usize::try_from(value).ok())
                .and_then(|index| document.images().nth(index));
            let fallback_image = texture_source_image(document, &texture);
            let (image, uses_basisu) = if cfg!(feature = "ktx2") {
                if let Some(image) = basisu_image {
                    (image, true)
                } else if let Some(image) = fallback_image {
                    (image, false)
                } else {
                    return None;
                }
            } else if let Some(image) = fallback_image {
                (image, false)
            } else if let Some(image) = basisu_image {
                (image, true)
            } else {
                return None;
            };
            let (image_path, source_bytes) = match image.source() {
                ImageSource::Uri { uri, .. } => {
                    if uri.starts_with("data:") {
                        let (_, bytes) = decode_data_uri(uri)?;
                        (AssetPath::from(uri), Some(bytes))
                    } else {
                        let resolved = resolve_relative_path(path, uri);
                        let bytes = external_images.get(&resolved).cloned();
                        (resolved, bytes)
                    }
                }
                ImageSource::View { view, mime_type } => {
                    let bytes = buffers.view_bytes(&view)?.to_vec();
                    let extension = extension_for_mime(Some(mime_type)).unwrap_or("png");
                    (
                        AssetPath::from(format!("memory:image-{}.{extension}", image.index())),
                        Some(bytes),
                    )
                }
            };
            Some(GltfTexture {
                path: image_path,
                sampler: from_gltf_sampler(texture.sampler()),
                uses_basisu,
                source_bytes,
            })
        })
        .collect()
}

fn texture_source_image<'a>(document: &'a Document, texture: &Texture<'a>) -> Option<Image<'a>> {
    let source_index = document
        .as_json()
        .textures
        .get(texture.index())?
        .source
        .value();
    if source_index == u32::MAX as usize {
        None
    } else {
        document.images().nth(source_index)
    }
}

pub(super) fn texture_slot(
    path: &AssetPath,
    material_slot: &'static str,
    texture_index: usize,
    textures: &[GltfTexture],
    storage: &mut AssetStorage,
    color_space: TextureColorSpace,
) -> Result<TextureHandle, AssetError> {
    let texture = textures
        .get(texture_index)
        .ok_or_else(|| AssetError::MissingTexture {
            path: path.as_str().to_string(),
            material_slot: material_slot.to_string(),
            texture_index,
            help: "export the referenced image or remove the broken material slot",
        })?;
    let source_format = if texture.uses_basisu {
        basisu_texture_source_format(&texture.path)?
    } else {
        validate_texture_source_format(&texture.path)?
    };
    insert_texture(
        storage,
        texture.path.clone(),
        color_space,
        texture.sampler,
        source_format,
        texture.source_bytes.as_deref(),
    )
}

fn insert_texture(
    storage: &mut AssetStorage,
    path: AssetPath,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
    source_bytes: Option<&[u8]>,
) -> Result<TextureHandle, AssetError> {
    let cache_key = TextureCacheKey {
        path,
        color_space,
        sampler,
        source_format,
    };
    if let Some(handle) = storage.texture_lookup.get(&cache_key) {
        if source_bytes.is_some() {
            let texture = storage
                .textures
                .get_mut(*handle)
                .ok_or_else(|| AssetError::Parse {
                    path: cache_key.path.as_str().to_string(),
                    reason: "texture cache lookup pointed at a missing texture descriptor"
                        .to_string(),
                })?;
            texture.decode_missing_pixels_from_bytes(source_bytes)?;
        }
        return Ok(*handle);
    }
    let texture = TextureDesc::new_with_bytes(
        cache_key.path.clone(),
        cache_key.color_space,
        cache_key.sampler,
        cache_key.source_format,
        source_bytes,
    )?;
    let handle = storage.textures.insert(texture);
    storage.texture_lookup.insert(cache_key, handle);
    Ok(handle)
}

#[cfg(not(feature = "ktx2"))]
fn basisu_texture_source_format(path: &AssetPath) -> Result<TextureSourceFormat, AssetError> {
    Err(AssetError::UnsupportedOptionalExtensionUsed {
        path: path.as_str().to_string(),
        extension: "KHR_texture_basisu".to_string(),
        help: "enable the ktx2 feature or export a PNG/JPEG/WebP fallback texture".to_string(),
    })
}

#[cfg(feature = "ktx2")]
fn basisu_texture_source_format(path: &AssetPath) -> Result<TextureSourceFormat, AssetError> {
    let source_format = validate_texture_source_format(path)?;
    if source_format == TextureSourceFormat::Ktx2Basisu {
        return Ok(source_format);
    }
    Err(AssetError::UnsupportedTextureFormat {
        path: path.as_str().to_string(),
        help: "KHR_texture_basisu must reference a .ktx2 Basis Universal texture source",
    })
}

fn from_gltf_sampler(sampler: ::gltf::texture::Sampler) -> TextureSamplerDesc {
    TextureSamplerDesc::new(
        sampler.mag_filter().and_then(from_mag_filter),
        sampler.min_filter().and_then(from_min_filter),
        from_wrap(sampler.wrap_s()),
        from_wrap(sampler.wrap_t()),
    )
}

fn from_mag_filter(value: MagFilter) -> Option<TextureFilter> {
    Some(match value {
        MagFilter::Nearest => TextureFilter::Nearest,
        MagFilter::Linear => TextureFilter::Linear,
    })
}

fn from_min_filter(value: MinFilter) -> Option<TextureFilter> {
    Some(match value {
        MinFilter::Nearest => TextureFilter::Nearest,
        MinFilter::Linear => TextureFilter::Linear,
        MinFilter::NearestMipmapNearest => TextureFilter::NearestMipmapNearest,
        MinFilter::LinearMipmapNearest => TextureFilter::LinearMipmapNearest,
        MinFilter::NearestMipmapLinear => TextureFilter::NearestMipmapLinear,
        MinFilter::LinearMipmapLinear => TextureFilter::LinearMipmapLinear,
    })
}

fn from_wrap(value: WrappingMode) -> TextureWrap {
    match value {
        WrappingMode::ClampToEdge => TextureWrap::ClampToEdge,
        WrappingMode::MirroredRepeat => TextureWrap::MirroredRepeat,
        WrappingMode::Repeat => TextureWrap::Repeat,
    }
}

fn decode_data_uri(uri: &str) -> Option<(Option<String>, Vec<u8>)> {
    let (header, encoded) = uri.split_once(";base64,")?;
    let mime = header.strip_prefix("data:").map(|mime| mime.to_string());
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    Some((mime, bytes))
}

fn extension_for_mime(mime: Option<&str>) -> Option<&'static str> {
    match mime? {
        "image/png" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/ktx2" => Some("ktx2"),
        _ => None,
    }
}
