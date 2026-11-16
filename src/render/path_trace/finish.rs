//! Surface finishes: the grain, waviness, tool marks, gloss and tone
//! variation, wear and grime that a [`SurfaceFinish`] asks for, generated
//! from the position on the surface.
//!
//! Every pattern is a function of a point in the mesh's own frame, measured
//! in metres, so it needs no texture coordinates, never tiles, and keeps its
//! physical size however the mesh is scaled or placed.

use glam::{Affine3A, Mat3, Vec3};

use crate::assets::PhotographicSurfaceKind as Kind;
use crate::material::SurfaceFinish;

use super::material::{MIN_ROUGHNESS, Surface};
use super::noise::{GRADIENT_RMS, OCTAVE_TURN, noise, value};
use super::rounding::SQUARE_EDGE;
use super::sampling::Rng;

/// Each octave of a fractal relief is this much shallower than the last.
const OCTAVE_FALLOFF: f32 = 0.8;
/// Root-mean-square of one component of the noise gradient.
const AXIS_RMS: f32 = GRADIENT_RMS * 0.577_350_3;
/// Root-mean-square length of the noise gradient along a surface.
const SURFACE_RMS: f32 = AXIS_RMS * std::f32::consts::SQRT_2;
/// How much longer than wide the streaks of a brushed finish are.
const BRUSH_ELONGATION: f32 = 60.0;
/// Bare steel, which shows where paint has worn through.
const BARE_METAL: Vec3 = Vec3::new(0.56, 0.57, 0.58);
const DIRT: Vec3 = Vec3::new(0.05, 0.043, 0.036);
/// Distance within which nearby surfaces shelter dirt, at scale one.
const GRIME_REACH_M: f32 = 0.012;

#[derive(Clone, Copy, PartialEq)]
enum Lay {
    None,
    /// Fine parallel streaks along the finish's axis.
    Brushed,
    /// Grooves cut by turning about the finish's axis.
    Turned,
}

/// What wears: bare metal burnishes, a coating wears through to metal, and
/// plastic, rubber and cloth scuff paler.
#[derive(Clone, Copy, PartialEq)]
enum Body {
    Metal,
    Coated,
    Soft,
}

/// The measurements of one kind of finish. Sizes are metres, slopes are the
/// root-mean-square slope of the relief.
#[derive(Clone, Copy)]
struct Recipe {
    /// Fine isotropic relief: wavelength, slope, octaves.
    grain: (f32, f32, u32),
    /// Broad relief, such as the orange peel of paint: wavelength, slope.
    wave: (f32, f32),
    lay: Lay,
    /// Width of a brushed streak, or the feed between two turned grooves.
    lay_pitch: f32,
    lay_slope: f32,
    /// Patches of duller and glossier surface: size, share of the roughness.
    gloss: (f32, f32),
    /// Patches of lighter and darker surface: size, share of the colour.
    tone: (f32, f32),
    edge_radius: f32,
    body: Body,
}

