use super::*;

/// The direct-light BRDF exactly as the CPU renderer evaluated it before the
/// layered surface model existed (commit 4da02d2). It is kept here as the
/// oracle for the hard requirement on the port: a surface without layers must
/// produce the same bits.
fn pre_port_punctual_light_contribution(
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
    let alpha = pbr_brdf::alpha_roughness(material.roughness);
    let specular_brdf = pbr_brdf::brdf_specular_ggx(alpha, n_dot_l, n_dot_v, n_dot_h);
    let fresnel = fresnel_schlick(v_dot_h, material.f0());
    let specular = scale_vec3(fresnel, specular_brdf);
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

/// Pre-port shading of one area-light point sample: diffuse only.
fn pre_port_area_light_diffuse_contribution(
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
    let half_vector = normalize_or(add_vec3(view, incoming), normal);
    let v_dot_h = dot_vec3(view, half_vector).max(0.0);
    let fresnel = fresnel_schlick(v_dot_h, material.f0());
    let diffuse_energy = scale_vec3(
        subtract_vec3(Vec3::new(1.0, 1.0, 1.0), fresnel),
        1.0 - material.metallic,
    );
    let diffuse = scale_vec3(multiply_vec3(diffuse_energy, material.base), PI.recip());
    scale_vec3(multiply_vec3(diffuse, radiance), n_dot_l)
}

/// Deterministic values in `[0, 1)`; the sweep must not depend on a seed
/// source outside this file.
struct Sequence(u64);

impl Sequence {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / (1_u64 << 24) as f32
    }

    fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }

    fn direction(&mut self) -> Vec3 {
        normalize_or(
            Vec3::new(self.signed(), self.signed(), self.signed()),
            Vec3::new(0.0, 0.0, 1.0),
        )
    }
}

fn bits(value: Vec3) -> [u32; 3] {
    [value.x.to_bits(), value.y.to_bits(), value.z.to_bits()]
}

#[test]
fn surface_without_layers_reproduces_the_pre_port_direct_light_bits() {
    let mut sequence = Sequence(0x5ce7_a1a7_e12e_d5ed);
    let mut lit_samples = 0;
    for index in 0..20_000 {
        let base = Vec3::new(sequence.next(), sequence.next(), sequence.next());
        // Every eighth sample is a pure metal or a pure dielectric, the two
        // ends real materials sit at.
        let metallic = match index % 8 {
            0 => 0.0,
            1 => 1.0,
            _ => sequence.next(),
        };
        let material = PbrMaterial::new(base, metallic, sequence.next());
        let normal = sequence.direction();
        let view = sequence.direction();
        // One in sixteen lights sits exactly opposite the viewer, where the
        // half vector degenerates.
        let incoming = if index % 16 == 5 {
            -view
        } else {
            sequence.direction()
        };
        let radiance = Vec3::new(
            sequence.next() * 4.0,
            sequence.next() * 4.0,
            sequence.next() * 4.0,
        );
        let surface = Surface::new(material, normal, view);

        let expected =
            pre_port_punctual_light_contribution(material, normal, view, incoming, radiance);
        let actual = surface_direct_light(&surface, incoming, radiance, 1.0);
        assert_eq!(
            bits(actual),
            bits(expected),
            "sample {index}: punctual light changed for {material:?}, n={normal:?}, v={view:?}, \
             l={incoming:?}: {actual:?} vs {expected:?}"
        );

        let expected =
            pre_port_area_light_diffuse_contribution(material, normal, view, incoming, radiance);
        let actual = surface_direct_light(&surface, incoming, radiance, 0.0);
        assert_eq!(
            bits(actual),
            bits(expected),
            "sample {index}: area-light sample changed for {material:?}: {actual:?} vs {expected:?}"
        );
        if expected != Vec3::ZERO {
            lit_samples += 1;
        }
    }
    assert!(
        lit_samples > 5_000,
        "the sweep must light enough samples to mean something, lit {lit_samples}"
    );
}

#[test]
fn absent_layers_leave_the_surface_exactly_as_built() {
    let material = PbrMaterial::new(Vec3::new(0.6, 0.3, 0.1), 0.2, 0.5);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let view = normalize_or(Vec3::new(0.3, 0.1, 1.0), normal);
    let plain = Surface::new(material, normal, view);

    assert_eq!(
        plain
            .with_clearcoat(0.0, 0.3, Vec3::new(0.0, 1.0, 0.0))
            .with_sheen(Vec3::ZERO, 0.9),
        plain,
        "a zero clearcoat factor and a black sheen colour are not layers"
    );
    assert_eq!(plain.clearcoat_attenuation, 1.0);
    assert_eq!(plain.sheen_scaling, 1.0);
    assert!(!plain.has_sheen());
}

