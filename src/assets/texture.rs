use std::sync::Arc;

use crate::diagnostics::AssetError;
use crate::material::TextureColorSpace;

use super::{AssetPath, AssetProvenance};

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn texture_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_texture_step(path: &AssetPath, label: &str, start_ms: f64) -> f64 {
    let now = texture_now_ms();
    if crate::diagnostics::browser_timing_enabled() {
        web_sys::console::log_1(
            &format!(
                "[scena-demo] texture {} {label}: {:.1}ms",
                path.as_str(),
                now - start_ms
            )
            .into(),
        );
    }
    now
}

mod memory;
mod sampling;
#[path = "texture_format.rs"]
mod texture_format;
#[path = "texture_image_decode.rs"]
mod texture_image_decode;
#[path = "texture_ktx2.rs"]
pub(super) mod texture_ktx2;
#[path = "texture_limits.rs"]
mod texture_limits;
#[path = "texture_reload.rs"]
mod texture_reload;
#[path = "texture_source.rs"]
mod texture_source;

use memory::mip_policy_for_sampler;
pub use memory::{
    TextureMemoryDesc, TextureMemoryId, TextureMipPolicy, TexturePixelFormat, TextureSlot,
};

pub(crate) use texture_format::validate_texture_source_format;
use texture_image_decode::{decode_jpeg_rgba8, decode_png_rgba8, decode_webp_rgba8};
use texture_ktx2::decode_ktx2_basisu_rgba8;
#[cfg(all(
    feature = "ktx2",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
use texture_ktx2::validate_rgba8_payload_len;
pub(crate) use texture_reload::TextureCacheUpdatePolicy;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
pub(crate) use texture_source::BROWSER_TEXTURE_MAX_DIMENSION_2D;
#[cfg(target_arch = "wasm32")]
use texture_source::browser_native_decode_format;
#[cfg(target_arch = "wasm32")]
pub(crate) use texture_source::decode_browser_image_bitmap;
use texture_source::resolve_texture_source_bytes;

