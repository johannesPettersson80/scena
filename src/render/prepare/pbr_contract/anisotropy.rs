//! KHR_materials_anisotropy: the Rust side of the anisotropy functions in
//! `src/render/gpu/surface_lighting.wgsl`.

use std::f32::consts::PI;

use crate::scene::Vec3;

use super::{
    MIN_DENOMINATOR, add_vec3, cross_vec3, dot_vec3, length_vec3, normalize_or, scale_vec3,
    subtract_vec3,
};

/// Unit tangent of the anisotropic highlight in world space: the mesh tangent
/// made perpendicular to `normal`, then turned by the authored direction and
/// rotation within the tangent plane.
pub(super) fn surface_anisotropy_tangent(
    normal: Vec3,
    tangent: Vec3,
    tangent_handedness: f32,
    direction: (f32, f32),
    rotation: f32,
) -> Vec3 {
    let safe_tangent = normalize_or(
        subtract_vec3(tangent, scale_vec3(normal, dot_vec3(tangent, normal))),
        fallback_tangent(normal),
    );
    let handedness = if tangent_handedness >= 0.0 { 1.0 } else { -1.0 };
    let bitangent = normalize_or(
        scale_vec3(cross_vec3(normal, safe_tangent), handedness),
        fallback_tangent(normal),
    );
    let rotated = rotated_anisotropy_direction(direction, rotation);
    normalize_or(
        add_vec3(
            scale_vec3(safe_tangent, rotated.0),
            scale_vec3(bitangent, rotated.1),
        ),
        safe_tangent,
    )
}

