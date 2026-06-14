use crate::capture::CapturePixelBounds;

use super::types::AppearanceRectV1;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ContentSample {
    pub(super) sampled_pixels: u64,
    pub(super) bbox: Option<CapturePixelBounds>,
    pub(super) average_rgba8: [u8; 4],
    pub(super) color_family: String,
    pub(super) luminance_mean: f32,
}

impl ContentSample {
    pub(super) fn from_rgba8(
        width: u32,
        height: u32,
        rgba8: &[u8],
        background: [u8; 4],
        tolerance: u8,
    ) -> Self {
        let mut sampled_pixels = 0_u64;
        let mut rgb_sum = [0_u64; 3];
        let mut alpha_sum = 0_u64;
        let mut luma_sum = 0_f32;
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0_u32;
        let mut max_y = 0_u32;

        for y in 0..height {
            for x in 0..width {
                let offset = ((y as usize) * (width as usize) + (x as usize)) * 4;
                let Some(pixel) = rgba8.get(offset..offset + 4) else {
                    continue;
                };
                if !differs_from_background(pixel, background, tolerance) {
                    continue;
                }
                sampled_pixels = sampled_pixels.saturating_add(1);
                rgb_sum[0] = rgb_sum[0].saturating_add(u64::from(pixel[0]));
                rgb_sum[1] = rgb_sum[1].saturating_add(u64::from(pixel[1]));
                rgb_sum[2] = rgb_sum[2].saturating_add(u64::from(pixel[2]));
                alpha_sum = alpha_sum.saturating_add(u64::from(pixel[3]));
                luma_sum += luma_from_srgb8(pixel);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }

        let average_rgba8 = if sampled_pixels == 0 {
            [0, 0, 0, 0]
        } else {
            [
                average_channel(rgb_sum[0], sampled_pixels),
                average_channel(rgb_sum[1], sampled_pixels),
                average_channel(rgb_sum[2], sampled_pixels),
                average_channel(alpha_sum, sampled_pixels),
            ]
        };
        let bbox = (sampled_pixels > 0).then_some(CapturePixelBounds {
            min_x,
            min_y,
            max_x,
            max_y,
            width: max_x.saturating_sub(min_x).saturating_add(1),
            height: max_y.saturating_sub(min_y).saturating_add(1),
        });

        Self {
            sampled_pixels,
            bbox,
            average_rgba8,
            color_family: color_family(average_rgba8),
            luminance_mean: if sampled_pixels == 0 {
                0.0
            } else {
                luma_sum / sampled_pixels as f32
            },
        }
    }
}

impl From<CapturePixelBounds> for AppearanceRectV1 {
    fn from(bounds: CapturePixelBounds) -> Self {
        Self {
            min_x: bounds.min_x as f32,
            min_y: bounds.min_y as f32,
            max_x: bounds.max_x as f32,
            max_y: bounds.max_y as f32,
            width: bounds.width as f32,
            height: bounds.height as f32,
        }
    }
}

pub(super) fn swatch_distance(sample: [u8; 4], swatch: [u8; 3]) -> f32 {
    let dr = f32::from(sample[0]) - f32::from(swatch[0]);
    let dg = f32::from(sample[1]) - f32::from(swatch[1]);
    let db = f32::from(sample[2]) - f32::from(swatch[2]);
    round3(((dr * dr + dg * dg + db * db).sqrt()) / (255.0 * 3.0_f32.sqrt()))
}

pub(super) fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}

fn differs_from_background(pixel: &[u8], background: [u8; 4], tolerance: u8) -> bool {
    pixel[0].abs_diff(background[0]) > tolerance
        || pixel[1].abs_diff(background[1]) > tolerance
        || pixel[2].abs_diff(background[2]) > tolerance
}

fn average_channel(sum: u64, count: u64) -> u8 {
    ((sum as f32 / count as f32).round()).clamp(0.0, 255.0) as u8
}

fn color_family(rgba: [u8; 4]) -> String {
    let [r, g, b, a] = rgba;
    if a < 8 {
        return "transparent".to_owned();
    }
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max < 16 {
        return "black".to_owned();
    }
    if max.saturating_sub(min) < 18 {
        return if max > 220 { "white" } else { "gray" }.to_owned();
    }
    let margin = 18;
    if r > 180 && g > 160 && b < 96 {
        return "yellow".to_owned();
    }
    if r > 160 && b > 160 && g < 128 {
        return "magenta".to_owned();
    }
    if g > 160 && b > 160 && r < 128 {
        return "cyan".to_owned();
    }
    if r >= g.saturating_add(margin) && r >= b.saturating_add(margin) {
        return "red".to_owned();
    }
    if g >= r.saturating_add(margin) && g >= b.saturating_add(margin) {
        return "green".to_owned();
    }
    if b >= r.saturating_add(margin) && b >= g.saturating_add(margin) {
        return "blue".to_owned();
    }
    "mixed".to_owned()
}

fn luma_from_srgb8(pixel: &[u8]) -> f32 {
    round3(
        (0.2126 * f32::from(pixel[0])
            + 0.7152 * f32::from(pixel[1])
            + 0.0722 * f32::from(pixel[2]))
            / 255.0,
    )
}
