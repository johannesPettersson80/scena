use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

use super::super::{AssetMaterialFallback, AssetPath, AssetStorage, TextureHandle};
use super::textures::{GltfTexture, IndexedGltfTextures, TextureSlotRequest, texture_slot};

pub(super) struct TextureSlotFallbackRequest<'a> {
    pub(super) path: &'a AssetPath,
    pub(super) slot: &'static str,
    pub(super) material_index: usize,
    pub(super) texture_index: usize,
    pub(super) color_space: TextureColorSpace,
}

pub(super) struct MaterialFallbackSinks<'a> {
    pub(super) all: &'a mut Vec<AssetMaterialFallback>,
    pub(super) source: &'a mut Vec<AssetMaterialFallback>,
}

impl<'a> TextureSlotFallbackRequest<'a> {
    pub(super) fn new(
        path: &'a AssetPath,
        slot: &'static str,
        material_index: usize,
        texture_index: usize,
        color_space: TextureColorSpace,
    ) -> Self {
        Self {
            path,
            slot,
            material_index,
            texture_index,
            color_space,
        }
    }
}

pub(super) fn texture_slot_with_fallback(
    request: TextureSlotFallbackRequest<'_>,
    textures: &IndexedGltfTextures,
    storage: &mut AssetStorage,
    sinks: &mut MaterialFallbackSinks<'_>,
) -> Result<TextureHandle, AssetError> {
    let texture = texture_slot(
        TextureSlotRequest {
            path: request.path,
            material_index: Some(request.material_index),
            material_name: None,
            material_slot: request.slot,
            texture_index: request.texture_index,
            color_space: request.color_space,
        },
        textures,
        storage,
    )?;
    if let Some(fallback) = textures
        .resolved(request.texture_index)
        .and_then(GltfTexture::basisu_fallback)
    {
        let fallback = AssetMaterialFallback::texture_basisu_fallback(
            request.material_index,
            request.slot,
            request.texture_index,
            fallback.source_path.clone(),
            fallback.fallback_path.clone(),
        );
        sinks.all.push(fallback.clone());
        sinks.source.push(fallback);
    }
    if let Some(source) = textures
        .resolved(request.texture_index)
        .filter(|texture| texture.source_bytes_missing())
    {
        let fallback = AssetMaterialFallback::missing_texture_fallback(
            request.material_index,
            request.slot,
            request.texture_index,
            source.path().clone(),
            AssetPath::from("scena.material.fallback_texture"),
        );
        sinks.all.push(fallback.clone());
        sinks.source.push(fallback);
    }
    Ok(texture)
}
