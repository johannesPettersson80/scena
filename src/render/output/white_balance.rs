use super::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhiteBalance {
    illuminant_kelvin: f32,
    tint: f32,
    linear_multipliers: [f32; 3],
}

impl WhiteBalance {
    pub const fn neutral() -> Self {
        Self {
            illuminant_kelvin: 6_500.0,
            tint: 0.0,
            linear_multipliers: [1.0, 1.0, 1.0],
        }
    }

    pub fn from_illuminant_kelvin(illuminant_kelvin: f32) -> Self {
        Self::from_illuminant_kelvin_with_tint(illuminant_kelvin, 0.0)
    }

    pub fn from_illuminant_kelvin_with_tint(illuminant_kelvin: f32, tint: f32) -> Self {
        let illuminant_kelvin = valid_kelvin(illuminant_kelvin);
        let tint = valid_tint(tint);
        let reference = Color::from_kelvin(6_500.0);
        let illuminant = Color::from_kelvin(illuminant_kelvin);
        Self::with_multipliers(
            illuminant_kelvin,
            tint,
            [
                safe_channel_ratio(reference.r, illuminant.r),
                safe_channel_ratio(reference.g, illuminant.g),
                safe_channel_ratio(reference.b, illuminant.b),
            ],
        )
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn from_linear_illuminant_rgb(
        illuminant_kelvin: f32,
        tint: f32,
        illuminant_rgb: [f32; 3],
    ) -> Self {
        Self::with_multipliers(
            valid_kelvin(illuminant_kelvin),
            valid_tint(tint),
            illuminant_rgb.map(|channel| {
                if channel.is_finite() {
                    channel.max(1.0e-4).recip()
                } else {
                    1.0
                }
            }),
        )
    }

    fn with_multipliers(illuminant_kelvin: f32, tint: f32, mut multipliers: [f32; 3]) -> Self {
        multipliers[1] *= 2.0_f32.powf(-tint * 0.25);
        let normalization = multipliers[1].max(1.0e-4);
        for channel in &mut multipliers {
            *channel = (*channel / normalization).clamp(0.25, 4.0);
        }
        Self {
            illuminant_kelvin,
            tint,
            linear_multipliers: multipliers,
        }
    }

    pub const fn illuminant_kelvin(self) -> f32 {
        self.illuminant_kelvin
    }

    pub const fn tint(self) -> f32 {
        self.tint
    }

    pub const fn linear_multipliers(self) -> [f32; 3] {
        self.linear_multipliers
    }

    pub(super) fn apply(self, color: Color) -> Color {
        Color::from_linear_rgba(
            color.r * self.linear_multipliers[0],
            color.g * self.linear_multipliers[1],
            color.b * self.linear_multipliers[2],
            color.a,
        )
    }
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self::neutral()
    }
}

fn valid_kelvin(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(1_000.0, 20_000.0)
    } else {
        6_500.0
    }
}

fn valid_tint(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn safe_channel_ratio(reference: f32, illuminant: f32) -> f32 {
    if reference.is_finite() && illuminant.is_finite() && illuminant > 1.0e-4 {
        reference / illuminant
    } else {
        1.0
    }
}
