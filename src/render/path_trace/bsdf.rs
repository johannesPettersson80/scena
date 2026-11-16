//! The reflective layers of a surface: a diffuse base, a GGX specular lobe,
//! an optional sheen layer and an optional clearcoat.
//!
//! This is the same layering the rasterizer's `surface_lighting.wgsl` uses:
//! sheen and clearcoat each take their share of energy from what lies beneath
//! them instead of being added on top. Directions are in the shading frame,
//! with the normal along `+z`, and both point away from the surface.
//!
//! Transmission is not part of this model. The integrator sends the
//! transmitted share of each path through the surface as glass and evaluates
//! these layers for the rest.

use std::f32::consts::PI;

use glam::Vec3;

use super::material::Surface;
use super::sampling::{Rng, cosine_hemisphere, luminance};

const CLEARCOAT_F0: f32 = 0.04;
/// Entries along each axis of the sheen albedo table.
const SHEEN_TABLE: usize = 16;
/// Lowest sheen roughness the table covers.
const SHEEN_MIN_ROUGHNESS: f32 = 0.07;
/// Lowest view cosine the table covers.
const SHEEN_MIN_COSINE: f32 = 0.02;

fn schlick(f0: Vec3, cos_theta: f32) -> Vec3 {
    let weight = (1.0 - cos_theta).clamp(0.0, 1.0).powi(5);
    f0 + (Vec3::ONE - f0) * weight
}

fn ggx_distribution(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha_squared = alpha * alpha;
    let denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    alpha_squared / (PI * denominator * denominator).max(1.0e-12)
}

pub(super) fn smith_g1(n_dot_v: f32, alpha: f32) -> f32 {
    let alpha_squared = alpha * alpha;
    2.0 * n_dot_v
        / (n_dot_v + (alpha_squared + (1.0 - alpha_squared) * n_dot_v * n_dot_v).sqrt())
            .max(1.0e-12)
}

/// GGX reflection without the Fresnel term.
fn ggx_reflection(outgoing: Vec3, incoming: Vec3, half: Vec3, alpha: f32) -> f32 {
    ggx_distribution(half.z, alpha) * smith_g1(outgoing.z, alpha) * smith_g1(incoming.z, alpha)
        / (4.0 * outgoing.z * incoming.z).max(1.0e-12)
}

/// Samples a half vector from the distribution of normals visible from
/// `outgoing` (Heitz, "Sampling the GGX Distribution of Visible Normals").
pub(super) fn sample_visible_normal(outgoing: Vec3, alpha: f32, u: (f32, f32)) -> Vec3 {
    let stretched = Vec3::new(alpha * outgoing.x, alpha * outgoing.y, outgoing.z).normalize();
    let length_squared = stretched.x * stretched.x + stretched.y * stretched.y;
    let tangent = if length_squared > 0.0 {
        Vec3::new(-stretched.y, stretched.x, 0.0) / length_squared.sqrt()
    } else {
        Vec3::X
    };
    let bitangent = stretched.cross(tangent);
    let radius = u.0.sqrt();
    let phi = 2.0 * PI * u.1;
    let x = radius * phi.cos();
    let blend = 0.5 * (1.0 + stretched.z);
    let y = (1.0 - blend) * (1.0 - x * x).max(0.0).sqrt() + blend * radius * phi.sin();
    let normal = tangent * x + bitangent * y + stretched * (1.0 - x * x - y * y).max(0.0).sqrt();
    Vec3::new(alpha * normal.x, alpha * normal.y, normal.z.max(1.0e-6)).normalize()
}

/// Density of the direction obtained by reflecting `outgoing` about a
/// visible-normal sample.
fn visible_normal_reflection_pdf(outgoing: Vec3, half: Vec3, alpha: f32) -> f32 {
    smith_g1(outgoing.z, alpha) * ggx_distribution(half.z, alpha) / (4.0 * outgoing.z).max(1.0e-12)
}