impl Recipe {
    fn of(kind: Kind) -> Self {
        let recipe = |grain, wave, gloss, tone, edge_radius, body| Self {
            grain,
            wave,
            lay: Lay::None,
            lay_pitch: 0.0,
            lay_slope: 0.0,
            gloss,
            tone,
            edge_radius,
            body,
        };
        match kind {
            Kind::PolishedMetal => recipe(
                (0.0, 0.0, 0),
                (0.02, 0.004),
                (0.015, 0.45),
                (0.03, 0.02),
                0.000_25,
                Body::Metal,
            ),
            Kind::SatinMetal => recipe(
                (0.000_1, 0.1, 2),
                (0.025, 0.004),
                (0.02, 0.2),
                (0.03, 0.03),
                0.000_3,
                Body::Metal,
            ),
            Kind::BrushedMetal => Self {
                lay: Lay::Brushed,
                lay_pitch: 0.000_08,
                lay_slope: 0.16,
                ..recipe(
                    (0.0, 0.0, 0),
                    (0.03, 0.003),
                    (0.02, 0.15),
                    (0.03, 0.03),
                    0.000_3,
                    Body::Metal,
                )
            },
            Kind::MachinedMetal => Self {
                lay: Lay::Turned,
                lay_pitch: 0.000_25,
                lay_slope: 0.14,
                ..recipe(
                    (0.0, 0.0, 0),
                    (0.0, 0.0),
                    (0.02, 0.12),
                    (0.03, 0.03),
                    0.000_2,
                    Body::Metal,
                )
            },
            Kind::CastMetal => recipe(
                (0.001_2, 0.24, 3),
                (0.009, 0.04),
                (0.02, 0.22),
                (0.025, 0.1),
                0.001_5,
                Body::Metal,
            ),
            Kind::PaintedMetal => recipe(
                (0.0, 0.0, 0),
                (0.002_5, 0.022),
                (0.04, 0.12),
                (0.05, 0.03),
                0.000_5,
                Body::Coated,
            ),
            Kind::PowderCoatedMetal => recipe(
                (0.000_3, 0.05, 2),
                (0.001_6, 0.06),
                (0.03, 0.15),
                (0.04, 0.04),
                0.000_8,
                Body::Coated,
            ),
            Kind::MoldedPlastic => recipe(
                (0.000_18, 0.09, 2),
                (0.012, 0.006),
                (0.03, 0.18),
                (0.04, 0.03),
                0.000_4,
                Body::Soft,
            ),
            Kind::ClearcoatPlastic => recipe(
                (0.0, 0.0, 0),
                (0.003, 0.012),
                (0.05, 0.1),
                (0.05, 0.02),
                0.000_5,
                Body::Soft,
            ),
            Kind::Rubber => recipe(
                (0.000_25, 0.14, 2),
                (0.01, 0.01),
                (0.02, 0.2),
                (0.02, 0.06),
                0.000_6,
                Body::Soft,
            ),
            Kind::Fabric => recipe(
                (0.000_4, 0.3, 2),
                (0.006, 0.05),
                (0.02, 0.15),
                (0.015, 0.12),
                0.001,
                Body::Soft,
            ),
        }
    }
}

/// A [`SurfaceFinish`] placed on one object of the traced scene.
pub(super) struct TraceFinish {
    recipe: Recipe,
    /// Camera-relative world position to the finish's frame: the mesh's own
    /// frame without its scale, in metres over the finish's scale.
    to_finish: Affine3A,
    /// Directions in the finish's frame back to the world.
    to_world: Mat3,
    scale: f32,
    relief: f32,
    wear: f32,
    grime: f32,
    axis: Vec3,
    seed: u32,
    pub(super) edge_radius: f32,
    /// Distance within which nearby surfaces shelter dirt, or zero for a
    /// finish without grime.
    pub(super) grime_reach: f32,
}

/// The finish at one point of a surface.
pub(super) struct Relief {
    /// Slope of the relief in world orientation; the normal leans against
    /// the part of it that lies along the surface.
    pub(super) slope: Vec3,
    /// Slope variance too fine for the image to show. It widens the
    /// highlight instead, as it does in a photograph.
    unresolved: f32,
    gloss: f32,
    tone: f32,
    point: Vec3,
}

fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Share of a pattern of wavelength `size` that an image with pixels
/// `footprint` wide resolves.
fn resolved(size: f32, footprint: f32) -> f32 {
    smoothstep(0.5, 2.0, size / footprint.max(1.0e-12))
}

/// A standard normal variate, kept within three deviations.
fn gaussian(rng: &mut Rng) -> f32 {
    let (u, v) = rng.pair();
    let radius = (-2.0 * (1.0 - u).max(1.0e-7).ln()).sqrt();
    (radius * (2.0 * std::f32::consts::PI * v).cos()).clamp(-3.0, 3.0)
}

impl TraceFinish {
    /// Places `finish` on an object whose camera-relative world placement is
    /// `world`.
    pub(super) fn new(finish: SurfaceFinish, world: Affine3A) -> Self {
        let recipe = Recipe::of(finish.kind());
        let (x, y) = (
            Vec3::from(world.matrix3.x_axis),
            Vec3::from(world.matrix3.y_axis),
        );
        let right = x.normalize_or(Vec3::X);
        let up = (y - right * right.dot(y)).normalize_or(right.any_orthonormal_vector());
        let to_world = Mat3::from_cols(right, up, right.cross(up));
        let scale = finish.scale();
        let rotation = to_world.transpose();
        let to_finish = Affine3A::from_mat3_translation(
            rotation * (1.0 / scale),
            rotation * Vec3::from(world.translation) * (-1.0 / scale),
        );
        Self {
            recipe,
            to_finish,
            to_world,
            scale,
            relief: finish.relief(),
            wear: finish.wear(),
            grime: finish.grime(),
            axis: (rotation * world.transform_vector3(finish.axis())).normalize_or(Vec3::Y),
            seed: finish.seed().wrapping_mul(0x9E37_79B9),
            edge_radius: finish.edge_radius_m().unwrap_or(recipe.edge_radius * scale),
            grime_reach: if finish.grime() > 0.0 {
                GRIME_REACH_M * scale
            } else {
                0.0
            },
        }
    }

