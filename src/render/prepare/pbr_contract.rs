//! Khronos/glTF metallic-roughness shading contract helpers.
//!
//! This module is the single Rust owner for CPU/reference PBR math. GPU WGSL
//! and WebGL2 shader code mirrors these formulas and is doctor-guarded against
//! private material-response tuning constants.

use std::f32::consts::PI;

use crate::scene::Vec3;

pub(super) const DIELECTRIC_F0: f32 = 0.04;
pub(super) const MIN_ROUGHNESS: f32 = 0.04;
const MIN_DENOMINATOR: f32 = 0.0001;
const MIN_N_DOT_V: f32 = 0.001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PbrMaterial {
    pub(super) base: Vec3,
    pub(super) metallic: f32,
    pub(super) roughness: f32,
}

impl PbrMaterial {
    pub(super) fn new(base: Vec3, metallic: f32, roughness: f32) -> Self {
        Self {
            base,
            metallic: clamp_unit(metallic),
            roughness: roughness_or_min(roughness),
        }
    }

    pub(super) fn f0(self) -> Vec3 {
        mix_vec3(
            Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0),
            self.base,
            self.metallic,
        )
    }
}

pub(super) fn roughness_or_min(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_ROUGHNESS, 1.0)
    } else {
        1.0
    }
}

pub(super) fn punctual_light_contribution(
    material: PbrMaterial,
    normal: Vec3,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
) -> Vec3 {
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let n_dot_l = dot_vec3(normal, incoming).max(0.0);
    if n_dot_l <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let n_dot_v = dot_vec3(normal, view).max(MIN_N_DOT_V);
    let half_vector = normalize_or(add_vec3(view, incoming), normal);
    let n_dot_h = dot_vec3(normal, half_vector).max(0.0);
    let v_dot_h = dot_vec3(view, half_vector).max(0.0);
    let alpha = material.roughness * material.roughness;
    let distribution = distribution_ggx(n_dot_h, alpha);
    let geometry = geometry_smith(n_dot_v, n_dot_l, material.roughness);
    let fresnel = fresnel_schlick(v_dot_h, material.f0());
    let specular = scale_vec3(
        fresnel,
        distribution * geometry / (4.0 * n_dot_v * n_dot_l).max(MIN_DENOMINATOR),
    );
    let diffuse_energy = scale_vec3(
        subtract_vec3(Vec3::new(1.0, 1.0, 1.0), fresnel),
        1.0 - material.metallic,
    );
    let diffuse = scale_vec3(multiply_vec3(diffuse_energy, material.base), PI.recip());
    scale_vec3(
        multiply_vec3(add_vec3(diffuse, specular), radiance),
        n_dot_l,
    )
}

pub(super) fn environment_split_sum_contribution(
    material: PbrMaterial,
    normal: Vec3,
    view: Vec3,
    diffuse_irradiance: Vec3,
    prefiltered_specular: Vec3,
    brdf_scale_bias: (f32, f32),
) -> Vec3 {
    let n_dot_v = dot_vec3(normal, view).max(MIN_N_DOT_V);
    let fresnel = fresnel_schlick(n_dot_v, material.f0());
    let diffuse_energy = scale_vec3(
        subtract_vec3(Vec3::new(1.0, 1.0, 1.0), fresnel),
        1.0 - material.metallic,
    );
    let diffuse = multiply_vec3(
        multiply_vec3(diffuse_energy, material.base),
        diffuse_irradiance,
    );
    let f0 = material.f0();
    let specular_factor = add_vec3(
        scale_vec3(f0, brdf_scale_bias.0),
        Vec3::new(brdf_scale_bias.1, brdf_scale_bias.1, brdf_scale_bias.1),
    );
    let specular = multiply_vec3(prefiltered_specular, specular_factor);
    add_vec3(diffuse, specular)
}

pub(super) fn directional_illuminance_lux(value: f32) -> f32 {
    finite_non_negative(value)
}

pub(super) fn punctual_intensity_candela(value: f32) -> f32 {
    finite_non_negative(value)
}

pub(super) fn inverse_square_range_attenuation(to_light: Vec3, range: Option<f32>) -> f32 {
    let distance_squared = dot_vec3(to_light, to_light).max(MIN_DENOMINATOR);
    let inverse_square = distance_squared.recip();
    let Some(range) = range else {
        return inverse_square;
    };
    if range <= f32::EPSILON || !range.is_finite() {
        return 0.0;
    }
    let distance = distance_squared.sqrt();
    let range_falloff = (1.0 - (distance / range).powi(4)).clamp(0.0, 1.0);
    inverse_square * range_falloff * range_falloff
}

pub(super) fn spot_cone_attenuation(
    cos_angle: f32,
    inner_cone_cos: f32,
    outer_cone_cos: f32,
) -> f32 {
    if cos_angle >= inner_cone_cos {
        1.0
    } else if cos_angle <= outer_cone_cos {
        0.0
    } else {
        ((cos_angle - outer_cone_cos) / (inner_cone_cos - outer_cone_cos)).clamp(0.0, 1.0)
    }
}

pub(super) fn reflect_vec3(vector: Vec3, normal: Vec3) -> Vec3 {
    subtract_vec3(vector, scale_vec3(normal, 2.0 * dot_vec3(vector, normal)))
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha_squared = alpha * alpha;
    let denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    alpha_squared / (PI * denominator * denominator).max(MIN_DENOMINATOR)
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let k = ((roughness + 1.0) * (roughness + 1.0)) / 8.0;
    geometry_schlick_ggx(n_dot_v, k) * geometry_schlick_ggx(n_dot_l, k)
}

fn geometry_schlick_ggx(n_dot: f32, k: f32) -> f32 {
    n_dot / (n_dot * (1.0 - k) + k).max(MIN_DENOMINATOR)
}

fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    let factor = (1.0 - cos_theta.clamp(0.0, 1.0)).powi(5);
    add_vec3(
        f0,
        scale_vec3(subtract_vec3(Vec3::new(1.0, 1.0, 1.0), f0), factor),
    )
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn multiply_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x * right.x, left.y * right.y, left.z * right.z)
}

fn mix_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    let amount = clamp_unit(amount);
    add_vec3(scale_vec3(left, 1.0 - amount), scale_vec3(right, amount))
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn length_vec3(vector: Vec3) -> f32 {
    dot_vec3(vector, vector).sqrt()
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = length_vec3(vector);
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbr_material_uses_gltf_dielectric_and_metallic_f0() {
        let base = Vec3::new(0.8, 0.2, 0.1);
        let dielectric = PbrMaterial::new(base, 0.0, 0.5);
        assert_eq!(
            dielectric.f0(),
            Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0)
        );
        let metal = PbrMaterial::new(base, 1.0, 0.5);
        assert_eq!(metal.f0(), base);
    }

    #[test]
    fn light_units_do_not_apply_scene_tuned_divisors_or_clamps() {
        assert_eq!(directional_illuminance_lux(20_000.0), 20_000.0);
        assert_eq!(punctual_intensity_candela(800.0), 800.0);
        let near = inverse_square_range_attenuation(Vec3::new(0.0, 0.0, 1.0), Some(10.0));
        let far = inverse_square_range_attenuation(Vec3::new(0.0, 0.0, 2.0), Some(10.0));
        assert!(
            near > far * 3.5,
            "KHR_lights_punctual point/spot intensity must use inverse-square distance falloff"
        );
    }
}
