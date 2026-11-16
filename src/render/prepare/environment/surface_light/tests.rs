use std::sync::Arc;

use crate::scene::Vec3;

use super::super::super::super::pbr_brdf;
use super::super::super::pbr_contract::{PbrMaterial, Surface, sheen_environment_brdf};
use super::super::{
    DiffuseIrradianceSh, PreparedEnvironmentCubemap, PreparedEnvironmentLighting,
    rotate_environment_y, sample_prefiltered_specular,
};

/// Image-based lighting exactly as the CPU renderer evaluated it before the
/// layered surface model existed (commit 4da02d2), written out with plain
/// component arithmetic. A surface without layers must reproduce its bits.
fn pre_port_pbr_contribution(
    lighting: &PreparedEnvironmentLighting,
    material: PbrMaterial,
    normal: Vec3,
    view: Vec3,
) -> Vec3 {
    if !lighting.is_active() {
        return Vec3::ZERO;
    }
    let dot = |a: Vec3, b: Vec3| a.x * b.x + a.y * b.y + a.z * b.z;
    let diffuse_irradiance = lighting.diffuse_radiance(normal);
    let incident = Vec3::new(-view.x, -view.y, -view.z);
    let twice = 2.0 * dot(incident, normal);
    let mirrored = Vec3::new(
        incident.x - normal.x * twice,
        incident.y - normal.y * twice,
        incident.z - normal.z * twice,
    );
    let reflection = rotate_environment_y(mirrored, lighting.rotation_y_radians);
    let prefiltered = lighting
        .cubemap
        .as_deref()
        .map(|cubemap| sample_prefiltered_specular(cubemap, reflection, material.roughness))
        .unwrap_or(lighting.specular_rgb);
    let brdf = if lighting.cubemap.is_some() {
        pbr_brdf::split_sum_brdf_approx(dot(normal, view), material.roughness)
    } else {
        (1.0, 0.0)
    };

    let n_dot_v = dot(normal, view).max(pbr_brdf::MIN_N_DOT_V);
    let f0 = material.f0();
    let fresnel = pbr_brdf::fresnel_schlick(f0, n_dot_v);
    let keep = 1.0 - material.metallic;
    let diffuse_energy = Vec3::new(
        (1.0 - fresnel.x) * keep,
        (1.0 - fresnel.y) * keep,
        (1.0 - fresnel.z) * keep,
    );
    let diffuse = Vec3::new(
        diffuse_energy.x * material.base.x * diffuse_irradiance.x,
        diffuse_energy.y * material.base.y * diffuse_irradiance.y,
        diffuse_energy.z * material.base.z * diffuse_irradiance.z,
    );
    let specular_factor = Vec3::new(
        f0.x * brdf.0 + brdf.1,
        f0.y * brdf.0 + brdf.1,
        f0.z * brdf.0 + brdf.1,
    );
    let lit = Vec3::new(
        diffuse.x + prefiltered.x * specular_factor.x,
        diffuse.y + prefiltered.y * specular_factor.y,
        diffuse.z + prefiltered.z * specular_factor.z,
    );
    Vec3::new(
        lit.x * lighting.intensity,
        lit.y * lighting.intensity,
        lit.z * lighting.intensity,
    )
}

struct Sequence(u64);

impl Sequence {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / (1_u64 << 24) as f32
    }

    fn direction(&mut self) -> Vec3 {
        let raw = Vec3::new(
            self.next() * 2.0 - 1.0,
            self.next() * 2.0 - 1.0,
            self.next() * 2.0 - 1.0,
        );
        let length = raw.length();
        if length > 1.0e-3 {
            raw / length
        } else {
            Vec3::Z
        }
    }
}

/// A cubemap whose faces and texels all differ, so a wrong reflection
/// direction or roughness cannot go unnoticed.
fn structured_cubemap(brdf_lut: Vec<f32>, brdf_lut_size: u32) -> PreparedEnvironmentCubemap {
    let face = |index: usize| {
        let mut texels = Vec::with_capacity(2 * 2 * 4);
        for texel in 0..4 {
            let level = 0.2 + 0.3 * index as f32 + 0.05 * texel as f32;
            texels.extend_from_slice(&[level, level * 0.8 + 0.1, 1.5 - level * 0.4, 1.0]);
        }
        texels
    };
    PreparedEnvironmentCubemap {
        resolution: 2,
        mips: vec![std::array::from_fn(face)],
        mip_count: 1,
        brdf_lut,
        brdf_lut_size,
    }
}

