use std::error;
use std::fmt;

use palette::Srgb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::from_linear_rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::from_linear_rgba(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::from_linear_rgba(1.0, 1.0, 1.0, 1.0);
    pub const GRAY: Self = Self::from_linear_rgba(0.215_860_5, 0.215_860_5, 0.215_860_5, 1.0);
    pub const LIGHT_GRAY: Self =
        Self::from_linear_rgba(0.693_871_74, 0.723_055_1, 0.768_151_16, 1.0);
    pub const DARK_GRAY: Self =
        Self::from_linear_rgba(0.029_556_835, 0.034_339_808, 0.043_735_03, 1.0);
    pub const CHARCOAL: Self =
        Self::from_linear_rgba(0.010_329_823, 0.012_286_488, 0.021_219_01, 1.0);
    pub const STUDIO_BACKDROP: Self =
        Self::from_linear_rgba(0.887_923_1, 0.896_269_4, 0.913_098_63, 1.0);
    pub const WARM_WHITE: Self = Self::from_linear_rgba(1.0, 0.904_661_2, 0.791_297_9, 1.0);
    pub const COOL_WHITE: Self = Self::from_linear_rgba(0.854_992_6, 0.921_581_86, 1.0, 1.0);
    pub const RED: Self = Self::from_linear_rgba(1.0, 0.043_735_03, 0.029_556_835, 1.0);
    pub const GREEN: Self = Self::from_linear_rgba(0.034_339_808, 0.571_124_85, 0.099_898_73, 1.0);
    pub const BLUE: Self = Self::from_linear_rgba(0.003_035_27, 0.230_740_06, 1.0, 1.0);
    pub const ORANGE: Self = Self::from_linear_rgba(1.0, 0.300_543_8, 0.0, 1.0);
    pub const YELLOW: Self = Self::from_linear_rgba(1.0, 0.603_827_36, 0.0, 1.0);
    pub const CYAN: Self = Self::from_linear_rgba(0.031_896_032, 0.417_885_07, 0.791_297_9, 1.0);
    pub const MAGENTA: Self = Self::from_linear_rgba(0.520_995_56, 0.102_241_73, 0.887_923_1, 1.0);

    pub const fn from_linear_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_linear_rgba(r, g, b, 1.0)
    }

    pub const fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_srgb(r: f32, g: f32, b: f32) -> Self {
        let linear =
            Srgb::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)).into_linear();
        Self::from_linear_rgb(linear.red, linear.green, linear.blue)
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
        parse_hex_srgb_value(value)
    }

    /// Parses a six-digit sRGB hex color, with or without a leading `#`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::Color;
    ///
    /// let blue = Color::from_hex("#0a84ff")?;
    /// assert!((blue.g - Color::BLUE.g).abs() < 1.0e-5);
    ///
    /// let charcoal = Color::from_hex("1a1d28")?;
    /// assert!((charcoal.r - Color::CHARCOAL.r).abs() < 1.0e-5);
    /// # Ok::<(), scena::ColorParseError>(())
    /// ```
    pub fn from_hex(hex: &str) -> Result<Self, ColorParseError> {
        let value = hex.strip_prefix('#').unwrap_or(hex);
        if value.len() != 6 {
            return Err(ColorParseError::InvalidHexSrgb);
        }
        parse_hex_srgb_value(value)
    }

    /// Approximates a warm-to-neutral white point from color temperature.
    ///
    /// Inputs are clamped to the intended first-path lighting range of
    /// 2700K-6500K; non-finite values fall back to 6500K.
    pub fn from_kelvin(kelvin: f32) -> Self {
        let kelvin = if kelvin.is_finite() { kelvin } else { 6500.0 };
        let temperature = kelvin.clamp(2700.0, 6500.0) / 100.0;
        let red = if temperature <= 66.0 {
            255.0
        } else {
            329.698_73 * (temperature - 60.0).powf(-0.133_204_76)
        };
        let green = if temperature <= 66.0 {
            99.470_8 * temperature.ln() - 161.119_57
        } else {
            288.122_16 * (temperature - 60.0).powf(-0.075_514_846)
        };
        let blue = if temperature >= 66.0 {
            255.0
        } else if temperature <= 19.0 {
            0.0
        } else {
            138.517_73 * (temperature - 10.0).ln() - 305.044_8
        };
        Self::from_srgb(
            (red / 255.0).clamp(0.0, 1.0),
            (green / 255.0).clamp(0.0, 1.0),
            (blue / 255.0).clamp(0.0, 1.0),
        )
    }
}

fn parse_hex_srgb_value(value: &str) -> Result<Color, ColorParseError> {
    let r = parse_hex_channel(&value[0..2])?;
    let g = parse_hex_channel(&value[2..4])?;
    let b = parse_hex_channel(&value[4..6])?;
    Ok(Color::from_srgb_u8(r, g, b))
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
