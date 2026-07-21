use crate::material::{Color, srgb_u8_to_linear};

use super::{RasterTarget, ReconstructionFilter};

#[allow(clippy::too_many_arguments)]
pub(super) fn downsample_cpu_supersample(
    source_target: RasterTarget,
    scale: u32,
    source_linear: &[Color],
    source_depth: &[f32],
    source_frame: &[u8],
    target: RasterTarget,
    target_linear: &mut [Color],
    target_depth: &mut [f32],
    target_frame: &mut Vec<u8>,
    reconstruction_filter: ReconstructionFilter,
) {
    debug_assert_eq!(source_target.width, target.width.saturating_mul(scale));
    debug_assert_eq!(source_target.height, target.height.saturating_mul(scale));
    let sample_count = scale.saturating_mul(scale).max(1) as f32;
    for y in 0..target.height {
        for x in 0..target.width {
            let target_index = target.pixel_index(x, y);
            let mut linear = Color::TRANSPARENT;
            let mut depth = f32::INFINITY;
            for sy in 0..scale {
                for sx in 0..scale {
                    let source_x = x.saturating_mul(scale).saturating_add(sx);
                    let source_y = y.saturating_mul(scale).saturating_add(sy);
                    let source_index = source_target.pixel_index(source_x, source_y);
                    let source_color = source_linear[source_index];
                    linear.r += source_color.r;
                    linear.g += source_color.g;
                    linear.b += source_color.b;
                    linear.a += source_color.a;
                    depth = depth.min(source_depth[source_index]);
                }
            }
            linear.r /= sample_count;
            linear.g /= sample_count;
            linear.b /= sample_count;
            linear.a /= sample_count;
            target_linear[target_index] = linear;
            target_depth[target_index] = depth;
        }
    }
    downsample_rgba8_reconstruction_filter(
        source_target,
        scale,
        source_frame,
        target,
        target_frame,
        reconstruction_filter,
    );
}

#[cfg(test)]
pub(super) fn downsample_rgba8_box_filter(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target: RasterTarget,
    target_frame: &mut Vec<u8>,
) {
    downsample_rgba8_reconstruction_filter(
        source_target,
        scale,
        source_frame,
        target,
        target_frame,
        ReconstructionFilter::Box,
    );
}

pub(super) fn downsample_rgba8_reconstruction_filter(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target: RasterTarget,
    target_frame: &mut Vec<u8>,
    reconstruction_filter: ReconstructionFilter,
) {
    debug_assert_eq!(source_target.width, target.width.saturating_mul(scale));
    debug_assert_eq!(source_target.height, target.height.saturating_mul(scale));
    target_frame.resize(target.byte_len(), 0);
    for y in 0..target.height {
        for x in 0..target.width {
            let target_offset = target.pixel_index(x, y).saturating_mul(4);
            target_frame[target_offset..target_offset + 4].copy_from_slice(&sample_rgba8_kernel(
                source_target,
                scale,
                source_frame,
                x,
                y,
                reconstruction_filter,
            ));
        }
    }
}

fn sample_rgba8_kernel(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target_x: u32,
    target_y: u32,
    reconstruction_filter: ReconstructionFilter,
) -> [u8; 4] {
    match reconstruction_filter {
        ReconstructionFilter::Box => {
            sample_rgba8_box(source_target, scale, source_frame, target_x, target_y)
        }
        ReconstructionFilter::Tent | ReconstructionFilter::Gaussian => sample_rgba8_weighted(
            source_target,
            scale,
            source_frame,
            target_x,
            target_y,
            reconstruction_filter,
        ),
    }
}

fn sample_rgba8_box(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target_x: u32,
    target_y: u32,
) -> [u8; 4] {
    let mut linear = [0.0_f32; 3];
    let mut alpha = 0.0_f32;
    let mut weight_sum = 0.0_f32;
    for sy in 0..scale {
        for sx in 0..scale {
            let source_x = target_x.saturating_mul(scale).saturating_add(sx);
            let source_y = target_y.saturating_mul(scale).saturating_add(sy);
            accumulate_rgba8_sample(
                source_target,
                source_frame,
                source_x,
                source_y,
                1.0,
                &mut linear,
                &mut alpha,
                &mut weight_sum,
            );
        }
    }
    encode_linear_average(linear, alpha, weight_sum)
}

