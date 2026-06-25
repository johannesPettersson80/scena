use crate::material::Color;
use crate::scene::Vec3;

/// Scalar KHR_materials_transmission / KHR_materials_volume scene-color
/// transmission convention shared by the CPU renderer and the GPU/WebGL2
/// shaders. Keep this file, `output_shader.wgsl`, and
/// `output_shader_texture_2d.wgsl` formula-equivalent: the parity sweep in
/// `tests/transmission_parity.rs` exists to catch drift between them.
const MIN_DENOMINATOR: f32 = 0.0001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedPhysicalTransmission {
    transmission: f32,
    ior: f32,
    thickness: f32,
    attenuation_color: Vec3,
    attenuation_distance: f32,
    roughness: f32,
}

pub(in crate::render) struct PreparedPhysicalTransmissionInput {
    pub(in crate::render) transmission: f32,
    pub(in crate::render) transmission_texture: f32,
    pub(in crate::render) ior: f32,
    pub(in crate::render) thickness: f32,
    pub(in crate::render) thickness_texture: f32,
    pub(in crate::render) attenuation_color: Color,
    pub(in crate::render) attenuation_distance: f32,
    pub(in crate::render) roughness: f32,
}

impl PreparedPhysicalTransmission {
    pub(in crate::render) fn new(input: PreparedPhysicalTransmissionInput) -> Option<Self> {
        let transmission = finite_or(input.transmission, 0.0).clamp(0.0, 1.0)
            * finite_or(input.transmission_texture, 1.0).clamp(0.0, 1.0);
        if transmission <= 0.001 {
            return None;
        }
        let thickness = finite_or(input.thickness, 0.0).max(0.0)
            * finite_or(input.thickness_texture, 1.0).max(0.0);
        Some(Self {
            transmission,
            ior: finite_or(input.ior, 1.5).max(1.01),
            thickness,
            attenuation_color: Vec3::new(
                finite_or(input.attenuation_color.r, 1.0).clamp(0.0, 1.0),
                finite_or(input.attenuation_color.g, 1.0).clamp(0.0, 1.0),
                finite_or(input.attenuation_color.b, 1.0).clamp(0.0, 1.0),
            ),
            attenuation_distance: input.attenuation_distance,
            roughness: finite_or(input.roughness, 1.0).clamp(0.04, 1.0),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::render) struct PhysicalTransmissionInputs {
    pub(in crate::render) frag_coord: [f32; 2],
    pub(in crate::render) viewport: [f32; 2],
    pub(in crate::render) normal: Vec3,
    pub(in crate::render) view: Vec3,
    pub(in crate::render) tint: Vec3,
    pub(in crate::render) surface_rgb: Vec3,
}

pub(in crate::render) fn physical_transmission_color(
    material: PreparedPhysicalTransmission,
    input: PhysicalTransmissionInputs,
    mut sample_scene_color: impl FnMut([f32; 2]) -> Vec3,
) -> Vec3 {
    let viewport = [input.viewport[0].max(1.0), input.viewport[1].max(1.0)];
    let uv = [
        (input.frag_coord[0] / viewport[0]).clamp(0.001, 0.999),
        (input.frag_coord[1] / viewport[1]).clamp(0.001, 0.999),
    ];
    let view_dir = normalize_or(input.view, input.normal);
    let normal_dir = normalize_or(input.normal, Vec3::Y);
    let n_dot_v = normal_dir.dot(view_dir).max(0.0);
    let rim_fresnel = (1.0 - n_dot_v).powi(5);
    let refracted = refract_vec3(-view_dir, normal_dir, material.ior.recip());
    let thickness_scale = 0.004 + material.thickness.min(1.0) * 0.028;
    let refracted_uv = [
        (uv[0] + refracted.x * thickness_scale * material.transmission).clamp(0.001, 0.999),
        (uv[1] - refracted.y * thickness_scale * material.transmission).clamp(0.001, 0.999),
    ];
    let texel = [viewport[0].recip(), viewport[1].recip()];
    let blur_px = material.roughness * material.roughness * 48.0;
    let blur = [texel[0] * blur_px, texel[1] * blur_px];
    let straight = sample_scene_color(uv);
    let refracted_center = sample_scene_color(refracted_uv);
    let refracted_blurred = scale_vec3(refracted_center, 0.36)
        + scale_vec3(
            sample_scene_color([refracted_uv[0] + blur[0], refracted_uv[1]]),
            0.16,
        )
        + scale_vec3(
            sample_scene_color([refracted_uv[0] - blur[0], refracted_uv[1]]),
            0.16,
        )
        + scale_vec3(
            sample_scene_color([refracted_uv[0], refracted_uv[1] + blur[1]]),
            0.16,
        )
        + scale_vec3(
            sample_scene_color([refracted_uv[0], refracted_uv[1] - blur[1]]),
            0.16,
        );
    let refraction_mix = (0.58 + material.roughness * 0.40 + rim_fresnel * 0.10).clamp(0.58, 0.96);
    let scene_color = mix_vec3(straight, refracted_blurred, refraction_mix);
    let tint_strength = (material.transmission * 0.035).clamp(0.0, 0.035);
    let volume_tint = volume_transmittance(
        material.thickness,
        material.attenuation_color,
        material.attenuation_distance,
    );
    let transmitted = multiply_vec3(
        multiply_vec3(scene_color, volume_tint),
        mix_vec3(Vec3::new(1.0, 1.0, 1.0), input.tint, tint_strength),
    );
    let reflection_weight =
        (0.08 + rim_fresnel * 0.42 + (1.0 - material.transmission) * 0.10).clamp(0.08, 0.50);
    let reflected = scale_vec3(
        multiply_vec3(input.surface_rgb, volume_tint),
        1.08 + rim_fresnel * 0.72,
    );
    mix_vec3(transmitted, reflected, reflection_weight)
}

pub(in crate::render) fn volume_transmittance(
    thickness: f32,
    attenuation_color: Vec3,
    attenuation_distance: f32,
) -> Vec3 {
    if thickness <= f32::EPSILON || !attenuation_distance.is_finite() {
        return Vec3::new(1.0, 1.0, 1.0);
    }
    let exponent = thickness / attenuation_distance.max(MIN_DENOMINATOR);
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

fn refract_vec3(incident: Vec3, normal: Vec3, eta: f32) -> Vec3 {
    let n_dot_i = normal.dot(incident);
    let k = 1.0 - eta * eta * (1.0 - n_dot_i * n_dot_i);
    if k < 0.0 {
        Vec3::ZERO
    } else {
        incident * eta - normal * (eta * n_dot_i + k.sqrt())
    }
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length_squared = value.length_squared();
    if length_squared > 0.000_000_01 && length_squared.is_finite() {
        value * length_squared.sqrt().recip()
    } else {
        fallback
    }
}

fn mix_vec3(left: Vec3, right: Vec3, t: f32) -> Vec3 {
    left * (1.0 - t) + right * t
}

fn multiply_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x * right.x, left.y * right.y, left.z * right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_transmittance_matches_beer_lambert_probe() {
        let tint = volume_transmittance(0.42, Vec3::new(0.35, 0.58, 1.0), 0.8);
        assert!((tint.x - 0.576_282_86).abs() <= 0.000_001);
        assert!((tint.y - 0.751_276_3).abs() <= 0.000_001);
        assert_eq!(tint.z, 1.0);
    }

    // ---- External ground-truth oracles (closed-form analytic results, not CPU↔GPU
    // parity and not scena's own rendered output). ----

    #[test]
    fn volume_transmittance_satisfies_beer_lambert_closed_form() {
        // KHR_materials_volume: T = attenuation_color^(thickness/attenuation_distance).
        let color = Vec3::new(0.35, 0.58, 0.9);
        let distance = 0.8_f32;
        // No thickness -> no attenuation.
        assert_eq!(
            volume_transmittance(0.0, color, distance),
            Vec3::new(1.0, 1.0, 1.0)
        );
        // Exactly one attenuation-distance of travel -> the attenuation color.
        let at_distance = volume_transmittance(distance, color, distance);
        assert!((at_distance.x - color.x).abs() <= 1.0e-6);
        assert!((at_distance.y - color.y).abs() <= 1.0e-6);
        assert!((at_distance.z - color.z).abs() <= 1.0e-6);
        // General closed form + strictly decreasing with thickness.
        let mut previous = 1.0_f32;
        for &thickness in &[0.2_f32, 0.5, 1.0, 2.0] {
            let t = volume_transmittance(thickness, color, distance);
            let expected = color.x.powf(thickness / distance);
            assert!(
                (t.x - expected).abs() <= 1.0e-6,
                "Beer-Lambert closed form at thickness {thickness}"
            );
            assert!(
                t.x < previous,
                "transmittance must strictly decrease with thickness"
            );
            previous = t.x;
        }
    }

    #[test]
    fn refract_vec3_matches_snells_law() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        // Normal incidence (air->glass): the ray passes straight through, unbent.
        let straight = refract_vec3(Vec3::new(0.0, 0.0, -1.0), normal, 1.0 / 1.5);
        assert!(straight.x.abs() <= 1.0e-6 && straight.y.abs() <= 1.0e-6);
        assert!(
            (straight.z + 1.0).abs() <= 1.0e-6,
            "normal incidence is unbent"
        );
        // Oblique air->glass: compare against the independently computed Snell
        // refraction direction, sin(theta_t) = eta * sin(theta_i).
        let eta = 1.0 / 1.5;
        for &theta_i in &[0.3_f32, 0.6, 1.0] {
            let incident = Vec3::new(theta_i.sin(), 0.0, -theta_i.cos());
            let r = refract_vec3(incident, normal, eta);
            let sin_t = eta * theta_i.sin();
            let cos_t = (1.0 - sin_t * sin_t).sqrt();
            assert!(
                (r.x - sin_t).abs() <= 1.0e-5 && (r.z + cos_t).abs() <= 1.0e-5,
                "Snell refraction direction at theta_i={theta_i}"
            );
        }
        // Total internal reflection (glass->air past the critical angle) -> no ray.
        let theta = 60.0_f32.to_radians();
        let tir = refract_vec3(Vec3::new(theta.sin(), 0.0, -theta.cos()), normal, 1.5);
        assert_eq!(tir, Vec3::ZERO, "TIR returns zero (no transmitted ray)");
    }

    #[test]
    fn scene_color_transmission_uses_refraction_volume_and_reflection_terms() {
        let material = PreparedPhysicalTransmission::new(PreparedPhysicalTransmissionInput {
            transmission: 1.0,
            transmission_texture: 1.0,
            ior: 1.48,
            thickness: 0.42,
            thickness_texture: 1.0,
            attenuation_color: Color::from_linear_rgb(0.35, 0.58, 1.0),
            attenuation_distance: 0.8,
            roughness: 0.08,
        })
        .expect("transmission material is valid");
        let color = physical_transmission_color(
            material,
            PhysicalTransmissionInputs {
                frag_coord: [48.0, 32.0],
                viewport: [96.0, 64.0],
                normal: Vec3::Z,
                view: Vec3::Z,
                tint: Vec3::new(0.82, 0.88, 1.0),
                surface_rgb: Vec3::new(0.45, 0.50, 0.65),
            },
            |uv| Vec3::new(uv[0], 1.0 - uv[0], uv[1]),
        );
        assert!(
            color.z > color.x && color.y > color.x,
            "volume-tinted transmission should preserve sampled scene structure with blue-biased glass: {color:?}"
        );
    }
}