fn charlie_distribution(n_dot_h: f32, alpha: f32) -> f32 {
    let inverse = 1.0 / alpha.max(0.01);
    let sin_squared = (1.0 - n_dot_h * n_dot_h).max(0.0);
    (2.0 + inverse) * sin_squared.powf(inverse * 0.5) / (2.0 * PI)
}

/// The sheen lobe at full strength, without its colour.
fn sheen_lobe(outgoing: Vec3, incoming: Vec3, half: Vec3, alpha: f32) -> f32 {
    let visibility = 1.0 / (4.0 * (outgoing.z + incoming.z - outgoing.z * incoming.z)).max(1.0e-6);
    charlie_distribution(half.z, alpha) * visibility
}

/// Share of light the full-strength sheen lobe reflects in total. Rows run
/// over roughness from [`SHEEN_MIN_ROUGHNESS`] to 1, columns over the view
/// cosine from [`SHEEN_MIN_COSINE`] to 1. The lobe has no closed-form
/// integral, so it was integrated numerically once; a test repeats the
/// integration and compares. Entries above 1 mark views where the lobe alone
/// would reflect more than it receives.
#[rustfmt::skip]
#[allow(clippy::approx_constant, reason = "tabulated albedo; one entry happens to read like log10(2)")]
const SHEEN_ALBEDO: [[f32; SHEEN_TABLE]; SHEEN_TABLE] = [
    [2.6932, 1.0882, 0.4630, 0.1901, 0.0731, 0.0257, 0.0081, 0.0022, 0.0005, 0.0001, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000],
    [2.2268, 1.1000, 0.5779, 0.3010, 0.1519, 0.0732, 0.0332, 0.0139, 0.0053, 0.0018, 0.0005, 0.0001, 0.0000, 0.0000, 0.0000, 0.0000],
    [1.6762, 1.0156, 0.6537, 0.4241, 0.2727, 0.1721, 0.1058, 0.0628, 0.0357, 0.0191, 0.0095, 0.0043, 0.0017, 0.0005, 0.0001, 0.0000],
    [1.3572, 0.9159, 0.6535, 0.4720, 0.3405, 0.2436, 0.1717, 0.1186, 0.0798, 0.0519, 0.0323, 0.0188, 0.0101, 0.0047, 0.0017, 0.0003],
    [1.1520, 0.8310, 0.6311, 0.4860, 0.3750, 0.2882, 0.2196, 0.1652, 0.1220, 0.0881, 0.0617, 0.0414, 0.0262, 0.0151, 0.0074, 0.0023],
    [1.0109, 0.7628, 0.6040, 0.4851, 0.3912, 0.3150, 0.2523, 0.2003, 0.1570, 0.1210, 0.0912, 0.0667, 0.0467, 0.0306, 0.0180, 0.0083],
    [0.9091, 0.7084, 0.5778, 0.4782, 0.3977, 0.3309, 0.2743, 0.2260, 0.1845, 0.1487, 0.1178, 0.0912, 0.0684, 0.0489, 0.0324, 0.0186],
    [0.8331, 0.6650, 0.5544, 0.4691, 0.3991, 0.3400, 0.2891, 0.2448, 0.2057, 0.1713, 0.1407, 0.1135, 0.0893, 0.0679, 0.0489, 0.0322],
    [0.7748, 0.6299, 0.5341, 0.4595, 0.3979, 0.3451, 0.2992, 0.2585, 0.2222, 0.1895, 0.1599, 0.1331, 0.1087, 0.0864, 0.0661, 0.0476],
    [0.7292, 0.6014, 0.5167, 0.4505, 0.3953, 0.3478, 0.3060, 0.2687, 0.2349, 0.2042, 0.1760, 0.1500, 0.1260, 0.1037, 0.0829, 0.0635],
    [0.6928, 0.5780, 0.5019, 0.4422, 0.3923, 0.3490, 0.3108, 0.2763, 0.2450, 0.2161, 0.1894, 0.1646, 0.1413, 0.1194, 0.0987, 0.0792],
    [0.6633, 0.5586, 0.4892, 0.4347, 0.3890, 0.3494, 0.3141, 0.2822, 0.2530, 0.2259, 0.2007, 0.1770, 0.1547, 0.1335, 0.1133, 0.0939],
    [0.6392, 0.5424, 0.4784, 0.4281, 0.3859, 0.3492, 0.3164, 0.2867, 0.2594, 0.2340, 0.2102, 0.1877, 0.1664, 0.1460, 0.1264, 0.1076],
    [0.6191, 0.5288, 0.4691, 0.4223, 0.3830, 0.3487, 0.3181, 0.2903, 0.2646, 0.2407, 0.2182, 0.1969, 0.1766, 0.1571, 0.1383, 0.1200],
    [0.6024, 0.5173, 0.4612, 0.4172, 0.3803, 0.3481, 0.3193, 0.2931, 0.2689, 0.2464, 0.2251, 0.2049, 0.1855, 0.1669, 0.1488, 0.1313],
    [0.5882, 0.5074, 0.4543, 0.4127, 0.3778, 0.3474, 0.3202, 0.2954, 0.2725, 0.2511, 0.2309, 0.2117, 0.1933, 0.1755, 0.1583, 0.1414],
];

