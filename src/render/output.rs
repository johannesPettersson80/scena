use crate::material::Color;

use super::RasterTarget;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OutputTransform {
    exposure_ev: f32,
    tonemapper: Tonemapper,
}

impl OutputTransform {
    pub(super) fn encode_rgba8(self, color: Color) -> [u8; 4] {
        match self.tonemapper {
            Tonemapper::Aces => linear_rgba_to_srgb8(aces_tonemap(color, self.exposure_ev)),
        }
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
}

impl Default for OutputTransform {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            tonemapper: Tonemapper::Aces,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemapper {
    #[default]
    Aces,
}

fn aces_tonemap(color: Color, exposure_ev: f32) -> Color {
    let exposure = 2.0_f32.powf(exposure_ev);
    let scaled = [color.r * exposure, color.g * exposure, color.b * exposure];
    let input = mul_mat3_vec3(ACES_INPUT_MATRIX, scaled);
    let fitted = [
        rrt_and_odt_fit(input[0]),
        rrt_and_odt_fit(input[1]),
        rrt_and_odt_fit(input[2]),
    ];
    let output = mul_mat3_vec3(ACES_OUTPUT_MATRIX, fitted);
    Color::from_linear_rgba(output[0], output[1], output[2], color.a)
}

fn rrt_and_odt_fit(value: f32) -> f32 {
    let numerator = value * (value + 0.024_578_6) - 0.000_090_537;
    let denominator = value * (0.983_729 * value + 0.432_951) + 0.238_081;
    numerator / denominator
}

fn linear_rgba_to_srgb8(color: Color) -> [u8; 4] {
    [
        linear_channel_to_srgb_u8(color.r),
        linear_channel_to_srgb_u8(color.g),
        linear_channel_to_srgb_u8(color.b),
        linear_alpha_to_u8(color.a),
    ]
}

fn linear_channel_to_srgb_u8(value: f32) -> u8 {
    (linear_channel_to_srgb(value) * 255.0).round() as u8
}

fn linear_alpha_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn linear_channel_to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn mul_mat3_vec3(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
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
            if center_luma - min_luma > FXAA_LOCAL_MIN_EPSILON {
                continue;
            }
            let bright_neighbors = lumas
                .into_iter()
                .filter(|luma| *luma - center_luma >= FXAA_LUMA_THRESHOLD)
                .count();
            if bright_neighbors < 3 {
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

const ACES_INPUT_MATRIX: [[f32; 3]; 3] = [
    [0.597_19, 0.354_58, 0.048_23],
    [0.076, 0.908_34, 0.015_66],
    [0.028_4, 0.133_83, 0.837_77],
];

const FXAA_LUMA_THRESHOLD: f32 = 16.0;
const FXAA_LOCAL_MIN_EPSILON: f32 = 1.0;

const ACES_OUTPUT_MATRIX: [[f32; 3]; 3] = [
    [1.604_75, -0.531_08, -0.073_67],
    [-0.102_08, 1.108_13, -0.006_05],
    [-0.003_27, -0.072_76, 1.076_02],
];

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_01;

    #[test]
    fn aces_rrt_and_odt_fit_uses_pinned_coefficients() {
        assert_close(rrt_and_odt_fit(0.045), 0.011_714_242);
        assert_close(rrt_and_odt_fit(0.18), 0.105_591_25);
        assert_close(rrt_and_odt_fit(0.72), 0.505_775_45);
        assert_close(rrt_and_odt_fit(4.0), 0.909_013_75);
    }

    #[test]
    fn aces_tonemap_neutral_gray_tracks_exposure_ev() {
        let gray = Color::from_linear_rgb(0.18, 0.18, 0.18);

        assert_color_close(
            aces_tonemap(gray, -2.0),
            Color::from_linear_rgb(0.011_714_242, 0.011_714_242, 0.011_714_125),
        );
        assert_color_close(
            aces_tonemap(gray, 0.0),
            Color::from_linear_rgb(0.105_591_25, 0.105_591_25, 0.105_590_19),
        );
        assert_color_close(
            aces_tonemap(gray, 2.0),
            Color::from_linear_rgb(0.505_775_45, 0.505_775_45, 0.505_770_4),
        );
    }

    #[test]
    fn aces_tonemap_preserves_alpha_and_handles_color_channels() {
        let color = Color::from_linear_rgba(0.8, 0.2, 0.05, 0.375);

        assert_color_close(
            aces_tonemap(color, 0.0),
            Color::from_linear_rgba(0.567_594_47, 0.137_542_43, 0.026_417_239, 0.375),
        );
    }

    #[test]
    fn srgb_output_encoding_uses_standard_transfer_curve() {
        assert_eq!(linear_channel_to_srgb_u8(0.0), 0);
        assert_eq!(linear_channel_to_srgb_u8(0.003_130_8), 10);
        assert_eq!(linear_channel_to_srgb_u8(0.18), 118);
        assert_eq!(linear_channel_to_srgb_u8(0.5), 188);
        assert_eq!(linear_channel_to_srgb_u8(1.0), 255);
        assert_eq!(linear_channel_to_srgb_u8(2.0), 255);

        assert_eq!(
            linear_rgba_to_srgb8(Color::from_linear_rgba(0.18, 0.5, 2.0, 0.25)),
            [118, 188, 255, 64]
        );
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {actual} to be within {EPSILON} of {expected}"
        );
    }

    fn assert_color_close(actual: Color, expected: Color) {
        assert_close(actual.r, expected.r);
        assert_close(actual.g, expected.g);
        assert_close(actual.b, expected.b);
        assert_close(actual.a, expected.a);
    }
}