/// Reference scene for the layered direct-light values: normal `+Z`, view and
/// light in the XZ plane on opposite sides with `n.v = 0.8` and `n.l = 0.6`.
/// Expected radiances are double-precision evaluations of the published
/// formulas, computed outside this crate.
fn reference_surface() -> (Surface, Vec3, Vec3) {
    let material = PbrMaterial::new(Vec3::new(0.6, 0.3, 0.1), 0.0, 0.5);
    let surface = Surface::new(material, Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.6, 0.0, 0.8));
    (surface, Vec3::new(-0.8, 0.0, 0.6), Vec3::new(2.0, 1.5, 1.0))
}

#[test]
fn base_layer_matches_the_reference_radiance() {
    let (surface, incoming, radiance) = reference_surface();
    assert_vec3_close(
        surface_direct_light(&surface, incoming, radiance, 1.0),
        Vec3::new(0.296_043, 0.139_704_2, 0.056_545_8),
        2.0e-6,
        "plain base layer",
    );
}

#[test]
fn clearcoat_at_normal_incidence_dims_the_base_by_exactly_its_fresnel_reflectance() {
    let material = PbrMaterial::new(Vec3::new(0.6, 0.3, 0.1), 0.0, 0.5);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let plain = Surface::new(material, normal, normal);
    for factor in [1.0_f32, 0.5, 0.25] {
        let coated = plain.with_clearcoat(factor, 0.3, normal);
        assert_eq!(
            coated.clearcoat_attenuation,
            1.0 - factor * DIELECTRIC_F0,
            "a coat seen head-on reflects F0 = 0.04 of the light away from the base"
        );
    }

    // Light from the side reaches the base but not the coat's mirror lobe in
    // any amount that matters, so what is left is the dimmed base.
    let coated = plain.with_clearcoat(1.0, 0.04, normal);
    let incoming = normalize_or(Vec3::new(0.9, 0.0, 0.2), normal);
    let radiance = Vec3::new(1.0, 1.0, 1.0);
    let base = surface_direct_light(&plain, incoming, radiance, 1.0);
    let dimmed = surface_direct_light(&coated, incoming, radiance, 1.0);
    assert_vec3_close(
        dimmed,
        scale_vec3(base, 0.96),
        1.0e-6,
        "base under a full clearcoat at normal incidence",
    );
}

#[test]
fn clearcoat_attenuates_the_layers_beneath_and_adds_its_own_lobe() {
    let (surface, incoming, radiance) = reference_surface();
    let coated = surface.with_clearcoat(0.75, 0.3, surface.normal);
    assert_close(
        coated.clearcoat_attenuation,
        0.969_769_6,
        1.0e-6,
        "attenuation at n.v 0.8",
    );
    assert_vec3_close(
        surface_direct_light(&coated, incoming, radiance, 1.0),
        Vec3::new(0.351_927_6, 0.184_106_4, 0.087_253_5),
        2.0e-6,
        "clearcoat over the base layer",
    );

    // Grazing view: the coat reflects almost everything, so almost nothing
    // reaches the base. The pre-port model added the coat without dimming it.
    let grazing_view = normalize_or(Vec3::new(1.0, 0.0, 0.02), surface.normal);
    let grazing = Surface::new(
        PbrMaterial::new(Vec3::new(0.6, 0.3, 0.1), 0.0, 0.5),
        surface.normal,
        grazing_view,
    )
    .with_clearcoat(1.0, 0.3, surface.normal);
    assert!(
        grazing.clearcoat_attenuation < 0.1,
        "a grazing clearcoat must hide the base, attenuation {}",
        grazing.clearcoat_attenuation
    );
}

