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

use super::super::provenance::sha256_hex;
#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
use super::super::texture::texture_ktx2::validate_ktx2_material_color_space;
use super::super::{
    AssetPath, AssetStorage, TextureCacheKey, TextureCacheUpdatePolicy, TextureDesc, TextureFilter,
    TextureHandle, TextureSamplerDesc, TextureSourceFormat, TextureWrap,
    validate_texture_source_format,
};
use super::buffers::ResolvedGltfBuffers;
use super::external::resolve_relative_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::assets::gltf) struct GltfTexture {
    path: AssetPath,
    sampler: TextureSamplerDesc,
    uses_basisu: bool,
    source_bytes: Option<Vec<u8>>,
    basisu_fallback: Option<GltfTextureBasisuFallback>,
}

#[derive(Debug)]
pub(in crate::assets::gltf) struct IndexedGltfTextures {
    entries: Vec<IndexedGltfTexture>,
}

#[derive(Debug)]
enum IndexedGltfTexture {
    Resolved(GltfTexture),
    Unresolved(GltfTextureResolutionFailure),
}

#[derive(Debug)]
struct GltfTextureResolutionFailure {
    image_source: Option<String>,
    reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::assets::gltf) struct GltfTextureBasisuFallback {
    pub source_path: AssetPath,
    pub fallback_path: AssetPath,
}

pub(in crate::assets::gltf) fn parse_textures(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    external_images: &BTreeMap<AssetPath, Vec<u8>>,
    _storage: &mut AssetStorage,
) -> IndexedGltfTextures {
    let entries = document
        .textures()
        .map(|texture| {
            let basisu_source_index = texture
                .extension_value("KHR_texture_basisu")
                .and_then(|value| value.get("source"))
                .and_then(|value| value.as_u64())
                .and_then(|value| usize::try_from(value).ok());
            let basisu_image = basisu_source_index.and_then(|index| document.images().nth(index));
            let fallback_source_index = texture_source_image_index(document, &texture);
            let fallback_image =
                fallback_source_index.and_then(|index| document.images().nth(index));
            let (image, uses_basisu, basisu_fallback_source) = if cfg!(feature = "ktx2") {
                if let Some(image) = basisu_image {
                    (image, true, None)
                } else if let Some(image) = fallback_image {
                    (image, false, None)
                } else {
                    return unresolved_texture_source(
                        fallback_source_index.or(basisu_source_index),
                        document.images().len(),
                    );
                }
            } else if let Some(image) = fallback_image {
                (
                    image,
                    false,
                    basisu_image
                        .as_ref()
                        .and_then(|image| image_path(path, image, buffers)),
                )
            } else if let Some(image) = basisu_image {
                (image, true, None)
            } else {
                return unresolved_texture_source(
                    fallback_source_index.or(basisu_source_index),
                    document.images().len(),
                );
            };
            let (image_path, source_bytes) = match image.source() {
                ImageSource::Uri { uri, .. } => {
                    if uri.starts_with("data:") {
                        let Some((path, bytes)) = canonical_data_uri_image(uri) else {
                            return IndexedGltfTexture::Unresolved(GltfTextureResolutionFailure {
                                image_source: Some(describe_image_uri(uri)),
                                reason: "image data URI is invalid or uses an unsupported encoding"
                                    .to_owned(),
                            });
                        };
                        (path, Some(bytes))
                    } else {
                        let resolved = resolve_relative_path(path, uri);
                        let bytes = external_images.get(&resolved).cloned();
                        (resolved, bytes)
                    }
                }
                ImageSource::View { view, mime_type } => {
                    let Some(bytes) = buffers.view_bytes(&view) else {
                        return IndexedGltfTexture::Unresolved(GltfTextureResolutionFailure {
                            image_source: Some(format!("bufferView {}", view.index())),
                            reason: format!(
                                "image bufferView {} bytes could not be resolved",
                                view.index()
                            ),
                        });
                    };
                    let bytes = bytes.to_vec();
                    let extension = extension_for_mime(Some(mime_type)).unwrap_or("png");
                    (embedded_image_path(&bytes, extension), Some(bytes))
                }
            };
            IndexedGltfTexture::Resolved(GltfTexture {
                basisu_fallback: basisu_fallback_source.map(|source_path| {
                    GltfTextureBasisuFallback {
                        source_path,
                        fallback_path: image_path.clone(),
                    }
                }),
                path: image_path,
                sampler: from_gltf_sampler(texture.sampler()),
                uses_basisu,
                source_bytes,
            })
        })
        .collect();
    IndexedGltfTextures { entries }
}

