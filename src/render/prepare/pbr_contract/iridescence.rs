//! KHR_materials_iridescence: the Rust side of `iridescence_fresnel` and
//! `iridescence_sensitivity` in `src/render/gpu/surface_lighting.wgsl`.

use std::f32::consts::PI;

use crate::scene::Vec3;

use super::f0_from_ior;

/// Thin-film interference (Belcour and Barla 2017), as specified by
/// KHR_materials_iridescence. Returns the reflectance of a film of
/// `thickness_nm` and index `film_ior_input` over a base with reflectance
/// `base_f0`, seen at `cos_theta` from air.
pub(super) fn iridescence_fresnel(
    film_ior_input: f32,
    cos_theta: f32,
    thickness_nm: f32,
    base_f0: Vec3,
) -> Vec3 {
    // A vanishing film must not leave a hard interface behind.
    let film_ior = mix(1.0, film_ior_input, smoothstep(0.0, 0.03, thickness_nm));
    let sin_theta2_squared = (1.0 / (film_ior * film_ior)) * (1.0 - cos_theta * cos_theta);
    let cos_theta2_squared = 1.0 - sin_theta2_squared;
    if cos_theta2_squared < 0.0 {
        return Vec3::ONE;
    }
    let cos_theta2 = cos_theta2_squared.sqrt();

    // Air to film.
    let r0 = f0_from_ior(film_ior);
    let r12 = r0 + (1.0 - r0) * pow5(1.0 - cos_theta);
    let t121 = 1.0 - r12;
    let phi21 = if film_ior < 1.0 { 0.0 } else { PI };

    let optical_path_difference = 2.0 * film_ior * thickness_nm * cos_theta2;
    let film_to_base = |f0: f32| {
        let sqrt_f0 = f0.clamp(0.0, 0.9999).sqrt();
        let base_ior = (1.0 + sqrt_f0) / (1.0 - sqrt_f0);
        let base_ratio = (base_ior - film_ior) / (base_ior + film_ior);
        let r1 = base_ratio * base_ratio;
        let r23 = r1 + (1.0 - r1) * pow5(1.0 - cos_theta2);
        let phi23 = if base_ior < film_ior { PI } else { 0.0 };
        (r23, phi21 + phi23)
    };
    let (r23_r, phi_r) = film_to_base(base_f0.x);
    let (r23_g, phi_g) = film_to_base(base_f0.y);
    let (r23_b, phi_b) = film_to_base(base_f0.z);
    let r23 = Vec3::new(r23_r, r23_g, r23_b);
    let phi = Vec3::new(phi_r, phi_g, phi_b);

    let r123 = (r23 * r12).clamp(Vec3::splat(0.000_01), Vec3::splat(0.9999));
    let r123_sqrt = Vec3::new(r123.x.sqrt(), r123.y.sqrt(), r123.z.sqrt());
    let rs = r23 * (t121 * t121) / (Vec3::ONE - r123);

    let mut reflectance = Vec3::splat(r12) + rs;
    let mut cm = rs - Vec3::splat(t121);
    cm *= r123_sqrt;
    reflectance += cm * 2.0 * iridescence_sensitivity(optical_path_difference, phi);
    cm *= r123_sqrt;
    reflectance += cm * 2.0 * iridescence_sensitivity(2.0 * optical_path_difference, phi * 2.0);
    reflectance.max(Vec3::ZERO)
}

/// CIE XYZ sensitivity curves evaluated in Fourier space, converted to linear
/// Rec.709.
pub(super) fn iridescence_sensitivity(optical_path_difference: f32, shift: Vec3) -> Vec3 {
    let phase = 2.0 * PI * optical_path_difference * 1.0e-9;
    let value = Vec3::new(5.4856e-13, 4.4201e-13, 5.2481e-13);
    let position = Vec3::new(1.6810e+06, 1.7953e+06, 2.2084e+06);
    let variance = Vec3::new(4.3278e+09, 9.3046e+09, 6.6121e+09);
    let lobe = |value: f32, position: f32, variance: f32, shift: f32| {
        value
            * (2.0 * PI * variance).sqrt()
            * (position * phase + shift).cos()
            * (-phase * phase * variance).exp()
    };
    let mut xyz = Vec3::new(
        lobe(value.x, position.x, variance.x, shift.x),
        lobe(value.y, position.y, variance.y, shift.y),
        lobe(value.z, position.z, variance.z, shift.z),
    );
    xyz.x += 9.7470e-14
        * (2.0 * PI * 4.5282e+09_f32).sqrt()
        * (2.2399e+06 * phase + shift.x).cos()
        * (-4.5282e+09 * phase * phase).exp();
    xyz /= 1.0685e-7;
    Vec3::new(
        Vec3::new(3.240_454_2, -1.537_138_5, -0.498_531_4).dot(xyz),
        Vec3::new(-0.969_266, 1.876_010_8, 0.041_556).dot(xyz),
        Vec3::new(0.055_643_4, -0.204_025_9, 1.057_225_2).dot(xyz),
    )
}

