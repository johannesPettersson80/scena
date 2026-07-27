use super::*;

#[allow(dead_code)]
pub(in crate::render) fn apply_screen_space_ambient_occlusion_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    scratch: &mut [u8],
    depth_frame: &[f32],
    config: ScreenSpaceAmbientOcclusionConfig,
) -> u64 {
    let radius = u32::from(config.radius_px());
    let intensity = config.intensity().clamp(0.0, 1.0);
    if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
        return 0;
    }
    scratch.fill(0);
    let threshold = config.depth_threshold().max(0.0);
    for y in 0..target.height {
        for x in 0..target.width {
            let index = target.pixel_index(x, y);
            let center = quantize_screen_space_depth(depth_frame[index]);
            if !center.is_finite() {
                continue;
            }
            let near = (radius / 2).max(1);
            let offsets = [
                (-(near as i32), 0),
                (near as i32, 0),
                (0, -(near as i32)),
                (0, near as i32),
                (-(radius as i32), 0),
                (radius as i32, 0),
                (0, -(radius as i32)),
                (0, radius as i32),
            ];
            let mut finite = 0_u32;
            let mut occluders = 0_u32;
            for (dx, dy) in offsets {
                let sx = x as i32 + dx;
                let sy = y as i32 + dy;
                if sx < 0 || sy < 0 || sx >= target.width as i32 || sy >= target.height as i32 {
                    continue;
                }
                let sample = quantize_screen_space_depth(
                    depth_frame[target.pixel_index(sx as u32, sy as u32)],
                );
                if sample.is_finite() {
                    finite += 1;
                    occluders += u32::from(sample + threshold < center);
                }
            }
            if finite > 0 {
                let darkening = (occluders as f32 / finite as f32 * intensity).clamp(0.0, 0.65);
                scratch[index] = (darkening * 255.0).round() as u8;
            }
        }
    }
    for y in 0..target.height {
        for x in 0..target.width {
            let index = target.pixel_index(x, y);
            if !depth_frame[index].is_finite() {
                continue;
            }
            let mut sum = 0_u32;
            let mut count = 0_u32;
            for sy in y.saturating_sub(1)..=y.saturating_add(1).min(target.height - 1) {
                for sx in x.saturating_sub(1)..=x.saturating_add(1).min(target.width - 1) {
                    let sample = target.pixel_index(sx, sy);
                    if depth_frame[sample].is_finite() {
                        sum = sum.saturating_add(u32::from(scratch[sample]));
                        count = count.saturating_add(1);
                    }
                }
            }
            if count == 0 || sum == 0 {
                continue;
            }
            let darkening = (sum as f32 / count as f32) / 255.0;
            let offset = pixel_offset(target, x, y);
            for channel in 0..3 {
                frame[offset + channel] =
                    (f32::from(frame[offset + channel]) * (1.0 - darkening)).round() as u8;
            }
        }
    }
    1
}

