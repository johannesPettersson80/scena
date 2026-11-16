//! Smooth random fields in three dimensions, with their gradients. Surface
//! finishes are built from these, so they need no texture coordinates.

use glam::{Mat3, Vec3};

/// Root-mean-square length of [`noise`]'s gradient, measured over many cells.
pub(super) const GRADIENT_RMS: f32 = 1.23;

/// The twelve edge directions of a cube, four of them twice, so a lattice
/// corner picks one with four bits of its hash.
const GRADIENTS: [Vec3; 16] = [
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(1.0, -1.0, 0.0),
    Vec3::new(-1.0, -1.0, 0.0),
    Vec3::new(1.0, 0.0, 1.0),
    Vec3::new(-1.0, 0.0, 1.0),
    Vec3::new(1.0, 0.0, -1.0),
    Vec3::new(-1.0, 0.0, -1.0),
    Vec3::new(0.0, 1.0, 1.0),
    Vec3::new(0.0, -1.0, 1.0),
    Vec3::new(0.0, 1.0, -1.0),
    Vec3::new(0.0, -1.0, -1.0),
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(0.0, -1.0, 1.0),
    Vec3::new(0.0, -1.0, -1.0),
];

/// A rotation with no axis in common with the lattice. Each octave of a
/// fractal sum is turned by it, which hides the lattice's grain.
pub(super) const OCTAVE_TURN: Mat3 = Mat3::from_cols(
    Vec3::new(0.0, 0.8, 0.6),
    Vec3::new(-0.8, 0.36, -0.48),
    Vec3::new(-0.6, -0.48, 0.64),
);

fn hash(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    let mut bits = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D)
        ^ seed.wrapping_mul(0x27D4_EB2F);
    bits ^= bits >> 15;
    bits = bits.wrapping_mul(0x2C1B_3C6D);
    bits ^= bits >> 12;
    bits = bits.wrapping_mul(0x297A_2D39);
    bits ^ (bits >> 15)
}

/// Gradient noise at `point`: a value in about `-1..=1` that varies smoothly
/// over a distance of one, and its gradient.
pub(super) fn noise(point: Vec3, seed: u32) -> (f32, Vec3) {
    let cell = point.floor();
    let f = point - cell;
    let (x, y, z) = (cell.x as i32, cell.y as i32, cell.z as i32);
    // Quintic fade and its derivative: the field and its gradient are both
    // continuous across cells.
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let corner = |dx: i32, dy: i32, dz: i32| {
        let bits = hash(
            x.wrapping_add(dx),
            y.wrapping_add(dy),
            z.wrapping_add(dz),
            seed,
        );
        let gradient = GRADIENTS[(bits & 15) as usize];
        (
            gradient,
            gradient.dot(f - Vec3::new(dx as f32, dy as f32, dz as f32)),
        )
    };
    let (ga, va) = corner(0, 0, 0);
    let (gb, vb) = corner(1, 0, 0);
    let (gc, vc) = corner(0, 1, 0);
    let (gd, vd) = corner(1, 1, 0);
    let (ge, ve) = corner(0, 0, 1);
    let (gf, vf) = corner(1, 0, 1);
    let (gg, vg) = corner(0, 1, 1);
    let (gh, vh) = corner(1, 1, 1);
    let (kx, ky, kz) = (vb - va, vc - va, ve - va);
    let (kxy, kyz, kzx) = (va - vb - vc + vd, va - vc - ve + vg, va - vb - ve + vf);
    let kxyz = -va + vb + vc - vd + ve - vf - vg + vh;
    let value = va
        + u.x * kx
        + u.y * ky
        + u.z * kz
        + u.x * u.y * kxy
        + u.y * u.z * kyz
        + u.z * u.x * kzx
        + u.x * u.y * u.z * kxyz;
    let gradient = ga
        + (gb - ga) * u.x
        + (gc - ga) * u.y
        + (ge - ga) * u.z
        + (ga - gb - gc + gd) * (u.x * u.y)
        + (ga - gc - ge + gg) * (u.y * u.z)
        + (ga - gb - ge + gf) * (u.z * u.x)
        + (-ga + gb + gc - gd + ge - gf - gg + gh) * (u.x * u.y * u.z)
        + du * Vec3::new(
            kx + u.y * kxy + u.z * kzx + u.y * u.z * kxyz,
            ky + u.z * kyz + u.x * kxy + u.z * u.x * kxyz,
            kz + u.x * kzx + u.y * kyz + u.x * u.y * kxyz,
        );
    (value, gradient)
}

/// [`noise`] without its gradient, for fields that only tint or dull.
pub(super) fn value(point: Vec3, seed: u32) -> f32 {
    noise(point, seed).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points() -> impl Iterator<Item = Vec3> {
        (0..4000).map(|index| {
            let t = index as f32;
            Vec3::new(t * 0.3713 - 37.0, t * 0.1931 + 5.5, 91.0 - t * 0.2577)
        })
    }

    #[test]
    fn the_gradient_matches_finite_differences() {
        let step = 2.0e-3;
        for point in points().step_by(7).map(|point| point * 0.05) {
            let (_, gradient) = noise(point, 11);
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let slope = (value(point + axis * step, 11) - value(point - axis * step, 11))
                    / (2.0 * step);
                assert!(
                    (slope - gradient.dot(axis)).abs() < 0.02,
                    "at {point}: {slope} against {}",
                    gradient.dot(axis)
                );
            }
        }
    }

    #[test]
    fn the_field_is_balanced_bounded_and_seeded() {
        let (mut sum, mut peak, mut squared, mut differs) = (0.0_f64, 0.0_f32, 0.0_f64, 0);
        let count = points().count() as f64;
        for point in points() {
            let (value, gradient) = noise(point, 3);
            sum += f64::from(value);
            peak = peak.max(value.abs());
            squared += f64::from(gradient.length_squared());
            differs += usize::from((value - noise(point, 4).0).abs() > 1.0e-3);
        }
        assert!((sum / count).abs() < 0.02, "mean {}", sum / count);
        assert!(peak <= 1.05 && peak > 0.5, "peak {peak}");
        let rms = (squared / count).sqrt() as f32;
        assert!(
            (rms - GRADIENT_RMS).abs() < 0.05 * GRADIENT_RMS,
            "gradient rms {rms}"
        );
        assert!(differs > 3500, "another seed gives another field");
    }

    #[test]
    fn the_field_is_continuous_across_cells() {
        for cell in -3..3 {
            let edge = cell as f32;
            let below = noise(Vec3::new(edge - 1.0e-4, 0.37, 0.61), 5);
            let above = noise(Vec3::new(edge + 1.0e-4, 0.37, 0.61), 5);
            assert!((below.0 - above.0).abs() < 1.0e-3);
            assert!((below.1 - above.1).length() < 2.0e-2);
        }
    }

    #[test]
    fn the_octave_turn_is_a_rotation() {
        let product = OCTAVE_TURN * OCTAVE_TURN.transpose();
        assert!((product.x_axis - Vec3::X).length() < 1.0e-6);
        assert!((product.y_axis - Vec3::Y).length() < 1.0e-6);
        assert!((OCTAVE_TURN.determinant() - 1.0).abs() < 1.0e-6);
    }
}
