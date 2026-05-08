use std::io::Cursor;
use std::sync::Arc;

use base64::Engine;

use crate::diagnostics::AssetError;
use crate::material::{Color, TextureColorSpace};

use super::AssetPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDesc {
    path: AssetPath,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
    pixels: Option<Arc<TexturePixels>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TexturePixels {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

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
        let pixels = decode_texture_pixels(&path, source_format, source_bytes)?.map(Arc::new);
        Ok(Self {
            path,
            color_space,
            sampler,
            source_format,
            pixels,
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
        self.pixels.is_some()
    }

    pub fn decoded_dimensions(&self) -> Option<(u32, u32)> {
        self.pixels
            .as_ref()
            .map(|pixels| (pixels.width, pixels.height))
    }

    pub(crate) fn decoded_rgba8(&self) -> Option<(u32, u32, &[u8])> {
        self.pixels
            .as_ref()
            .map(|pixels| (pixels.width, pixels.height, pixels.rgba8.as_slice()))
    }

    pub(crate) fn decode_missing_pixels_from_bytes(
        &mut self,
        source_bytes: Option<&[u8]>,
    ) -> Result<(), AssetError> {
        if self.pixels.is_none() {
            self.pixels =
                decode_texture_pixels(&self.path, self.source_format, source_bytes)?.map(Arc::new);
        }
        Ok(())
    }

    pub(crate) fn sample_nearest(&self, uv: [f32; 2]) -> Option<Color> {
        let pixels = self.pixels.as_ref()?;
        let u = wrap_texture_coordinate(uv[0], self.sampler.wrap_s);
        let v = wrap_texture_coordinate(uv[1], self.sampler.wrap_t);
        let x = ((u * pixels.width as f32).floor() as u32).min(pixels.width.saturating_sub(1));
        let y = (((1.0 - v) * pixels.height as f32).floor() as u32)
            .min(pixels.height.saturating_sub(1));
        let offset = ((y * pixels.width + x) as usize) * 4;
        let rgba = pixels.rgba8.get(offset..offset + 4)?;
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
    source_format: TextureSourceFormat,
    source_bytes: Option<&[u8]>,
) -> Result<Option<TexturePixels>, AssetError> {
    let bytes = if let Some(bytes) = source_bytes {
        bytes.to_vec()
    } else if path.as_str().starts_with("data:") {
        decode_data_uri(path)?
    } else {
        return Ok(None);
    };
    match source_format {
        TextureSourceFormat::Png => decode_png_rgba8(path, &bytes).map(Some),
        TextureSourceFormat::Jpeg => decode_jpeg_rgba8(path, &bytes).map(Some),
        TextureSourceFormat::Webp | TextureSourceFormat::Ktx2Basisu => Ok(None),
    }
}

fn decode_data_uri(path: &AssetPath) -> Result<Vec<u8>, AssetError> {
    let Some((_, encoded)) = path.as_str().split_once(";base64,") else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "only base64 texture data URIs are supported for embedded texture decoding"
                .to_string(),
        });
    };
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid embedded texture base64: {error}"),
        })
}

fn decode_png_rgba8(path: &AssetPath, bytes: &[u8]) -> Result<TexturePixels, AssetError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("invalid PNG texture header: {error}"),
    })?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid PNG texture payload: {error}"),
        })?;
    let payload = &buffer[..info.buffer_size()];
    let rgba8 = match info.color_type {
        png::ColorType::Rgba => payload.to_vec(),
        png::ColorType::Rgb => payload
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => payload
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => payload
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Indexed => {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: "indexed PNG texture did not expand to RGB/RGBA".to_string(),
            });
        }
    };
    Ok(TexturePixels {
        width: info.width,
        height: info.height,
        rgba8,
    })
}

fn decode_jpeg_rgba8(path: &AssetPath, bytes: &[u8]) -> Result<TexturePixels, AssetError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let payload = decoder.decode().map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("invalid JPEG texture payload: {error}"),
    })?;
    let info = decoder.info().ok_or_else(|| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "invalid JPEG texture header: missing image info".to_string(),
    })?;
    let rgba8 = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => payload
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        jpeg_decoder::PixelFormat::L16 => payload
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], 255])
            .collect(),
        jpeg_decoder::PixelFormat::RGB24 => payload
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        jpeg_decoder::PixelFormat::CMYK32 => payload
            .chunks_exact(4)
            .flat_map(|pixel| cmyk_to_rgba8(pixel[0], pixel[1], pixel[2], pixel[3]))
            .collect(),
    };
    Ok(TexturePixels {
        width: u32::from(info.width),
        height: u32::from(info.height),
        rgba8,
    })
}

fn cmyk_to_rgba8(cyan: u8, magenta: u8, yellow: u8, black: u8) -> [u8; 4] {
    let c = u16::from(cyan);
    let m = u16::from(magenta);
    let y = u16::from(yellow);
    let k = u16::from(black);
    [
        255_u16.saturating_sub((c + k).min(255)) as u8,
        255_u16.saturating_sub((m + k).min(255)) as u8,
        255_u16.saturating_sub((y + k).min(255)) as u8,
        255,
    ]
}
