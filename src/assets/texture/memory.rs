use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TexturePixelFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba16Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureMipPolicy {
    None,
    Generate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureSlot {
    BaseColor,
    Emissive,
    SheenColor,
    Normal,
    MetallicRoughness,
    Occlusion,
    Clearcoat,
    ClearcoatRoughness,
    ClearcoatNormal,
    SheenRoughness,
    Anisotropy,
    Iridescence,
    IridescenceThickness,
    Transmission,
    Thickness,
}

impl TextureSlot {
    pub const fn color_space(self) -> TextureColorSpace {
        match self {
            Self::BaseColor | Self::Emissive | Self::SheenColor => TextureColorSpace::Srgb,
            Self::Normal
            | Self::MetallicRoughness
            | Self::Occlusion
            | Self::Clearcoat
            | Self::ClearcoatRoughness
            | Self::ClearcoatNormal
            | Self::SheenRoughness
            | Self::Anisotropy
            | Self::Iridescence
            | Self::IridescenceThickness
            | Self::Transmission
            | Self::Thickness => TextureColorSpace::Linear,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BaseColor => "base_color",
            Self::Emissive => "emissive",
            Self::SheenColor => "sheen_color",
            Self::Normal => "normal",
            Self::MetallicRoughness => "metallic_roughness",
            Self::Occlusion => "occlusion",
            Self::Clearcoat => "clearcoat",
            Self::ClearcoatRoughness => "clearcoat_roughness",
            Self::ClearcoatNormal => "clearcoat_normal",
            Self::SheenRoughness => "sheen_roughness",
            Self::Anisotropy => "anisotropy",
            Self::Iridescence => "iridescence",
            Self::IridescenceThickness => "iridescence_thickness",
            Self::Transmission => "transmission",
            Self::Thickness => "thickness",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureMemoryId(String);

impl TextureMemoryId {
    pub fn new(identity: impl Into<String>) -> Result<Self, AssetError> {
        let identity = identity.into();
        if identity.trim().is_empty() {
            return Err(AssetError::InvalidTextureIdentity {
                identity,
                reason: "identity must not be empty or whitespace-only".to_string(),
            });
        }
        if identity.chars().any(char::is_control) {
            return Err(AssetError::InvalidTextureIdentity {
                identity,
                reason: "identity must not contain control characters".to_string(),
            });
        }
        Ok(Self(identity))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TextureMemoryPixels {
    Rgba8(Vec<u8>),
    LinearRgba32Float(Vec<[f32; 4]>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureMemoryDesc {
    identity: TextureMemoryId,
    width: u32,
    height: u32,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    mip_policy: TextureMipPolicy,
    pixels: TextureMemoryPixels,
}

impl TextureMemoryDesc {
    pub fn rgba8(
        identity: TextureMemoryId,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
        color_space: TextureColorSpace,
    ) -> Self {
        Self {
            identity,
            width,
            height,
            color_space,
            sampler: TextureSamplerDesc::default(),
            mip_policy: TextureMipPolicy::None,
            pixels: TextureMemoryPixels::Rgba8(rgba8),
        }
    }

    pub fn rgba8_for_slot(
        identity: TextureMemoryId,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
        slot: TextureSlot,
    ) -> Self {
        Self::rgba8(identity, width, height, rgba8, slot.color_space())
    }

    pub fn linear_rgba32f(
        identity: TextureMemoryId,
        width: u32,
        height: u32,
        rgba32f: Vec<[f32; 4]>,
    ) -> Self {
        Self {
            identity,
            width,
            height,
            color_space: TextureColorSpace::Linear,
            sampler: TextureSamplerDesc::default(),
            mip_policy: TextureMipPolicy::None,
            pixels: TextureMemoryPixels::LinearRgba32Float(rgba32f),
        }
    }

    pub fn with_sampler(mut self, sampler: TextureSamplerDesc) -> Self {
        self.sampler = sampler;
        self
    }

    pub fn with_mip_policy(mut self, mip_policy: TextureMipPolicy) -> Self {
        self.mip_policy = mip_policy;
        self
    }

    pub fn identity(&self) -> &TextureMemoryId {
        &self.identity
    }

    pub const fn color_space(&self) -> TextureColorSpace {
        self.color_space
    }
}

impl TextureMemoryDesc {
    pub(crate) fn into_texture_desc(self) -> Result<TextureDesc, AssetError> {
        use self::texture_limits::{IMAGE_DECODE_MAX_ALLOC_BYTES, IMAGE_DECODE_MAX_DIMENSION};

        let pixel_count = u64::from(self.width).checked_mul(u64::from(self.height));
        let bytes_per_pixel = match &self.pixels {
            TextureMemoryPixels::Rgba8(_) => 4,
            TextureMemoryPixels::LinearRgba32Float(_) => 8,
        };
        let required_bytes = pixel_count
            .and_then(|count| count.checked_mul(bytes_per_pixel))
            .unwrap_or(u64::MAX);
        let path = format!("memory://{}", self.identity.as_str());
        if self.width == 0
            || self.height == 0
            || self.width > IMAGE_DECODE_MAX_DIMENSION
            || self.height > IMAGE_DECODE_MAX_DIMENSION
            || required_bytes > IMAGE_DECODE_MAX_ALLOC_BYTES
        {
            return Err(AssetError::TextureSizeLimit {
                path,
                width: self.width,
                height: self.height,
                maximum_dimension: IMAGE_DECODE_MAX_DIMENSION,
                required_bytes,
                maximum_bytes: IMAGE_DECODE_MAX_ALLOC_BYTES,
            });
        }

        let expected_pixels = usize::try_from(pixel_count.expect("validated texture dimensions"))
            .unwrap_or(usize::MAX);
        if self.mip_policy == TextureMipPolicy::None
            && mip_policy_for_sampler(self.sampler) == TextureMipPolicy::Generate
        {
            return Err(AssetError::InvalidTextureData {
                identity: self.identity.as_str().to_string(),
                width: self.width,
                height: self.height,
                expected_elements: expected_pixels,
                actual_elements: expected_pixels,
                reason: "mipmap minification filter requires TextureMipPolicy::Generate"
                    .to_string(),
            });
        }
        let (pixels, source_format, source_bytes) = match self.pixels {
            TextureMemoryPixels::Rgba8(rgba8) => {
                let expected_elements = expected_pixels.saturating_mul(4);
                if rgba8.len() != expected_elements {
                    return Err(AssetError::InvalidTextureData {
                        identity: self.identity.as_str().to_string(),
                        width: self.width,
                        height: self.height,
                        expected_elements,
                        actual_elements: rgba8.len(),
                        reason: "RGBA8 input must contain four bytes per pixel".to_string(),
                    });
                }
                let source_bytes = rgba8.clone();
                (
                    TexturePixels::single_level(self.width, self.height, rgba8),
                    TextureSourceFormat::MemoryRgba8,
                    source_bytes,
                )
            }
            TextureMemoryPixels::LinearRgba32Float(rgba32f) => {
                if rgba32f.len() != expected_pixels {
                    return Err(AssetError::InvalidTextureData {
                        identity: self.identity.as_str().to_string(),
                        width: self.width,
                        height: self.height,
                        expected_elements: expected_pixels,
                        actual_elements: rgba32f.len(),
                        reason: "linear-float input must contain one RGBA value per pixel"
                            .to_string(),
                    });
                }
                if rgba32f.iter().flatten().any(|value| !value.is_finite()) {
                    return Err(AssetError::InvalidTextureData {
                        identity: self.identity.as_str().to_string(),
                        width: self.width,
                        height: self.height,
                        expected_elements: expected_pixels,
                        actual_elements: rgba32f.len(),
                        reason: "linear-float channels must all be finite".to_string(),
                    });
                }
                let rgba16f_bits = rgba32f
                    .iter()
                    .flat_map(|pixel| pixel.iter())
                    .map(|value| half::f16::from_f32(*value).to_bits())
                    .collect::<Vec<_>>();
                if rgba16f_bits
                    .iter()
                    .any(|bits| !half::f16::from_bits(*bits).is_finite())
                {
                    return Err(AssetError::InvalidTextureData {
                        identity: self.identity.as_str().to_string(),
                        width: self.width,
                        height: self.height,
                        expected_elements: expected_pixels,
                        actual_elements: rgba32f.len(),
                        reason: "linear-float channels exceed the finite RGBA16Float range"
                            .to_string(),
                    });
                }
                let source_bytes = bytemuck::cast_slice(&rgba16f_bits).to_vec();
                (
                    TexturePixels::LinearRgba16Float {
                        width: self.width,
                        height: self.height,
                        rgba16f_bits,
                    },
                    TextureSourceFormat::MemoryRgba16Float,
                    source_bytes,
                )
            }
        };
        let asset_path = AssetPath::from(path);
        Ok(TextureDesc {
            provenance: AssetProvenance::from_source_bytes(asset_path.clone(), &source_bytes),
            path: asset_path,
            memory_identity: Some(self.identity),
            color_space: self.color_space,
            sampler: self.sampler,
            mip_policy: self.mip_policy,
            source_format,
            pixels: Some(Arc::new(pixels)),
            #[cfg(target_arch = "wasm32")]
            encoded_source_bytes: None,
            #[cfg(target_arch = "wasm32")]
            browser_image: None,
        })
    }
}

pub(super) const fn mip_policy_for_sampler(sampler: TextureSamplerDesc) -> TextureMipPolicy {
    match sampler.min_filter() {
        Some(
            TextureFilter::NearestMipmapNearest
            | TextureFilter::LinearMipmapNearest
            | TextureFilter::NearestMipmapLinear
            | TextureFilter::LinearMipmapLinear,
        ) => TextureMipPolicy::Generate,
        _ => TextureMipPolicy::None,
    }
}
