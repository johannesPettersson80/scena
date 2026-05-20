use crate::material::Color;

use super::RasterTarget;
use super::color_contract::{
    aces_tonemap, apply_exposure, linear_rgba_to_srgb8, pbr_neutral_tonemap,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OutputTransform {
    exposure_ev: f32,
    tonemapper: Tonemapper,
}

impl OutputTransform {
    pub(super) fn encode_rgba8(self, color: Color) -> [u8; 4] {
        match self.tonemapper {
            Tonemapper::Aces => linear_rgba_to_srgb8(aces_tonemap(color, self.exposure_ev)),
            Tonemapper::PbrNeutral => {
                linear_rgba_to_srgb8(pbr_neutral_tonemap(color, self.exposure_ev))
            }
            Tonemapper::Standard => linear_rgba_to_srgb8(apply_exposure(color, self.exposure_ev)),
        }
    }

    pub(super) fn encode_clear_rgba8(self, color: Color) -> [u8; 4] {
        linear_rgba_to_srgb8(color)
    }

    pub(super) const fn exposure_ev(self) -> f32 {
        self.exposure_ev
    }

    pub(super) fn set_exposure_ev(&mut self, exposure_ev: f32) {
        self.exposure_ev = if exposure_ev.is_finite() {
            exposure_ev
        } else {
            0.0
        };
    }

    pub(super) const fn tonemapper(self) -> Tonemapper {
        self.tonemapper
    }

    pub(super) const fn set_tonemapper(&mut self, tonemapper: Tonemapper) {
        self.tonemapper = tonemapper;
    }

    pub(super) const fn color_management_uniform(self) -> [f32; 4] {
        match self.tonemapper {
            Tonemapper::Standard => [0.0, 0.0, 0.0, 0.0],
            Tonemapper::Aces => [1.0, 0.0, 0.0, 0.0],
            Tonemapper::PbrNeutral => [2.0, 0.0, 0.0, 0.0],
        }
    }
}

impl Default for OutputTransform {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            tonemapper: Tonemapper::PbrNeutral,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemapper {
    Aces,
    Standard,
    #[default]
    PbrNeutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AntiAliasing {
    None,
    #[default]
    Fxaa,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderIndependentTransparencyConfig {
    coverage_boost: f32,
}

impl OrderIndependentTransparencyConfig {
    pub const fn weighted_blended() -> Self {
        Self {
            coverage_boost: 1.0,
        }
    }

    pub fn new(coverage_boost: f32) -> Self {
        Self {
            coverage_boost: if coverage_boost.is_finite() {
                coverage_boost.clamp(0.25, 4.0)
            } else {
                1.0
            },
        }
    }

    pub const fn coverage_boost(self) -> f32 {
        self.coverage_boost
    }
}

impl Default for OrderIndependentTransparencyConfig {
    fn default() -> Self {
        Self::weighted_blended()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostBloomConfig {
    threshold_srgb: u8,
    intensity: f32,
    radius_px: u8,
}

impl PostBloomConfig {
    pub const fn subtle() -> Self {
        Self {
            threshold_srgb: 208,
            intensity: 0.28,
            radius_px: 3,
        }
    }

    pub fn new(threshold_srgb: u8, intensity: f32, radius_px: u8) -> Self {
        Self {
            threshold_srgb,
            intensity: if intensity.is_finite() {
                intensity.clamp(0.0, 1.0)
            } else {
                0.0
            },
            radius_px: radius_px.min(12),
        }
    }

    pub const fn threshold_srgb(self) -> u8 {
        self.threshold_srgb
    }

    pub const fn intensity(self) -> f32 {
        self.intensity
    }

    pub const fn radius_px(self) -> u8 {
        self.radius_px
    }
}

impl Default for PostBloomConfig {
    fn default() -> Self {
        Self::subtle()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSpaceAmbientOcclusionConfig {
    radius_px: u8,
    intensity: f32,
    depth_threshold: f32,
}

impl ScreenSpaceAmbientOcclusionConfig {
    pub const fn subtle() -> Self {
        Self {
            radius_px: 3,
            intensity: 0.45,
            depth_threshold: 0.025,
        }
    }

    pub fn new(radius_px: u8, intensity: f32, depth_threshold: f32) -> Self {
        Self {
            radius_px: radius_px.min(12),
            intensity: if intensity.is_finite() {
                intensity.clamp(0.0, 1.0)
            } else {
                0.0
            },
            depth_threshold: if depth_threshold.is_finite() {
                depth_threshold.max(0.0)
            } else {
                0.0
            },
        }
    }

    pub const fn radius_px(self) -> u8 {
        self.radius_px
    }

    pub const fn intensity(self) -> f32 {
        self.intensity
    }

    pub const fn depth_threshold(self) -> f32 {
        self.depth_threshold
    }
}

impl Default for ScreenSpaceAmbientOcclusionConfig {
    fn default() -> Self {
        Self::subtle()
    }
}

pub(super) fn apply_screen_space_ambient_occlusion_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    depth_frame: &[f32],
    config: ScreenSpaceAmbientOcclusionConfig,
) -> u64 {
    let radius = u32::from(config.radius_px());
    let intensity = config.intensity().clamp(0.0, 1.0);
    if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.byte_len());
    debug_assert_eq!(depth_frame.len(), target.pixel_len());

    let threshold = config.depth_threshold().max(0.0);
    for y in 0..target.height {
        for x in 0..target.width {
            let pixel_index = target.pixel_index(x, y);
            let center_depth = depth_frame[pixel_index];
            if !center_depth.is_finite() {
                continue;
            }
            let min_x = x.saturating_sub(radius);
            let max_x = x.saturating_add(radius).min(target.width - 1);
            let min_y = y.saturating_sub(radius);
            let max_y = y.saturating_add(radius).min(target.height - 1);
            let mut finite_samples = 0_u32;
            let mut occluders = 0_u32;
            for sample_y in min_y..=max_y {
                for sample_x in min_x..=max_x {
                    if sample_x == x && sample_y == y {
                        continue;
                    }
                    let sample_depth = depth_frame[target.pixel_index(sample_x, sample_y)];
                    if !sample_depth.is_finite() {
                        continue;
                    }
                    finite_samples += 1;
                    if sample_depth + threshold < center_depth {
                        occluders += 1;
                    }
                }
            }
            if occluders == 0 || finite_samples == 0 {
                continue;
            }
            let coverage = occluders as f32 / finite_samples as f32;
            let darkening = (coverage * intensity).clamp(0.0, 0.65);
            let offset = pixel_offset(target, x, y);
            for channel in 0..3 {
                frame[offset + channel] =
                    (f32::from(frame[offset + channel]) * (1.0 - darkening)).round() as u8;
            }
        }
    }

    1
}

pub(super) fn apply_bloom_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    scratch: &mut [u8],
    config: PostBloomConfig,
) -> u64 {
    let radius = u32::from(config.radius_px());
    let intensity = config.intensity().clamp(0.0, 1.0);
    if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.byte_len());
    debug_assert_eq!(scratch.len(), target.byte_len());
    scratch.fill(0);

    for y in 0..target.height {
        for x in 0..target.width {
            let offset = pixel_offset(target, x, y);
            if luma_from_srgb8(&frame[offset..offset + 4]) >= f32::from(config.threshold_srgb()) {
                scratch[offset] = frame[offset];
                scratch[offset + 1] = frame[offset + 1];
                scratch[offset + 2] = frame[offset + 2];
                scratch[offset + 3] = frame[offset + 3];
            }
        }
    }

    for y in 0..target.height {
        for x in 0..target.width {
            let min_x = x.saturating_sub(radius);
            let max_x = x.saturating_add(radius).min(target.width - 1);
            let min_y = y.saturating_sub(radius);
            let max_y = y.saturating_add(radius).min(target.height - 1);
            let mut sum = [0_u32; 3];
            let mut sample_count = 0_u32;
            for sample_y in min_y..=max_y {
                for sample_x in min_x..=max_x {
                    let sample = pixel_offset(target, sample_x, sample_y);
                    sum[0] += u32::from(scratch[sample]);
                    sum[1] += u32::from(scratch[sample + 1]);
                    sum[2] += u32::from(scratch[sample + 2]);
                    sample_count += 1;
                }
            }
            if sum == [0, 0, 0] {
                continue;
            }
            let output = pixel_offset(target, x, y);
            for channel in 0..3 {
                let bloom = (sum[channel] as f32 / sample_count as f32) * intensity;
                frame[output + channel] = (f32::from(frame[output + channel]) + bloom)
                    .round()
                    .min(255.0) as u8;
            }
        }
    }

    1
}

pub(super) fn apply_fxaa_rgba8(target: RasterTarget, frame: &mut [u8], scratch: &mut [u8]) -> u64 {
    if target.width < 3 || target.height < 3 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.byte_len());
    debug_assert_eq!(scratch.len(), target.byte_len());
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
            let bright_neighbors = lumas
                .iter()
                .filter(|luma| **luma - center_luma >= FXAA_LUMA_THRESHOLD)
                .count();
            let dark_neighbors = lumas
                .iter()
                .filter(|luma| center_luma - **luma >= FXAA_LUMA_THRESHOLD)
                .count();
            let dark_edge =
                center_luma - min_luma <= FXAA_LOCAL_MIN_EPSILON && bright_neighbors >= 2;
            let light_edge =
                max_luma - center_luma <= FXAA_LOCAL_MIN_EPSILON && dark_neighbors >= 2;
            if !dark_edge && !light_edge {
                continue;
            }
            average_kernel_rgba8(scratch, frame, center, samples);
        }
    }

    1
}

fn pixel_offset(target: RasterTarget, x: u32, y: u32) -> usize {
    target.pixel_index(x, y) * 4
}

fn luma_from_srgb8(pixel: &[u8]) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbr_neutral_uses_dedicated_shader_branch_marker() {
        let mut output = OutputTransform::default();
        output.set_tonemapper(Tonemapper::PbrNeutral);

        assert_eq!(output.color_management_uniform(), [2.0, 0.0, 0.0, 0.0]);
    }
}
