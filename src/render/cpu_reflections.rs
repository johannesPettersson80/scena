use crate::scene::Vec3;

use super::RasterTarget;
use super::camera::CameraProjection;
use super::screen_space_reflections::{MaterialReflectionPixel, ScreenSpaceReflectionConfig};

pub(super) struct MaterialReflectionRecord<'a> {
    pub(super) target: RasterTarget,
    pub(super) camera: &'a CameraProjection,
    pub(super) material_reflections: &'a mut [MaterialReflectionPixel],
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) position: Vec3,
    pub(super) normal: Vec3,
    pub(super) reflection: super::prepare::PreparedMaterialReflection,
    pub(super) config: ScreenSpaceReflectionConfig,
}

pub(super) fn record_material_reflection_pixel(input: MaterialReflectionRecord<'_>) {
    let strength = input.config.strength().clamp(0.0, 1.0);
    if strength <= 0.001 {
        return;
    }
    let view = normalize_or(input.camera.camera_position() - input.position, Vec3::Z);
    let normal = normalize_or(input.normal, Vec3::Z);
    let reflected = (-view).reflect(normal);
    let reflect_scale = (0.12 + (1.0 - input.reflection.roughness()) * 0.24) * strength;
    let raw_sample_x = input.x as f32 + reflected.x * reflect_scale * input.target.width as f32;
    let raw_sample_y = input.y as f32 - reflected.y * reflect_scale * input.target.height as f32;
    let edge_fade = reflection_edge_fade(
        raw_sample_x / input.target.width.max(1) as f32,
        raw_sample_y / input.target.height.max(1) as f32,
    );
    if edge_fade <= 0.0 {
        return;
    }
    let sample_x = raw_sample_x.clamp(0.0, input.target.width.saturating_sub(1) as f32);
    let sample_y = raw_sample_y.clamp(0.0, input.target.height.saturating_sub(1) as f32);
    let n_dot_v = normal.dot(view).max(0.0);
    let fresnel = (1.0 - n_dot_v.clamp(0.0, 1.0)).powi(5);
    let weight = (strength
        * input.reflection.metallic()
        * (0.38 + fresnel * 0.52)
        * (1.0 - input.reflection.roughness() * 0.55)
        * edge_fade)
        .clamp(0.0, 0.88);
    let Some(pixel) =
        MaterialReflectionPixel::new(sample_x, sample_y, weight, input.reflection.roughness())
    else {
        return;
    };
    let index = input.target.pixel_index(input.x, input.y);
    input.material_reflections[index] = pixel;
}

fn reflection_edge_fade(u: f32, v: f32) -> f32 {
    let distance = u.min(v).min(1.0 - u).min(1.0 - v);
    smoothstep(0.0, 0.02, distance)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length_sq = value.length_squared();
    if length_sq <= f32::EPSILON || !length_sq.is_finite() {
        fallback
    } else {
        value / length_sq.sqrt()
    }
}