    /// The finish at `position` on a surface whose outward normal is
    /// `normal`, seen with pixels `footprint` wide there.
    pub(super) fn relief(
        &self,
        position: Vec3,
        normal: Vec3,
        footprint: f32,
        rng: &mut Rng,
    ) -> Relief {
        let point = self.to_finish.transform_point3(position);
        let footprint = footprint / self.scale;
        let recipe = &self.recipe;
        let (mut slope, mut unresolved) = (Vec3::ZERO, 0.0);
        let (size, sigma, octaves) = recipe.grain;
        self.fractal(
            point,
            (size, sigma, octaves),
            footprint,
            1,
            &mut slope,
            &mut unresolved,
        );
        let (size, sigma) = recipe.wave;
        self.fractal(
            point,
            (size, sigma, 2),
            footprint,
            2,
            &mut slope,
            &mut unresolved,
        );
        match recipe.lay {
            Lay::None => {}
            Lay::Brushed => {
                let across = self.axis.cross(self.to_world.transpose() * normal);
                self.brushed(point, across, footprint, &mut slope, &mut unresolved, rng);
            }
            Lay::Turned => slope += self.turned(point, footprint, rng),
        }
        let patch = |(size, amount): (f32, f32), salt: u32| {
            1.0 + amount
                * (0.75 * value(point / size, self.seed ^ salt)
                    + 0.5 * value(OCTAVE_TURN * point / (size * 0.31), self.seed ^ (salt + 1)))
        };
        Relief {
            slope: self.to_world * (slope * self.relief),
            unresolved: unresolved * self.relief * self.relief,
            gloss: patch(recipe.gloss, 0x10).clamp(0.4, 2.0),
            tone: patch(recipe.tone, 0x20).clamp(0.5, 1.5),
            point,
        }
    }

    /// Adds a fractal relief whose coarsest octave has wavelength `size` and
    /// whose slope is `sigma` in all.
    fn fractal(
        &self,
        point: Vec3,
        (size, sigma, octaves): (f32, f32, u32),
        footprint: f32,
        salt: u32,
        slope: &mut Vec3,
        unresolved: &mut f32,
    ) {
        if sigma <= 0.0 || octaves == 0 {
            return;
        }
        let norm = (0..octaves)
            .map(|octave| OCTAVE_FALLOFF.powi(2 * octave as i32))
            .sum::<f32>()
            .sqrt();
        let mut turn = Mat3::IDENTITY;
        for octave in 0..octaves {
            let size = size / (1 << octave) as f32;
            let share = sigma * OCTAVE_FALLOFF.powi(octave as i32) / norm;
            let seen = resolved(size, footprint);
            *unresolved += share * share * (1.0 - seen * seen);
            if seen > 0.0 {
                let seed = self.seed ^ (salt << 8 | octave);
                let (_, gradient) = noise(turn * point / size, seed);
                *slope += turn.transpose() * gradient * (seen * share / SURFACE_RMS);
            }
            turn = OCTAVE_TURN * turn;
        }
    }

    /// Adds the streaks of a brushed finish. `across` lies in the surface,
    /// square to the brushing, and need not be a unit vector.
    fn brushed(
        &self,
        point: Vec3,
        across: Vec3,
        footprint: f32,
        slope: &mut Vec3,
        unresolved: &mut f32,
        rng: &mut Rng,
    ) {
        let along = point.dot(self.axis);
        let cross = point - self.axis * along;
        // The brushing itself, and the broader bands it gathers into.
        for (band, (widening, share)) in [(1.0, 0.92), (12.0, 0.38)].into_iter().enumerate() {
            let width = self.recipe.lay_pitch * widening;
            let sigma = self.recipe.lay_slope * share;
            let seen = resolved(width, footprint);
            if seen > 0.0 {
                let stretched = cross / width + self.axis * (along / (width * BRUSH_ELONGATION));
                let (_, gradient) = noise(stretched, self.seed ^ (0x300 + band as u32));
                let lengthwise = gradient.dot(self.axis);
                *slope += (gradient - self.axis * (lengthwise * (1.0 - 1.0 / BRUSH_ELONGATION)))
                    * (seen * sigma / AXIS_RMS);
            }
            // Streaks finer than a pixel still spread the highlight across
            // the brushing and not along it, which is the look of the finish.
            let hidden = sigma * (1.0 - seen * seen).max(0.0).sqrt();
            if hidden > 0.0 {
                if across.length_squared() > 0.01 {
                    *slope += across.normalize() * (hidden * gaussian(rng));
                } else {
                    *unresolved += hidden * hidden;
                }
            }
        }
    }

