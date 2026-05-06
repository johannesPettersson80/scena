//! Material descriptors, texture slots, color space, alpha modes, and technical materials.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self::from_linear_rgba(0.0, 0.0, 0.0, 1.0);

    pub const fn from_linear_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_linear_rgba(r, g, b, 1.0)
    }

    pub const fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
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
