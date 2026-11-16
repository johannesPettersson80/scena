//! Rounded edges as a shading effect, and two measurements of the surface
//! around a point that a finish wears and soils by.
//!
//! No manufactured edge is perfectly sharp, and the thin highlight along a
//! rounded edge is much of what tells a photograph from a diagram. Instead of
//! changing the mesh, the tracer looks at the surface within a small radius
//! of each point it shades and uses the average normal it finds there. On a
//! flat or smoothly curved surface that is the normal it already had; next
//! to an edge it leans towards the neighbouring face, which shades as a
//! fillet of about that radius. The silhouette keeps its corners.

use std::f32::consts::PI;

use glam::Vec3;

use super::bvh::Hit;
use super::sampling::{Frame, Rng, cosine_hemisphere};
use super::scene::TraceScene;
use super::shading::lift;

/// Probe lines per shaded point, shared equally between the surface normal
/// and two tangents. Probes along the tangents are what find the far side of
/// a square edge. The image averages its samples, so a few probes each are
/// enough.
const PROBES: usize = 6;
/// Cosine of the largest turn between the shaded surface and a neighbour that
/// counts as the next face round an edge. Anything turned further is the
/// far side of a thin part.
const MIN_FACING: f32 = -0.5;
/// Lowest density a neighbour is weighted by, which bounds the weight of
/// one found at a glancing angle.
const MIN_DENSITY: f32 = 0.05;

pub(super) struct Rounded {
    /// Outward normal averaged over the surface within the radius.
    pub(super) normal: Vec3,
    /// How far the surface around the point falls below its tangent plane,
    /// as a share of the radius: nothing on a flat or gently curved surface,
    /// [`SQUARE_EDGE`] on a square edge, more at a corner.
    pub(super) convexity: f32,
}

/// Weighted sums over the surface a set of probes found.
pub(super) struct Gathered {
    pub(super) sum: Vec3,
    pub(super) total: f32,
    pub(super) fall: f32,
}

/// [`Rounded::convexity`] on a square edge.
pub(super) const SQUARE_EDGE: f32 = 0.145;

impl TraceScene {
    /// Unit winding normal and shading normal at a hit, both facing out of
    /// the surface's front.
    pub(super) fn outward_normals(&self, hit: &Hit) -> (Vec3, Vec3) {
        let normals = &self.attributes[hit.triangle as usize].normals;
        let winding = self.winding_normal(hit.triangle).normalize_or_zero();
        let smooth = (normals[0] * (1.0 - hit.u - hit.v) + normals[1] * hit.u + normals[2] * hit.v)
            .normalize_or_zero();
        // Vertex normals that disagree with the winding would light the
        // wrong side; the face normal is the safe reading.
        if smooth.dot(winding) <= 0.0 {
            (winding, winding)
        } else {
            (winding, smooth)
        }
    }

    /// The surface of `triangle`'s object within `radius` of `position`,
    /// averaged. `normal` is the unit winding normal there and `fallback`
    /// the result when the probes find nothing.
    pub(super) fn rounded(
        &self,
        triangle: u32,
        position: Vec3,
        normal: Vec3,
        fallback: Vec3,
        radius: f32,
        rng: &mut Rng,
    ) -> Rounded {
        let gathered = self.gather(triangle, position, normal, radius, rng);
        let length = gathered.sum.length();
        if !(gathered.total > 0.0 && length > 1.0e-4 * gathered.total) {
            return Rounded {
                normal: fallback,
                convexity: 0.0,
            };
        }
        Rounded {
            normal: gathered.sum / length,
            convexity: (gathered.fall / gathered.total).max(0.0),
        }
    }

    /// One estimate of the surface around a point: its normals, weighted by
    /// nearness, and how far it lies below the tangent plane.
    pub(super) fn gather(
        &self,
        triangle: u32,
        position: Vec3,
        normal: Vec3,
        radius: f32,
        rng: &mut Rng,
    ) -> Gathered {
        let object = self.attributes[triangle as usize].object;
        // The tangents are spun at random so no edge direction is favoured.
        let frame = Frame::new(normal);
        let (sin, cos) = (2.0 * PI * rng.next()).sin_cos();
        let axes = [
            frame.tangent * cos + frame.bitangent * sin,
            frame.bitangent * cos - frame.tangent * sin,
            normal,
        ];
        let inverse_square = 1.0 / (radius * radius);
        let mut gathered = Gathered {
            sum: Vec3::ZERO,
            total: 0.0,
            fall: 0.0,
        };
        for probe in 0..PROBES {
            let axis = axes[probe % 3];
            let (across_a, across_b) = (axes[(probe + 1) % 3], axes[(probe + 2) % 3]);
            // A point on the disc across the axis, denser towards its
            // centre: its density falls as `1 - (rho / radius)^2`.
            let (u, v) = rng.pair();
            let rho_square = 1.0 - (1.0 - u).max(0.0).sqrt();
            let rho = radius * rho_square.sqrt();
            let phi = 2.0 * PI * v;
            // The probe is the chord of the sphere through that point.
            let half = radius * (1.0 - rho_square).max(0.0).sqrt();
            if half <= radius * 1.0e-3 {
                continue;
            }
            let start = position
                + across_a * (rho * phi.cos())
                + across_b * (rho * phi.sin())
                + axis * half;
            self.bvh
                .intersect(&self.positions, start, -axis, 2.0 * half, true, |hit| {
                    if self.attributes[hit.triangle as usize].object != object {
                        return false;
                    }
                    let (winding, smooth) = self.outward_normals(hit);
                    if smooth.dot(normal) < MIN_FACING {
                        return false;
                    }
                    let reach = start - axis * hit.distance - position;
                    let distance_square = reach.length_squared() * inverse_square;
                    if distance_square >= 1.0 {
                        return false;
                    }
                    // Density with which any of the three probe directions
                    // would have found this point; weighing by it makes the
                    // three one estimate of the surface around the hit.
                    let density: f32 = axes
                        .iter()
                        .map(|other| {
                            let height = reach.dot(*other);
                            let across =
                                (reach.length_squared() - height * height) * inverse_square;
                            (1.0 - across).max(0.0) * winding.dot(*other).abs()
                        })
                        .sum();
                    let weight = (1.0 - distance_square).powi(2) / density.max(MIN_DENSITY);
                    gathered.sum += smooth * weight;
                    gathered.total += weight;
                    gathered.fall -= weight * reach.dot(normal) / radius;
                    // Never accepted: the probe gathers every surface it crosses.
                    false
                });
        }
        gathered
    }

    /// Whether something stands within `reach` of the surface in a random
    /// direction above it. Averaged over an image's samples this is how
    /// enclosed the point is: nothing on an open face, most in a crevice.
    pub(super) fn enclosed(
        &self,
        triangle: u32,
        position: Vec3,
        normal: Vec3,
        reach: f32,
        rng: &mut Rng,
    ) -> bool {
        let direction = Frame::new(normal).to_world(cosine_hemisphere(rng.pair()));
        self.bvh
            .intersect(
                &self.positions,
                position + normal * lift(position),
                direction,
                reach,
                true,
                |hit| hit.triangle != triangle,
            )
            .is_some()
    }
}