/// Share of light a full-strength sheen layer of GGX-style width `alpha`
/// reflects for a view at `n_dot_v`, interpolated from [`SHEEN_ALBEDO`].
fn sheen_albedo(alpha: f32, n_dot_v: f32) -> f32 {
    let position = |value: f32, low: f32| {
        let scaled = ((value - low) / (1.0 - low)).clamp(0.0, 1.0) * (SHEEN_TABLE - 1) as f32;
        let index = (scaled as usize).min(SHEEN_TABLE - 2);
        (index, scaled - index as f32)
    };
    let (row, row_blend) = position(alpha.sqrt(), SHEEN_MIN_ROUGHNESS);
    let (column, column_blend) = position(n_dot_v, SHEEN_MIN_COSINE);
    let along = |row: usize| {
        SHEEN_ALBEDO[row][column] * (1.0 - column_blend)
            + SHEEN_ALBEDO[row][column + 1] * column_blend
    };
    along(row) * (1.0 - row_blend) + along(row + 1) * row_blend
}

/// Share of light a GGX specular layer of reflectance `f0` reflects in total
/// for a view at `n_dot_v`, by Karis's fit to the integrated BRDF ("Physically
/// Based Shading on Mobile").
///
/// What lies beneath a layer receives the rest. Weighting the base by this
/// rather than by the Fresnel term of each light direction keeps the layers
/// from reflecting more than arrives at grazing views.
fn specular_albedo(f0: f32, alpha: f32, n_dot_v: f32) -> f32 {
    let roughness = alpha.sqrt();
    let (r0, r1) = (1.0 - roughness, 0.0425 - 0.0275 * roughness);
    let (r2, r3) = (1.04 - 0.572 * roughness, 0.022 * roughness - 0.04);
    let a004 = (r0 * r0).min((-9.28 * n_dot_v).exp2()) * r0 + r1;
    (f0 * (r2 - 1.04 * a004) + 1.04 * a004 + r3).clamp(0.0, 1.0)
}

/// Probabilities of sampling the diffuse, specular and clearcoat lobes.
struct LobeWeights {
    diffuse: f32,
    specular: f32,
    clearcoat: f32,
}

fn lobe_weights(surface: &Surface, n_dot_v: f32) -> LobeWeights {
    let fresnel = luminance(schlick(surface.f0, n_dot_v));
    let clearcoat = surface.clearcoat * schlick(Vec3::splat(CLEARCOAT_F0), n_dot_v).x;
    let diffuse = (1.0 - surface.metallic)
        * (luminance(surface.base) + 0.5 * luminance(surface.sheen) + 0.02)
        * (1.0 - fresnel);
    let specular = fresnel.max(0.02);
    let total = diffuse + specular + clearcoat;
    LobeWeights {
        diffuse: diffuse / total,
        specular: specular / total,
        clearcoat: clearcoat / total,
    }
}