fn sample_rgba8_weighted(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target_x: u32,
    target_y: u32,
    reconstruction_filter: ReconstructionFilter,
) -> [u8; 4] {
    let scale_f = scale.max(1) as f32;
    let center_x = (target_x as f32 + 0.5) * scale_f;
    let center_y = (target_y as f32 + 0.5) * scale_f;
    let radius = match reconstruction_filter {
        ReconstructionFilter::Tent => scale_f,
        ReconstructionFilter::Gaussian => scale_f,
        ReconstructionFilter::Box => scale_f * 0.5,
    };
    let min_x = ((center_x - radius).floor() as i64).max(0) as u32;
    let max_x = ((center_x + radius).ceil() as u32).min(source_target.width.saturating_sub(1));
    let min_y = ((center_y - radius).floor() as i64).max(0) as u32;
    let max_y = ((center_y + radius).ceil() as u32).min(source_target.height.saturating_sub(1));
    let mut linear = [0.0_f32; 3];
    let mut alpha = 0.0_f32;
    let mut weight_sum = 0.0_f32;
    for sy in min_y..=max_y {
        let sample_y = sy as f32 + 0.5;
        let wy = reconstruction_weight((sample_y - center_y) / scale_f, reconstruction_filter);
        if wy <= 0.0 {
            continue;
        }
        for sx in min_x..=max_x {
            let sample_x = sx as f32 + 0.5;
            let wx = reconstruction_weight((sample_x - center_x) / scale_f, reconstruction_filter);
            let weight = wx * wy;
            if weight <= 0.0 {
                continue;
            }
            accumulate_rgba8_sample(
                source_target,
                source_frame,
                sx,
                sy,
                weight,
                &mut linear,
                &mut alpha,
                &mut weight_sum,
            );
        }
    }
    encode_linear_average(linear, alpha, weight_sum)
}

fn reconstruction_weight(
    distance_in_output_pixels: f32,
    reconstruction_filter: ReconstructionFilter,
) -> f32 {
    match reconstruction_filter {
        ReconstructionFilter::Box => {
            if distance_in_output_pixels.abs() <= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ReconstructionFilter::Tent => (1.0 - distance_in_output_pixels.abs()).max(0.0),
        ReconstructionFilter::Gaussian => {
            let sigma = 0.42_f32;
            (-0.5 * (distance_in_output_pixels / sigma).powi(2)).exp()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn accumulate_rgba8_sample(
    source_target: RasterTarget,
    source_frame: &[u8],
    source_x: u32,
    source_y: u32,
    weight: f32,
    linear: &mut [f32; 3],
    alpha: &mut f32,
    weight_sum: &mut f32,
) {
    let source_offset = source_target
        .pixel_index(source_x, source_y)
        .saturating_mul(4);
    linear[0] += srgb_u8_to_linear(source_frame[source_offset]) * weight;
    linear[1] += srgb_u8_to_linear(source_frame[source_offset + 1]) * weight;
    linear[2] += srgb_u8_to_linear(source_frame[source_offset + 2]) * weight;
    *alpha += (f32::from(source_frame[source_offset + 3]) / 255.0) * weight;
    *weight_sum += weight;
}

fn encode_linear_average(linear: [f32; 3], alpha: f32, weight_sum: f32) -> [u8; 4] {
    if weight_sum <= f32::EPSILON {
        return [0, 0, 0, 0];
    }
    [
        linear_to_srgb8(linear[0] / weight_sum),
        linear_to_srgb8(linear[1] / weight_sum),
        linear_to_srgb8(linear[2] / weight_sum),
        ((alpha / weight_sum) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

#[cfg(test)]
fn srgb8_to_linear_formula(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    #[test]
    fn srgb8_lookup_is_bit_identical_to_the_transfer_formula() {
        for value in u8::MIN..=u8::MAX {
            assert_eq!(
                srgb_u8_to_linear(value).to_bits(),
                super::srgb8_to_linear_formula(value).to_bits(),
                "lookup entry {value} drifted from the transfer formula"
            );
        }
    }

    use super::*;
    use crate::diagnostics::Backend;

    #[test]
    fn rgba8_supersample_downsample_averages_rgb_in_linear_light() {
        let source_target = RasterTarget {
            width: 2,
            height: 2,
            backend: Backend::HeadlessGpu,
        };
        let target = RasterTarget {
            width: 1,
            height: 1,
            backend: Backend::HeadlessGpu,
        };
        let source_frame = [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let mut target_frame = Vec::new();

        downsample_rgba8_box_filter(source_target, 2, &source_frame, target, &mut target_frame);

        assert!(
            (185..=190).contains(&target_frame[0])
                && (185..=190).contains(&target_frame[1])
                && (185..=190).contains(&target_frame[2]),
            "linear-light average of 50% black + 50% white should encode near 188, got {:?}",
            &target_frame[..4]
        );
        assert_eq!(target_frame[3], 255);
    }
}
