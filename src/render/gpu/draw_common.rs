use crate::material::Color;

use super::super::camera::CameraProjection;

pub(super) fn wgpu_clear_color_for_target(
    color: Color,
    target_format: wgpu::TextureFormat,
) -> wgpu::Color {
    let shader_encodes_srgb = shader_encodes_srgb_for_target(target_format);
    wgpu::Color {
        r: clear_channel_f64(color.r, shader_encodes_srgb),
        g: clear_channel_f64(color.g, shader_encodes_srgb),
        b: clear_channel_f64(color.b, shader_encodes_srgb),
        a: clear_alpha_f64(color.a),
    }
}

fn clear_channel_f64(channel: f32, encode_srgb: bool) -> f64 {
    if encode_srgb {
        crate::render::color_contract::linear_channel_to_srgb(channel) as f64
    } else {
        channel.clamp(0.0, 1.0) as f64
    }
}

fn clear_alpha_f64(channel: f32) -> f64 {
    channel.clamp(0.0, 1.0) as f64
}

pub(super) fn target_color_management_uniform(
    mut color_management: [f32; 4],
    target_format: wgpu::TextureFormat,
) -> [f32; 4] {
    color_management[1] = if target_format == wgpu::TextureFormat::Rgba16Float {
        -1.0
    } else if shader_encodes_srgb_for_target(target_format) {
        1.0
    } else {
        0.0
    };
    color_management
}

pub(super) const fn shader_encodes_srgb_for_target(target_format: wgpu::TextureFormat) -> bool {
    matches!(
        target_format,
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
    )
}

pub(super) fn camera_position_uniform(camera_projection: &CameraProjection) -> [f32; 3] {
    let position = camera_projection.camera_position();
    [position.x, position.y, position.z]
}

pub(super) fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_post_unorm_readback_encodes_known_linear_value_as_srgb8() {
        let uniform = target_color_management_uniform([0.0; 4], wgpu::TextureFormat::Rgba8Unorm);
        let actual = shader_contract_byte(0.18, uniform[1]);

        assert_eq!(actual, 118, "linear 0.18 must be labeled as sRGB byte 118");
        assert_ne!(actual, 46, "linear byte 46 is the old too-dark result");
    }

    #[test]
    fn target_format_selects_exactly_one_srgb_transfer() {
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ] {
            assert_eq!(target_color_management_uniform([0.0; 4], format)[1], 1.0);
            let clear =
                wgpu_clear_color_for_target(Color::from_linear_rgb(0.18, 0.18, 0.18), format);
            assert!((clear.r - 118.0 / 255.0).abs() < 0.002);
        }
        for format in [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            assert_eq!(target_color_management_uniform([0.0; 4], format)[1], 0.0);
            let clear =
                wgpu_clear_color_for_target(Color::from_linear_rgb(0.18, 0.18, 0.18), format);
            assert!((clear.r - 0.18).abs() < 1.0e-6);
        }
        assert_eq!(
            target_color_management_uniform([1.0; 4], wgpu::TextureFormat::Rgba16Float)[1],
            -1.0,
            "floating-point post targets must defer tonemapping and display encoding"
        );
    }

    fn shader_contract_byte(linear: f32, output_transfer_mode: f32) -> u8 {
        let encoded = if output_transfer_mode > 0.5 {
            if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
        } else {
            linear
        };
        (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}
