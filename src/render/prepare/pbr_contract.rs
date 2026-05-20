//! Khronos/glTF metallic-roughness shading contract helpers.
//!
//! This module is the single Rust owner for CPU/reference PBR math. GPU WGSL
//! and WebGL2 shader code mirrors these formulas and is doctor-guarded against
//! private material-response tuning constants.

use std::f32::consts::PI;

use crate::scene::Vec3;

mod dispersion;
pub(super) use dispersion::dispersion_light_contribution;

pub(super) const DIELECTRIC_F0: f32 = 0.04;
pub(super) const MIN_ROUGHNESS: f32 = 0.04;
const MIN_DENOMINATOR: f32 = 0.0001;
const MIN_N_DOT_V: f32 = 0.001;
const DIRECTIONAL_LUX_TO_SCENE_RADIANCE: f32 = 1.0 / 10_000.0;

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

pub(super) fn clearcoat_light_contribution(
    normal: Vec3,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
    factor: f32,
    roughness: f32,
) -> Vec3 {
    let factor = clamp_unit(factor);
    if factor <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let n_dot_l = dot_vec3(normal, incoming).max(0.0);
    if n_dot_l <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let n_dot_v = dot_vec3(normal, view).max(MIN_N_DOT_V);
    let half_vector = normalize_or(add_vec3(view, incoming), normal);
    let n_dot_h = dot_vec3(normal, half_vector).max(0.0);
    let v_dot_h = dot_vec3(view, half_vector).max(0.0);
    let roughness = roughness_or_min(roughness);
    let alpha = roughness * roughness;
    let distribution = distribution_ggx(n_dot_h, alpha);
    let geometry = geometry_smith(n_dot_v, n_dot_l, roughness);
    let fresnel = fresnel_schlick(
        v_dot_h,
        Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0),
    );
    let specular = scale_vec3(
        fresnel,
        distribution * geometry * factor / (4.0 * n_dot_v * n_dot_l).max(MIN_DENOMINATOR),
    );
    scale_vec3(multiply_vec3(specular, radiance), n_dot_l)
}

