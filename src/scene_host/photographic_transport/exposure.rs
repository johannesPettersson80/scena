use super::Guide;
use crate::{CaptureRgba8, NodeKey, Vec3};

pub(super) struct ExposureMatch {
    pub(super) scale: f32,
    pub(super) target: f32,
    pub(super) measured: f32,
    pub(super) sample_count: usize,
}

pub(super) fn raster_background_linear(capture: &CaptureRgba8) -> Vec3 {
    let pixel = capture.rgba8.get(0..3).unwrap_or(&[46, 48, 52]);
    Vec3::new(
        srgb_to_linear(pixel[0]),
        srgb_to_linear(pixel[1]),
        srgb_to_linear(pixel[2]),
    )
}

pub(super) fn match_final_subject_exposure(
    linear: &[Vec3],
    guides: &[Guide],
    raster: &CaptureRgba8,
    exposure: f32,
    subject_nodes: Option<&[NodeKey]>,
) -> ExposureMatch {
    let background = raster.rgba8.get(0..3).unwrap_or(&[0, 0, 0]);
    let indices = subject_sample_indices(guides, raster, background, subject_nodes);
    if indices.is_empty() {
        return ExposureMatch {
            scale: 1.0,
            target: 0.0,
            measured: 0.0,
            sample_count: 0,
        };
    }
    let target = indices
        .iter()
        .filter_map(|index| raster.rgba8.get(index * 4..index * 4 + 3))
        .map(srgb_luminance)
        .sum::<f32>()
        / indices.len() as f32;
    let mut scale = 1.0_f32;
    for _ in 0..12 {
        let measured = measured_subject_luminance(linear, &indices, exposure, scale);
        if measured <= 0.5 {
            scale = (scale * 4.0).min(256.0);
            continue;
        }
        if (target - measured).abs() <= 0.35 {
            break;
        }
        let correction = (target / measured).max(0.01).log2().clamp(-2.0, 2.0);
        scale = (scale * 2.0_f32.powf(correction * 0.85)).clamp(0.015625, 256.0);
    }
    scale = scale.clamp(0.015625, 256.0);
    ExposureMatch {
        scale,
        target,
        measured: measured_subject_luminance(linear, &indices, exposure, scale),
        sample_count: indices.len(),
    }
}

fn subject_sample_indices(
    guides: &[Guide],
    raster: &CaptureRgba8,
    background: &[u8],
    subject_nodes: Option<&[NodeKey]>,
) -> Vec<usize> {
    guides
        .iter()
        .enumerate()
        .filter_map(|(index, guide)| {
            if !guide.hit
                || subject_nodes
                    .is_some_and(|nodes| guide.target.is_none_or(|target| !nodes.contains(&target)))
            {
                return None;
            }
            let pixel = raster.rgba8.get(index * 4..index * 4 + 3)?;
            let minimum_distance = if subject_nodes.is_some() { 2 } else { 6 };
            (max_rgb_distance(pixel, background) > minimum_distance).then_some(index)
        })
        .collect()
}

fn measured_subject_luminance(
    linear: &[Vec3],
    indices: &[usize],
    exposure: f32,
    scale: f32,
) -> f32 {
    let encoded_background = display_rgba8(linear[0] * exposure * scale);
    let mut measured_sum = 0.0;
    let mut measured_count = 0_u32;
    for index in indices {
        let encoded = display_rgba8(linear[*index] * exposure * scale);
        if max_rgb_distance(&encoded[..3], &encoded_background[..3]) <= 2 {
            continue;
        }
        measured_sum += srgb_luminance(&encoded[..3]);
        measured_count = measured_count.saturating_add(1);
    }
    if measured_count == 0 {
        0.0
    } else {
        measured_sum / measured_count as f32
    }
}

fn max_rgb_distance(left: &[u8], right: &[u8]) -> u8 {
    left.iter()
        .zip(right.iter())
        .take(3)
        .map(|(left, right)| left.abs_diff(*right))
        .max()
        .unwrap_or(0)
}

fn srgb_luminance(pixel: &[u8]) -> f32 {
    pixel[0] as f32 * 0.2126 + pixel[1] as f32 * 0.7152 + pixel[2] as f32 * 0.0722
}

pub(super) fn display_rgba8(linear: Vec3) -> [u8; 4] {
    let mapped = linear.max(Vec3::ZERO) / (Vec3::ONE + linear.max(Vec3::ZERO));
    [
        linear_to_srgb_u8(mapped.x),
        linear_to_srgb_u8(mapped.y),
        linear_to_srgb_u8(mapped.z),
        255,
    ]
}

fn srgb_to_linear(value: u8) -> f32 {
    let encoded = value as f32 / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_u8(value: f32) -> u8 {
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        value.powf(1.0 / 2.4).mul_add(1.055, -0.055)
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}
