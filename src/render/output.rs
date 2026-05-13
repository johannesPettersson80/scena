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
