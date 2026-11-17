use crate::material::Color;
use crate::scene::Vec3;

/// Scalar KHR_materials_transmission / KHR_materials_volume scene-color
/// transmission shared by the CPU renderer and the GPU/WebGL2 shaders. Keep
/// this file and `gpu/surface_effects.wgsl` formula-equivalent: the parity
/// sweep in `tests/transmission_parity.rs` exists to catch drift between
/// them.
const MIN_DENOMINATOR: f32 = 0.0001;

/// Taps in the disc that blurs the scene colour behind rough glass.
const SCENE_BLUR_TAPS: u32 = 16;
/// Angle between successive taps of the spiral, in radians.
const SCENE_BLUR_GOLDEN_ANGLE: f32 = 2.399_963_2;
/// Largest blur radius the disc resolves, as a share of the viewport height.
const SCENE_BLUR_MAX_RADIUS: f32 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedPhysicalTransmission {
    transmission: f32,
    ior: f32,
    thickness: f32,
    attenuation_color: Vec3,
    attenuation_distance: f32,
    roughness: f32,
    base_color: Vec3,
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
    pub(in crate::render) metallic: f32,
    pub(in crate::render) base_color: Color,
}

impl PreparedPhysicalTransmission {
    pub(in crate::render) fn new(input: PreparedPhysicalTransmissionInput) -> Option<Self> {
        let transmission = transmitted_share(
            input.transmission,
            input.transmission_texture,
            input.metallic,
        );
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
            base_color: Vec3::new(
                finite_or(input.base_color.r, 1.0).max(0.0),
                finite_or(input.base_color.g, 1.0).max(0.0),
                finite_or(input.base_color.b, 1.0).max(0.0),
            ),
        })
    }

    /// Base colour, which tints the light that passes through.
    pub(in crate::render) const fn base_color(self) -> Vec3 {
        self.base_color
    }
}

/// Share of the light entering a surface that passes through it instead of
/// scattering back as diffuse. Only the dielectric part of a material
/// transmits, so a metal passes nothing.
pub(in crate::render) fn transmitted_share(factor: f32, texture: f32, metallic: f32) -> f32 {
    finite_or(factor, 0.0).clamp(0.0, 1.0)
        * finite_or(texture, 1.0).clamp(0.0, 1.0)
        * (1.0 - finite_or(metallic, 0.0).clamp(0.0, 1.0))
}

#[derive(Debug, Clone, Copy)]
pub(in crate::render) struct PhysicalTransmissionInputs {
    pub(in crate::render) frag_coord: [f32; 2],
    pub(in crate::render) viewport: [f32; 2],
    /// World-space position of the surface point.
    pub(in crate::render) position: Vec3,
    pub(in crate::render) normal: Vec3,
    pub(in crate::render) view: Vec3,
    /// Lengths of the model matrix columns: world units per model unit.
    pub(in crate::render) model_scale: Vec3,
    /// Colour that tints the light passing through: the base colour.
    pub(in crate::render) tint: Vec3,
    /// Light the surface itself sends toward the viewer.
    pub(in crate::render) surface_rgb: Vec3,
}

/// Colour of a transmissive surface point.
///
/// Fresnel reflectance from the index of refraction decides how much light
/// passes through. That light is the scene colour seen along the refracted
/// ray: the view direction is refracted at the surface and travels the
/// material thickness to an exit point, and the scene colour is read where
/// that point lands on screen, as the Khronos glTF Sample Viewer does.
/// Roughness blurs it, the volume absorbs along the path, and the base colour
/// tints it. A surface without thickness reads the scene straight behind it.
///
/// `project` maps a world position to scene-colour texture coordinates, or
/// `None` when the position lies outside the view volume.
pub(in crate::render) fn physical_transmission_color(
    material: PreparedPhysicalTransmission,
    input: PhysicalTransmissionInputs,
    project: impl Fn(Vec3) -> Option<[f32; 2]>,
    mut sample_scene_color: impl FnMut([f32; 2]) -> Vec3,
) -> Vec3 {
    let viewport = [input.viewport[0].max(1.0), input.viewport[1].max(1.0)];
    let view_dir = normalize_or(input.view, input.normal);
    let normal_dir = normalize_or(input.normal, Vec3::Y);
    let n_dot_v = normal_dir.dot(view_dir).clamp(0.0, 1.0);
    let fresnel = schlick_fresnel(f0_from_ior(material.ior), n_dot_v);

    let ray = multiply_vec3(
        refract_vec3(-view_dir, normal_dir, material.ior.recip()),
        input.model_scale * material.thickness,
    );
    let uv = project(input.position + ray).unwrap_or([
        input.frag_coord[0] / viewport[0],
        input.frag_coord[1] / viewport[1],
    ]);
    let spread = material.roughness * (material.ior * 2.0 - 2.0).clamp(0.0, 1.0);
    let scene_color = blurred_scene_color(
        uv,
        scene_blur_radius_px(spread, viewport),
        input.frag_coord,
        viewport,
        &mut sample_scene_color,
    );
    let absorption = volume_transmittance(
        ray.length(),
        material.attenuation_color,
        material.attenuation_distance,
    );
    let transmitted = multiply_vec3(multiply_vec3(scene_color, absorption), input.tint);
    input.surface_rgb + transmitted * ((1.0 - fresnel) * material.transmission)
}

