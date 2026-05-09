//! Plan line 778 commit 2: per-role texture upload descriptor shared by the
//! per-material and batched material allocation paths. Encapsulates the
//! decoded RGBA8 pixel buffer plus the wgpu format / sampler metadata so
//! both paths produce identical layer content.

use crate::assets::{TextureDesc, TextureFilter, TextureSamplerDesc, TextureWrap};
use crate::material::TextureColorSpace;

use super::material_mips::mip_level_extents;

const FALLBACK_WHITE_RGBA8: &[u8; 4] = &[255, 255, 255, 255];
const FALLBACK_NORMAL_RGBA8: &[u8; 4] = &[128, 128, 255, 255];
const FALLBACK_METALLIC_ROUGHNESS_RGBA8: &[u8; 4] = &[255, 255, 0, 255];

#[derive(Debug, Clone, Copy)]
pub(super) struct MaterialTextureUpload<'a> {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba8: &'a [u8],
    pub(super) format: wgpu::TextureFormat,
    pub(super) sampler: TextureSamplerDesc,
    pub(super) uses_decoded_texture: bool,
}

impl<'a> MaterialTextureUpload<'a> {
    pub(super) fn from_base_color_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_texture(
            texture,
            FALLBACK_WHITE_RGBA8,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    pub(super) fn from_normal_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_linear_texture(texture, FALLBACK_NORMAL_RGBA8)
    }

    pub(super) fn from_metallic_roughness_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_linear_texture(texture, FALLBACK_METALLIC_ROUGHNESS_RGBA8)
    }

    pub(super) fn from_occlusion_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_linear_texture(texture, FALLBACK_WHITE_RGBA8)
    }

    pub(super) fn from_emissive_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_base_color_texture(texture)
    }

    pub(super) fn from_linear_texture(
        texture: Option<&'a TextureDesc>,
        fallback_rgba8: &'a [u8; 4],
    ) -> Self {
        Self::from_texture(texture, fallback_rgba8, wgpu::TextureFormat::Rgba8Unorm)
    }

    fn from_texture(
        texture: Option<&'a TextureDesc>,
        fallback_rgba8: &'a [u8; 4],
        fallback_format: wgpu::TextureFormat,
    ) -> Self {
        if let Some(texture) = texture
            && let Some((width, height, rgba8)) = texture.decoded_rgba8()
            && width > 0
            && height > 0
            && !rgba8.is_empty()
        {
            let format = match texture.color_space() {
                TextureColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
                TextureColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
            };
            return Self {
                width,
                height,
                rgba8,
                format,
                sampler: texture.sampler(),
                uses_decoded_texture: true,
            };
        }

        Self {
            width: 1,
            height: 1,
            rgba8: fallback_rgba8,
            format: fallback_format,
            sampler: TextureSamplerDesc::default(),
            uses_decoded_texture: false,
        }
    }

    pub(super) fn byte_len(self) -> u64 {
        self.byte_len_for_layers(1)
    }

    pub(super) fn byte_len_for_layers(self, layers: u32) -> u64 {
        mip_level_extents(self.width, self.height, self.sampler.min_filter())
            .into_iter()
            .map(|(width, height)| {
                u64::from(width)
                    .saturating_mul(u64::from(height))
                    .saturating_mul(4)
                    .saturating_mul(u64::from(layers))
            })
            .sum()
    }
}

pub(super) fn address_mode(wrap: TextureWrap) -> wgpu::AddressMode {
    match wrap {
        TextureWrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        TextureWrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        TextureWrap::Repeat => wgpu::AddressMode::Repeat,
    }
}

pub(super) fn filter_mode(filter: Option<TextureFilter>) -> wgpu::FilterMode {
    match filter {
        Some(
            TextureFilter::Nearest
            | TextureFilter::NearestMipmapNearest
            | TextureFilter::NearestMipmapLinear,
        ) => wgpu::FilterMode::Nearest,
        Some(
            TextureFilter::Linear
            | TextureFilter::LinearMipmapNearest
            | TextureFilter::LinearMipmapLinear,
        )
        | None => wgpu::FilterMode::Linear,
    }
}

pub(super) fn mipmap_filter_mode(filter: Option<TextureFilter>) -> wgpu::MipmapFilterMode {
    match filter {
        Some(TextureFilter::NearestMipmapNearest | TextureFilter::LinearMipmapNearest) => {
            wgpu::MipmapFilterMode::Nearest
        }
        Some(TextureFilter::NearestMipmapLinear | TextureFilter::LinearMipmapLinear) => {
            wgpu::MipmapFilterMode::Linear
        }
        Some(TextureFilter::Nearest | TextureFilter::Linear) | None => {
            wgpu::MipmapFilterMode::Nearest
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MaterialTextureUpload;
    use crate::assets::{AssetPath, TextureDesc, TextureSamplerDesc, TextureSourceFormat};
    use crate::material::TextureColorSpace;

    #[test]
    fn decoded_base_color_texture_becomes_backend_upload() {
        let texture = TextureDesc::new_with_bytes(
            AssetPath::from(
                "data:image/png;base64,\
                 iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
            ),
            TextureColorSpace::Srgb,
            TextureSamplerDesc::default(),
            TextureSourceFormat::Png,
            None,
        )
        .expect("inline PNG texture decodes");

        let upload = MaterialTextureUpload::from_base_color_texture(Some(&texture));

        assert!(upload.uses_decoded_texture);
        assert_eq!(upload.width, 1);
        assert_eq!(upload.height, 1);
        assert_eq!(upload.rgba8, &[255, 0, 0, 255]);
        assert_eq!(upload.format, wgpu::TextureFormat::Rgba8UnormSrgb);
    }
}