fn lighting_with(cubemap: Option<PreparedEnvironmentCubemap>) -> PreparedEnvironmentLighting {
    let diffuse = Vec3::new(0.5, 0.4, 0.3);
    PreparedEnvironmentLighting {
        diffuse_rgb: diffuse,
        diffuse_sh: DiffuseIrradianceSh::uniform(diffuse),
        specular_rgb: Vec3::new(0.9, 0.8, 0.7),
        intensity: 1.25,
        rotation_y_radians: 0.4,
        cubemap: cubemap.map(Arc::new),
    }
}

#[test]
fn surface_without_layers_reproduces_the_pre_port_environment_bits() {
    let with_cubemap = lighting_with(Some(structured_cubemap(vec![0.9, 0.05], 1)));
    let scalar_fallback = lighting_with(None);
    let mut sequence = Sequence(0x0e71_10b1_5e9e_c7a1);
    for index in 0..10_000 {
        let base = Vec3::new(sequence.next(), sequence.next(), sequence.next());
        let metallic = match index % 8 {
            0 => 0.0,
            1 => 1.0,
            _ => sequence.next(),
        };
        let material = PbrMaterial::new(base, metallic, sequence.next());
        let normal = sequence.direction();
        let view = sequence.direction();
        let surface = Surface::new(material, normal, view);
        for lighting in [&with_cubemap, &scalar_fallback] {
            let expected = pre_port_pbr_contribution(lighting, material, normal, view);
            let actual = lighting.surface_environment_light(&surface);
            assert_eq!(
                [actual.x.to_bits(), actual.y.to_bits(), actual.z.to_bits()],
                [
                    expected.x.to_bits(),
                    expected.y.to_bits(),
                    expected.z.to_bits()
                ],
                "sample {index}: image-based lighting changed for {material:?}, n={normal:?}, \
                 v={view:?}: {actual:?} vs {expected:?}"
            );
        }
    }
}

/// A uniform environment makes every term a closed form: radiance `E` from all
/// directions, diffuse radiance `D`, one table texel `(scale, bias)`.
fn uniform_lighting() -> (PreparedEnvironmentLighting, Vec3, Vec3, (f32, f32)) {
    let radiance = Vec3::new(1.2, 1.0, 0.8);
    let diffuse = Vec3::new(0.5, 0.4, 0.3);
    let table = (0.82, 0.07);
    let face = vec![radiance.x, radiance.y, radiance.z, 1.0];
    let lighting = PreparedEnvironmentLighting {
        diffuse_rgb: diffuse,
        diffuse_sh: DiffuseIrradianceSh::uniform(diffuse),
        specular_rgb: Vec3::ONE,
        intensity: 1.0,
        rotation_y_radians: 0.0,
        cubemap: Some(Arc::new(PreparedEnvironmentCubemap {
            resolution: 1,
            mips: vec![std::array::from_fn(|_| face.clone())],
            mip_count: 1,
            brdf_lut: vec![table.0, table.1],
            brdf_lut_size: 1,
        })),
    };
    (lighting, radiance, diffuse, table)
}

fn reference_surface() -> Surface {
    Surface::new(
        PbrMaterial::new(Vec3::new(0.6, 0.3, 0.1), 0.0, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.6, 0.0, 0.8),
    )
}

#[test]
fn clearcoat_attenuates_image_based_light_and_adds_its_own_reflection() {
    let (lighting, radiance, _, table) = uniform_lighting();
    let plain = reference_surface();
    let coated = plain.with_clearcoat(0.75, 0.3, plain.normal);
    let base = lighting.surface_environment_light(&plain);
    let actual = lighting.surface_environment_light(&coated);

    let coat_reflectance = (0.04 * table.0 + table.1) * 0.75;
    let expected = base * coated.clearcoat_attenuation + radiance * coat_reflectance;
    assert_vec3_close(actual, expected, 1.0e-6, "clearcoat over image-based light");
    assert!(
        (coated.clearcoat_attenuation - 0.969_769_6).abs() < 1.0e-6,
        "the base keeps 1 - 0.75 * F(0.8) of its light"
    );
}

#[test]
fn sheen_scales_image_based_light_and_adds_its_directional_albedo() {
    let (lighting, _, diffuse, _) = uniform_lighting();
    let plain = reference_surface();
    let sheen_color = Vec3::new(0.9, 0.5, 0.2);
    let sheened = plain.with_sheen(sheen_color, 0.45);
    let base = lighting.surface_environment_light(&plain);
    let actual = lighting.surface_environment_light(&sheened);

    // 0.8587 = 1 - 0.157 * 0.9; 0.0097693 is the published albedo fit at
    // n.v = 0.8 and roughness 0.45, evaluated outside this crate.
    let albedo = sheen_environment_brdf(0.8, 0.45);
    assert!((albedo - 0.009_769_3).abs() < 2.0e-6, "albedo {albedo}");
    let expected = base * 0.858_7 + sheen_color * diffuse * albedo;
    assert_vec3_close(actual, expected, 1.0e-6, "sheen over image-based light");
}

