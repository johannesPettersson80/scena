use crate::material::{Color, srgb_u8_to_linear};

use super::cpu::CpuFrame;

pub(super) fn write_label_overlay_pixel(
    cpu_frame: &mut CpuFrame<'_>,
    x: u32,
    y: u32,
    color: Color,
    coverage: f32,
    depth: f32,
) {
    if !depth.is_finite() {
        return;
    }
    let source_alpha = clamp_alpha_or(color.a, 1.0) * coverage.clamp(0.0, 1.0);
    if source_alpha <= 0.0 {
        return;
    }
    let Some(pixel_index) = cpu_frame.local_pixel_index(x, y) else {
        return;
    };
    if depth > cpu_frame.depth_frame[pixel_index] + f32::EPSILON {
        return;
    }

    let byte_index = pixel_index * 4;
    // label_overlay_display_linear: overlays are display-referred UI and are
    // composited after the scene's exposure, tonemap, encoding, and FXAA,
    // matching the GPU overlay pass order.
    blend_display_source_over(
        [color.r, color.g, color.b],
        source_alpha,
        &mut cpu_frame.frame[byte_index..byte_index + 4],
    );
}

fn blend_display_source_over(source_rgb: [f32; 3], source_alpha: f32, destination: &mut [u8]) {
    let alpha = source_alpha.clamp(0.0, 1.0);
    let inverse = 1.0 - alpha;
    for channel in 0..3 {
        let blended = source_rgb[channel].clamp(0.0, 1.0) * alpha
            + srgb_u8_to_linear(destination[channel]) * inverse;
        destination[channel] = linear_channel_to_srgb8(blended);
    }
    destination[3] = 255;
}

fn linear_channel_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

fn clamp_alpha_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}
