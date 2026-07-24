use crate::material::{Color, TextureColorSpace};

use super::texture_format::wrap_texture_coordinate;
use super::{TextureDesc, TextureMipLevel};

impl TextureDesc {
    pub(crate) fn sample_bilinear(&self, uv: [f32; 2]) -> Option<Color> {
        let pixels = self.pixels.as_ref()?;
        if let Some((width, height, rgba16f_bits)) = pixels.linear_rgba16f() {
            let u = wrap_texture_coordinate(uv[0], self.sampler.wrap_s);
            let v = wrap_texture_coordinate(uv[1], self.sampler.wrap_t);
            let x = u * width.saturating_sub(1) as f32;
            let y = v * height.saturating_sub(1) as f32;
            let x0 = x.floor() as u32;
            let y0 = y.floor() as u32;
            let x1 = (x0 + 1).min(width.saturating_sub(1));
            let y1 = (y0 + 1).min(height.saturating_sub(1));
            let tx = x - x0 as f32;
            let ty = y - y0 as f32;
            let sample = |sample_x: u32, sample_y: u32| -> Option<Color> {
                let offset = ((sample_y * width + sample_x) as usize) * 4;
                let rgba = rgba16f_bits.get(offset..offset + 4)?;
                Some(Color::from_linear_rgba(
                    half::f16::from_bits(rgba[0]).to_f32(),
                    half::f16::from_bits(rgba[1]).to_f32(),
                    half::f16::from_bits(rgba[2]).to_f32(),
                    half::f16::from_bits(rgba[3]).to_f32(),
                ))
            };
            let c00 = sample(x0, y0)?;
            let c10 = sample(x1, y0)?;
            let c01 = sample(x0, y1)?;
            let c11 = sample(x1, y1)?;
            return Some(lerp_color(
                lerp_color(c00, c10, tx),
                lerp_color(c01, c11, tx),
                ty,
            ));
        }
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