#[test]
fn clearcoat_over_sheen_orders_the_environment_layers_like_the_shader() {
    let (lighting, radiance, diffuse, table) = uniform_lighting();
    let plain = reference_surface();
    let sheen_color = Vec3::new(0.9, 0.5, 0.2);
    let layered = plain
        .with_sheen(sheen_color, 0.45)
        .with_clearcoat(0.75, 0.3, plain.normal);
    let base = lighting.surface_environment_light(&plain);

    let under_coat = base * 0.858_7 + sheen_color * diffuse * sheen_environment_brdf(0.8, 0.45);
    let expected =
        under_coat * layered.clearcoat_attenuation + radiance * ((0.04 * table.0 + table.1) * 0.75);
    assert_vec3_close(
        lighting.surface_environment_light(&layered),
        expected,
        1.0e-6,
        "clearcoat over sheen over image-based light",
    );
}

#[test]
fn environment_intensity_scales_every_layer() {
    let (mut lighting, ..) = uniform_lighting();
    let layered = reference_surface()
        .with_sheen(Vec3::new(0.9, 0.5, 0.2), 0.45)
        .with_clearcoat(0.75, 0.3, Vec3::new(0.0, 0.0, 1.0));
    let unit = lighting.surface_environment_light(&layered);
    lighting.intensity = 2.5;
    assert_vec3_close(
        lighting.surface_environment_light(&layered),
        unit * 2.5,
        1.0e-6,
        "environment intensity",
    );
    lighting.intensity = 0.0;
    assert_eq!(lighting.surface_environment_light(&layered), Vec3::ZERO);
}

fn assert_vec3_close(actual: Vec3, expected: Vec3, tolerance: f32, label: &str) {
    let delta = (actual - expected).abs();
    assert!(
        delta.max_element() <= tolerance,
        "{label}: expected {expected:?}, got {actual:?}, delta {delta:?}"
    );
}

#[test]
fn iridescence_drives_the_integrated_reflectance_of_image_based_light() {
    // Expected radiances are double-precision evaluations outside this crate:
    // the film reflectance at the view angle is inverted through Schlick and
    // weighted by the analytic split-sum fit.
    let (lighting, ..) = uniform_lighting();
    let plain = reference_surface();
    assert_vec3_close(
        lighting.surface_environment_light(&plain),
        Vec3::new(0.327_232_9, 0.147_934, 0.055_007_5),
        2.0e-6,
        "plain dielectric",
    );
    assert_vec3_close(
        lighting.surface_environment_light(&plain.with_iridescence(1.0, 1.3, 400.0)),
        Vec3::new(0.304_617_6, 0.135_238_1, 0.053_917_5),
        2.0e-4,
        "full iridescence",
    );
    assert_vec3_close(
        lighting.surface_environment_light(&plain.with_iridescence(0.5, 1.3, 400.0)),
        Vec3::new(0.315_925_3, 0.141_586_1, 0.054_462_5),
        2.0e-4,
        "half iridescence",
    );
}

#[test]
fn anisotropy_mirrors_the_environment_about_the_bent_normal() {
    let lighting = lighting_with(Some(structured_cubemap(vec![0.9, 0.05], 1)));
    let base = Vec3::new(0.9, 0.8, 0.7);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let view = Vec3::new(0.3, 0.5, 0.8).normalize();
    let plain = Surface::new(PbrMaterial::new(base, 1.0, 0.3), normal, view);
    let brushed = plain.with_anisotropy(0.8, (1.0, 0.0), 0.0, Vec3::new(1.0, 0.0, 0.0), 1.0);
    let bent = brushed.reflection_normal();
    assert!(
        bent.dot(normal) < 0.999,
        "an anisotropic gloss must bend the reflection normal, got {bent:?}"
    );

    // A metal has no diffuse term: what it reflects is the prefiltered
    // radiance along the bent mirror direction times the unchanged weight.
    let lut = pbr_brdf::split_sum_brdf_approx(normal.dot(view), 0.3);
    let weight = base * lut.0 + Vec3::splat(lut.1);
    let expected = lighting.environment_prefiltered(view, bent, 0.3) * weight * lighting.intensity;
    let actual = lighting.surface_environment_light(&brushed);
    assert_vec3_close(actual, expected, 1.0e-6, "anisotropic reflection");
    assert_ne!(
        actual,
        lighting.surface_environment_light(&plain),
        "the bent reflection must reach a different part of the environment"
    );
}