pub(super) fn distribution_ggx_anisotropic(
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
pub(super) fn visibility_ggx_anisotropic(
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

pub(super) fn rotated_anisotropy_direction(direction: (f32, f32), rotation: f32) -> (f32, f32) {
    let length = direction.0.hypot(direction.1);
    let (x, y) = if length <= MIN_DENOMINATOR || !length.is_finite() {
        (1.0, 0.0)
    } else {
        (direction.0 / length, direction.1 / length)
    };
    let rotation = if rotation.is_finite() { rotation } else { 0.0 };
    let (sin, cos) = rotation.sin_cos();
    (x * cos - y * sin, x * sin + y * cos)
}

/// Normal the environment is mirrored about for an anisotropic surface: bent
/// toward the anisotropic bitangent so the reflection stretches along the
/// grooves. `anisotropy` is the strength in `(0, 1]`.
pub(super) fn anisotropic_reflection_normal(
    normal: Vec3,
    view: Vec3,
    anisotropy_b: Vec3,
    anisotropy: f32,
    roughness: f32,
) -> Vec3 {
    let groove_tangent = cross_vec3(anisotropy_b, view);
    let groove_normal = cross_vec3(groove_tangent, anisotropy_b);
    let bend = 1.0 - anisotropy * (1.0 - roughness);
    let keep = bend * bend * bend * bend;
    normalize_or(
        add_vec3(
            scale_vec3(groove_normal, 1.0 - keep),
            scale_vec3(normal, keep),
        ),
        normal,
    )
}

fn fallback_tangent(normal: Vec3) -> Vec3 {
    let axis = if normal.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    normalize_or(cross_vec3(axis, normal), Vec3::new(1.0, 0.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::pbr_brdf;

    // Expected values are double-precision evaluations of the published
    // formulas, computed outside this crate.

    #[test]
    fn anisotropic_ggx_matches_reference_values() {
        let n_dot_h = (1.0_f32 - 0.13).sqrt();
        assert_close(
            distribution_ggx_anisotropic(n_dot_h, 0.3, 0.2, 0.6, 0.2),
            0.590_197_2,
            2.0e-6,
            "anisotropic D",
        );
        assert_close(
            visibility_ggx_anisotropic(0.7, 0.5, 0.3, 0.4, -0.2, 0.6, 0.6, 0.2),
            0.666_101_8,
            2.0e-6,
            "anisotropic V",
        );
    }

    #[test]
    fn equal_alphas_reduce_to_the_isotropic_ggx_lobe() {
        // Frame: tangent +X, bitangent +Y, normal +Z.
        for (alpha, view, light) in [
            (
                0.1764_f32,
                Vec3::new(0.6, 0.0, 0.8),
                Vec3::new(-0.8, 0.0, 0.6),
            ),
            (
                0.09,
                Vec3::new(0.3, 0.4, 0.866_025_4),
                Vec3::new(-0.5, 0.2, 0.842_614_9),
            ),
            (
                0.64,
                Vec3::new(0.0, 0.7, 0.714_142_8),
                Vec3::new(0.6, -0.3, 0.741_619_8),
            ),
        ] {
            let half = normalize_or(add_vec3(view, light), Vec3::new(0.0, 0.0, 1.0));
            let anisotropic = distribution_ggx_anisotropic(half.z, half.x, half.y, alpha, alpha)
                * visibility_ggx_anisotropic(
                    light.z, view.z, view.y, view.x, light.x, light.y, alpha, alpha,
                );
            let isotropic = pbr_brdf::brdf_specular_ggx(alpha, light.z, view.z, half.z);
            assert!(
                (anisotropic - isotropic).abs() <= isotropic * 2.0e-5,
                "alpha {alpha}: anisotropic {anisotropic} must equal isotropic {isotropic}"
            );
        }
    }

    #[test]
    fn direction_is_normalized_and_rotated_in_the_tangent_plane() {
        let (x, y) = rotated_anisotropy_direction((2.0, 0.0), 0.0);
        assert_close(x, 1.0, 1.0e-6, "unrotated x");
        assert_close(y, 0.0, 1.0e-6, "unrotated y");
        let (x, y) = rotated_anisotropy_direction((1.0, 0.0), std::f32::consts::FRAC_PI_2);
        assert_close(x, 0.0, 1.0e-6, "quarter-turn x");
        assert_close(y, 1.0, 1.0e-6, "quarter-turn y");
        assert_eq!(
            rotated_anisotropy_direction((0.0, 0.0), 0.0),
            (1.0, 0.0),
            "a missing direction means along the tangent"
        );
    }

    #[test]
    fn anisotropy_tangent_is_perpendicular_to_the_normal_and_follows_handedness() {
        let normal = normalize_or(Vec3::new(0.2, 0.1, 1.0), Vec3::new(0.0, 0.0, 1.0));
        let mesh_tangent = Vec3::new(1.0, 0.0, 0.0);
        let along = surface_anisotropy_tangent(normal, mesh_tangent, 1.0, (1.0, 0.0), 0.0);
        assert_close(dot_vec3(along, normal), 0.0, 1.0e-6, "tangent . normal");
        assert_close(length_vec3(along), 1.0, 1.0e-6, "unit tangent");
        assert!(
            along.x > 0.9,
            "direction (1, 0) stays along the mesh tangent"
        );

        let quarter = std::f32::consts::FRAC_PI_2;
        let right_handed =
            surface_anisotropy_tangent(normal, mesh_tangent, 1.0, (1.0, 0.0), quarter);
        let left_handed =
            surface_anisotropy_tangent(normal, mesh_tangent, -1.0, (1.0, 0.0), quarter);
        let bitangent = cross_vec3(normal, along);
        assert_close(dot_vec3(right_handed, bitangent), 1.0, 1.0e-5, "+bitangent");
        assert_close(dot_vec3(left_handed, bitangent), -1.0, 1.0e-5, "-bitangent");
    }

    #[test]
    fn reflection_normal_bends_toward_the_grooves_only_as_far_as_strength_and_gloss_allow() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let bitangent = Vec3::new(0.0, 1.0, 0.0);
        let view = normalize_or(Vec3::new(0.3, 0.5, 0.8), normal);

        // A fully rough surface keeps its normal whatever the strength.
        let rough = anisotropic_reflection_normal(normal, view, bitangent, 1.0, 1.0);
        assert_close(dot_vec3(rough, normal), 1.0, 1.0e-6, "rough surface");

        // Full strength on a mirror: the normal becomes the view direction
        // with its component along the grooves removed.
        let bent = anisotropic_reflection_normal(normal, view, bitangent, 1.0, 0.0);
        let expected = normalize_or(Vec3::new(view.x, 0.0, view.z), normal);
        assert_close(
            dot_vec3(bent, expected),
            1.0,
            1.0e-6,
            "mirror at full strength",
        );
        assert_close(
            dot_vec3(bent, bitangent),
            0.0,
            1.0e-6,
            "no groove component",
        );
    }

    fn assert_close(actual: f32, expected: f32, tolerance: f32, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: expected {expected}, got {actual}"
        );
    }
}
