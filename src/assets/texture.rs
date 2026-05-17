use std::sync::Arc;

use crate::diagnostics::AssetError;
use crate::material::{Color, TextureColorSpace};

use super::AssetPath;

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn texture_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_texture_step(path: &AssetPath, label: &str, start_ms: f64) -> f64 {
    let now = texture_now_ms();
    web_sys::console::log_1(
        &format!(
            "[scena-demo] texture {} {label}: {:.1}ms",
            path.as_str(),
            now - start_ms
        )
        .into(),
    );
    now
}

#[path = "texture_ktx2.rs"]
mod texture_ktx2;
#[path = "texture_source.rs"]
mod texture_source;

use texture_ktx2::decode_ktx2_basisu_rgba8;
#[cfg(feature = "ktx2")]
use texture_ktx2::validate_rgba8_payload_len;
#[cfg(target_arch = "wasm32")]
use texture_source::browser_native_decode_format;
#[cfg(target_arch = "wasm32")]
pub(crate) use texture_source::decode_browser_image_bitmap;
use texture_source::resolve_texture_source_bytes;

#[derive(Debug, Clone)]
pub struct TextureDesc {
    path: AssetPath,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
    pixels: Option<Arc<TexturePixels>>,
    #[cfg(target_arch = "wasm32")]
    encoded_source_bytes: Option<Arc<[u8]>>,
    #[cfg(target_arch = "wasm32")]
    browser_image: Option<web_sys::ImageBitmap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TexturePixels {
    levels: Vec<TextureMipLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextureMipLevel {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

impl TexturePixels {
    fn single_level(width: u32, height: u32, rgba8: Vec<u8>) -> Self {
        Self {
            levels: vec![TextureMipLevel {
                width,
                height,
                rgba8,
            }],
        }
    }

    #[cfg(feature = "ktx2")]
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
        Ok(Self { levels })
    }

    fn base_level(&self) -> Option<&TextureMipLevel> {
        self.levels.first()
    }

    fn mip_metadata(&self) -> Vec<(u32, u32, usize)> {
        self.levels
            .iter()
            .map(|level| (level.width, level.height, level.rgba8.len()))
            .collect()
    }
}

impl PartialEq for TextureDesc {
    fn eq(&self, other: &Self) -> bool {
        let base = self.path == other.path
            && self.color_space == other.color_space
            && self.sampler == other.sampler
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
        #[cfg(target_arch = "wasm32")]
        if browser_native_decode_format(source_format) {
            let encoded_source_bytes =
                resolve_texture_source_bytes(&path, source_format, source_bytes)?.map(Arc::from);
            return Ok(Self {
                path,
                color_space,
                sampler,
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
            color_space,
            sampler,
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

    pub const fn color_space(&self) -> TextureColorSpace {
        self.color_space
    }

    pub const fn sampler(&self) -> TextureSamplerDesc {
        self.sampler
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
        self.pixels
            .as_ref()
            .and_then(|pixels| pixels.base_level())
            .map(|level| (level.width, level.height))
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

    pub(crate) fn decode_missing_pixels_from_bytes(
        &mut self,
        source_bytes: Option<&[u8]>,
    ) -> Result<(), AssetError> {
        #[cfg(target_arch = "wasm32")]
        if browser_native_decode_format(self.source_format) {
            if self.encoded_source_bytes.is_none() {
                self.encoded_source_bytes =
                    resolve_texture_source_bytes(&self.path, self.source_format, source_bytes)?
                        .map(Arc::from);
            }
            return Ok(());
        }
        if self.pixels.is_none() {
            self.pixels = decode_texture_pixels(
                &self.path,
                self.color_space,
                self.source_format,
                source_bytes,
            )?
            .map(Arc::new);
        }
        Ok(())
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

    pub(crate) fn sample_bilinear(&self, uv: [f32; 2]) -> Option<Color> {
        let pixels = self.pixels.as_ref()?;
        let level = pixels.base_level()?;
        let u = wrap_texture_coordinate(uv[0], self.sampler.wrap_s);
        let v = wrap_texture_coordinate(uv[1], self.sampler.wrap_t);
        let x = u * level.width.saturating_sub(1) as f32;
        let y = v * level.height.saturating_sub(1) as f32;
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(level.width.saturating_sub(1));
        let y1 = (y0 + 1).min(level.height.saturating_sub(1));
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let c00 = self.sample_pixel_color(level, x0, y0)?;
        let c10 = self.sample_pixel_color(level, x1, y0)?;
        let c01 = self.sample_pixel_color(level, x0, y1)?;
        let c11 = self.sample_pixel_color(level, x1, y1)?;
        Some(lerp_color(
            lerp_color(c00, c10, tx),
            lerp_color(c01, c11, tx),
            ty,
        ))
    }

    fn sample_pixel_color(&self, level: &TextureMipLevel, x: u32, y: u32) -> Option<Color> {
        let offset = ((y * level.width + x) as usize) * 4;
        let rgba = level.rgba8.get(offset..offset + 4)?;
        let alpha = f32::from(rgba[3]) / 255.0;
        let mut color = match self.color_space {
            TextureColorSpace::Srgb => Color::from_srgb_u8(rgba[0], rgba[1], rgba[2]),
            TextureColorSpace::Linear => Color::from_linear_rgba(
                f32::from(rgba[0]) / 255.0,
                f32::from(rgba[1]) / 255.0,
                f32::from(rgba[2]) / 255.0,
                alpha,
            ),
        };
        color.a = alpha;
        Some(color)
    }
}

fn lerp_color(left: Color, right: Color, amount: f32) -> Color {
    Color::from_linear_rgba(
        left.r + (right.r - left.r) * amount,
        left.g + (right.g - left.g) * amount,
        left.b + (right.b - left.b) * amount,
        left.a + (right.a - left.a) * amount,
    )
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
    {
        if lower.ends_with(".ktx2") || lower.starts_with("data:image/ktx2") {
            return Ok(TextureSourceFormat::Ktx2Basisu);
        }
    }
    Err(AssetError::UnsupportedTextureFormat {
        path: path.as_str().to_string(),
        help: "supported texture format set is PNG, JPEG, and WebP; compressed texture decoders need an explicit feature/policy",
    })
}

fn wrap_texture_coordinate(value: f32, wrap: TextureWrap) -> f32 {
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
        TextureSourceFormat::Webp => Ok(None),
        TextureSourceFormat::Ktx2Basisu => {
            decode_ktx2_basisu_rgba8(path, &bytes, color_space).map(Some)
        }
    };
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    {
        log_texture_step(path, "decode pixels", decode_start);
        log_texture_step(path, "decode_texture_pixels total", total_start);
    }
    pixels
}

fn decode_png_rgba8(path: &AssetPath, bytes: &[u8]) -> Result<TexturePixels, AssetError> {
    decode_via_image_crate(path, bytes, image::ImageFormat::Png)
}

fn decode_jpeg_rgba8(path: &AssetPath, bytes: &[u8]) -> Result<TexturePixels, AssetError> {
    decode_via_image_crate(path, bytes, image::ImageFormat::Jpeg)
}

/// Stage B2: delegate PNG/JPEG decode to the `image` crate. `image` wraps
/// the same `png` and `jpeg-decoder` crates scena previously used directly,
/// but its unified `DynamicImage::into_rgba8` handles every color-type
/// expansion (RGB→RGBA, Grayscale→RGBA, Grayscale+Alpha→RGBA, 16-bit→8-bit,
/// CMYK→RGBA) without us re-implementing each path by hand. Removes the
/// hand-rolled `cmyk_to_rgba8` + 4 color-type match arms that previously
/// lived here.
fn decode_via_image_crate(
    path: &AssetPath,
    bytes: &[u8],
    format: image::ImageFormat,
) -> Result<TexturePixels, AssetError> {
    let image =
        image::load_from_memory_with_format(bytes, format).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid texture payload: {error}"),
        })?;
    let rgba = image.into_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    Ok(TexturePixels::single_level(width, height, rgba.into_raw()))
}