/// Reflectance at normal incidence of a boundary between air and a medium of
/// the given index of refraction.
pub(in crate::render) fn f0_from_ior(ior: f32) -> f32 {
    let ratio = (ior - 1.0) / (ior + 1.0);
    ratio * ratio
}

/// Schlick's approximation of Fresnel reflectance.
pub(in crate::render) fn schlick_fresnel(f0: f32, cos_theta: f32) -> f32 {
    f0 + (1.0 - f0) * (1.0 - cos_theta.clamp(0.0, 1.0)).powi(5)
}

/// Radius in pixels of the blur that a lobe of the given spread needs. This
/// is the Khronos glTF Sample Viewer's mapping: it reads the scene colour at
/// level of detail `log2(width) * spread`, whose texels span `width^spread`
/// pixels. The disc resolves radii up to a share of the viewport height.
pub(in crate::render) fn scene_blur_radius_px(spread: f32, viewport: [f32; 2]) -> f32 {
    let footprint = viewport[0].powf(spread.clamp(0.0, 1.0));
    (0.5 * (footprint - 1.0)).min(SCENE_BLUR_MAX_RADIUS * viewport[1])
}

/// Scene colour around `uv`, blurred by a Gaussian-weighted disc of the given
/// radius. The taps follow a golden-angle spiral, which covers the disc
/// evenly, rotated per pixel by interleaved gradient noise so that
/// under-sampling shows as fine grain instead of copies of the scene.
pub(in crate::render) fn blurred_scene_color(
    uv: [f32; 2],
    radius_px: f32,
    frag_coord: [f32; 2],
    viewport: [f32; 2],
    sample_scene_color: &mut impl FnMut([f32; 2]) -> Vec3,
) -> Vec3 {
    let noise = (52.982_918
        * (frag_coord[0] * 0.067_110_56 + frag_coord[1] * 0.005_837_15).fract())
    .fract();
    let rotation = std::f32::consts::TAU * noise;
    let mut sum = Vec3::ZERO;
    let mut weight_sum = 0.0;
    for tap in 0..SCENE_BLUR_TAPS {
        let distance = ((tap as f32 + 0.5) / SCENE_BLUR_TAPS as f32).sqrt();
        let angle = tap as f32 * SCENE_BLUR_GOLDEN_ANGLE + rotation;
        // Standard deviation of half the disc radius.
        let weight = (-2.0 * distance * distance).exp();
        let reach = distance * radius_px;
        sum += sample_scene_color([
            uv[0] + angle.cos() * reach / viewport[0],
            uv[1] + angle.sin() * reach / viewport[1],
        ]) * weight;
        weight_sum += weight;
    }
    sum * weight_sum.recip()
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

fn multiply_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x * right.x, left.y * right.y, left.z * right.z)
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

    const VIEWPORT: [f32; 2] = [96.0, 64.0];
    const FRAG_COORD: [f32; 2] = [48.0, 32.0];

    fn glass(ior: f32, thickness: f32, roughness: f32) -> PreparedPhysicalTransmission {
        PreparedPhysicalTransmission::new(PreparedPhysicalTransmissionInput {
            transmission: 1.0,
            transmission_texture: 1.0,
            ior,
            thickness,
            thickness_texture: 1.0,
            attenuation_color: Color::WHITE,
            attenuation_distance: f32::INFINITY,
            roughness,
            metallic: 0.0,
            base_color: Color::WHITE,
        })
        .expect("transmission material is valid")
    }

    fn inputs(normal: Vec3) -> PhysicalTransmissionInputs {
        PhysicalTransmissionInputs {
            frag_coord: FRAG_COORD,
            viewport: VIEWPORT,
            position: Vec3::ZERO,
            normal,
            view: Vec3::Z,
            model_scale: Vec3::new(1.0, 1.0, 1.0),
            tint: Vec3::new(1.0, 1.0, 1.0),
            surface_rgb: Vec3::ZERO,
        }
    }

    /// A camera at z = 2 looking down -z: one world unit spans a quarter of
    /// the texture at the z = 0 plane.
    fn project(world: Vec3) -> Option<[f32; 2]> {
        let depth = 2.0 - world.z;
        (depth > 0.0).then(|| [0.5 + world.x / (2.0 * depth), 0.5 - world.y / (2.0 * depth)])
    }

    /// Weighted mean of the texture coordinates a call reads, in pixels.
    fn mean_sample_px(
        material: PreparedPhysicalTransmission,
        input: PhysicalTransmissionInputs,
    ) -> [f32; 2] {
        // The scene colour encodes its own coordinates. Fresnel passes the
        // same share of both, so their ratio to a white scene is the mean.
        let coordinates = physical_transmission_color(material, input, project, |uv| {
            Vec3::new(uv[0], uv[1], 0.0)
        });
        let white =
            physical_transmission_color(material, input, project, |_| Vec3::new(1.0, 1.0, 1.0));
        [
            coordinates.x / white.x * VIEWPORT[0],
            coordinates.y / white.y * VIEWPORT[1],
        ]
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
            metallic: 0.0,
            base_color: Color::from_linear_rgb(0.82, 0.88, 1.0),
        })
        .expect("transmission material is valid");
        let color = physical_transmission_color(
            material,
            PhysicalTransmissionInputs {
                tint: material.base_color(),
                surface_rgb: Vec3::new(0.02, 0.02, 0.03),
                ..inputs(Vec3::Z)
            },
            project,
            |_| Vec3::new(1.0, 1.0, 1.0),
        );
        // White seen at normal incidence through 0.42 units of the volume:
        // Fresnel passes 1 - ((1.48 - 1) / (1.48 + 1))^2, the volume keeps
        // attenuation_color^(0.42 / 0.8), the base colour tints, and the
        // surface's own light is added.
        let passed = 1.0 - (0.48_f32 / 2.48).powi(2);
        for (channel, attenuation, tint, surface) in [
            (color.x, 0.35_f32, 0.82_f32, 0.02_f32),
            (color.y, 0.58, 0.88, 0.02),
            (color.z, 1.0, 1.0, 0.03),
        ] {
            let expected = surface + passed * attenuation.powf(0.42 / 0.8) * tint;
            assert!(
                (channel - expected).abs() <= 1.0e-4,
                "expected {expected}, got {channel} in {color:?}"
            );
        }
    }

    #[test]
    fn fresnel_from_the_ior_splits_reflection_and_transmission() {
        let white = |_: [f32; 2]| Vec3::new(1.0, 1.0, 1.0);
        // Normal incidence on an index of 1.5 reflects ((1.5 - 1) / (1.5 + 1))^2
        // = 4 % and passes the other 96 %.
        let facing =
            physical_transmission_color(glass(1.5, 0.0, 0.0), inputs(Vec3::Z), project, white);
        assert!(
            (facing.x - 0.96).abs() <= 1.0e-4,
            "a white backdrop behind clear glass at normal incidence shows 0.96, got {facing:?}"
        );
        // Toward grazing incidence less and less passes: Schlick's
        // 0.04 + 0.96 (1 - cos)^5 at 60 and 85 degrees.
        for (degrees, reflectance) in [(60.0_f32, 0.07_f32), (85.0, 0.648_5)] {
            let angle = degrees.to_radians();
            let tilted = physical_transmission_color(
                glass(1.5, 0.0, 0.0),
                inputs(Vec3::new(angle.sin(), 0.0, angle.cos())),
                project,
                white,
            );
            assert!(
                (tilted.x - (1.0 - reflectance)).abs() <= 1.0e-3,
                "at {degrees} degrees {reflectance} reflects, got {tilted:?}"
            );
        }
        // The index sets the split: an index of 2 reflects a ninth head-on.
        let dense =
            physical_transmission_color(glass(2.0, 0.0, 0.0), inputs(Vec3::Z), project, white);
        assert!((dense.x - 8.0 / 9.0).abs() <= 1.0e-4, "{dense:?}");
    }

    #[test]
    fn thin_walled_glass_samples_the_scene_straight_through() {
        // With no thickness there is no volume for the refracted ray to
        // cross, whatever the surface orientation.
        let read = mean_sample_px(
            glass(1.5, 0.0, 0.0),
            inputs(Vec3::new(0.5, 0.0, 0.866_025_4)),
        );
        assert!(
            (read[0] - FRAG_COORD[0]).abs() <= 0.02 && (read[1] - FRAG_COORD[1]).abs() <= 0.02,
            "thin-walled glass must read the scene behind the fragment, read {read:?} px"
        );
    }

    #[test]
    fn thick_glass_reads_the_scene_at_the_projected_exit_point_of_the_refracted_ray() {
        // The surface is tilted 30 degrees about y. Snell: sin(t) = sin(30) / 1.5.
        let ior = 1.5_f32;
        let thickness = 0.4_f32;
        let incidence = 30.0_f32.to_radians();
        let refraction = (incidence.sin() / ior).asin();
        // The refracted ray leaves the view axis by the difference of the two
        // angles, toward -x for a normal tilted toward +x.
        let deviation = incidence - refraction;
        let exit = Vec3::new(
            -deviation.sin() * thickness,
            0.0,
            -deviation.cos() * thickness,
        );
        let expected = project(exit).expect("exit point is in view");
        let read = mean_sample_px(
            glass(ior, thickness, 0.0),
            inputs(Vec3::new(incidence.sin(), 0.0, incidence.cos())),
        );
        assert!(
            (read[0] - expected[0] * VIEWPORT[0]).abs() <= 0.02
                && (read[1] - expected[1] * VIEWPORT[1]).abs() <= 0.02,
            "expected the scene at {:?} px, read {read:?} px",
            [expected[0] * VIEWPORT[0], expected[1] * VIEWPORT[1]]
        );
        assert!(
            FRAG_COORD[0] - read[0] > 1.0,
            "the refracted read must be displaced from the fragment, read {read:?} px"
        );
    }

    #[test]
    fn thickness_is_in_model_units_and_follows_the_model_scale() {
        let normal = Vec3::new(0.5, 0.0, 0.866_025_4);
        let offset = |scale: f32| {
            let read = mean_sample_px(
                glass(1.5, 0.2, 0.0),
                PhysicalTransmissionInputs {
                    model_scale: Vec3::new(scale, scale, scale),
                    ..inputs(normal)
                },
            );
            FRAG_COORD[0] - read[0]
        };
        let (unit, doubled) = (offset(1.0), offset(2.0));
        // Twice the model scale is twice the travelled distance. The read
        // moves a little less than twice as far, because the exit point is
        // also further from the camera.
        assert!(unit > 0.5, "refraction must displace the read: {unit} px");
        assert!(
            doubled > unit * 1.7 && doubled < unit * 2.0,
            "scale 1 moves {unit} px, scale 2 moves {doubled} px"
        );
    }

    #[test]
    fn volume_absorbs_along_the_refracted_path_length() {
        // A model scale of 3 makes 0.2 model units 0.6 world units of glass:
        // two attenuation distances.
        let material = PreparedPhysicalTransmission {
            attenuation_color: Vec3::new(0.25, 0.5, 1.0),
            attenuation_distance: 0.3,
            ..glass(1.5, 0.2, 0.0)
        };
        let color = physical_transmission_color(
            material,
            PhysicalTransmissionInputs {
                model_scale: Vec3::new(3.0, 3.0, 3.0),
                ..inputs(Vec3::Z)
            },
            project,
            |_| Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(
            (color.x - 0.96 * 0.25_f32.powi(2)).abs() <= 1.0e-4,
            "{color:?}"
        );
        assert!(
            (color.y - 0.96 * 0.5_f32.powi(2)).abs() <= 1.0e-4,
            "{color:?}"
        );
        assert!((color.z - 0.96).abs() <= 1.0e-4, "{color:?}");
    }

    #[test]
    fn roughness_blurs_the_scene_with_a_disc_sized_by_roughness_and_ior() {
        let viewport = [512.0_f32, 512.0];
        let centre = [256.5_f32, 256.5];
        // Distances in pixels from the fragment of every read.
        let reads = |roughness: f32, ior: f32| {
            let mut reads = Vec::new();
            physical_transmission_color(
                glass(ior, 0.0, roughness),
                PhysicalTransmissionInputs {
                    frag_coord: centre,
                    viewport,
                    ..inputs(Vec3::Z)
                },
                |_| None,
                |uv| {
                    reads.push([
                        uv[0] * viewport[0] - centre[0],
                        uv[1] * viewport[1] - centre[1],
                    ]);
                    Vec3::ZERO
                },
            );
            reads
        };
        let reach = |reads: &[[f32; 2]]| {
            reads
                .iter()
                .map(|read| read[0].hypot(read[1]))
                .fold(0.0_f32, f32::max)
        };
        let rough = reads(0.5, 1.5);
        assert!(rough.len() >= 12, "the blur needs at least twelve taps");
        // Khronos: the footprint is width^(roughness * clamp(2 ior - 2, 0, 1))
        // pixels across, so the disc is half that less the pixel itself. The
        // outermost of sixteen taps sits at sqrt(15.5 / 16) of the radius.
        let radius = 0.5 * (512.0_f32.powf(0.5) - 1.0);
        let outermost = (15.5_f32 / 16.0).sqrt();
        assert!(
            (reach(&rough) - outermost * radius).abs() <= 0.05,
            "outermost tap at {} px, expected {} px",
            reach(&rough),
            outermost * radius
        );
        // The taps surround the centre instead of lining up on an axis.
        for quadrant in [(1.0_f32, 1.0_f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            assert!(
                rough
                    .iter()
                    .filter(|read| read[0] * quadrant.0 > 0.0 && read[1] * quadrant.1 > 0.0)
                    .count()
                    >= 2,
                "every quadrant of the disc needs taps: {rough:?}"
            );
        }
        // Smoother glass blurs less, and an index near 1 does not blur.
        assert!(reach(&reads(0.25, 1.5)) < reach(&rough) * 0.25);
        assert!(reach(&reads(0.5, 1.01)) < 0.05);
        // The disc resolves radii up to 3 % of the viewport height.
        assert!((reach(&reads(1.0, 1.5)) - outermost * 0.03 * 512.0).abs() <= 0.05);
    }

    #[test]
    fn blur_keeps_a_uniform_scene_and_smooths_a_step() {
        let viewport = [512.0_f32, 512.0];
        let through = |frag_x: f32, scene: &dyn Fn([f32; 2]) -> Vec3| {
            physical_transmission_color(
                glass(1.5, 0.0, 0.5),
                PhysicalTransmissionInputs {
                    frag_coord: [frag_x, 256.5],
                    viewport,
                    ..inputs(Vec3::Z)
                },
                |_| None,
                scene,
            )
        };
        let uniform = through(256.5, &|_| Vec3::new(0.5, 0.5, 0.5));
        assert!((uniform.x - 0.5 * 0.96).abs() <= 1.0e-4, "{uniform:?}");
        // A step at the centre column: well to the left reads black, well to
        // the right white, and inside the blur radius it is in between.
        let step = |uv: [f32; 2]| {
            let value = if uv[0] >= 0.5 { 1.0 } else { 0.0 };
            Vec3::new(value, value, value)
        };
        let profile: Vec<f32> = (0..=8)
            .map(|index| through(256.5 + (index as f32 - 4.0) * 4.0, &step).x / 0.96)
            .collect();
        assert!(
            profile[0] <= 1.0e-4 && profile[8] >= 1.0 - 1.0e-4,
            "{profile:?}"
        );
        assert!(
            profile[3] > 0.1 && profile[3] < 0.9 && profile[5] > 0.1 && profile[5] < 0.9,
            "the step must be spread over the blur radius: {profile:?}"
        );
    }

    #[test]
    fn metals_and_opaque_shares_do_not_transmit() {
        let input = |transmission: f32, metallic: f32| PreparedPhysicalTransmissionInput {
            transmission,
            transmission_texture: 1.0,
            ior: 1.5,
            thickness: 0.0,
            thickness_texture: 1.0,
            attenuation_color: Color::WHITE,
            attenuation_distance: f32::INFINITY,
            roughness: 0.0,
            metallic,
            base_color: Color::WHITE,
        };
        assert!(PreparedPhysicalTransmission::new(input(1.0, 1.0)).is_none());
        assert!(PreparedPhysicalTransmission::new(input(0.0, 0.0)).is_none());
        let half = PreparedPhysicalTransmission::new(input(0.5, 0.0)).expect("half transmissive");
        let color = physical_transmission_color(
            half,
            PhysicalTransmissionInputs {
                surface_rgb: Vec3::new(0.1, 0.1, 0.1),
                ..inputs(Vec3::Z)
            },
            project,
            |_| Vec3::new(1.0, 1.0, 1.0),
        );
        assert!((color.x - (0.1 + 0.5 * 0.96)).abs() <= 1.0e-4, "{color:?}");
        assert_eq!(transmitted_share(1.0, 1.0, 0.25), 0.75);
    }
}