fn unresolved_texture_source(image_index: Option<usize>, image_count: usize) -> IndexedGltfTexture {
    match image_index {
        Some(image_index) => IndexedGltfTexture::Unresolved(GltfTextureResolutionFailure {
            image_source: Some(format!("images[{image_index}]")),
            reason: format!(
                "image index {image_index} is outside the document image table of length {image_count}"
            ),
        }),
        None => IndexedGltfTexture::Unresolved(GltfTextureResolutionFailure {
            image_source: Some("missing source".to_owned()),
            reason: "texture has neither a source nor a resolvable KHR_texture_basisu source"
                .to_owned(),
        }),
    }
}

fn describe_image_uri(uri: &str) -> String {
    if uri.starts_with("data:") {
        uri.split_once(',')
            .map_or("data URI", |(header, _)| header)
            .chars()
            .take(96)
            .collect()
    } else {
        uri.chars().take(256).collect()
    }
}

impl IndexedGltfTextures {
    pub(in crate::assets::gltf) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn resolved(&self, index: usize) -> Option<&GltfTexture> {
        match self.entries.get(index) {
            Some(IndexedGltfTexture::Resolved(texture)) => Some(texture),
            _ => None,
        }
    }

    fn failure(&self, index: usize) -> GltfTextureResolutionFailure {
        match self.entries.get(index) {
            Some(IndexedGltfTexture::Unresolved(failure)) => GltfTextureResolutionFailure {
                image_source: failure.image_source.clone(),
                reason: failure.reason.clone(),
            },
            Some(IndexedGltfTexture::Resolved(_)) => GltfTextureResolutionFailure {
                image_source: None,
                reason: "texture unexpectedly failed to resolve after parsing".to_owned(),
            },
            None => GltfTextureResolutionFailure {
                image_source: None,
                reason: format!(
                    "texture index {index} is outside the document texture table of length {}",
                    self.entries.len()
                ),
            },
        }
    }
}

impl GltfTexture {
    pub(in crate::assets::gltf) fn basisu_fallback(&self) -> Option<&GltfTextureBasisuFallback> {
        self.basisu_fallback.as_ref()
    }

    pub(in crate::assets::gltf) fn path(&self) -> &AssetPath {
        &self.path
    }

    pub(in crate::assets::gltf) const fn source_bytes_missing(&self) -> bool {
        self.source_bytes.is_none()
    }
}

fn image_path(
    path: &AssetPath,
    image: &Image<'_>,
    buffers: &ResolvedGltfBuffers,
) -> Option<AssetPath> {
    match image.source() {
        ImageSource::Uri { uri, .. } => {
            if uri.starts_with("data:") {
                canonical_data_uri_image(uri).map(|(path, _bytes)| path)
            } else {
                Some(resolve_relative_path(path, uri))
            }
        }
        ImageSource::View { view, mime_type } => {
            let extension = extension_for_mime(Some(mime_type)).unwrap_or("png");
            buffers
                .view_bytes(&view)
                .map(|bytes| embedded_image_path(bytes, extension))
        }
    }
}

fn embedded_image_path(bytes: &[u8], extension: &str) -> AssetPath {
    AssetPath::from(format!(
        "memory:image-sha256-{}.{extension}",
        sha256_hex(bytes)
    ))
}

fn texture_source_image_index(document: &Document, texture: &Texture<'_>) -> Option<usize> {
    let source_index = document
        .as_json()
        .textures
        .get(texture.index())?
        .source
        .value();
    if source_index == u32::MAX as usize {
        None
    } else {
        Some(source_index)
    }
}

pub(super) struct TextureSlotRequest<'a> {
    pub(super) path: &'a AssetPath,
    pub(super) material_index: Option<usize>,
    pub(super) material_name: Option<&'a str>,
    pub(super) material_slot: &'static str,
    pub(super) texture_index: usize,
    pub(super) color_space: TextureColorSpace,
}

