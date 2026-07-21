use std::error;
use std::fmt;
use std::sync::OnceLock;

use palette::Srgb;
use serde::{Deserialize, Serialize};

/// Linear RGBA color in scene space.
///
/// The four channels (`r`, `g`, `b`, `a`) are stored in linear light, not sRGB.
/// Use [`Color::from_srgb`], [`Color::from_srgb_u8`], or [`Color::from_hex`]
/// to convert from sRGB inputs. The named constants
/// ([`Color::WHITE`], [`Color::CHARCOAL`], etc.) are first-path defaults for
/// scene authoring; raw `from_linear_rgb`/`from_srgb` constructors remain
/// available as escape hatches when an exact value is needed.
///
/// # Examples
///
/// ```
/// use scena::Color;
///
/// // Named constants for first-path scene authoring.
/// let background = Color::CHARCOAL;
///
/// // Designer-friendly hex parsing.
/// let accent = Color::from_hex("#0a84ff")?;
///
/// // Light color temperature in Kelvin.
/// let bulb = Color::from_kelvin(3200.0);
///
/// // Raw escape hatches.
/// let exact = Color::from_linear_rgb(0.1, 0.2, 0.3);
/// let designer = Color::from_srgb_u8(64, 160, 255);
/// # let _ = (background, accent, bulb, exact, designer);
/// # Ok::<(), scena::ColorParseError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Fully transparent (alpha = 0).
    pub const TRANSPARENT: Self = Self::from_linear_rgba(0.0, 0.0, 0.0, 0.0);
    /// Pure opaque black.
    pub const BLACK: Self = Self::from_linear_rgba(0.0, 0.0, 0.0, 1.0);
    /// Pure opaque white.
    pub const WHITE: Self = Self::from_linear_rgba(1.0, 1.0, 1.0, 1.0);
    /// Neutral mid-gray (sRGB ≈ #808080).
    pub const GRAY: Self = Self::from_linear_rgba(0.215_860_5, 0.215_860_5, 0.215_860_5, 1.0);
    /// Light neutral gray (sRGB ≈ #d8dce2).
    pub const LIGHT_GRAY: Self =
        Self::from_linear_rgba(0.693_871_74, 0.723_055_1, 0.768_151_16, 1.0);
    /// Dark neutral gray (sRGB ≈ #2d3038).
    pub const DARK_GRAY: Self =
        Self::from_linear_rgba(0.029_556_835, 0.034_339_808, 0.043_735_03, 1.0);
    /// Deep charcoal background (sRGB ≈ #1a1d28).
    pub const CHARCOAL: Self =
        Self::from_linear_rgba(0.010_329_823, 0.012_286_488, 0.021_219_01, 1.0);
    /// Studio-backdrop near-white for product photography (sRGB ≈ #f2f4f7).
    pub const STUDIO_BACKDROP: Self =
        Self::from_linear_rgba(0.887_923_1, 0.896_269_4, 0.913_098_63, 1.0);
    /// Warm white close to a 3200K tungsten bulb.
    pub const WARM_WHITE: Self = Self::from_linear_rgba(1.0, 0.904_661_2, 0.791_297_9, 1.0);
    /// Cool white close to a 5600K daylight bulb.
    pub const COOL_WHITE: Self = Self::from_linear_rgba(0.854_992_6, 0.921_581_86, 1.0, 1.0);
    /// Saturated red.
    pub const RED: Self = Self::from_linear_rgba(1.0, 0.043_735_03, 0.029_556_835, 1.0);
    /// Saturated green.
    pub const GREEN: Self = Self::from_linear_rgba(0.034_339_808, 0.571_124_85, 0.099_898_73, 1.0);
    /// Saturated blue.
    pub const BLUE: Self = Self::from_linear_rgba(0.003_035_27, 0.230_740_06, 1.0, 1.0);
    /// Saturated orange.
    pub const ORANGE: Self = Self::from_linear_rgba(1.0, 0.300_543_8, 0.0, 1.0);
    /// Saturated yellow.
    pub const YELLOW: Self = Self::from_linear_rgba(1.0, 0.603_827_36, 0.0, 1.0);
    /// Saturated cyan.
    pub const CYAN: Self = Self::from_linear_rgba(0.031_896_032, 0.417_885_07, 0.791_297_9, 1.0);
    /// Saturated magenta.
    pub const MAGENTA: Self = Self::from_linear_rgba(0.520_995_56, 0.102_241_73, 0.887_923_1, 1.0);

    pub const NAMED_CONSTANTS: &'static [(&'static str, Self)] = &[
        ("transparent", Self::TRANSPARENT),
        ("black", Self::BLACK),
        ("white", Self::WHITE),
        ("gray", Self::GRAY),
        ("light_gray", Self::LIGHT_GRAY),
        ("dark_gray", Self::DARK_GRAY),
        ("charcoal", Self::CHARCOAL),
        ("studio_backdrop", Self::STUDIO_BACKDROP),
        ("warm_white", Self::WARM_WHITE),
        ("cool_white", Self::COOL_WHITE),
        ("red", Self::RED),
        ("green", Self::GREEN),
        ("blue", Self::BLUE),
        ("orange", Self::ORANGE),
        ("yellow", Self::YELLOW),
        ("cyan", Self::CYAN),
        ("magenta", Self::MAGENTA),
    ];

    pub fn from_named_constant(name: &str) -> Option<Self> {
        Self::NAMED_CONSTANTS
            .iter()
            .find_map(|(candidate, color)| (*candidate == name).then_some(*color))
    }

    /// Constructs a fully opaque color from linear RGB channels in `[0.0, 1.0]`.
    ///
    /// Inputs are stored as-is; no sRGB conversion is performed. Reach for
    /// [`Color::from_srgb`] or [`Color::from_hex`] when working from sRGB.
    pub const fn from_linear_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_linear_rgba(r, g, b, 1.0)
    }

    /// Constructs a color from linear RGBA channels in `[0.0, 1.0]`.
    ///
    /// Inputs are stored as-is; no sRGB conversion is performed.
    pub const fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs an opaque color from sRGB float channels in `[0.0, 1.0]`.
    ///
    /// The channels are converted to linear space using the standard sRGB
    /// transfer function before being stored.
    pub fn from_srgb(r: f32, g: f32, b: f32) -> Self {
        let linear =
            Srgb::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)).into_linear();
        Self::from_linear_rgb(linear.red, linear.green, linear.blue)
    }

    /// Constructs an opaque color from 8-bit sRGB channels (`0..=255`).
    ///
    /// Convenient for porting designer-supplied byte triples. The channels are
    /// converted to linear space before being stored.
    pub fn from_srgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::from_linear_rgb(
            srgb_u8_to_linear(r),
            srgb_u8_to_linear(g),
            srgb_u8_to_linear(b),
        )
    }

    /// Parses a six-digit sRGB hex color, requiring a leading `#`.
    ///
    /// Prefer the alias [`Color::from_hex`] for new code; it accepts hex
    /// strings with or without the `#` prefix.
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

