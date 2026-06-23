//! Khronos/glTF core metallic-roughness BRDF contract.
//!
//! This module owns the scalar/vector math for the GGX/Smith/Schlick BRDF
//! used by CPU preparation and mirrored by `pbr_brdf.wgsl` for WebGPU/WebGL2.
//! The formulas are ported from KhronosGroup/glTF-Sample-Renderer commit
//! bec106e (`source/Renderer/shaders/brdf.glsl`):
//! `BRDF_specularGGX = D_GGX(alphaRoughness) * V_GGX(correlated Smith)`.

use std::f32::consts::PI;

use crate::scene::Vec3;

pub(in crate::render) const DIELECTRIC_F0: f32 = 0.04;
pub(in crate::render) const MIN_PERCEPTUAL_ROUGHNESS: f32 = 0.04;
pub(in crate::render) const MIN_N_DOT_V: f32 = 0.001;

pub(in crate::render) fn perceptual_roughness_or_min(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_PERCEPTUAL_ROUGHNESS, 1.0)
    } else {
        1.0
    }
}

pub(in crate::render) fn alpha_roughness(perceptual_roughness: f32) -> f32 {
    let perceptual_roughness = perceptual_roughness_or_min(perceptual_roughness);
    perceptual_roughness * perceptual_roughness
}

pub(in crate::render) fn f0_metallic_roughness(base: Vec3, metallic: f32) -> Vec3 {
    let metallic = clamp_unit(metallic);
    mix_vec3(
        Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0),
        base,
        metallic,
    )
}

pub(in crate::render) fn fresnel_schlick(f0: Vec3, v_dot_h: f32) -> Vec3 {
    let x = (1.0 - v_dot_h).clamp(0.0, 1.0);
    let x2 = x * x;
    let x5 = x * x2 * x2;
    add_vec3(
        f0,
        scale_vec3(subtract_vec3(Vec3::new(1.0, 1.0, 1.0), f0), x5),
    )
}

pub(in crate::render) fn ggx_normal_distribution(n_dot_h: f32, alpha_roughness: f32) -> f32 {
    let alpha_squared = alpha_roughness * alpha_roughness;
    let f = n_dot_h.clamp(0.0, 1.0) * n_dot_h.clamp(0.0, 1.0) * (alpha_squared - 1.0) + 1.0;
    if !f.is_finite() || f <= 0.0 {
        return 0.0;
    }
    alpha_squared / (PI * f * f)
}

/// Correlated Smith GGX visibility term from Khronos Sample Renderer `V_GGX`.
///
/// This is already `G / (4 * NdotL * NdotV)`, so direct specular is
/// `F * D * V` and then the light contribution multiplies by `NdotL`.
pub(in crate::render) fn ggx_visibility_correlated(
    n_dot_l: f32,
    n_dot_v: f32,
    alpha_roughness: f32,
) -> f32 {
    let alpha_squared = alpha_roughness * alpha_roughness;
    let n_dot_l = n_dot_l.clamp(0.0, 1.0);
    let n_dot_v = n_dot_v.clamp(0.0, 1.0);
    let ggx_v = n_dot_l * (n_dot_v * n_dot_v * (1.0 - alpha_squared) + alpha_squared).sqrt();
    let ggx_l = n_dot_v * (n_dot_l * n_dot_l * (1.0 - alpha_squared) + alpha_squared).sqrt();
    let ggx = ggx_v + ggx_l;
    if ggx > 0.0 && ggx.is_finite() {
        0.5 / ggx
    } else {
        0.0
    }
}

pub(in crate::render) fn brdf_specular_ggx(
    alpha_roughness: f32,
    n_dot_l: f32,
    n_dot_v: f32,
    n_dot_h: f32,
) -> f32 {
    ggx_normal_distribution(n_dot_h, alpha_roughness)
        * ggx_visibility_correlated(n_dot_l, n_dot_v, alpha_roughness)
}

/// Analytic split-sum BRDF approximation used by the runtime environment path.
///
/// WebGL2's fragment texture-unit floor leaves no safe room for binding the
/// baked BRDF LUT alongside the existing material/global texture set. Keeping
/// this approximation in the shared contract makes CPU, WebGPU, and WebGL2 use
/// one runtime convention instead of CPU sampling a LUT while GPU approximates.
pub(in crate::render) fn split_sum_brdf_approx(n_dot_v: f32, roughness: f32) -> (f32, f32) {
    let n_dot_v = n_dot_v.clamp(0.0, 1.0);
    let roughness = roughness.clamp(0.0, 1.0);
    let r_x = roughness.mul_add(-1.0, 1.0);
    let r_y = roughness.mul_add(-0.0275, 0.0425);
    let r_z = roughness.mul_add(-0.572, 1.04);
    let r_w = roughness.mul_add(0.022, -0.04);
    let a004 = (r_x * r_x).min(2.0_f32.powf(-9.28 * n_dot_v)) * r_x + r_y;
    ((-1.04_f32).mul_add(a004, r_z), 1.04_f32.mul_add(a004, r_w))
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

fn mix_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    let amount = clamp_unit(amount);
    add_vec3(scale_vec3(left, 1.0 - amount), scale_vec3(right, amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn khronos_correlated_ggx_reference_probes_match_bec106e() {
        assert_close(
            brdf_specular_ggx(0.04, 0.05, 0.05, 1.0),
            15_542.475,
            0.75,
            "grazing low-roughness BRDF",
        );
        assert_close(
            brdf_specular_ggx(0.1764, 0.78446454, 0.9284767, 0.90250033),
            0.07601712,
            0.00001,
            "oblique dielectric BRDF",
        );
        assert_close(
            brdf_specular_ggx(0.6724, 0.9246621, 0.9591663, 0.9596132),
            0.16065252,
            0.00001,
            "rough dielectric BRDF",
        );
    }

    #[test]
    fn fresnel_schlick_matches_khronos_reference_probe() {
        let value = fresnel_schlick(Vec3::new(0.7, 0.72, 0.75), 0.05);
        assert_vec3_close(
            value,
            Vec3::new(0.9321343, 0.9366587, 0.9434452),
            0.00001,
            "metallic grazing fresnel",
        );
    }

    #[test]
    fn split_sum_brdf_approx_matches_shared_wgsl_probe_values() {
        let low = split_sum_brdf_approx(0.2, 0.1);
        assert_close(low.0, 0.6828983, 0.00001, "low roughness scale");
        assert_close(low.1, 0.2621017, 0.00001, "low roughness bias");

        let high = split_sum_brdf_approx(0.8, 0.7);
        assert_close(high.0, 0.6136032, 0.00001, "high roughness scale");
        assert_close(high.1, 0.0013968, 0.00001, "high roughness bias");
    }

    fn assert_close(actual: f32, expected: f32, tolerance: f32, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label} expected {expected}, got {actual}"
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