#[derive(Debug, Clone)]
pub struct TextureDesc {
    path: AssetPath,
    memory_identity: Option<TextureMemoryId>,
    provenance: AssetProvenance,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    mip_policy: TextureMipPolicy,
    source_format: TextureSourceFormat,
    pixels: Option<Arc<TexturePixels>>,
    #[cfg(target_arch = "wasm32")]
    encoded_source_bytes: Option<Arc<[u8]>>,
    #[cfg(target_arch = "wasm32")]
    browser_image: Option<web_sys::ImageBitmap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TexturePixels {
    Rgba8 {
        levels: Vec<TextureMipLevel>,
    },
    LinearRgba16Float {
        width: u32,
        height: u32,
        rgba16f_bits: Vec<u16>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextureMipLevel {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

impl TexturePixels {
    fn single_level(width: u32, height: u32, rgba8: Vec<u8>) -> Self {
        Self::Rgba8 {
            levels: vec![TextureMipLevel {
                width,
                height,
                rgba8,
            }],
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
    fn from_mip_levels(path: &AssetPath, levels: Vec<TextureMipLevel>) -> Result<Self, AssetError> {
        if levels.is_empty() {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: "texture decode returned zero mip levels".to_string(),
            });
        }
        for (index, level) in levels.iter().enumerate() {
            validate_rgba8_payload_len(path, level.width, level.height, level.rgba8.len())
                .map_err(|error| match error {
                    AssetError::Parse { path, reason } => AssetError::Parse {
                        path,
                        reason: format!("mip level {index}: {reason}"),
                    },
                    other => other,
                })?;
        }
        Ok(Self::Rgba8 { levels })
    }

    fn base_level(&self) -> Option<&TextureMipLevel> {
        match self {
            Self::Rgba8 { levels } => levels.first(),
            Self::LinearRgba16Float { .. } => None,
        }
    }

    fn mip_metadata(&self) -> Vec<(u32, u32, usize)> {
        match self {
            Self::Rgba8 { levels } => levels
                .iter()
                .map(|level| (level.width, level.height, level.rgba8.len()))
                .collect(),
            Self::LinearRgba16Float {
                width,
                height,
                rgba16f_bits,
            } => vec![(
                *width,
                *height,
                rgba16f_bits.len() * std::mem::size_of::<u16>(),
            )],
        }
    }

    fn dimensions(&self) -> Option<(u32, u32)> {
        match self {
            Self::Rgba8 { levels } => levels.first().map(|level| (level.width, level.height)),
            Self::LinearRgba16Float { width, height, .. } => Some((*width, *height)),
        }
    }

    fn linear_rgba16f(&self) -> Option<(u32, u32, &[u16])> {
        match self {
            Self::LinearRgba16Float {
                width,
                height,
                rgba16f_bits,
            } => Some((*width, *height, rgba16f_bits)),
            Self::Rgba8 { .. } => None,
        }
    }
}

impl PartialEq for TextureDesc {
    fn eq(&self, other: &Self) -> bool {
        let base = self.path == other.path
            && self.memory_identity == other.memory_identity
            && self.provenance == other.provenance
            && self.color_space == other.color_space
            && self.sampler == other.sampler
            && self.mip_policy == other.mip_policy
            && self.source_format == other.source_format
            && self.pixels == other.pixels;
        #[cfg(target_arch = "wasm32")]
        {
            base && self.encoded_source_bytes == other.encoded_source_bytes
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            base
        }
    }
}

impl Eq for TextureDesc {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureSourceFormat {
    Png,
    Jpeg,
    Webp,
    Ktx2Basisu,
    MemoryRgba8,
    MemoryRgba16Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureFilter {
    Nearest,
    Linear,
    NearestMipmapNearest,
    LinearMipmapNearest,
    NearestMipmapLinear,
    LinearMipmapLinear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureWrap {
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureSamplerDesc {
    mag_filter: Option<TextureFilter>,
    min_filter: Option<TextureFilter>,
    wrap_s: TextureWrap,
    wrap_t: TextureWrap,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TextureCacheKey {
    pub(crate) path: AssetPath,
    pub(crate) color_space: TextureColorSpace,
    pub(crate) sampler: TextureSamplerDesc,
    pub(crate) source_format: TextureSourceFormat,
}

impl TextureDesc {
    pub(crate) fn new_with_bytes(
        path: AssetPath,
        color_space: TextureColorSpace,
        sampler: TextureSamplerDesc,
        source_format: TextureSourceFormat,
        source_bytes: Option<&[u8]>,
    ) -> Result<Self, AssetError> {
        let provenance = if let Some(bytes) =
            resolve_texture_source_bytes(&path, source_format, source_bytes)?
        {
            AssetProvenance::from_source_bytes(path.clone(), &bytes)
        } else {
            AssetProvenance::new(path.clone())
        };
        #[cfg(target_arch = "wasm32")]
        if browser_native_decode_format(source_format) {
            let encoded_source_bytes =
                resolve_texture_source_bytes(&path, source_format, source_bytes)?.map(Arc::from);
            return Ok(Self {
                path,
                memory_identity: None,
                provenance,
                color_space,
                sampler,
                mip_policy: mip_policy_for_sampler(sampler),
                source_format,
                pixels: None,
                encoded_source_bytes,
                browser_image: None,
            });
        }
        let pixels =
            decode_texture_pixels(&path, color_space, source_format, source_bytes)?.map(Arc::new);
        Ok(Self {
            path,
            memory_identity: None,
            provenance,
            color_space,
            sampler,
            mip_policy: mip_policy_for_sampler(sampler),
            source_format,
            pixels,
            #[cfg(target_arch = "wasm32")]
            encoded_source_bytes: None,
            #[cfg(target_arch = "wasm32")]
            browser_image: None,
        })
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn memory_identity(&self) -> Option<&TextureMemoryId> {
        self.memory_identity.as_ref()
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

    pub const fn mip_policy(&self) -> TextureMipPolicy {
        self.mip_policy
    }

    pub fn pixel_format(&self) -> TexturePixelFormat {
        if self.pixels.as_ref().is_some_and(|pixels| {
            matches!(pixels.as_ref(), TexturePixels::LinearRgba16Float { .. })
        }) {
            return TexturePixelFormat::Rgba16Float;
        }
        match self.color_space {
            TextureColorSpace::Srgb => TexturePixelFormat::Rgba8UnormSrgb,
            TextureColorSpace::Linear => TexturePixelFormat::Rgba8Unorm,
        }
    }

    pub const fn source_format(&self) -> TextureSourceFormat {
        self.source_format
    }

    pub fn has_decoded_pixels(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            self.pixels.is_some() || self.browser_image.is_some()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pixels.is_some()
        }
    }

    pub fn decoded_dimensions(&self) -> Option<(u32, u32)> {
        #[cfg(target_arch = "wasm32")]
        if let Some(image) = &self.browser_image {
            return Some((image.width(), image.height()));
        }
        self.pixels.as_ref().and_then(|pixels| pixels.dimensions())
    }

    pub fn decoded_rgba8(&self) -> Option<(u32, u32, &[u8])> {
        self.pixels
            .as_ref()
            .and_then(|pixels| pixels.base_level())
            .map(|level| (level.width, level.height, level.rgba8.as_slice()))
    }

    pub fn decoded_mip_metadata(&self) -> Option<Vec<(u32, u32, usize)>> {
        self.pixels.as_ref().map(|pixels| pixels.mip_metadata())
    }

    pub(crate) fn decoded_linear_rgba16f(&self) -> Option<(u32, u32, &[u16])> {
        self.pixels
            .as_ref()
            .and_then(|pixels| pixels.linear_rgba16f())
    }

    pub(crate) fn decode_missing_pixels_from_bytes(
        &mut self,
        source_bytes: Option<&[u8]>,
    ) -> Result<(), AssetError> {
        let Some(source_bytes) = source_bytes else {
            return Ok(());
        };
        let incoming_provenance =
            AssetProvenance::from_source_bytes(self.path.clone(), source_bytes);
        if self.has_source_payload() {
            if self.provenance != incoming_provenance {
                return Err(AssetError::Parse {
                    path: self.path.as_str().to_string(),
                    reason: "texture cache identity collision: incoming source bytes do not match \
                             the immutable provenance of the already-decoded pixels"
                        .to_string(),
                });
            }
            return Ok(());
        }
        #[cfg(target_arch = "wasm32")]
        if browser_native_decode_format(self.source_format) {
            self.encoded_source_bytes = Some(Arc::from(source_bytes));
            self.provenance = incoming_provenance;
            return Ok(());
        }
        let pixels = decode_texture_pixels(
            &self.path,
            self.color_space,
            self.source_format,
            Some(source_bytes),
        )?
        .map(Arc::new);
        self.pixels = pixels;
        self.provenance = incoming_provenance;
        Ok(())
    }

    fn has_source_payload(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            self.pixels.is_some()
                || self.encoded_source_bytes.is_some()
                || self.browser_image.is_some()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pixels.is_some()
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn browser_decode_source(&self) -> Option<Arc<[u8]>> {
        if self.browser_image.is_some() {
            return None;
        }
        self.encoded_source_bytes.clone()
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn set_browser_image(&mut self, image: web_sys::ImageBitmap) {
        self.browser_image = Some(image);
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn browser_image(&self) -> Option<&web_sys::ImageBitmap> {
        self.browser_image.as_ref()
    }
}

impl TextureSamplerDesc {
    pub const fn new(
        mag_filter: Option<TextureFilter>,
        min_filter: Option<TextureFilter>,
        wrap_s: TextureWrap,
        wrap_t: TextureWrap,
    ) -> Self {
        Self {
            mag_filter,
            min_filter,
            wrap_s,
            wrap_t,
        }
    }

    pub const fn mag_filter(self) -> Option<TextureFilter> {
        self.mag_filter
    }

    pub const fn min_filter(self) -> Option<TextureFilter> {
        self.min_filter
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) const fn without_mipmaps(self) -> Self {
        let min_filter = match self.min_filter {
            Some(TextureFilter::NearestMipmapNearest | TextureFilter::NearestMipmapLinear) => {
                Some(TextureFilter::Nearest)
            }
            Some(TextureFilter::LinearMipmapNearest | TextureFilter::LinearMipmapLinear) => {
                Some(TextureFilter::Linear)
            }
            other => other,
        };
        Self {
            mag_filter: self.mag_filter,
            min_filter,
            wrap_s: self.wrap_s,
            wrap_t: self.wrap_t,
        }
    }

    pub const fn wrap_s(self) -> TextureWrap {
        self.wrap_s
    }

    pub const fn wrap_t(self) -> TextureWrap {
        self.wrap_t
    }
}

impl Default for TextureSamplerDesc {
    fn default() -> Self {
        Self {
            mag_filter: None,
            min_filter: None,
            wrap_s: TextureWrap::Repeat,
            wrap_t: TextureWrap::Repeat,
        }
    }
}

fn decode_texture_pixels(
    path: &AssetPath,
    color_space: TextureColorSpace,
    source_format: TextureSourceFormat,
    source_bytes: Option<&[u8]>,
) -> Result<Option<TexturePixels>, AssetError> {
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    let total_start = texture_now_ms();
    let Some(bytes) = resolve_texture_source_bytes(path, source_format, source_bytes)? else {
        return Ok(None);
    };
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    let decode_start = log_texture_step(path, "resolve compressed bytes", total_start);
    let pixels = match source_format {
        TextureSourceFormat::Png => decode_png_rgba8(path, &bytes).map(Some),
        TextureSourceFormat::Jpeg => decode_jpeg_rgba8(path, &bytes).map(Some),
        TextureSourceFormat::Webp => decode_webp_rgba8(path, &bytes).map(Some),
        TextureSourceFormat::Ktx2Basisu => {
            decode_ktx2_basisu_rgba8(path, &bytes, color_space).map(Some)
        }
        TextureSourceFormat::MemoryRgba8 | TextureSourceFormat::MemoryRgba16Float => {
            Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: "in-memory texture formats must use TextureMemoryDesc".to_string(),
            })
        }
    };
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    {
        log_texture_step(path, "decode pixels", decode_start);
        log_texture_step(path, "decode_texture_pixels total", total_start);
    }
    pixels
}