pub(super) fn texture_slot(
    request: TextureSlotRequest<'_>,
    textures: &IndexedGltfTextures,
    storage: &mut AssetStorage,
) -> Result<TextureHandle, AssetError> {
    let texture = textures.resolved(request.texture_index).ok_or_else(|| {
        let failure = textures.failure(request.texture_index);
        AssetError::MissingTexture {
            path: request.path.as_str().to_string(),
            material_slot: request.material_slot.to_string(),
            texture_index: request.texture_index,
            context: Box::new(crate::diagnostics::MissingTextureDetails {
                material_index: request.material_index,
                material_name: request.material_name.map(str::to_owned),
                image_source: failure.image_source,
                reason: failure.reason,
            }),
            help: "export the referenced image or remove the broken material slot",
        }
    })?;
    let source_format = if texture.uses_basisu {
        basisu_texture_source_format(&texture.path)?
    } else {
        validate_texture_source_format(&texture.path)?
    };
    #[cfg(all(
        feature = "ktx2",
        not(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown"
        ))
    ))]
    if source_format == TextureSourceFormat::Ktx2Basisu
        && let Some(source_bytes) = texture.source_bytes.as_deref()
    {
        validate_ktx2_material_color_space(
            &texture.path,
            source_bytes,
            request.color_space,
            request.material_slot,
        )?;
    }
    insert_texture(
        storage,
        texture.path.clone(),
        request.color_space,
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
        if let Some(source_bytes) = source_bytes {
            let texture = storage
                .textures
                .get_mut(*handle)
                .ok_or_else(|| AssetError::Parse {
                    path: cache_key.path.as_str().to_string(),
                    reason: "texture cache lookup pointed at a missing texture descriptor"
                        .to_string(),
                })?;
            match storage.texture_cache_update_policy {
                TextureCacheUpdatePolicy::Immutable => {
                    std::sync::Arc::make_mut(texture)
                        .decode_missing_pixels_from_bytes(Some(source_bytes))?;
                }
                TextureCacheUpdatePolicy::ReplaceChangedSource => {
                    std::sync::Arc::make_mut(texture).replace_changed_source_bytes(source_bytes)?;
                }
            }
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
    let handle = storage.textures.insert(std::sync::Arc::new(texture));
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

fn canonical_data_uri_image(uri: &str) -> Option<(AssetPath, Vec<u8>)> {
    let (mime, bytes) = decode_data_uri(uri)?;
    let extension = extension_for_mime(mime.as_deref())?;
    let path = embedded_image_path(&bytes, extension);
    Some((path, bytes))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn single_pixel_png(pixel: [u8; 4]) -> Vec<u8> {
        use image::ImageEncoder;

        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&pixel, 1, 1, image::ExtendedColorType::Rgba8)
            .expect("single-pixel C01 PNG encodes");
        bytes
    }

    fn single_pixel_png_data_uri(pixel: [u8; 4]) -> String {
        use base64::Engine;

        let bytes = single_pixel_png(pixel);
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    }

    #[test]
    fn c01_texture_parsing_preserves_raw_indices_when_earlier_source_is_missing() {
        let first_image = single_pixel_png_data_uri([255, 0, 0, 255]);
        let second_image = single_pixel_png_data_uri([0, 0, 255, 255]);
        let source = serde_json::json!({
            "asset": { "version": "2.0" },
            "images": [
                { "uri": first_image.clone() },
                { "uri": second_image.clone() }
            ],
            "textures": [
                {},
                { "source": 0 },
                { "source": 1 }
            ]
        });
        let bytes = serde_json::to_vec(&source).expect("C01 fixture serializes");
        let path = AssetPath::from("memory:c01-texture-index-identity.gltf");
        let gltf = super::super::open_gltf_with_massage(&path, &bytes)
            .expect("C01 fixture parses through the production glTF path");
        let buffers = ResolvedGltfBuffers::new(Vec::new());
        let assets = crate::assets::Assets::new();
        let mut storage = assets.storage();

        let textures = parse_textures(
            &path,
            &gltf.document,
            &buffers,
            &BTreeMap::new(),
            &mut storage,
        );

        assert_eq!(
            textures.len(),
            3,
            "the parsed texture table must preserve all raw glTF indices even when an entry cannot resolve"
        );
        assert!(
            textures.resolved(0).is_none(),
            "the unresolved raw entry stays empty"
        );
        let first_path = canonical_data_uri_image(&first_image)
            .map(|(path, _)| path)
            .expect("first image URI resolves");
        let second_path = canonical_data_uri_image(&second_image)
            .map(|(path, _)| path)
            .expect("second image URI resolves");
        assert_eq!(
            textures.resolved(1).map(GltfTexture::path),
            Some(&first_path),
            "raw texture index 1 must still resolve image 0"
        );
        assert_eq!(
            textures.resolved(2).map(GltfTexture::path),
            Some(&second_path),
            "raw texture index 2 must still resolve image 1"
        );

        let handle = texture_slot(
            TextureSlotRequest {
                path: &path,
                material_index: None,
                material_name: None,
                material_slot: "baseColorTexture",
                texture_index: 1,
                color_space: TextureColorSpace::Srgb,
            },
            &textures,
            &mut storage,
        )
        .expect("raw texture index 1 binds");
        assert_eq!(
            storage
                .textures
                .get(handle)
                .expect("bound texture exists")
                .path(),
            &first_path,
            "material raw index 1 must bind the first image, not shifted index 2"
        );
        for (slot, color_space) in [
            ("baseColorTexture", TextureColorSpace::Srgb),
            ("metallicRoughnessTexture", TextureColorSpace::Linear),
            ("normalTexture", TextureColorSpace::Linear),
            ("occlusionTexture", TextureColorSpace::Linear),
            ("emissiveTexture", TextureColorSpace::Srgb),
            ("clearcoatTexture", TextureColorSpace::Linear),
            ("clearcoatRoughnessTexture", TextureColorSpace::Linear),
            ("clearcoatNormalTexture", TextureColorSpace::Linear),
            ("sheenColorTexture", TextureColorSpace::Srgb),
            ("sheenRoughnessTexture", TextureColorSpace::Linear),
            ("anisotropyTexture", TextureColorSpace::Linear),
            ("iridescenceTexture", TextureColorSpace::Linear),
            ("iridescenceThicknessTexture", TextureColorSpace::Linear),
            ("transmissionTexture", TextureColorSpace::Linear),
            ("thicknessTexture", TextureColorSpace::Linear),
        ] {
            let handle = texture_slot(
                TextureSlotRequest {
                    path: &path,
                    material_index: Some(3),
                    material_name: Some("all-slots"),
                    material_slot: slot,
                    texture_index: 1,
                    color_space,
                },
                &textures,
                &mut storage,
            )
            .unwrap_or_else(|error| panic!("{slot} must preserve raw index 1: {error}"));
            assert_eq!(
                storage
                    .textures
                    .get(handle)
                    .expect("slot texture exists")
                    .path(),
                &first_path,
                "{slot} rebound to a shifted texture"
            );
        }
        assert!(matches!(
            texture_slot(
                TextureSlotRequest {
                    path: &path,
                    material_index: Some(4),
                    material_name: Some("paint"),
                    material_slot: "normalTexture",
                    texture_index: 0,
                    color_space: TextureColorSpace::Linear,
                },
                &textures,
                &mut storage,
            ),
            Err(AssetError::MissingTexture {
                texture_index: 0,
                context,
                ..
            }) if context.material_index == Some(4)
                && context.material_name.as_deref() == Some("paint")
                && context.image_source.as_deref() == Some("missing source")
                && context.reason.contains("source")
        ));
    }

    #[test]
    fn c01_indexed_texture_table_preserves_uri_buffer_view_duplicates_and_samplers() {
        let data_uri = single_pixel_png_data_uri([255, 0, 0, 255]);
        let external_png = single_pixel_png([0, 255, 0, 255]);
        let embedded_png = single_pixel_png([0, 0, 255, 255]);
        let source = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": embedded_png.len() }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": embedded_png.len() }],
            "images": [
                { "uri": data_uri },
                { "uri": "external.png" },
                { "bufferView": 0, "mimeType": "image/png" }
            ],
            "samplers": [
                { "magFilter": 9728, "minFilter": 9728, "wrapS": 33071, "wrapT": 33648 },
                { "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 }
            ],
            "textures": [
                {},
                { "source": 0, "sampler": 0 },
                { "source": 1, "sampler": 1 },
                { "source": 2, "sampler": 0 },
                { "source": 0, "sampler": 1 }
            ]
        });
        let bytes = serde_json::to_vec(&source).expect("C01 source fixture serializes");
        let path = AssetPath::from("fixtures/c01/model.gltf");
        let gltf =
            super::super::open_gltf_with_massage(&path, &bytes).expect("C01 source fixture parses");
        let buffers = ResolvedGltfBuffers::new(vec![embedded_png.clone()]);
        let external_path = resolve_relative_path(&path, "external.png");
        let mut external_images = BTreeMap::new();
        external_images.insert(external_path.clone(), external_png);
        let assets = crate::assets::Assets::new();
        let mut storage = assets.storage();

        let textures = parse_textures(
            &path,
            &gltf.document,
            &buffers,
            &external_images,
            &mut storage,
        );

        assert_eq!(textures.len(), 5);
        assert!(textures.resolved(0).is_none());
        assert_eq!(
            textures.resolved(1).map(GltfTexture::path),
            Some(
                &canonical_data_uri_image(&single_pixel_png_data_uri([255, 0, 0, 255]))
                    .expect("data URI resolves")
                    .0
            )
        );
        assert_eq!(
            textures.resolved(2).map(GltfTexture::path),
            Some(&external_path)
        );
        assert_eq!(
            textures.resolved(3).map(GltfTexture::path),
            Some(&embedded_image_path(&embedded_png, "png"))
        );
        assert_eq!(
            textures.resolved(4).map(GltfTexture::path),
            textures.resolved(1).map(GltfTexture::path),
            "duplicate images retain distinct raw texture entries"
        );
        assert_ne!(
            textures.resolved(1).map(|texture| texture.sampler),
            textures.resolved(4).map(|texture| texture.sampler),
            "duplicate images retain per-texture sampler identity"
        );
    }

    #[test]
    fn c01_unresolved_referenced_texture_reports_material_slot_source_and_reason() {
        let source = serde_json::json!({
            "asset": { "version": "2.0" },
            "textures": [{}],
            "materials": [{
                "name": "paint",
                "pbrMetallicRoughness": { "baseColorTexture": { "index": 0 } }
            }]
        });
        let bytes = serde_json::to_vec(&source).expect("C01 error fixture serializes");
        let path = AssetPath::from("memory:c01-unresolved-referenced-texture.gltf");
        let gltf =
            super::super::open_gltf_with_massage(&path, &bytes).expect("C01 error fixture parses");
        let buffers = ResolvedGltfBuffers::new(Vec::new());
        let assets = crate::assets::Assets::new();
        let mut storage = assets.storage();
        let textures = parse_textures(
            &path,
            &gltf.document,
            &buffers,
            &BTreeMap::new(),
            &mut storage,
        );
        let error = super::super::materials::parse_materials(
            &path,
            &gltf.document,
            &mut storage,
            &textures,
            &mut Vec::new(),
        )
        .expect_err("referenced unresolved texture must fail closed");

        assert!(matches!(
            error,
            AssetError::MissingTexture {
                material_slot,
                texture_index: 0,
                context,
                ..
            } if context.material_index == Some(0)
                && context.material_name.as_deref() == Some("paint")
                && material_slot == "baseColorTexture"
                && context.image_source.as_deref() == Some("missing source")
                && context.reason.contains("neither a source")
        ));
    }

    #[test]
    fn pf10_data_uri_cache_identity_is_content_addressed_and_bounded() {
        use base64::Engine;
        let bytes = vec![0x5a; 512 * 1024];
        let uri = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        );
        let (path, decoded) = canonical_data_uri_image(&uri).expect("valid data URI");

        assert_eq!(decoded, bytes);
        assert!(path.as_str().starts_with("memory:image-sha256-"));
        assert!(path.as_str().ends_with(".png"));
        assert!(path.as_str().len() < 128, "digest key must stay bounded");
        assert!(!path.as_str().contains("base64"));
    }
}
