use super::{
    MIN_DENOMINATOR, MIN_N_DOT_V, PbrMaterial, add_vec3, distribution_ggx, dot_vec3,
    finite_non_negative, fresnel_schlick, geometry_smith, multiply_vec3, normalize_or,
    roughness_or_min, scale_vec3,
};
use crate::scene::Vec3;

pub(in crate::render::prepare) fn dispersion_light_contribution(
    material: PbrMaterial,
    normal: Vec3,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
    factor: f32,
    ior: f32,
) -> Vec3 {
    let factor = finite_non_negative(factor);
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
    let fresnel = fresnel_schlick(v_dot_h, dispersion_f0_from_ior(ior, factor));
    let specular = scale_vec3(
        fresnel,
        distribution * geometry * factor / (4.0 * n_dot_v * n_dot_l).max(MIN_DENOMINATOR),
    );
    scale_vec3(multiply_vec3(specular, radiance), n_dot_l)
}

fn dispersion_f0_from_ior(ior: f32, dispersion: f32) -> Vec3 {
    let ior = if ior.is_finite() && ior >= 1.0 {
        ior
    } else {
        1.5
    };
    let half_spread = (ior - 1.0) * 0.025 * finite_non_negative(dispersion);
    Vec3::new(
        f0_from_ior((ior - half_spread).max(1.0)),
        f0_from_ior(ior),
        f0_from_ior(ior + half_spread),
    )
}

fn f0_from_ior(ior: f32) -> f32 {
    let ratio = (ior - 1.0) / (ior + 1.0).max(MIN_DENOMINATOR);
    ratio * ratio
}

#[cfg(test)]
mod tests {
    use super::super::max_component;
    use super::*;

    #[test]
    fn dispersion_light_contribution_uses_factor_and_ior_spread() {
        let material = PbrMaterial::new(Vec3::new(0.72, 0.72, 0.72), 0.0, 0.24);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view = normalize_or(Vec3::new(0.25, 0.0, 1.0), normal);
        let incoming = normalize_or(Vec3::new(0.1, 0.0, 1.0), normal);
        let radiance = Vec3::new(1.0, 1.0, 1.0);
        let off =
            dispersion_light_contribution(material, normal, view, incoming, radiance, 0.0, 1.5);
        let on =
            dispersion_light_contribution(material, normal, view, incoming, radiance, 1.0, 1.5);

        assert_eq!(off, Vec3::ZERO);
        assert!(
            max_component(on) > 0.0,
            "dispersion should add a channel-separated specular contribution"
        );
        assert_ne!(
            dominant_channel(on),
            1,
            "KHR_materials_dispersion must shift red/blue channels around the green IOR lane"
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