#[test]
fn sheen_scales_the_base_by_its_energy_compensation_and_adds_the_charlie_lobe() {
    let (surface, incoming, radiance) = reference_surface();
    let sheen_color = Vec3::new(0.9, 0.5, 0.2);
    let sheened = surface.with_sheen(sheen_color, 0.45);
    assert_close(
        sheened.sheen_scaling,
        1.0 - SHEEN_ENERGY_COMPENSATION * 0.9,
        1.0e-7,
        "the base keeps 1 - 0.157 * max(sheen colour)",
    );
    assert_close(sheened.sheen_scaling, 0.858_7, 1.0e-6, "sheen scaling");
    assert_vec3_close(
        surface_direct_light(&sheened, incoming, radiance, 1.0),
        Vec3::new(0.254_232_8, 0.119_972_6, 0.048_558_2),
        2.0e-6,
        "sheen over the base layer",
    );

    // Sheen roughness below the shader's floor is evaluated at the floor.
    assert_eq!(
        surface.with_sheen(sheen_color, 0.0).sheen_roughness,
        MIN_SHEEN_ROUGHNESS
    );
}

#[test]
fn clearcoat_over_sheen_attenuates_both_lower_layers() {
    let (surface, incoming, radiance) = reference_surface();
    let layered = surface
        .with_sheen(Vec3::new(0.9, 0.5, 0.2), 0.45)
        .with_clearcoat(0.75, 0.3, surface.normal);
    assert_vec3_close(
        surface_direct_light(&layered, incoming, radiance, 1.0),
        Vec3::new(0.311_381_4, 0.164_971_3, 0.079_507_3),
        2.0e-6,
        "clearcoat over sheen over the base layer",
    );
}

#[test]
fn area_light_samples_keep_the_layer_lobes_but_drop_the_base_specular() {
    let (surface, incoming, radiance) = reference_surface();
    let layered = surface
        .with_sheen(Vec3::new(0.9, 0.5, 0.2), 0.45)
        .with_clearcoat(0.75, 0.3, surface.normal);
    let full = surface_direct_light(&layered, incoming, radiance, 1.0);
    let sampled = surface_direct_light(&layered, incoming, radiance, 0.0);
    let plain_full = surface_direct_light(&surface, incoming, radiance, 1.0);
    let plain_sampled = surface_direct_light(&surface, incoming, radiance, 0.0);

    // The only term weight 0 removes is the base specular lobe, which the
    // sheen scaling and the coat attenuate like the rest of the base.
    let base_specular = subtract_vec3(plain_full, plain_sampled);
    let expected_drop = scale_vec3(
        base_specular,
        layered.sheen_scaling * layered.clearcoat_attenuation,
    );
    assert_vec3_close(
        subtract_vec3(full, sampled),
        expected_drop,
        1.0e-6,
        "base specular removed from an area-light sample",
    );
    assert!(sampled.x > plain_sampled.x * 0.5 && sampled.x > 0.0);
}

fn assert_close(actual: f32, expected: f32, tolerance: f32, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected}, got {actual}"
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
        "{label}: expected {expected:?}, got {actual:?}, delta {delta:?}"
    );
}

#[test]
fn default_index_of_refraction_leaves_the_surface_exactly_as_built() {
    let material = PbrMaterial::new(Vec3::new(0.6, 0.3, 0.1), 0.0, 0.5);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let plain = Surface::new(material, normal, normal);
    assert_eq!(
        plain.with_ior(1.5),
        plain,
        "the glTF default index is the 0.04 the base layer is built with"
    );
    assert_eq!(
        plain.f0,
        Vec3::new(DIELECTRIC_F0, DIELECTRIC_F0, DIELECTRIC_F0)
    );
}

#[test]
fn index_of_refraction_sets_the_dielectric_reflectance() {
    let base = Vec3::new(0.6, 0.3, 0.1);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    // ((n - 1) / (n + 1))^2. An index of 0 is the glTF value for a surface
    // that reflects everything.
    for (ior, expected) in [
        (1.33_f32, 0.020_059_3_f32),
        (2.0, 0.111_111_1),
        (2.42, 0.172_394_9),
        (1.0, 0.0),
        (0.0, 1.0),
    ] {
        let surface = Surface::new(PbrMaterial::new(base, 0.0, 0.5), normal, normal).with_ior(ior);
        assert_vec3_close(
            surface.f0,
            Vec3::new(expected, expected, expected),
            1.0e-6,
            &format!("dielectric reflectance at index {ior}"),
        );
    }

    // A metal reflects its base colour whatever the index; a half metal mixes
    // the two reflectances.
    let metal = Surface::new(PbrMaterial::new(base, 1.0, 0.5), normal, normal).with_ior(2.0);
    assert_eq!(metal.f0, base);
    let half = Surface::new(PbrMaterial::new(base, 0.5, 0.5), normal, normal).with_ior(2.0);
    assert_vec3_close(
        half.f0,
        Vec3::new(0.355_555_6, 0.205_555_6, 0.105_555_6),
        1.0e-6,
        "half-metal reflectance at index 2",
    );
}

