//! Material descriptors, texture slots, color space, alpha modes, and technical materials.

use std::error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self::from_linear_rgba(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::from_linear_rgba(1.0, 1.0, 1.0, 1.0);

    pub const fn from_linear_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_linear_rgba(r, g, b, 1.0)
    }

    pub const fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_srgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_linear_rgb(
            srgb_channel_to_linear(r),
            srgb_channel_to_linear(g),
            srgb_channel_to_linear(b),
        )
    }

    pub fn from_srgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::from_srgb(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        )
    }

    pub fn from_hex_srgb(hex: &str) -> Result<Self, ColorParseError> {
        let value = hex
            .strip_prefix('#')
            .filter(|value| value.len() == 6)
            .ok_or(ColorParseError::InvalidHexSrgb)?;
        let r = parse_hex_channel(&value[0..2])?;
        let g = parse_hex_channel(&value[2..4])?;
        let b = parse_hex_channel(&value[4..6])?;
        Ok(Self::from_srgb_u8(r, g, b))
    }

    pub(crate) fn to_rgba8(self) -> [u8; 4] {
        [
            linear_channel_to_u8(self.r),
            linear_channel_to_u8(self.g),
            linear_channel_to_u8(self.b),
            linear_channel_to_u8(self.a),
        ]
    }
}

fn linear_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn srgb_channel_to_linear(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn parse_hex_channel(value: &str) -> Result<u8, ColorParseError> {
    u8::from_str_radix(value, 16).map_err(|_| ColorParseError::InvalidHexSrgb)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorParseError {
    InvalidHexSrgb,
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHexSrgb => write!(formatter, "expected # followed by six hex digits"),
        }
    }
}

impl error::Error for ColorParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureColorSpace {
    Linear,
    Srgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialKind {
    Unlit,
    PbrMetallicRoughness,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialDesc {
    pub kind: MaterialKind,
    pub base_color: Color,
    pub alpha_mode: AlphaMode,
    pub emissive: Color,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
}

impl MaterialDesc {
    pub const fn unlit(base_color: Color) -> Self {
        Self {
            kind: MaterialKind::Unlit,
            base_color,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            metallic_factor: 0.0,
            roughness_factor: 1.0,
        }
    }

    pub const fn pbr_metallic_roughness(
        base_color: Color,
        metallic_factor: f32,
        roughness_factor: f32,
    ) -> Self {
        Self {
            kind: MaterialKind::PbrMetallicRoughness,
            base_color,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            metallic_factor,
            roughness_factor,
        }
    }

    pub const fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.alpha_mode = alpha_mode;
        self
    }

    pub const fn with_emissive(mut self, emissive: Color) -> Self {
        self.emissive = emissive;
        self
    }
}

impl Default for MaterialDesc {
    fn default() -> Self {
        Self::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0)
    }
}