#[allow(dead_code)]
pub(in crate::render) fn apply_bloom_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    threshold_scratch: &mut [u8],
    horizontal_scratch: &mut [u8],
    config: PostBloomConfig,
) -> u64 {
    apply_bloom_rgba8_profiled(target, frame, threshold_scratch, horizontal_scratch, config).passes
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BloomWork {
    pub(super) passes: u64,
    pub(super) blur_sample_reads: u64,
}

pub(super) fn apply_bloom_rgba8_profiled(
    target: RasterTarget,
    frame: &mut [u8],
    threshold_scratch: &mut [u8],
    horizontal_scratch: &mut [u8],
    config: PostBloomConfig,
) -> BloomWork {
    let radius = u32::from(config.radius_px());
    let intensity = config.intensity().clamp(0.0, 1.0);
    if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
        return BloomWork::default();
    }
    threshold_scratch.fill(0);
    horizontal_scratch.fill(0);
    for y in 0..target.height {
        for x in 0..target.width {
            let offset = pixel_offset(target, x, y);
            if luma_from_srgb8(&frame[offset..offset + 4]) >= f32::from(config.threshold_srgb()) {
                threshold_scratch[offset..offset + 3].copy_from_slice(&frame[offset..offset + 3]);
            }
        }
    }
    let mut blur_sample_reads = 0_u64;
    for y in 0..target.height {
        for x in 0..target.width {
            let min_x = x.saturating_sub(radius);
            let max_x = x.saturating_add(radius).min(target.width - 1);
            let mut sum = [0_u32; 3];
            for sx in min_x..=max_x {
                let sample = pixel_offset(target, sx, y);
                for channel in 0..3 {
                    sum[channel] += u32::from(threshold_scratch[sample + channel]);
                }
                blur_sample_reads = blur_sample_reads.saturating_add(1);
            }
            let count = max_x - min_x + 1;
            let output = pixel_offset(target, x, y);
            for channel in 0..3 {
                horizontal_scratch[output + channel] =
                    (sum[channel] as f32 / count as f32).round() as u8;
            }
        }
    }
    for y in 0..target.height {
        for x in 0..target.width {
            let min_y = y.saturating_sub(radius);
            let max_y = y.saturating_add(radius).min(target.height - 1);
            let mut sum = [0_u32; 3];
            for sy in min_y..=max_y {
                let sample = pixel_offset(target, x, sy);
                for channel in 0..3 {
                    sum[channel] += u32::from(horizontal_scratch[sample + channel]);
                }
                blur_sample_reads = blur_sample_reads.saturating_add(1);
            }
            if sum == [0, 0, 0] {
                continue;
            }
            let count = max_y - min_y + 1;
            let output = pixel_offset(target, x, y);
            for channel in 0..3 {
                let bloom = (sum[channel] as f32 / count as f32) * intensity;
                frame[output + channel] = (f32::from(frame[output + channel]) + bloom)
                    .round()
                    .min(255.0) as u8;
            }
        }
    }
    BloomWork {
        passes: 1,
        blur_sample_reads,
    }
}

pub(in crate::render) fn apply_fxaa_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    scratch: &mut [u8],
) -> u64 {
    if target.width < 3 || target.height < 3 {
        return 0;
    }
    scratch.copy_from_slice(frame);
    for y in 1..target.height - 1 {
        for x in 1..target.width - 1 {
            let center = pixel_offset(target, x, y);
            let samples = [
                pixel_offset(target, x - 1, y - 1),
                pixel_offset(target, x, y - 1),
                pixel_offset(target, x + 1, y - 1),
                pixel_offset(target, x - 1, y),
                center,
                pixel_offset(target, x + 1, y),
                pixel_offset(target, x - 1, y + 1),
                pixel_offset(target, x, y + 1),
                pixel_offset(target, x + 1, y + 1),
            ];
            let center_luma = luma_from_srgb8(&scratch[center..center + 4]);
            let lumas = samples.map(|offset| luma_from_srgb8(&scratch[offset..offset + 4]));
            let min_luma = lumas.into_iter().fold(f32::INFINITY, f32::min);
            let max_luma = lumas.into_iter().fold(f32::NEG_INFINITY, f32::max);
            if max_luma - min_luma < FXAA_LUMA_THRESHOLD {
                continue;
            }
            let bright = lumas
                .iter()
                .filter(|luma| **luma - center_luma >= FXAA_LUMA_THRESHOLD)
                .count();
            let dark = lumas
                .iter()
                .filter(|luma| center_luma - **luma >= FXAA_LUMA_THRESHOLD)
                .count();
            if (center_luma - min_luma <= FXAA_LOCAL_MIN_EPSILON && bright >= 2)
                || (max_luma - center_luma <= FXAA_LOCAL_MIN_EPSILON && dark >= 2)
            {
                average_kernel_rgba8(scratch, frame, center, samples);
            }
        }
    }
    1
}

pub(super) fn pixel_offset(target: RasterTarget, x: u32, y: u32) -> usize {
    target.pixel_index(x, y) * 4
}

pub(super) fn luma_from_srgb8(pixel: &[u8]) -> f32 {
    f32::from(pixel[0]) * 0.299 + f32::from(pixel[1]) * 0.587 + f32::from(pixel[2]) * 0.114
}

fn average_kernel_rgba8(
    source: &[u8],
    target: &mut [u8],
    output_offset: usize,
    sample_offsets: [usize; 9],
) {
    for channel in 0..4 {
        let sum: u16 = sample_offsets
            .into_iter()
            .map(|offset| u16::from(source[offset + channel]))
            .sum();
        target[output_offset + channel] = (sum / 9) as u8;
    }
}

const FXAA_LUMA_THRESHOLD: f32 = 16.0;
const FXAA_LOCAL_MIN_EPSILON: f32 = 1.0;