/// Value of the BSDF for a reflection (without the cosine) and the density
/// with which [`sample`] produces `incoming`.
pub(super) fn evaluate(surface: &Surface, outgoing: Vec3, incoming: Vec3) -> (Vec3, f32) {
    if outgoing.z <= 0.0 || incoming.z <= 0.0 {
        return (Vec3::ZERO, 0.0);
    }
    let half = (outgoing + incoming).normalize();
    let v_dot_h = outgoing.dot(half).max(0.0);
    let weights = lobe_weights(surface, outgoing.z);
    let fresnel = schlick(surface.f0, v_dot_h);
    let specular = fresnel * ggx_reflection(outgoing, incoming, half, surface.alpha);
    let sheen_strength = surface.sheen.max_element();
    // The base keeps what the sheen layer does not reflect. Where the lobe
    // alone would reflect more than arrives, it is scaled back to all of it.
    let (base_scale, sheen_scale) = if sheen_strength > 0.0 {
        let albedo = sheen_albedo(surface.sheen_alpha, outgoing.z);
        (
            1.0 - sheen_strength * albedo.min(1.0),
            albedo.max(1.0).recip(),
        )
    } else {
        (1.0, 1.0)
    };
    let dielectric_f0 = ((surface.ior - 1.0) / (surface.ior + 1.0)).powi(2);
    let beneath_specular = 1.0 - specular_albedo(dielectric_f0, surface.alpha, outgoing.z);
    let diffuse = surface.base * ((1.0 - surface.metallic) * beneath_specular / PI);
    let mut value = (diffuse + specular) * base_scale;
    if sheen_strength > 0.0 {
        let lobe = sheen_lobe(outgoing, incoming, half, surface.sheen_alpha);
        value += surface.sheen * (lobe * sheen_scale);
    }
    let mut pdf = weights.diffuse * incoming.z / PI
        + weights.specular * visible_normal_reflection_pdf(outgoing, half, surface.alpha);
    if surface.clearcoat > 0.0 {
        let coat_fresnel = surface.clearcoat * schlick(Vec3::splat(CLEARCOAT_F0), v_dot_h).x;
        let coat = coat_fresnel * ggx_reflection(outgoing, incoming, half, surface.clearcoat_alpha);
        let beneath_coat = 1.0
            - surface.clearcoat
                * specular_albedo(CLEARCOAT_F0, surface.clearcoat_alpha, outgoing.z);
        value = value * beneath_coat + Vec3::splat(coat);
        pdf += weights.clearcoat
            * visible_normal_reflection_pdf(outgoing, half, surface.clearcoat_alpha);
    }
    (value, pdf)
}

/// Draws an incoming direction in proportion to the lobes' weights.
pub(super) fn sample(surface: &Surface, outgoing: Vec3, rng: &mut Rng) -> Vec3 {
    let weights = lobe_weights(surface, outgoing.z);
    let choice = rng.next();
    let u = rng.pair();
    if choice < weights.diffuse {
        return cosine_hemisphere(u);
    }
    let alpha = if choice < weights.diffuse + weights.specular {
        surface.alpha
    } else {
        surface.clearcoat_alpha
    };
    let half = sample_visible_normal(outgoing, alpha, u);
    half * (2.0 * outgoing.dot(half)) - outgoing
}