pub(super) fn sheen_light_contribution(
    normal: Vec3,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
    color: Vec3,
    roughness: f32,
) -> Vec3 {
    let color = clamp_vec3_unit(color);
    if max_component(color) <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let n_dot_l = dot_vec3(normal, incoming).max(0.0);
    if n_dot_l <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let n_dot_v = dot_vec3(normal, view).max(MIN_N_DOT_V);
    let half_vector = normalize_or(add_vec3(view, incoming), normal);
    let n_dot_h = dot_vec3(normal, half_vector).max(0.0);
    let roughness = roughness_or_min(roughness);
    let alpha = roughness * roughness;
    let distribution = distribution_ggx(n_dot_h, alpha);
    let geometry = geometry_smith(n_dot_v, n_dot_l, roughness);
    let sheen = scale_vec3(
        color,
        distribution * geometry / (4.0 * n_dot_v * n_dot_l).max(MIN_DENOMINATOR),
    );
    scale_vec3(multiply_vec3(sheen, radiance), n_dot_l)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn anisotropy_light_contribution(
    material: PbrMaterial,
    normal: Vec3,
    tangent: Vec3,
    tangent_handedness: f32,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
    strength: f32,
    rotation: f32,
    texture_direction_strength: Vec3,
) -> Vec3 {
    let strength = clamp_unit(strength) * clamp_unit(texture_direction_strength.z);
    if strength <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let n_dot_l = dot_vec3(normal, incoming).max(0.0);
    if n_dot_l <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let normal = normalize_or(normal, Vec3::new(0.0, 0.0, 1.0));
    let view = normalize_or(view, normal);
    let tangent = normalize_or(
        subtract_vec3(tangent, scale_vec3(normal, dot_vec3(tangent, normal))),
        fallback_tangent(normal),
    );
    let bitangent = normalize_or(
        scale_vec3(cross_vec3(normal, tangent), tangent_handedness.signum()),
        fallback_tangent(normal),
    );
    let direction = rotated_anisotropy_direction(texture_direction_strength, rotation);
    let anisotropic_tangent = normalize_or(
        add_vec3(
            scale_vec3(tangent, direction.0),
            scale_vec3(bitangent, direction.1),
        ),
        tangent,
    );
    let anisotropic_bitangent = normalize_or(cross_vec3(normal, anisotropic_tangent), bitangent);
    let half_vector = normalize_or(add_vec3(view, incoming), normal);
    let n_dot_v = dot_vec3(normal, view).max(MIN_N_DOT_V);
    let n_dot_h = dot_vec3(normal, half_vector).max(0.0);
    let v_dot_h = dot_vec3(view, half_vector).max(0.0);
    let t_dot_v = dot_vec3(anisotropic_tangent, view);
    let b_dot_v = dot_vec3(anisotropic_bitangent, view);
    let t_dot_l = dot_vec3(anisotropic_tangent, incoming);
    let b_dot_l = dot_vec3(anisotropic_bitangent, incoming);
    let t_dot_h = dot_vec3(anisotropic_tangent, half_vector);
    let b_dot_h = dot_vec3(anisotropic_bitangent, half_vector);
    let base_alpha = material.roughness * material.roughness;
    let tangent_alpha = mix_scalar(base_alpha, 1.0, strength * strength);
    let bitangent_alpha = base_alpha;
    let distribution =
        distribution_ggx_anisotropic(n_dot_h, t_dot_h, b_dot_h, tangent_alpha, bitangent_alpha);
    let visibility = visibility_ggx_anisotropic(
        n_dot_l,
        n_dot_v,
        b_dot_v,
        t_dot_v,
        t_dot_l,
        b_dot_l,
        tangent_alpha,
        bitangent_alpha,
    );
    let fresnel = fresnel_schlick(v_dot_h, material.f0());
    let specular = scale_vec3(fresnel, distribution * visibility * strength);
    scale_vec3(multiply_vec3(specular, radiance), n_dot_l)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn iridescence_light_contribution(
    material: PbrMaterial,
    normal: Vec3,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
    factor: f32,
    ior: f32,
    thickness_minimum_nm: f32,
    thickness_maximum_nm: f32,
    texture_strength: f32,
    thickness_texture: f32,
) -> Vec3 {
    let factor = clamp_unit(factor) * clamp_unit(texture_strength);
    if factor <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let n_dot_l = dot_vec3(normal, incoming).max(0.0);
    if n_dot_l <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let normal = normalize_or(normal, Vec3::new(0.0, 0.0, 1.0));
    let view = normalize_or(view, normal);
    let half_vector = normalize_or(add_vec3(view, incoming), normal);
    let n_dot_v = dot_vec3(normal, view).max(MIN_N_DOT_V);
    let n_dot_h = dot_vec3(normal, half_vector).max(0.0);
    let v_dot_h = dot_vec3(view, half_vector).max(0.0);
    let roughness = roughness_or_min(material.roughness);
    let alpha = roughness * roughness;
    let distribution = distribution_ggx(n_dot_h, alpha);
    let geometry = geometry_smith(n_dot_v, n_dot_l, roughness);
    let thickness = mix_scalar(
        finite_non_negative(thickness_minimum_nm),
        finite_non_negative(thickness_maximum_nm),
        thickness_texture,
    );
    let film_color = iridescence_film_color(thickness, ior);
    let tinted_f0 = multiply_vec3(material.f0(), film_color);
    let fresnel = fresnel_schlick(v_dot_h, tinted_f0);
    let specular = scale_vec3(
        multiply_vec3(fresnel, film_color),
        distribution * geometry * factor / (4.0 * n_dot_v * n_dot_l).max(MIN_DENOMINATOR),
    );
    scale_vec3(multiply_vec3(specular, radiance), n_dot_l)
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
    finite_non_negative(value) * DIRECTIONAL_LUX_TO_SCENE_RADIANCE
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

fn distribution_ggx_anisotropic(
    n_dot_h: f32,
    t_dot_h: f32,
    b_dot_h: f32,
    tangent_alpha: f32,
    bitangent_alpha: f32,
) -> f32 {
    let alpha_product = (tangent_alpha * bitangent_alpha).max(MIN_DENOMINATOR);
    let f = Vec3::new(
        bitangent_alpha * t_dot_h,
        tangent_alpha * b_dot_h,
        alpha_product * n_dot_h,
    );
    let w2 = alpha_product / dot_vec3(f, f).max(MIN_DENOMINATOR);
    alpha_product * w2 * w2 / PI
}

#[allow(clippy::too_many_arguments)]
fn visibility_ggx_anisotropic(
    n_dot_l: f32,
    n_dot_v: f32,
    b_dot_v: f32,
    t_dot_v: f32,
    t_dot_l: f32,
    b_dot_l: f32,
    tangent_alpha: f32,
    bitangent_alpha: f32,
) -> f32 {
    let ggx_v = n_dot_l
        * length_vec3(Vec3::new(
            tangent_alpha * t_dot_v,
            bitangent_alpha * b_dot_v,
            n_dot_v,
        ));
    let ggx_l = n_dot_v
        * length_vec3(Vec3::new(
            tangent_alpha * t_dot_l,
            bitangent_alpha * b_dot_l,
            n_dot_l,
        ));
    (0.5 / (ggx_v + ggx_l).max(MIN_DENOMINATOR)).clamp(0.0, 1.0)
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

fn clamp_vec3_unit(value: Vec3) -> Vec3 {
    Vec3::new(
        clamp_unit(value.x),
        clamp_unit(value.y),
        clamp_unit(value.z),
    )
}

fn max_component(value: Vec3) -> f32 {
    value.x.max(value.y).max(value.z)
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

fn mix_scalar(left: f32, right: f32, amount: f32) -> f32 {
    let amount = clamp_unit(amount);
    left * (1.0 - amount) + right * amount
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn length_vec3(vector: Vec3) -> f32 {
    dot_vec3(vector, vector).sqrt()
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = length_vec3(vector);
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

fn fallback_tangent(normal: Vec3) -> Vec3 {
    let axis = if normal.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    normalize_or(cross_vec3(axis, normal), Vec3::new(1.0, 0.0, 0.0))
}

fn rotated_anisotropy_direction(texture_direction_strength: Vec3, rotation: f32) -> (f32, f32) {
    let raw_x = texture_direction_strength.x;
    let raw_y = texture_direction_strength.y;
    let length = (raw_x * raw_x + raw_y * raw_y).sqrt();
    let (x, y) = if length <= f32::EPSILON || !length.is_finite() {
        (1.0, 0.0)
    } else {
        (raw_x / length, raw_y / length)
    };
    let rotation = if rotation.is_finite() { rotation } else { 0.0 };
    let sin = rotation.sin();
    let cos = rotation.cos();
    (x * cos - y * sin, x * sin + y * cos)
}

fn iridescence_film_color(thickness_nm: f32, ior: f32) -> Vec3 {
    let thickness_nm = finite_non_negative(thickness_nm);
    let ior = if ior.is_finite() && ior > 0.0 {
        ior
    } else {
        1.3
    };
    let phase = (thickness_nm * ior / 650.0) * PI * 1.25;
    Vec3::new(
        (phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0),
        ((phase + 2.0 * PI / 3.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
        ((phase + 4.0 * PI / 3.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
    )
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
    fn punctual_light_units_do_not_apply_scene_tuned_divisors_or_clamps() {
        assert_eq!(punctual_intensity_candela(800.0), 800.0);
        let near = inverse_square_range_attenuation(Vec3::new(0.0, 0.0, 1.0), Some(10.0));
        let far = inverse_square_range_attenuation(Vec3::new(0.0, 0.0, 2.0), Some(10.0));
        assert!(
            near > far * 3.5,
            "KHR_lights_punctual point/spot intensity must use inverse-square distance falloff"
        );
    }

    #[test]
    fn directional_lux_is_calibrated_to_scene_linear_radiance() {
        assert_eq!(directional_illuminance_lux(0.0), 0.0);
        assert!(
            (directional_illuminance_lux(10_000.0) - 1.0).abs() < 1e-6,
            "a default 10k-lux directional light must be calibrated to renderer scene-linear \
             units instead of being injected as raw 10000x HDR radiance"
        );
    }

    #[test]
    fn clearcoat_light_contribution_adds_dielectric_lobe() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view = normal;
        let incoming = normal;
        let radiance = Vec3::new(1.0, 1.0, 1.0);
        let off = clearcoat_light_contribution(normal, view, incoming, radiance, 0.0, 0.1);
        let on = clearcoat_light_contribution(normal, view, incoming, radiance, 1.0, 0.1);

        assert_eq!(off, Vec3::ZERO);
        assert!(
            on.x > 0.0 && on.y > 0.0 && on.z > 0.0,
            "clearcoat must add a white dielectric specular lobe"
        );
    }

    #[test]
    fn sheen_light_contribution_adds_colored_lobe() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view = normal;
        let incoming = normal;
        let radiance = Vec3::new(1.0, 1.0, 1.0);
        let off = sheen_light_contribution(normal, view, incoming, radiance, Vec3::ZERO, 0.35);
        let red = sheen_light_contribution(
            normal,
            view,
            incoming,
            radiance,
            Vec3::new(1.0, 0.0, 0.0),
            0.35,
        );

        assert_eq!(off, Vec3::ZERO);
        assert!(
            red.x > 0.0 && red.y == 0.0 && red.z == 0.0,
            "sheen must add a colored texture/factor-driven lobe"
        );
    }

    #[test]
    fn anisotropy_light_contribution_uses_strength_texture_and_direction() {
        let material = PbrMaterial::new(Vec3::new(0.8, 0.8, 0.8), 1.0, 0.42);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let tangent = Vec3::new(1.0, 0.0, 0.0);
        let view = normalize_or(Vec3::new(0.25, 0.0, 1.0), normal);
        let incoming = normalize_or(Vec3::new(0.25, 0.0, 1.0), normal);
        let radiance = Vec3::new(1.0, 1.0, 1.0);
        let off = anisotropy_light_contribution(
            material,
            normal,
            tangent,
            1.0,
            view,
            incoming,
            radiance,
            0.0,
            0.0,
            Vec3::new(1.0, 0.5, 1.0),
        );
        let along_tangent = anisotropy_light_contribution(
            material,
            normal,
            tangent,
            1.0,
            view,
            incoming,
            radiance,
            1.0,
            0.0,
            Vec3::new(1.0, 0.5, 1.0),
        );
        let along_bitangent = anisotropy_light_contribution(
            material,
            normal,
            tangent,
            1.0,
            view,
            incoming,
            radiance,
            1.0,
            PI * 0.5,
            Vec3::new(1.0, 0.5, 1.0),
        );

        assert_eq!(off, Vec3::ZERO);
        assert!(
            along_tangent.x > along_bitangent.x * 1.5,
            "anisotropy must shape the specular lobe around the tangent-space direction; \
             tangent={along_tangent:?} bitangent={along_bitangent:?}"
        );

        let zero_texture_strength = anisotropy_light_contribution(
            material,
            normal,
            tangent,
            1.0,
            view,
            incoming,
            radiance,
            1.0,
            0.0,
            Vec3::new(1.0, 0.5, 0.0),
        );
        assert_eq!(
            zero_texture_strength,
            Vec3::ZERO,
            "anisotropyTexture blue channel must multiply anisotropyStrength"
        );
    }

    #[test]
    fn iridescence_light_contribution_uses_factor_thickness_and_textures() {
        let material = PbrMaterial::new(Vec3::new(0.72, 0.72, 0.72), 0.0, 0.35);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view = normalize_or(Vec3::new(0.2, 0.0, 1.0), normal);
        let incoming = normalize_or(Vec3::new(0.15, 0.0, 1.0), normal);
        let radiance = Vec3::new(1.0, 1.0, 1.0);
        let off = iridescence_light_contribution(
            material, normal, view, incoming, radiance, 0.0, 1.3, 100.0, 400.0, 1.0, 1.0,
        );
        let thin = iridescence_light_contribution(
            material, normal, view, incoming, radiance, 1.0, 1.3, 100.0, 250.0, 1.0, 0.0,
        );
        let thick = iridescence_light_contribution(
            material, normal, view, incoming, radiance, 1.0, 1.3, 100.0, 650.0, 1.0, 1.0,
        );
        let texture_off = iridescence_light_contribution(
            material, normal, view, incoming, radiance, 1.0, 1.3, 100.0, 650.0, 0.0, 1.0,
        );

        assert_eq!(off, Vec3::ZERO);
        assert_eq!(texture_off, Vec3::ZERO);
        assert!(
            max_component(thin) > 0.0 && max_component(thick) > 0.0,
            "iridescence should add a thin-film colored specular contribution"
        );
        assert_ne!(
            dominant_channel(thin),
            dominant_channel(thick),
            "iridescence thickness must shift the visible hue, not only brighten uniformly"
        );
    }

    fn dominant_channel(value: Vec3) -> usize {
        if value.x >= value.y && value.x >= value.z {
            0
        } else if value.y >= value.z {
            1
        } else {
            2
        }
    }
}
