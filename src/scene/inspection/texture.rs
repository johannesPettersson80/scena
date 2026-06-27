use crate::assets::{
    AssetPath, AssetProvenance, TextureDesc, TextureHandle, TextureSamplerDesc, TextureSourceFormat,
};
use crate::material::TextureColorSpace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneTextureInspection {
    texture: TextureHandle,
    source_path: AssetPath,
    provenance: AssetProvenance,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
    decoded_dimensions: Option<(u32, u32)>,
    has_decoded_pixels: bool,
}

impl SceneTextureInspection {
    pub(super) fn new(texture: TextureHandle, desc: TextureDesc) -> Self {
        Self {
            texture,
            source_path: desc.path().clone(),
            provenance: desc.provenance().clone(),
            color_space: desc.color_space(),
            sampler: desc.sampler(),
            source_format: desc.source_format(),
            decoded_dimensions: desc.decoded_dimensions(),
            has_decoded_pixels: desc.has_decoded_pixels(),
        }
    }

    pub const fn texture(&self) -> TextureHandle {
        self.texture
    }

    pub fn source_path(&self) -> &AssetPath {
        &self.source_path
    }

    pub fn provenance(&self) -> &AssetProvenance {
        &self.provenance
    }

    pub const fn color_space(&self) -> TextureColorSpace {
        self.color_space
    }

    pub const fn sampler(&self) -> TextureSamplerDesc {
        self.sampler
    }

    pub const fn source_format(&self) -> TextureSourceFormat {
        self.source_format
    }

    pub const fn decoded_dimensions(&self) -> Option<(u32, u32)> {
        self.decoded_dimensions
    }

    pub const fn has_decoded_pixels(&self) -> bool {
        self.has_decoded_pixels
    }
}
