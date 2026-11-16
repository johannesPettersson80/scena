//! Khronos/glTF metallic-roughness shading contract helpers.
//!
//! This module is the single Rust owner for CPU/reference PBR math. GPU WGSL
//! and WebGL2 shader code mirrors these formulas and is doctor-guarded against
//! private material-response tuning constants. The layered surface model in
//! `surface` is the Rust side of `src/render/gpu/surface_lighting.wgsl`.

use crate::scene::Vec3;

use super::super::pbr_brdf;

mod anisotropy;
mod dispersion;
pub(super) use dispersion::dispersion_light_contribution;
mod iridescence;
mod sheen;
pub(super) use sheen::sheen_environment_brdf;
mod surface;
pub(super) use surface::{Surface, surface_direct_light};
mod transmission;
pub(super) use transmission::transmission_volume_light_contribution;

pub(super) const DIELECTRIC_F0: f32 = pbr_brdf::DIELECTRIC_F0;
/// The glTF default index of refraction, whose reflectance is [`DIELECTRIC_F0`].
const DEFAULT_IOR: f32 = 1.5;
const MIN_DENOMINATOR: f32 = 0.0001;
const MIN_N_DOT_V: f32 = pbr_brdf::MIN_N_DOT_V;
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
        pbr_brdf::f0_metallic_roughness(self.base, self.metallic)
    }
}

pub(super) fn roughness_or_min(value: f32) -> f32 {
    pbr_brdf::perceptual_roughness_or_min(value)
}

/// Dielectric reflectance at normal incidence for an index of refraction.
fn f0_from_ior(ior: f32) -> f32 {
    let ratio = (ior - 1.0) / (ior + 1.0).max(MIN_DENOMINATOR);
    ratio * ratio
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

fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    pbr_brdf::fresnel_schlick(f0, cos_theta)
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
    fn core_pbr_brdf_matches_khronos_correlated_ggx_grazing_probe() {
        // KhronosGroup/glTF-Sample-Renderer bec106e, brdf.glsl:
        // BRDF_specularGGX = D_GGX(alphaRoughness) * V_GGX(correlated Smith).
        let material = PbrMaterial::new(Vec3::new(0.7, 0.72, 0.75), 1.0, 0.2);
        let grazing_z = 0.05_f32;
        let grazing_x = (1.0 - grazing_z * grazing_z).sqrt();
        let actual = surface_direct_light(
            &Surface::new(
                material,
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(grazing_x, 0.0, grazing_z),
            ),
            Vec3::new(-grazing_x, 0.0, grazing_z),
            Vec3::new(1.0, 1.0, 1.0),
            1.0,
        );

        assert_vec3_close(
            actual,
            Vec3::new(724.38367, 727.89966, 733.1737),
            0.25,
            "core metallic GGX grazing probe",
        );
    }

    #[test]
    fn sampled_area_light_base_lobe_contains_diffuse_but_not_metal_specular() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view = normal;
        let incoming = normal;
        let radiance = Vec3::new(2.0, 2.0, 2.0);
        // Weight 0 is how area-light point samples are shaded: the LTC
        // integral owns their base specular lobe.
        let sampled = |metallic: f32| {
            surface_direct_light(
                &Surface::new(
                    PbrMaterial::new(Vec3::new(0.7, 0.3, 0.1), metallic, 0.05),
                    normal,
                    view,
                ),
                incoming,
                radiance,
                0.0,
            )
        };
        let dielectric = sampled(0.0);
        let metal = sampled(1.0);

        assert!(
            dielectric.x > 0.0 && dielectric.y > 0.0 && dielectric.z > 0.0,
            "sampled finite-emitter points must retain dielectric diffuse energy"
        );
        assert_eq!(
            metal,
            Vec3::ZERO,
            "continuous LTC already owns the finite-emitter metal specular lobe"
        );
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

    fn assert_vec3_close(actual: Vec3, expected: Vec3, tolerance: f32, label: &str) {
        let delta = Vec3::new(
            (actual.x - expected.x).abs(),
            (actual.y - expected.y).abs(),
            (actual.z - expected.z).abs(),
        );
        assert!(
            delta.x <= tolerance && delta.y <= tolerance && delta.z <= tolerance,
            "{label} expected {expected:?}, got {actual:?}, delta {delta:?}"
        );
    }
}
