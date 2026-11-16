//! KHR_materials_sheen lobe: the Rust side of the sheen functions in
//! `src/render/gpu/surface_lighting.wgsl`.

use std::f32::consts::PI;

/// Smallest sheen roughness the Charlie distribution is evaluated with.
pub(in crate::render::prepare) const MIN_SHEEN_ROUGHNESS: f32 = 0.07;
/// Share of the base layer's energy a white sheen layer reflects away.
pub(in crate::render::prepare) const SHEEN_ENERGY_COMPENSATION: f32 = 0.157;

/// Estevez and Kulla "Charlie" sheen distribution.
pub(super) fn distribution_charlie(alpha: f32, n_dot_h: f32) -> f32 {
    let inverse_alpha = 1.0 / alpha.max(0.0001);
    let sin_squared = (1.0 - n_dot_h * n_dot_h).max(0.007_812_5);
    (2.0 + inverse_alpha) * sin_squared.powf(inverse_alpha * 0.5) / (2.0 * PI)
}

/// Neubelt and Pettineo cloth visibility term.
pub(super) fn visibility_neubelt(n_dot_l: f32, n_dot_v: f32) -> f32 {
    (1.0 / (4.0 * (n_dot_l + n_dot_v - n_dot_l * n_dot_v))).clamp(0.0, 1.0)
}

/// Analytic fit of the sheen directional albedo used for image-based lighting.
pub(in crate::render::prepare) fn sheen_environment_brdf(n_dot_v: f32, roughness: f32) -> f32 {
    let r2 = roughness * roughness;
    let (a, b, offset) = if roughness < 0.25 {
        (
            -339.2 * r2 + 161.4 * roughness - 25.9,
            44.0 * r2 - 23.7 * roughness + 3.26,
            0.0,
        )
    } else {
        (
            -8.48 * r2 + 14.3 * roughness - 9.95,
            1.97 * r2 - 3.27 * roughness + 0.72,
            0.1 * (roughness - 0.25),
        )
    };
    let dg = (a * n_dot_v + b).exp() + offset;
    (dg / PI).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values are double-precision evaluations of the published
    // formulas, computed outside this crate.

    #[test]
    fn charlie_distribution_matches_estevez_kulla_reference_values() {
        for (alpha, n_dot_h, expected) in [
            (0.25, 0.6, 0.391_139_2),
            (0.2025, 0.95, 0.003_522_0),
            (0.0049, 0.3, 0.002_169_7),
            (1.0, 0.0, 0.477_464_8),
        ] {
            assert_close(
                distribution_charlie(alpha, n_dot_h),
                expected,
                2.0e-6,
                &format!("Charlie D at alpha {alpha}, n.h {n_dot_h}"),
            );
        }
        // The grazing clamp keeps the lobe finite where sin(theta) reaches zero.
        assert_close(
            distribution_charlie(0.5, 1.0),
            0.004_973_6,
            2.0e-6,
            "Charlie D at the clamped pole",
        );
    }

    #[test]
    fn neubelt_visibility_matches_reference_values_and_saturates_at_grazing() {
        assert_close(visibility_neubelt(0.8, 0.6), 0.271_739_1, 1.0e-6, "oblique");
        assert_close(
            visibility_neubelt(1.0, 1.0),
            0.25,
            1.0e-6,
            "normal incidence",
        );
        assert_close(visibility_neubelt(0.3, 0.9), 0.268_817_2, 1.0e-6, "mixed");
        assert_eq!(
            visibility_neubelt(0.05, 0.05),
            1.0,
            "grazing visibility is clamped so the lobe cannot diverge"
        );
    }

    #[test]
    fn sheen_environment_albedo_matches_the_fit_on_both_roughness_branches() {
        for (n_dot_v, roughness, expected) in [
            (0.9, 0.45, 0.008_382_9),
            (0.2, 0.45, 0.084_937_8),
            (0.5, 0.1, 0.001_677_0),
            (0.05, 1.0, 0.168_843),
            (0.6, 0.25, 0.005_184_4),
            (0.6, 0.2499, 0.006_041_7),
        ] {
            assert_close(
                sheen_environment_brdf(n_dot_v, roughness),
                expected,
                2.0e-6,
                &format!("sheen albedo at n.v {n_dot_v}, roughness {roughness}"),
            );
        }
    }

    #[test]
    fn sheen_energy_compensation_matches_the_shader_constants() {
        // `surface_lighting.wgsl` declares the same two constants; the layer
        // model is only one model if both sides use them.
        let shader = include_str!("../../gpu/surface_lighting.wgsl");
        assert!(shader.contains(&format!(
            "const SHEEN_ENERGY_COMPENSATION: f32 = {SHEEN_ENERGY_COMPENSATION};"
        )));
        assert!(shader.contains(&format!(
            "const MIN_SHEEN_ROUGHNESS: f32 = {MIN_SHEEN_ROUGHNESS};"
        )));
    }

    fn assert_close(actual: f32, expected: f32, tolerance: f32, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: expected {expected}, got {actual}"
        );
    }
}
