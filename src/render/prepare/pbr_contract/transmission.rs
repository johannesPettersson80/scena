use super::{
    MIN_DENOMINATOR, MIN_N_DOT_V, PbrMaterial, dot_vec3, finite_non_negative, fresnel_schlick,
    multiply_vec3, normalize_or, scale_vec3,
};
use crate::scene::Vec3;

#[allow(clippy::too_many_arguments)]
pub(in crate::render::prepare) fn transmission_volume_light_contribution(
    material: PbrMaterial,
    normal: Vec3,
    view: Vec3,
    incoming: Vec3,
    radiance: Vec3,
    factor: f32,
    ior: f32,
    thickness_factor: f32,
    thickness_texture: f32,
    attenuation_color: Vec3,
    attenuation_distance: f32,
) -> Vec3 {
    let factor = finite_non_negative(factor).clamp(0.0, 1.0) * (1.0 - material.metallic);
    if factor <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let normal = normalize_or(normal, Vec3::new(0.0, 0.0, 1.0));
    let view = normalize_or(view, normal);
    let incoming = normalize_or(incoming, Vec3::ZERO);
    let n_dot_l = dot_vec3(normal, incoming).abs();
    if n_dot_l <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let n_dot_v = dot_vec3(normal, view).abs().max(MIN_N_DOT_V);
    let ior = if ior.is_finite() && (ior == 0.0 || ior >= 1.0) {
        if ior == 0.0 { 1.5 } else { ior }
    } else {
        1.5
    };
    let f0 = f0_from_ior(ior);
    let fresnel = fresnel_schlick(n_dot_v, Vec3::new(f0, f0, f0));
    let transmittance = volume_transmittance(
        finite_non_negative(thickness_factor) * finite_non_negative(thickness_texture),
        attenuation_color,
        attenuation_distance,
    );
    let roughness_weight = (1.0 - material.roughness * 0.35).clamp(0.35, 1.0);
    let energy = scale_vec3(
        multiply_vec3(
            multiply_vec3(material.base, transmittance),
            Vec3::new(1.0 - fresnel.x, 1.0 - fresnel.y, 1.0 - fresnel.z),
        ),
        factor * roughness_weight * n_dot_l,
    );
    multiply_vec3(energy, radiance)
}

fn volume_transmittance(
    thickness: f32,
    attenuation_color: Vec3,
    attenuation_distance: f32,
) -> Vec3 {
    if thickness <= f32::EPSILON || !attenuation_distance.is_finite() {
        return Vec3::new(1.0, 1.0, 1.0);
    }
    let distance = attenuation_distance.max(MIN_DENOMINATOR);
    let exponent = thickness / distance;
    Vec3::new(
        attenuation_channel(attenuation_color.x, exponent),
        attenuation_channel(attenuation_color.y, exponent),
        attenuation_channel(attenuation_color.z, exponent),
    )
}

fn attenuation_channel(value: f32, exponent: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0).powf(exponent)
    } else {
        1.0
    }
}

fn f0_from_ior(ior: f32) -> f32 {
    let ratio = (ior - 1.0) / (ior + 1.0).max(MIN_DENOMINATOR);
    ratio * ratio
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transmission_volume_uses_factor_ior_thickness_and_attenuation() {
        let material = PbrMaterial::new(Vec3::new(0.9, 0.95, 1.0), 0.0, 0.08);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let view = normal;
        let incoming = normal;
        let radiance = Vec3::new(1.0, 1.0, 1.0);
        let off = transmission_volume_light_contribution(
            material,
            normal,
            view,
            incoming,
            radiance,
            0.0,
            1.5,
            2.0,
            1.0,
            Vec3::new(0.08, 0.35, 1.0),
            1.0,
        );
        let on = transmission_volume_light_contribution(
            material,
            normal,
            view,
            incoming,
            radiance,
            1.0,
            1.7,
            2.0,
            1.0,
            Vec3::new(0.08, 0.35, 1.0),
            1.0,
        );

        assert_eq!(off, Vec3::ZERO);
        assert!(
            on.z > on.x && on.z > on.y,
            "volume attenuation should tint transmitted glass toward the least-attenuated channel"
        );
    }
}
