use crate::material::Color;
use palette::{LinSrgb, Srgb};

/// Khronos glTF/PBR color contract helpers.
///
/// This module owns scene-referred linear Rec.709 to display-referred sRGB
/// transforms used by CPU output and mirrored by the GPU/WebGL shader code.
/// Asset-specific renders must not introduce private color-calibration
/// constants outside this module.
pub(super) fn apply_exposure(color: Color, exposure_ev: f32) -> Color {
    let exposure = 2.0_f32.powf(exposure_ev);
    Color::from_linear_rgba(
        color.r * exposure,
        color.g * exposure,
        color.b * exposure,
        color.a,
    )
}

/// Khronos PBR Neutral tone mapper.
///
/// Source contract: KhronosGroup/ToneMapping `PBR_Neutral/pbrNeutral.glsl`.
/// Input and output are linear Rec.709; output RGB is in [0, 1].
pub(super) fn pbr_neutral_tonemap(color: Color, exposure_ev: f32) -> Color {
    let exposed = apply_exposure(color, exposure_ev);
    let mut rgb = [exposed.r.max(0.0), exposed.g.max(0.0), exposed.b.max(0.0)];

    const START_COMPRESSION: f32 = 0.8 - PBR_NEUTRAL_F90;
    const DESATURATION: f32 = 0.15;

    let x = rgb[0].min(rgb[1]).min(rgb[2]);
    let offset = if x < 2.0 * PBR_NEUTRAL_F90 {
        x - (x * x) / (4.0 * PBR_NEUTRAL_F90)
    } else {
        PBR_NEUTRAL_F90
    };
    for channel in &mut rgb {
        *channel -= offset;
    }

    let peak = rgb[0].max(rgb[1]).max(rgb[2]);
    if peak < START_COMPRESSION {
        return Color::from_linear_rgba(rgb[0], rgb[1], rgb[2], color.a);
    }

    let compression_range = 1.0 - START_COMPRESSION;
    let new_peak = 1.0
        - compression_range * compression_range / (peak + compression_range - START_COMPRESSION);
    let scale = new_peak / peak;
    for channel in &mut rgb {
        *channel *= scale;
    }

    let desaturation_mix = 1.0 - 1.0 / (DESATURATION * (peak - new_peak) + 1.0);
    Color::from_linear_rgba(
        mix(rgb[0], new_peak, desaturation_mix),
        mix(rgb[1], new_peak, desaturation_mix),
        mix(rgb[2], new_peak, desaturation_mix),
        color.a,
    )
}

pub(super) fn aces_tonemap(color: Color, exposure_ev: f32) -> Color {
    let exposed = apply_exposure(color, exposure_ev);
    let scaled = [exposed.r, exposed.g, exposed.b];
    let input = mul_mat3_vec3(ACES_INPUT_MATRIX, scaled);
    let fitted = [
        rrt_and_odt_fit(input[0]),
        rrt_and_odt_fit(input[1]),
        rrt_and_odt_fit(input[2]),
    ];
    let output = mul_mat3_vec3(ACES_OUTPUT_MATRIX, fitted);
    Color::from_linear_rgba(output[0], output[1], output[2], color.a)
}

pub(super) fn linear_rgba_to_srgb8(color: Color) -> [u8; 4] {
    [
        linear_channel_to_srgb_u8(color.r),
        linear_channel_to_srgb_u8(color.g),
        linear_channel_to_srgb_u8(color.b),
        linear_alpha_to_u8(color.a),
    ]
}

fn mix(left: f32, right: f32, amount: f32) -> f32 {
    left * (1.0 - amount) + right * amount
}

fn linear_channel_to_srgb_u8(value: f32) -> u8 {
    (linear_channel_to_srgb(value) * 255.0).round() as u8
}

fn linear_alpha_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn linear_channel_to_srgb(value: f32) -> f32 {
    let encoded = Srgb::from_linear(LinSrgb::new(value.clamp(0.0, 1.0), 0.0, 0.0));
    encoded.red
}

fn mul_mat3_vec3(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn rrt_and_odt_fit(value: f32) -> f32 {
    let numerator = value * (value + 0.024_578_6) - 0.000_090_537;
    let denominator = value * (0.983_729 * value + 0.432_951) + 0.238_081;
    numerator / denominator
}

const PBR_NEUTRAL_F90: f32 = 0.04;

const ACES_INPUT_MATRIX: [[f32; 3]; 3] = [
    [0.597_19, 0.354_58, 0.048_23],
    [0.076, 0.908_34, 0.015_66],
    [0.028_4, 0.133_83, 0.837_77],
];

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
    fn pbr_neutral_matches_khronos_reference_vectors() {
        assert_color_close(
            pbr_neutral_tonemap(Color::from_linear_rgb(0.0, 0.0, 0.0), 0.0),
            Color::from_linear_rgb(0.0, 0.0, 0.0),
        );
        assert_color_close(
            pbr_neutral_tonemap(Color::from_linear_rgb(0.18, 0.18, 0.18), 0.0),
            Color::from_linear_rgb(0.14, 0.14, 0.14),
        );
        assert_color_close(
            pbr_neutral_tonemap(Color::from_linear_rgb(0.8, 0.2, 0.05), 0.0),
            Color::from_linear_rgb(0.765_496_2, 0.165_608_72, 0.015_636_861),
        );
        assert_color_close(
            pbr_neutral_tonemap(Color::from_linear_rgb(2.0, 1.0, 0.2), 0.0),
            Color::from_linear_rgb(0.96, 0.534_090_5, 0.193_362_9),
        );
    }

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
