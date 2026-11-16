//! The path integrator: follows light backwards from the camera one bounce
//! at a time, gathering what each surface emits and what reaches it directly
//! from the lights, the glowing surfaces and the environment.

use glam::Vec3;

use super::bsdf::{self, dielectric_fresnel};
use super::camera::CameraRay;
use super::emitters::Emitters;
use super::environment::TraceEnvironment;
use super::light::LightSampler;
use super::material::TraceMaterial;
use super::sampling::{Frame, Rng, power_heuristic};
use super::scene::{NONE, TraceScene};
use super::shading::{Shading, lift};

/// Bounces after which a path may be ended at random.
const ROULETTE_START: u32 = 3;
/// GGX width below which a glass interface is treated as perfectly smooth.
const SMOOTH_GLASS_ALPHA: f32 = 0.0015;

pub(super) struct Tracer<'a> {
    pub(super) scene: &'a TraceScene,
    pub(super) lights: LightSampler<'a>,
    pub(super) emitters: &'a Emitters,
    pub(super) environment: Option<&'a TraceEnvironment>,
    pub(super) max_bounces: u32,
    /// Width of an image pixel on a surface: per unit of distance along the
    /// path, plus a constant.
    pub(super) footprint: (f32, f32),
}

/// The first surface a camera path meets, which guides the denoiser.
#[derive(Clone, Copy)]
pub(super) struct FirstHit {
    pub(super) albedo: Vec3,
    pub(super) normal: Vec3,
    pub(super) position: Vec3,
}

/// What one camera path gathered.
pub(super) struct PathSample {
    /// Light the camera sees directly: a glowing or unlit surface.
    pub(super) emitted: Vec3,
    /// Light scattered towards the camera.
    pub(super) scattered: Vec3,
    /// `None` when the camera ray left the scene without meeting a surface.
    pub(super) first_hit: Option<FirstHit>,
}

/// How the path last changed direction. Light sources are sampled directly
/// at every rough bounce, so what a ray finds afterwards must be weighed
/// against that sampling, or dropped where the sampling already covered it.
#[derive(Clone, Copy)]
enum Arrival {
    /// Straight from the camera.
    Camera,
    /// Off a mirror-like event; nothing else accounts for light found here.
    Specular,
    /// Off a rough bounce at `origin`, in a direction drawn with `pdf`.
    Scattered { origin: Vec3, pdf: f32 },
    /// Through glass after a rough bounce. That bounce's direct sampling
    /// reached through the glass, so light found here is already counted.
    ThroughGlass,
}

impl Arrival {
    /// Weight for light a ray finds when direct sampling would have chosen
    /// its direction with `light_pdf`.
    fn weight(self, light_pdf: impl FnOnce(Vec3) -> f32) -> f32 {
        match self {
            Self::Camera | Self::Specular => 1.0,
            Self::Scattered { origin, pdf } => power_heuristic(pdf, light_pdf(origin)),
            Self::ThroughGlass => 0.0,
        }
    }
}