pub(crate) fn srgb_u8_to_linear(channel: u8) -> f32 {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        std::array::from_fn(|index| {
            let srgb = index as f32 / 255.0;
            if srgb <= 0.04045 {
                srgb / 12.92
            } else {
                ((srgb + 0.055) / 1.055).powf(2.4)
            }
        })
    })[usize::from(channel)]
}

fn parse_hex_srgb_value(value: &str) -> Result<Color, ColorParseError> {
    let bytes = value.as_bytes();
    if bytes.len() != 6 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return Err(ColorParseError::InvalidHexSrgb);
    }
    let r = parse_hex_channel(bytes[0], bytes[1])?;
    let g = parse_hex_channel(bytes[2], bytes[3])?;
    let b = parse_hex_channel(bytes[4], bytes[5])?;
    Ok(Color::from_srgb_u8(r, g, b))
}

fn parse_hex_channel(high: u8, low: u8) -> Result<u8, ColorParseError> {
    let nibble = |byte: u8| match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ColorParseError::InvalidHexSrgb),
    };
    Ok((nibble(high)? << 4) | nibble(low)?)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pf04_pf05_contract_srgb_u8_lut_is_bit_exact_for_every_byte() {
        for channel in u8::MIN..=u8::MAX {
            let srgb = f32::from(channel) / 255.0;
            let expected = if srgb <= 0.04045 {
                srgb / 12.92
            } else {
                ((srgb + 0.055) / 1.055).powf(2.4)
            };
            assert_eq!(
                srgb_u8_to_linear(channel).to_bits(),
                expected.to_bits(),
                "sRGB byte {channel} must retain exact transfer behavior"
            );
        }
    }
}
