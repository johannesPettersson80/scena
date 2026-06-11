use crate::material::Color;

use super::super::camera::CameraProjection;

pub(super) fn wgpu_clear_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: clear_channel_f64(color.r),
        g: clear_channel_f64(color.g),
        b: clear_channel_f64(color.b),
        a: clear_channel_f64(color.a),
    }
}

fn clear_channel_f64(channel: f32) -> f64 {
    channel.clamp(0.0, 1.0) as f64
}

pub(super) fn post_color_management_uniform(
    mut color_management: [f32; 4],
    post_enabled: bool,
) -> [f32; 4] {
    color_management[1] = if post_enabled { 1.0 } else { 0.0 };
    color_management
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