impl Tracer<'_> {
    pub(super) fn trace(&self, ray: CameraRay, rng: &mut Rng) -> PathSample {
        let mut sample = PathSample {
            emitted: Vec3::ZERO,
            scattered: Vec3::ZERO,
            first_hit: None,
        };
        let mut throughput = Vec3::ONE;
        let (mut origin, mut direction, mut reach) = (ray.origin, ray.direction, ray.max_distance);
        let mut skip = NONE;
        let mut arrival = Arrival::Camera;
        // Solid glass the ray is travelling inside, which absorbs along it.
        let mut medium: Option<&TraceMaterial> = None;
        let mut travelled = 0.0_f32;
        for bounce in 0..=self.max_bounces {
            let hit = self.scene.closest(origin, direction, reach, skip, rng);
            let camera_ray = matches!(arrival, Arrival::Camera);
            if !camera_ray {
                let nearest = hit.map_or(f32::INFINITY, |hit| hit.distance);
                sample.scattered +=
                    throughput * self.light_hits(origin, direction, nearest, arrival);
            }
            let Some(hit) = hit else {
                if !camera_ray {
                    sample.scattered += throughput * self.escaped(direction, arrival);
                }
                break;
            };
            let mut shading = self.scene.shade(direction, &hit);
            travelled += hit.distance;
            // Surface detail matters where the image shows the surface
            // itself, directly or in a mirror, not in the light it bounces.
            if !matches!(arrival, Arrival::Scattered { .. }) && self.scene.detailed(hit.triangle) {
                let footprint = self.footprint.0 * travelled + self.footprint.1;
                self.scene
                    .refine(&mut shading, &hit, direction, footprint, rng);
            }
            let surface = &shading.surface;
            if let Some(medium) = medium {
                throughput *= medium.transmittance(hit.distance);
            }
            if sample.first_hit.is_none() {
                sample.first_hit = Some(FirstHit {
                    albedo: surface.base.lerp(Vec3::ONE, surface.transmission),
                    normal: shading.normal,
                    position: shading.position,
                });
            }
            let emitted = if surface.unlit {
                // An unlit surface shows its colour whatever the lighting.
                surface.base + surface.emission
            } else if shading.front || shading.material.double_sided {
                surface.emission
                    * arrival.weight(|_| {
                        self.emitters.pdf(
                            hit.triangle,
                            hit.distance,
                            shading.geometric.dot(direction),
                        )
                    })
            } else {
                Vec3::ZERO
            };
            if camera_ray {
                sample.emitted += throughput * emitted;
            } else {
                sample.scattered += throughput * emitted;
            }
            if surface.unlit || bounce == self.max_bounces {
                break;
            }
            let next = if surface.transmission > 0.0 && rng.next() < surface.transmission {
                self.cross_glass(&shading, direction, arrival, &mut medium, rng)
            } else {
                let start = shading.position + shading.geometric * lift(shading.position);
                let frame = Frame::new(shading.normal);
                let outgoing = frame.to_local(-direction);
                sample.scattered +=
                    throughput * self.direct(&shading, frame, outgoing, start, hit.triangle, rng);
                let incoming = bsdf::sample(surface, outgoing, rng);
                let (value, pdf) = bsdf::evaluate(surface, outgoing, incoming);
                let onward = frame.to_world(incoming);
                (pdf > 1.0e-9 && onward.dot(shading.geometric) > 0.0).then(|| Step {
                    origin: start,
                    direction: onward.normalize(),
                    weight: value * (incoming.z / pdf),
                    arrival: Arrival::Scattered { origin: start, pdf },
                })
            };
            let Some(next) = next else {
                break;
            };
            throughput *= next.weight;
            if !(throughput.is_finite() && throughput.max_element() > 0.0) {
                break;
            }
            if bounce >= ROULETTE_START {
                let survival = throughput.max_element().clamp(0.05, 0.95);
                if rng.next() > survival {
                    break;
                }
                throughput /= survival;
            }
            (origin, direction, reach, skip, arrival) = (
                next.origin,
                next.direction,
                f32::INFINITY,
                hit.triangle,
                next.arrival,
            );
        }
        sample
    }

    /// Area lights the ray passes through before it reaches a surface.
    fn light_hits(&self, origin: Vec3, direction: Vec3, nearest: f32, arrival: Arrival) -> Vec3 {
        if matches!(arrival, Arrival::ThroughGlass) {
            return Vec3::ZERO;
        }
        self.lights
            .hits(origin, direction, nearest)
            .map(|hit| {
                hit.radiance
                    * arrival.weight(|from| hit.pdf * self.lights.probability(hit.light, from))
            })
            .sum()
    }

    /// Light arriving along a ray that left the scene.
    fn escaped(&self, direction: Vec3, arrival: Arrival) -> Vec3 {
        if matches!(arrival, Arrival::ThroughGlass) {
            return Vec3::ZERO;
        }
        let mut radiance = Vec3::ZERO;
        if let Some(environment) = self.environment {
            radiance +=
                environment.radiance(direction) * arrival.weight(|_| environment.pdf(direction));
        }
        for (light, disc, pdf) in self.lights.escaped(direction) {
            radiance += disc * arrival.weight(|from| pdf * self.lights.probability(light, from));
        }
        radiance
    }

    /// Light reaching a surface point straight from one analytic light, one
    /// glowing triangle and one direction of the environment.
    fn direct(
        &self,
        shading: &Shading<'_>,
        frame: Frame,
        outgoing: Vec3,
        start: Vec3,
        triangle: u32,
        rng: &mut Rng,
    ) -> Vec3 {
        // `weight` is radiance over the direction's density `light_pdf`.
        let lit = |direction: Vec3,
                   distance: f32,
                   weight: Vec3,
                   light_pdf: f32,
                   shadowed: bool,
                   rng: &mut Rng| {
            let incoming = frame.to_local(direction);
            if direction.dot(shading.geometric) <= 0.0
                || incoming.z <= 0.0
                || weight.max_element() <= 0.0
            {
                return Vec3::ZERO;
            }
            let (value, bsdf_pdf) = bsdf::evaluate(&shading.surface, outgoing, incoming);
            if value.max_element() <= 0.0 || value.is_nan() {
                return Vec3::ZERO;
            }
            let (transmittance, crossed_glass) = if shadowed {
                self.scene
                    .transmittance(start, direction, distance * (1.0 - 1.0e-4), triangle, rng)
            } else {
                (Vec3::ONE, false)
            };
            // Bounce rays that cross glass do not count light sources, so
            // this sample carries their share too.
            let balance = if crossed_glass {
                1.0
            } else {
                power_heuristic(light_pdf, bsdf_pdf)
            };
            value * weight * transmittance * (incoming.z * balance)
        };
        let mut total = Vec3::ZERO;
        if let Some((light, probability)) = self.lights.choose(start, rng.next())
            && let Some(sample) = light.sample(start, rng.pair())
        {
            let (weight, pdf) = (sample.weight / probability, sample.pdf * probability);
            total += lit(
                sample.direction,
                sample.distance,
                weight,
                pdf,
                sample.casts_shadows,
                rng,
            );
        }
        if !self.emitters.is_empty()
            && let Some(sample) = self
                .emitters
                .sample(self.scene, start, rng.next(), rng.pair())
        {
            total += lit(
                sample.direction,
                sample.distance,
                sample.radiance / sample.pdf,
                sample.pdf,
                true,
                rng,
            );
        }
        if let Some(environment) = self.environment {
            let sample = environment.sample(rng.pair(), rng.pair());
            if sample.pdf > 0.0 {
                total += lit(
                    sample.direction,
                    f32::INFINITY,
                    sample.radiance / sample.pdf,
                    sample.pdf,
                    true,
                    rng,
                );
            }
        }
        if total.is_finite() { total } else { Vec3::ZERO }
    }

    /// Sends the path through a glass interface: reflected or transmitted in
    /// proportion to the Fresnel reflectance.
    fn cross_glass<'m>(
        &self,
        shading: &Shading<'m>,
        direction: Vec3,
        arrival: Arrival,
        medium: &mut Option<&'m TraceMaterial>,
        rng: &mut Rng,
    ) -> Option<Step> {
        let surface = &shading.surface;
        let frame = Frame::new(shading.normal);
        // Frosted glass scatters about a microfacet rather than the surface.
        let rough = surface.alpha > SMOOTH_GLASS_ALPHA;
        let facet = if rough {
            frame.to_world(bsdf::sample_visible_normal(
                frame.to_local(-direction),
                surface.alpha,
                rng.pair(),
            ))
        } else {
            shading.normal
        };
        let cos_incident = (-direction).dot(facet).clamp(0.0, 1.0);
        let (reflectance, eta) = if surface.thin_walled {
            // Both faces of the sheet, and the bounces between them.
            let single = dielectric_fresnel(cos_incident, 1.0 / surface.ior);
            (
                single + (1.0 - single).powi(2) * single / (1.0 - single * single).max(1.0e-6),
                1.0,
            )
        } else {
            let eta = if shading.front {
                1.0 / surface.ior
            } else {
                surface.ior
            };
            (dielectric_fresnel(cos_incident, eta), eta)
        };
        let reflects = rng.next() < reflectance;
        let (onward, tint) = if reflects {
            (direction + facet * (2.0 * cos_incident), Vec3::ONE)
        } else if surface.thin_walled {
            (direction, surface.base)
        } else {
            let cos_transmitted = (1.0 - eta * eta * (1.0 - cos_incident * cos_incident))
                .max(0.0)
                .sqrt();
            let bent = direction * eta + facet * (eta * cos_incident - cos_transmitted);
            // Entering and leaving a solid each take half of its tint.
            (
                bent,
                Vec3::new(
                    surface.base.x.sqrt(),
                    surface.base.y.sqrt(),
                    surface.base.z.sqrt(),
                ),
            )
        };
        let onward = onward.normalize_or_zero();
        let side = onward.dot(shading.geometric);
        // A microfacet can send the ray to the wrong side of the surface.
        if onward == Vec3::ZERO || (side > 0.0) != reflects {
            return None;
        }
        if !reflects && !surface.thin_walled {
            *medium = shading.front.then_some(shading.material);
        }
        let masking = if rough {
            bsdf::smith_g1(frame.to_local(onward).z.abs(), surface.alpha)
        } else {
            1.0
        };
        let away = if reflects {
            shading.geometric
        } else {
            -shading.geometric
        };
        Some(Step {
            origin: shading.position + away * lift(shading.position),
            direction: onward,
            weight: tint * masking,
            arrival: match (arrival, reflects) {
                (Arrival::ThroughGlass, _) | (Arrival::Scattered { .. }, false) => {
                    Arrival::ThroughGlass
                }
                _ => Arrival::Specular,
            },
        })
    }
}

/// Where a path goes after an event, and what the event did to its weight.
struct Step {
    origin: Vec3,
    direction: Vec3,
    weight: Vec3,
    arrival: Arrival,
}