/// A white metal in the reference scene: its Fresnel term is 1, so the
/// radiance it reflects is the specular lobe itself. The mesh tangent is `+X`.
fn anisotropic_reference(strength: f32, rotation: f32) -> Vec3 {
    let surface = Surface::new(
        PbrMaterial::new(Vec3::new(1.0, 1.0, 1.0), 1.0, 0.42),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.6, 0.0, 0.8),
    )
    .with_anisotropy(
        strength,
        (1.0, 0.0),
        rotation,
        Vec3::new(1.0, 0.0, 0.0),
        1.0,
    );
    surface_direct_light(
        &surface,
        Vec3::new(-0.8, 0.0, 0.6),
        Vec3::new(1.0, 1.0, 1.0),
        1.0,
    )
}

#[test]
fn anisotropy_reshapes_the_base_lobe_instead_of_adding_one() {
    let isotropic = anisotropic_reference(0.0, 0.0);
    assert_close(isotropic.x, 1.192_502, 5.0e-6, "isotropic lobe");

    // Stretched along the tangent, which lies in the plane of incidence.
    assert_close(
        anisotropic_reference(0.5, 0.0).x,
        1.093_563_8,
        5.0e-6,
        "half strength along the tangent",
    );
    let along = anisotropic_reference(1.0, 0.0);
    assert_close(
        along.x,
        0.386_673_8,
        5.0e-6,
        "full strength along the tangent",
    );

    // Rotated a quarter turn, the lobe is stretched across the plane of
    // incidence instead.
    let quarter_turn = std::f32::consts::FRAC_PI_2;
    assert_close(
        anisotropic_reference(0.5, quarter_turn).x,
        0.550_241_6,
        5.0e-6,
        "half strength across the tangent",
    );
    let across = anisotropic_reference(1.0, quarter_turn);
    assert_close(
        across.x,
        0.210_357_3,
        5.0e-6,
        "full strength across the tangent",
    );

    assert!(
        along.x < isotropic.x && across.x < along.x,
        "anisotropy spreads the one base lobe; a lobe added on top could only brighten it: \
         isotropic {isotropic:?}, along {along:?}, across {across:?}"
    );
}

#[test]
fn zero_anisotropy_strength_is_the_isotropic_surface() {
    let plain = Surface::new(
        PbrMaterial::new(Vec3::new(0.9, 0.8, 0.7), 1.0, 0.42),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.6, 0.0, 0.8),
    );
    assert_eq!(
        plain.with_anisotropy(0.0, (0.3, 0.9), 1.2, Vec3::new(1.0, 0.0, 0.0), -1.0),
        plain
    );
    assert_eq!(plain.reflection_normal(), plain.normal);
}

#[test]
fn iridescence_replaces_the_base_fresnel_term() {
    let plain = Surface::new(
        PbrMaterial::new(Vec3::new(0.58, 0.61, 0.65), 1.0, 0.42),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.6, 0.0, 0.8),
    );
    let incoming = Vec3::new(-0.8, 0.0, 0.6);
    let radiance = Vec3::new(1.0, 1.0, 1.0);

    let film = plain.with_iridescence(1.0, 1.3, 250.0);
    assert_vec3_close(
        film.iridescence_fresnel,
        Vec3::new(0.593_683_1, 0.604_333_7, 0.556_842_7),
        2.0e-4,
        "film reflectance at the view angle",
    );
    // A metal has no diffuse term, so what it reflects is that reflectance
    // times the specular lobe.
    assert_vec3_close(
        surface_direct_light(&film, incoming, radiance, 1.0),
        Vec3::new(0.707_968_3, 0.720_669_2, 0.664_036),
        3.0e-4,
        "full iridescence",
    );
    assert_vec3_close(
        surface_direct_light(
            &plain.with_iridescence(0.5, 1.3, 250.0),
            incoming,
            radiance,
            1.0,
        ),
        Vec3::new(0.700_349_5, 0.724_548_9, 0.720_031),
        3.0e-4,
        "half iridescence mixes the film with the Schlick term",
    );
    assert_eq!(
        plain.with_iridescence(0.0, 1.3, 250.0),
        plain,
        "a zero iridescence factor is not a layer"
    );
    assert_eq!(plain.environment_f0(), plain.f0);
}
