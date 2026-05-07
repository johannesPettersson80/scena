use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

use super::super::super::{
    AssetPath, AssetStorage, TextureCacheKey, TextureDesc, TextureFilter, TextureHandle,
    TextureSamplerDesc, TextureSourceFormat, TextureWrap, validate_texture_source_format,
};
use super::super::accessor::optional_usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::assets::gltf) struct GltfTexture {
    path: AssetPath,
    sampler: TextureSamplerDesc,
    uses_basisu: bool,
}

pub(in crate::assets::gltf) fn parse_textures(
    path: &AssetPath,
    json: &JsonValue,
    _storage: &mut AssetStorage,
) -> Vec<GltfTexture> {
    let images = json
        .get("images")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let samplers = parse_samplers(json);
    json.get("textures")
        .and_then(JsonValue::as_array)
        .map(|textures| {
            textures
                .iter()
                .filter_map(|texture| {
                    let basisu_source = texture
                        .get("extensions")
                        .and_then(|extensions| extensions.get("KHR_texture_basisu"))
                        .and_then(|basisu| optional_usize(basisu, "source"));
                    let source = basisu_source.or_else(|| optional_usize(texture, "source"))?;
                    let uri = images
                        .get(source)
                        .and_then(|image| image.get("uri"))
                        .and_then(JsonValue::as_str)?;
                    let sampler = texture
                        .get("sampler")
                        .and_then(JsonValue::as_u64)
                        .and_then(|index| samplers.get(index as usize))
                        .copied()
                        .unwrap_or_default();
                    Some(GltfTexture {
                        path: resolve_relative_path(path, uri),
                        sampler,
                        uses_basisu: basisu_source.is_some(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn texture_slot(
    path: &AssetPath,
    material_slot: &'static str,
    texture_info: &JsonValue,
    textures: &[GltfTexture],
    storage: &mut AssetStorage,
    color_space: TextureColorSpace,
) -> Result<Option<TextureHandle>, AssetError> {
    let Some(texture_index) = texture_info.get("index").and_then(JsonValue::as_u64) else {
        return Ok(None);
    };
    let texture_index = texture_index as usize;
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
    Ok(Some(insert_texture(
        storage,
        texture.path.clone(),
        color_space,
        texture.sampler,
        source_format,
    )))
}

fn insert_texture(
    storage: &mut AssetStorage,
    path: AssetPath,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
) -> TextureHandle {
    let cache_key = TextureCacheKey {
        path,
        color_space,
        sampler,
        source_format,
    };
    if let Some(handle) = storage.texture_lookup.get(&cache_key) {
        return *handle;
    }
    let texture = TextureDesc {
        path: cache_key.path.clone(),
        color_space: cache_key.color_space,
        sampler: cache_key.sampler,
        source_format: cache_key.source_format,
    };
    let handle = storage.textures.insert(texture);
    storage.texture_lookup.insert(cache_key, handle);
    handle
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

fn parse_samplers(json: &JsonValue) -> Vec<TextureSamplerDesc> {
    json.get("samplers")
        .and_then(JsonValue::as_array)
        .map(|samplers| samplers.iter().map(parse_sampler).collect())
        .unwrap_or_default()
}

fn parse_sampler(sampler: &JsonValue) -> TextureSamplerDesc {
    TextureSamplerDesc::new(
        sampler
            .get("magFilter")
            .and_then(JsonValue::as_u64)
            .and_then(parse_mag_filter),
        sampler
            .get("minFilter")
            .and_then(JsonValue::as_u64)
            .and_then(parse_min_filter),
        sampler
            .get("wrapS")
            .and_then(JsonValue::as_u64)
            .and_then(parse_wrap)
            .unwrap_or(TextureWrap::Repeat),
        sampler
            .get("wrapT")
            .and_then(JsonValue::as_u64)
            .and_then(parse_wrap)
            .unwrap_or(TextureWrap::Repeat),
    )
}

fn parse_mag_filter(value: u64) -> Option<TextureFilter> {
    match value {
        9728 => Some(TextureFilter::Nearest),
        9729 => Some(TextureFilter::Linear),
        _ => None,
    }
}

fn parse_min_filter(value: u64) -> Option<TextureFilter> {
    match value {
        9728 => Some(TextureFilter::Nearest),
        9729 => Some(TextureFilter::Linear),
        9984 => Some(TextureFilter::NearestMipmapNearest),
        9985 => Some(TextureFilter::LinearMipmapNearest),
        9986 => Some(TextureFilter::NearestMipmapLinear),
        9987 => Some(TextureFilter::LinearMipmapLinear),
        _ => None,
    }
}

fn parse_wrap(value: u64) -> Option<TextureWrap> {
    match value {
        33071 => Some(TextureWrap::ClampToEdge),
        33648 => Some(TextureWrap::MirroredRepeat),
        10497 => Some(TextureWrap::Repeat),
        _ => None,
    }
}

fn resolve_relative_path(base: &AssetPath, uri: &str) -> AssetPath {
    if uri.starts_with("data:") || uri.starts_with('/') || uri.contains("://") {
        return AssetPath::from(uri);
    }
    let Some((directory, _file)) = base.as_str().rsplit_once('/') else {
        return AssetPath::from(uri);
    };
    AssetPath::from(format!("{directory}/{uri}"))
}
