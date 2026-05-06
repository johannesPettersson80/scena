//! Material descriptors, texture slots, color space, alpha modes, and technical materials.

use std::error;
use std::fmt;

use crate::assets::TextureHandle;

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
    kind: MaterialKind,
    base_color: Color,
    base_color_texture: Option<TextureHandle>,
    normal_texture: Option<TextureHandle>,
    metallic_roughness_texture: Option<TextureHandle>,
    occlusion_texture: Option<TextureHandle>,
    emissive_texture: Option<TextureHandle>,
    alpha_mode: AlphaMode,
    emissive: Color,
    emissive_strength: f32,
    metallic_factor: f32,
    roughness_factor: f32,
    double_sided: bool,
}

impl MaterialDesc {
    pub const fn unlit(base_color: Color) -> Self {
        Self {
            kind: MaterialKind::Unlit,
            base_color,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            double_sided: false,
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
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            metallic_factor: clamp_unit_or(metallic_factor, 0.0),
            roughness_factor: clamp_unit_or(roughness_factor, 1.0),
            double_sided: false,
        }
    }

    pub const fn kind(&self) -> MaterialKind {
        self.kind
    }

    pub const fn base_color(&self) -> Color {
        self.base_color
    }

    pub const fn base_color_texture(&self) -> Option<TextureHandle> {
        self.base_color_texture
    }

    pub const fn normal_texture(&self) -> Option<TextureHandle> {
        self.normal_texture
    }

    pub const fn metallic_roughness_texture(&self) -> Option<TextureHandle> {
        self.metallic_roughness_texture
    }

    pub const fn occlusion_texture(&self) -> Option<TextureHandle> {
        self.occlusion_texture
    }

    pub const fn emissive_texture(&self) -> Option<TextureHandle> {
        self.emissive_texture
    }

    pub const fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    pub const fn emissive(&self) -> Color {
        self.emissive
    }

    pub const fn emissive_strength(&self) -> f32 {
        self.emissive_strength
    }

    pub const fn metallic_factor(&self) -> f32 {
        self.metallic_factor
    }

    pub const fn roughness_factor(&self) -> f32 {
        self.roughness_factor
    }

    pub const fn double_sided(&self) -> bool {
        self.double_sided
    }

    pub const fn with_base_color_texture(mut self, texture: TextureHandle) -> Self {
        self.base_color_texture = Some(texture);
        self
    }

    pub const fn with_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.normal_texture = Some(texture);
        self
    }

    pub const fn with_metallic_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.metallic_roughness_texture = Some(texture);
        self
    }

    pub const fn with_occlusion_texture(mut self, texture: TextureHandle) -> Self {
        self.occlusion_texture = Some(texture);
        self
    }

    pub const fn with_emissive_texture(mut self, texture: TextureHandle) -> Self {
        self.emissive_texture = Some(texture);
        self
    }

    pub const fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.alpha_mode = sanitize_alpha_mode(alpha_mode);
        self
    }

    pub const fn with_emissive(mut self, emissive: Color) -> Self {
        self.emissive = emissive;
        self
    }

    pub const fn with_emissive_strength(mut self, emissive_strength: f32) -> Self {
        self.emissive_strength = non_negative_or(emissive_strength, 1.0);
        self
    }

    pub const fn with_double_sided(mut self, double_sided: bool) -> Self {
        self.double_sided = double_sided;
        self
    }
}

impl Default for MaterialDesc {
    fn default() -> Self {
        Self::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0)
    }
}

const fn sanitize_alpha_mode(alpha_mode: AlphaMode) -> AlphaMode {
    match alpha_mode {
        AlphaMode::Opaque => AlphaMode::Opaque,
        AlphaMode::Mask { cutoff } => AlphaMode::Mask {
            cutoff: clamp_unit_or(cutoff, 0.5),
        },
        AlphaMode::Blend => AlphaMode::Blend,
    }
}

const fn clamp_unit_or(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

const fn non_negative_or(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}