fn pow5(value: f32) -> f32 {
    let squared = value * value;
    squared * squared * value
}

fn mix(left: f32, right: f32, amount: f32) -> f32 {
    left * (1.0 - amount) + right * amount
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values are double-precision evaluations of the model as listed
    // in KHR_materials_iridescence, computed outside this crate.

    #[test]
    fn sensitivity_matches_reference_values() {
        assert_vec3_close(
            iridescence_sensitivity(0.0, Vec3::ZERO),
            Vec3::new(1.205_368_8, 0.948_319_5, 0.910_009_7),
            2.0e-5,
            "zero path difference",
        );
        assert_vec3_close(
            iridescence_sensitivity(800.0, Vec3::splat(PI)),
            Vec3::new(0.189_067_2, 0.984_115_5, -0.220_752_6),
            5.0e-5,
            "800 nm path difference, half-wave shift",
        );
    }

    #[test]
    fn thin_film_fresnel_matches_reference_values() {
        let dielectric = Vec3::splat(0.04);
        for (film_ior, cos_theta, thickness_nm, base_f0, expected) in [
            (
                1.3,
                0.8,
                400.0,
                dielectric,
                Vec3::new(0.0, 0.019_193_2, 0.038_314_7),
            ),
            (
                1.3,
                0.3,
                400.0,
                dielectric,
                Vec3::new(0.261_154_4, 0.164_662_9, 0.133_352_2),
            ),
            (
                1.3,
                1.0,
                400.0,
                dielectric,
                Vec3::new(0.007_456_5, 0.038_351_7, 0.016_038_2),
            ),
            (
                1.3,
                0.8,
                250.0,
                Vec3::new(0.58, 0.61, 0.65),
                Vec3::new(0.593_683_1, 0.604_333_7, 0.556_842_7),
            ),
            (
                1.8,
                0.6,
                550.0,
                Vec3::new(0.95, 0.64, 0.54),
                Vec3::new(0.982_258_4, 0.413_807_8, 0.454_307_7),
            ),
        ] {
            assert_vec3_close(
                iridescence_fresnel(film_ior, cos_theta, thickness_nm, base_f0),
                expected,
                2.0e-4,
                &format!(
                    "film ior {film_ior}, cos {cos_theta}, {thickness_nm} nm over {base_f0:?}"
                ),
            );
        }
    }

    #[test]
    fn a_film_of_zero_thickness_reduces_to_the_plain_fresnel_term() {
        // With no thickness the film's index fades to that of air and the path
        // difference vanishes. What remains differs from Schlick only by the
        // two-term truncation of the interference series, which the reference
        // model shares: under one percent of reflectance.
        let base_f0 = Vec3::splat(0.04);
        for (cos_theta, schlick, expected) in [
            (
                1.0_f32,
                0.04_f32,
                Vec3::new(0.047_295_7, 0.045_740_0, 0.045_508_1),
            ),
            (
                0.8,
                0.040_307_2,
                Vec3::new(0.048_878_5, 0.047_113_5, 0.046_850_5),
            ),
        ] {
            let actual = iridescence_fresnel(1.3, cos_theta, 0.0, base_f0);
            assert_vec3_close(actual, expected, 2.0e-5, "zero-thickness film");
            assert_vec3_close(
                actual,
                Vec3::splat(schlick),
                0.01,
                "zero-thickness film against plain Schlick",
            );
        }
        // The film index must not matter once the film is gone.
        assert_vec3_close(
            iridescence_fresnel(2.4, 0.8, 0.0, base_f0),
            iridescence_fresnel(1.3, 0.8, 0.0, base_f0),
            1.0e-6,
            "film index at zero thickness",
        );
    }

    #[test]
    fn film_thickness_shifts_the_reflected_hue() {
        let base_f0 = Vec3::splat(0.04);
        // 300 nm reflects red (0.045, 0.018, 0.005); 400 nm reflects blue
        // (0.000, 0.019, 0.038).
        let thin = iridescence_fresnel(1.3, 0.8, 300.0, base_f0);
        let thick = iridescence_fresnel(1.3, 0.8, 400.0, base_f0);
        let dominant = |value: Vec3| {
            if value.x >= value.y && value.x >= value.z {
                0
            } else if value.y >= value.z {
                1
            } else {
                2
            }
        };
        assert_ne!(
            dominant(thin),
            dominant(thick),
            "thickness must move the interference colour: {thin:?} vs {thick:?}"
        );
    }

    fn assert_vec3_close(actual: Vec3, expected: Vec3, tolerance: f32, label: &str) {
        let delta = (actual - expected).abs();
        assert!(
            delta.max_element() <= tolerance,
            "{label}: expected {expected:?}, got {actual:?}, delta {delta:?}"
        );
    }
}