/// Unpolarised Fresnel reflectance of a smooth dielectric boundary.
/// `eta` is the incident index over the transmitted index.
pub(super) fn dielectric_fresnel(cos_incident: f32, eta: f32) -> f32 {
    let sin_squared_transmitted = eta * eta * (1.0 - cos_incident * cos_incident);
    if sin_squared_transmitted >= 1.0 {
        return 1.0;
    }
    let cos_transmitted = (1.0 - sin_squared_transmitted).sqrt();
    let perpendicular =
        (eta * cos_incident - cos_transmitted) / (eta * cos_incident + cos_transmitted);
    let parallel = (cos_incident - eta * cos_transmitted) / (cos_incident + eta * cos_transmitted);
    0.5 * (perpendicular * perpendicular + parallel * parallel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(base: Vec3, metallic: f32, roughness: f32) -> Surface {
        Surface {
            base,
            metallic,
            alpha: roughness * roughness,
            f0: Vec3::splat(0.04).lerp(base, metallic),
            clearcoat: 0.0,
            clearcoat_alpha: 0.01,
            sheen: Vec3::ZERO,
            sheen_alpha: 0.25,
            transmission: 0.0,
            ior: 1.5,
            thin_walled: true,
            emission: Vec3::ZERO,
            normal_map: None,
            unlit: false,
        }
    }

    /// Estimates the reflected fraction of a unit environment and the
    /// integral of the sampling density, using the BSDF's own sampler.
    fn furnace(surface: &Surface, outgoing: Vec3) -> (Vec3, f32) {
        let mut rng = Rng::new(11, 3);
        let samples = 60_000;
        let mut albedo = Vec3::ZERO;
        let mut density_mass = 0.0;
        for _ in 0..samples {
            let incoming = sample(surface, outgoing, &mut rng);
            let (value, pdf) = evaluate(surface, outgoing, incoming);
            if pdf > 0.0 {
                albedo += value * (incoming.z / pdf);
            }
            // Uniform hemisphere estimate of the density's integral.
            let (a, b) = rng.pair();
            let z = a;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let uniform = Vec3::new(r * (2.0 * PI * b).cos(), r * (2.0 * PI * b).sin(), z);
            density_mass += evaluate(surface, outgoing, uniform).1 * 2.0 * PI;
        }
        (albedo / samples as f32, density_mass / samples as f32)
    }

    #[test]
    fn no_layer_combination_reflects_more_light_than_it_receives() {
        let outgoing_directions = [
            Vec3::Z,
            Vec3::new(0.6, 0.0, 0.8),
            Vec3::new(0.95, 0.0, 0.312_25),
        ];
        let mut coated = surface(Vec3::splat(0.9), 0.0, 0.5);
        coated.clearcoat = 1.0;
        coated.clearcoat_alpha = 0.05 * 0.05;
        let mut velvet = surface(Vec3::splat(0.9), 0.0, 0.9);
        velvet.sheen = Vec3::ONE;
        let cases = [
            ("white plastic", surface(Vec3::ONE, 0.0, 0.5)),
            ("rough white", surface(Vec3::ONE, 0.0, 1.0)),
            ("polished metal", surface(Vec3::ONE, 1.0, 0.15)),
            ("rough metal", surface(Vec3::ONE, 1.0, 0.8)),
            ("clearcoat", coated),
            ("sheen", velvet),
        ];
        for (name, surface) in &cases {
            for outgoing in outgoing_directions {
                let (albedo, _) = furnace(surface, outgoing);
                assert!(
                    albedo.max_element() <= 1.03,
                    "{name} reflects {albedo:?} of a unit environment at {outgoing:?}"
                );
                assert!(
                    albedo.min_element() > 0.02,
                    "{name} went black at {outgoing:?}: {albedo:?}"
                );
            }
        }
    }

    #[test]
    fn sampling_density_integrates_to_one_over_the_hemisphere() {
        let mut coated = surface(Vec3::splat(0.5), 0.0, 0.4);
        coated.clearcoat = 1.0;
        coated.clearcoat_alpha = 0.2 * 0.2;
        for surface in [
            surface(Vec3::splat(0.5), 0.0, 0.6),
            surface(Vec3::splat(0.9), 1.0, 0.5),
            coated,
        ] {
            for outgoing in [Vec3::Z, Vec3::new(0.6, 0.0, 0.8)] {
                let (_, mass) = furnace(&surface, outgoing);
                // Lobes that reflect below the horizon lose a little mass.
                assert!(
                    (0.9..=1.03).contains(&mass),
                    "density mass {mass} at {outgoing:?}"
                );
            }
        }
    }

    #[test]
    fn lambertian_base_reflects_its_albedo() {
        // A dielectric loses only its specular share from the diffuse term.
        let (albedo, _) = furnace(&surface(Vec3::splat(0.5), 0.0, 1.0), Vec3::Z);
        assert!((albedo.x - 0.5).abs() < 0.06, "albedo {albedo:?}");
    }

    #[test]
    fn sheen_albedo_table_matches_the_integrated_lobe() {
        const POLAR: usize = 160;
        const AZIMUTH: usize = 320;
        let coordinate =
            |index: usize, low: f32| low + (1.0 - low) * index as f32 / (SHEEN_TABLE - 1) as f32;
        for (row, column) in [(0, 3), (2, 0), (5, 5), (7, 1), (10, 9), (15, 0), (15, 15)] {
            let roughness = coordinate(row, SHEEN_MIN_ROUGHNESS);
            let cos_view = coordinate(column, SHEEN_MIN_COSINE).min(1.0);
            let outgoing = Vec3::new((1.0 - cos_view * cos_view).sqrt(), 0.0, cos_view);
            let mut albedo = 0.0_f64;
            for polar in 0..POLAR {
                // Uniform in cos(theta), so each sample covers equal solid angle.
                let cos_theta = (polar as f32 + 0.5) / POLAR as f32;
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
                for azimuth in 0..AZIMUTH {
                    let phi = (azimuth as f32 + 0.5) / AZIMUTH as f32 * 2.0 * PI;
                    let incoming =
                        Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);
                    let half = (outgoing + incoming).normalize();
                    albedo += f64::from(
                        sheen_lobe(outgoing, incoming, half, roughness * roughness) * cos_theta,
                    );
                }
            }
            let albedo = (albedo * f64::from(2.0 * PI) / (POLAR * AZIMUTH) as f64) as f32;
            let stored = SHEEN_ALBEDO[row][column];
            assert!(
                (stored - albedo).abs() <= 0.03 * albedo + 2.0e-3,
                "table entry ({row}, {column}) holds {stored}, the lobe integrates to {albedo}"
            );
            // On a grid point the lookup returns the entry itself.
            assert!((sheen_albedo(roughness * roughness, cos_view) - stored).abs() < 1.0e-3);
        }
    }

    #[test]
    fn a_tight_sheen_lobe_at_a_grazing_view_still_conserves_energy() {
        let mut satin = surface(Vec3::splat(0.9), 0.0, 0.7);
        satin.sheen = Vec3::ONE;
        satin.sheen_alpha = 0.1 * 0.1;
        for cos_view in [0.03_f32, 0.08, 0.2] {
            let outgoing = Vec3::new((1.0 - cos_view * cos_view).sqrt(), 0.0, cos_view);
            let (albedo, _) = furnace(&satin, outgoing);
            assert!(
                albedo.max_element() <= 1.05,
                "reflects {albedo:?} at a view cosine of {cos_view}"
            );
        }
    }

    #[test]
    fn dielectric_fresnel_matches_known_values() {
        // Air to glass at normal incidence reflects ((n-1)/(n+1))^2 = 4 %.
        assert!((dielectric_fresnel(1.0, 1.0 / 1.5) - 0.04).abs() < 1.0e-4);
        // Grazing incidence reflects everything.
        assert!(dielectric_fresnel(0.0, 1.0 / 1.5) > 0.999);
        // Beyond the critical angle inside glass, reflection is total.
        assert_eq!(dielectric_fresnel(0.5, 1.5), 1.0);
    }
}