    /// Slope of the grooves left by turning: hoops along a cylinder, rings
    /// on a face.
    fn turned(&self, point: Vec3, footprint: f32, rng: &mut Rng) -> Vec3 {
        let pitch = self.recipe.lay_pitch;
        let along = point.dot(self.axis);
        let radial = point - self.axis * along;
        let radius = radial.length();
        let outward = if radius > 1.0e-9 {
            radial / radius
        } else {
            Vec3::ZERO
        };
        // A cutting tool leaves scallops: the slope ramps across each one.
        let ramp = |x: f32| 2.0 * (x - x.floor()) - 1.0;
        let depth =
            self.recipe.lay_slope * (0.8 + 0.2 * value(point / (pitch * 40.0), self.seed ^ 0x400));
        let seen = resolved(pitch, footprint);
        let hidden = (1.0 - seen * seen).max(0.0).sqrt();
        let (jitter_along, jitter_radial) = if hidden > 0.0 {
            let (u, v) = rng.pair();
            (hidden * (2.0 * u - 1.0), hidden * (2.0 * v - 1.0))
        } else {
            (0.0, 0.0)
        };
        self.axis * (depth * (seen * ramp(along / pitch) + jitter_along))
            + outward * (depth * (seen * ramp(radius / pitch + 0.37) + jitter_radial))
    }

    /// Changes `surface` to the finish at `relief`. `convexity` is how
    /// sharply the surface turns outward there and `enclosed` whether a
    /// probe found the point sheltered.
    pub(super) fn apply(
        &self,
        surface: &mut Surface,
        relief: &Relief,
        convexity: f32,
        enclosed: bool,
    ) {
        let mix = |from: f32, to: f32, share: f32| from + (to - from) * share;
        let mut roughness = surface.alpha.sqrt() * relief.gloss;
        let mut base = surface.base * relief.tone;
        let (mut metallic, mut coat) = (surface.metallic, surface.clearcoat);
        if self.wear > 0.0 && convexity > 0.0 {
            // Wear starts on the edge itself, in chips, and spreads.
            let chipped =
                convexity / SQUARE_EDGE + 0.35 * value(relief.point / 0.002, self.seed ^ 0x500);
            let threshold = 1.0 - 1.2 * self.wear;
            let worn = smoothstep(threshold, threshold + 0.35, chipped);
            if self.recipe.body == Body::Soft {
                base = base.lerp(base * 0.5 + Vec3::splat(0.3), 0.4 * worn);
                roughness = mix(roughness, 0.75, 0.5 * worn);
            } else {
                let bare = if metallic > 0.5 {
                    (base * 1.35).min(Vec3::splat(0.9))
                } else {
                    BARE_METAL
                };
                base = base.lerp(bare, worn);
                metallic = mix(metallic, 1.0, worn);
                roughness = mix(roughness, roughness.min(0.3), worn);
            }
            coat *= 1.0 - worn;
        }
        if enclosed && self.grime > 0.0 {
            let patch = smoothstep(
                -0.5,
                0.3,
                value(relief.point / 0.015, self.seed ^ 0x600)
                    + 0.4 * value(OCTAVE_TURN * relief.point / 0.004, self.seed ^ 0x601),
            );
            let soiled = (self.grime * 1.3 * patch).min(0.92);
            base = base.lerp(DIRT, soiled);
            metallic *= 1.0 - soiled;
            roughness = mix(roughness, 0.9, soiled);
            coat *= 1.0 - soiled;
        }
        let alpha = roughness.clamp(MIN_ROUGHNESS, 1.0).powi(2);
        surface.alpha = (alpha * alpha + relief.unresolved).sqrt().min(1.0);
        surface.base = base.clamp(Vec3::ZERO, Vec3::ONE);
        surface.metallic = metallic;
        surface.clearcoat = coat;
        let dielectric = ((surface.ior - 1.0) / (surface.ior + 1.0)).powi(2);
        surface.f0 = Vec3::splat(dielectric).lerp(surface.base, metallic);
    }
}

#[cfg(test)]
mod tests;
