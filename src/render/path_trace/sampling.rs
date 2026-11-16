//! Random numbers and the sampling routines shared by the integrator.

use std::f32::consts::PI;

use glam::Vec3;

const WEYL_STEP: u64 = 0x9E37_79B9_7F4A_7C15;

/// The SplitMix64 output function: every input bit affects every output bit.
fn scramble(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// SplitMix64 stream. One instance per pixel, seeded from the pixel index
/// and the user's seed, so a render is reproducible whatever the thread
/// count and tile order.
pub(super) struct Rng(u64);

impl Rng {
    pub(super) fn new(seed: u64, stream: u64) -> Self {
        // The generator steps its state by a constant, so two states a few
        // steps apart yield the same numbers a few draws apart. Scrambling
        // the stream index scatters neighbouring pixels across the whole
        // period instead of leaving them one step from each other, which
        // would show as streaks along image rows.
        Self(scramble(scramble(stream.wrapping_add(WEYL_STEP)) ^ seed))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(WEYL_STEP);
        scramble(self.0)
    }

    /// Uniform in `[0, 1)`.
    pub(super) fn next(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) * (1.0 / 16_777_216.0)
    }

    pub(super) fn pair(&mut self) -> (f32, f32) {
        (self.next(), self.next())
    }
}

/// Tangent and bitangent completing `normal` to an orthonormal basis
/// (Duff et al., "Building an Orthonormal Basis, Revisited").
pub(super) fn orthonormal_basis(normal: Vec3) -> (Vec3, Vec3) {
    let sign = 1.0_f32.copysign(normal.z);
    let a = -1.0 / (sign + normal.z);
    let b = normal.x * normal.y * a;
    (
        Vec3::new(
            1.0 + sign * normal.x * normal.x * a,
            sign * b,
            -sign * normal.x,
        ),
        Vec3::new(b, sign + normal.y * normal.y * a, -normal.y),
    )
}

/// Shading frame with the normal along local `z`.
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) tangent: Vec3,
    pub(super) bitangent: Vec3,
    pub(super) normal: Vec3,
}

impl Frame {
    pub(super) fn new(normal: Vec3) -> Self {
        let (tangent, bitangent) = orthonormal_basis(normal);
        Self {
            tangent,
            bitangent,
            normal,
        }
    }

    pub(super) fn to_local(self, world: Vec3) -> Vec3 {
        Vec3::new(
            world.dot(self.tangent),
            world.dot(self.bitangent),
            world.dot(self.normal),
        )
    }

    pub(super) fn to_world(self, local: Vec3) -> Vec3 {
        self.tangent * local.x + self.bitangent * local.y + self.normal * local.z
    }
}

/// Cosine-weighted direction on the local `+z` hemisphere; its density is
/// `z / PI`.
pub(super) fn cosine_hemisphere(u: (f32, f32)) -> Vec3 {
    let radius = u.0.sqrt();
    let phi = 2.0 * PI * u.1;
    Vec3::new(
        radius * phi.cos(),
        radius * phi.sin(),
        (1.0 - u.0).max(0.0).sqrt(),
    )
}

/// Uniform point on the unit disc.
pub(super) fn uniform_disc(u: (f32, f32)) -> (f32, f32) {
    let radius = u.0.sqrt();
    let phi = 2.0 * PI * u.1;
    (radius * phi.cos(), radius * phi.sin())
}

/// Uniform direction inside the cone around local `+z` whose spherical cap
/// has `height = 1 - cos(half angle)`; its density is `1 / (2 PI height)`.
/// Working with the height keeps very narrow cones exact, where the cosine
/// itself rounds to one.
pub(super) fn uniform_cone(u: (f32, f32), height: f32) -> Vec3 {
    let drop = u.0 * height;
    let sin_theta = (drop * (2.0 - drop)).max(0.0).sqrt();
    let phi = 2.0 * PI * u.1;
    Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), 1.0 - drop)
}

/// Balance between two sampling strategies (Veach's power heuristic).
pub(super) fn power_heuristic(pdf: f32, other: f32) -> f32 {
    if pdf.is_infinite() {
        return 1.0;
    }
    let (a, b) = (pdf * pdf, other * other);
    if a + b > 0.0 { a / (a + b) } else { 0.0 }
}

pub(super) fn luminance(color: Vec3) -> f32 {
    0.2126 * color.x + 0.7152 * color.y + 0.0722 * color.z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbouring_streams_do_not_repeat_each_other() {
        for seed in [0, 1, 0xDEAD_BEEF] {
            let draws = |stream: u64| {
                let mut rng = Rng::new(seed, stream);
                (0..16).map(|_| rng.next_u64()).collect::<Vec<_>>()
            };
            for stream in 0..2_000 {
                let (here, next) = (draws(stream), draws(stream + 1));
                assert!(
                    here.iter().all(|value| !next.contains(value)),
                    "streams {stream} and {} share a draw for seed {seed}",
                    stream + 1
                );
            }
        }
    }

    #[test]
    fn draws_are_uniform_and_streams_uncorrelated() {
        let (mut sum, mut product, count) = (0.0_f64, 0.0_f64, 40_000);
        for stream in 0..count {
            let (a, b) = (Rng::new(5, stream).next(), Rng::new(5, stream + 1).next());
            assert!((0.0..1.0).contains(&a));
            sum += f64::from(a);
            product += f64::from(a) * f64::from(b);
        }
        let (mean, mean_product) = (sum / count as f64, product / count as f64);
        assert!((mean - 0.5).abs() < 0.01, "mean {mean}");
        // Independent uniform draws multiply to a quarter on average.
        assert!(
            (mean_product - 0.25).abs() < 0.01,
            "mean product {mean_product}"
        );
    }

    #[test]
    fn narrow_cones_keep_their_width() {
        // A cone of half-angle 1e-3 rad has a cap 5e-7 high, which rounds
        // away when the cosine is formed first.
        let height = 2.0 * (0.5e-3_f32).sin().powi(2);
        let edge = uniform_cone((1.0, 0.0), height);
        let angle = edge.x.atan2(edge.z);
        assert!(
            (angle - 1.0e-3).abs() < 1.0e-6,
            "edge of the cone lies at {angle} rad"
        );
        assert!((edge.length() - 1.0).abs() < 1.0e-6);
    }
}
