//! The scene's lights as the path tracer samples them.
//!
//! Units follow the rasterizer so one scene exposes the same either way: a
//! directional light delivers `illuminance_lux / 10_000` of irradiance, point
//! and spot lights deliver `intensity_candela / d^2`, and an area light
//! behaves like a source of intensity `luminous_flux_lumens / (4 pi)` spread
//! over its shape, with the same range window as the rasterizer.

use std::f32::consts::PI;

use glam::Vec3;

use crate::scene::{AreaLightShape, Light, Scene};

use super::sampling::{Frame, luminance, uniform_cone, uniform_disc};

const DIRECTIONAL_LUX_TO_IRRADIANCE: f32 = 1.0 / 10_000.0;
/// Smallest emitter cosine used for a flat light. Its radiance grows as it
/// turns edge-on, to keep its intensity the same from every side; below this
/// cosine, about three degrees from edge-on, the growth stops.
const MIN_EMITTER_COSINE: f32 = 0.05;

#[derive(Clone, Copy)]
enum Shape {
    /// `height` is one minus the cosine of the angular radius: the height of
    /// the cap the light's disc cuts from the unit sphere. Zero for a
    /// parallel beam.
    Directional {
        to_light: Vec3,
        height: f32,
    },
    Point,
    Spot {
        axis: Vec3,
        inner_cos: f32,
        outer_cos: f32,
    },
    /// A flat emitter spanned by two half-axes; a disc when `round`.
    Flat {
        half_x: Vec3,
        half_y: Vec3,
        normal: Vec3,
        area: f32,
        round: bool,
    },
    Sphere {
        radius: f32,
    },
}

#[derive(Clone, Copy)]
pub(super) struct TraceLight {
    shape: Shape,
    position: Vec3,
    /// Irradiance for a directional light, radiant intensity otherwise.
    strength: Vec3,
    range: Option<f32>,
    /// Radius of a sphere around `position` that contains the emitter.
    extent: f32,
    /// A directional light may be set not to cast shadows; it then reaches
    /// every surface that faces it, as it does in the rasterizer.
    casts_shadows: bool,
}

/// Light arriving at a point from one sampled direction.
pub(super) struct LightSample {
    pub(super) direction: Vec3,
    /// Distance to the emitter; infinite for a directional light.
    pub(super) distance: f32,
    /// Incident radiance divided by the direction's density.
    pub(super) weight: Vec3,
    /// Density per unit solid angle; infinite for a light of no size.
    pub(super) pdf: f32,
    /// Whether surfaces in the way block the light.
    pub(super) casts_shadows: bool,
}

/// A light met by a ray that was not aimed at it.
pub(super) struct LightHit {
    pub(super) light: usize,
    pub(super) radiance: Vec3,
    /// Density with which [`TraceLight::sample`] would have chosen the ray.
    pub(super) pdf: f32,
}

fn window(distance: f32, range: Option<f32>) -> f32 {
    let Some(range) = range else {
        return 1.0;
    };
    if !(range.is_finite() && range > f32::EPSILON) {
        return 0.0;
    }
    let falloff = (1.0 - (distance / range).powi(4)).clamp(0.0, 1.0);
    falloff * falloff
}

fn cone_attenuation(cos_angle: f32, inner_cos: f32, outer_cos: f32) -> f32 {
    if cos_angle >= inner_cos {
        1.0
    } else if cos_angle <= outer_cos {
        0.0
    } else {
        ((cos_angle - outer_cos) / (inner_cos - outer_cos)).clamp(0.0, 1.0)
    }
}

/// Density of a uniform direction inside a cone whose cap has `height`.
fn cone_pdf(height: f32) -> f32 {
    1.0 / (2.0 * PI * height)
}

/// Cap height of the cone a sphere of `radius` subtends from `distance`,
/// computed without the cancellation `1 - cos` suffers for a small sphere.
fn sphere_cap_height(radius: f32, distance: f32) -> f32 {
    let sin_squared = (radius / distance).powi(2).min(1.0);
    sin_squared / (1.0 + (1.0 - sin_squared).sqrt())
}

/// Collects the scene's visible lights, positioned relative to `origin`.
pub(super) fn collect(
    scene: &Scene,
    origin: Vec3,
    directional_angular_radius: f32,
) -> Vec<TraceLight> {
    let height = 2.0
        * (0.5 * directional_angular_radius.clamp(0.0, 0.5))
            .sin()
            .powi(2);
    let mut lights = Vec::new();
    for (node, _key, light, _transform) in scene.light_nodes() {
        let Some(world) = scene.resolved_world(node) else {
            continue;
        };
        let direction = |axis: Vec3| world.transform_vector(axis).normalize_or_zero();
        let forward = direction(Vec3::NEG_Z);
        let position = world.translation() - origin;
        let color = |color: crate::material::Color, scale: f32| {
            Vec3::new(color.r, color.g, color.b) * scale
        };
        let finite = |value: f32| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        let traced = match light {
            Light::Directional(light) => TraceLight {
                // Without shadows there is no penumbra to soften, and a
                // parallel beam keeps the light out of chance encounters.
                shape: Shape::Directional {
                    to_light: -forward,
                    height: if light.casts_shadows() { height } else { 0.0 },
                },
                position,
                strength: color(
                    light.color(),
                    finite(light.illuminance_lux()) * DIRECTIONAL_LUX_TO_IRRADIANCE,
                ),
                range: None,
                extent: 0.0,
                casts_shadows: light.casts_shadows(),
            },
            Light::Point(light) => TraceLight {
                shape: Shape::Point,
                position,
                strength: color(light.color(), finite(light.intensity_candela())),
                range: light.range(),
                extent: 0.0,
                casts_shadows: true,
            },
            Light::Spot(light) => TraceLight {
                shape: Shape::Spot {
                    axis: forward,
                    inner_cos: light.inner_cone_angle().radians().cos(),
                    outer_cos: light.outer_cone_angle().radians().cos(),
                },
                position,
                strength: color(light.color(), finite(light.intensity_candela())),
                range: light.range(),
                extent: 0.0,
                casts_shadows: true,
            },
            Light::Area(light) => {
                let strength = color(
                    light.color(),
                    finite(light.luminous_flux_lumens()) / (4.0 * PI),
                );
                let (right, up) = (direction(Vec3::X), direction(Vec3::Y));
                let flat = |half_x: Vec3, half_y: Vec3, area: f32, round: bool| Shape::Flat {
                    half_x,
                    half_y,
                    normal: half_x.cross(half_y).normalize_or_zero(),
                    area,
                    round,
                };
                let (shape, extent) = match light.shape() {
                    AreaLightShape::Rect { width, height } => {
                        let (w, h) = (width.max(0.001), height.max(0.001));
                        (
                            flat(right * (w * 0.5), up * (h * 0.5), w * h, false),
                            0.5 * (w * w + h * h).sqrt(),
                        )
                    }
                    AreaLightShape::Disc { radius } => {
                        let r = radius.max(0.001);
                        (flat(right * r, up * r, PI * r * r, true), r)
                    }
                    AreaLightShape::Sphere { radius } => (
                        Shape::Sphere {
                            radius: radius.max(0.001),
                        },
                        radius.max(0.001),
                    ),
                };
                TraceLight {
                    shape,
                    position,
                    strength,
                    range: light.range(),
                    extent,
                    casts_shadows: true,
                }
            }
        };
        if traced.strength.max_element() > 0.0 && forward != Vec3::ZERO {
            lights.push(traced);
        }
    }
    lights
}

impl TraceLight {
    /// Unshadowed brightness estimate at `point`, used to choose which light
    /// to sample. It is positive wherever the light can contribute.
    pub(super) fn importance(&self, point: Vec3) -> f32 {
        let strength = luminance(self.strength);
        let to_light = self.position - point;
        let distance = to_light.length();
        match self.shape {
            Shape::Directional { .. } => strength,
            Shape::Point => {
                strength * window(distance, self.range) / (distance * distance).max(1.0e-8)
            }
            Shape::Spot {
                axis,
                inner_cos,
                outer_cos,
            } => {
                let cos_angle = -to_light.dot(axis) / distance.max(1.0e-8);
                strength
                    * window(distance, self.range)
                    * cone_attenuation(cos_angle, inner_cos, outer_cos)
                    / (distance * distance).max(1.0e-8)
            }
            Shape::Flat { .. } | Shape::Sphere { .. } => {
                // Measured to the nearest the emitter can be, so a receiver
                // close to one edge still gives the light a fair share.
                let nearest = (distance - self.extent).max(0.0);
                let effective = nearest.max(0.5 * self.extent);
                strength * window(nearest, self.range) / (effective * effective).max(1.0e-8)
            }
        }
    }

    /// Whether the light has a size, so a ray can meet it by chance.
    pub(super) fn is_delta(&self) -> bool {
        match self.shape {
            Shape::Directional { height, .. } => height < 1.0e-9,
            Shape::Point | Shape::Spot { .. } => true,
            Shape::Flat { .. } | Shape::Sphere { .. } => false,
        }
    }

    /// Draws a direction from `point` towards the light.
    pub(super) fn sample(&self, point: Vec3, u: (f32, f32)) -> Option<LightSample> {
        let (direction, distance, weight, pdf) = self.sample_direction(point, u)?;
        Some(LightSample {
            direction,
            distance,
            weight,
            pdf,
            casts_shadows: self.casts_shadows,
        })
    }

    /// Direction, distance, radiance over density, and density.
    fn sample_direction(&self, point: Vec3, u: (f32, f32)) -> Option<(Vec3, f32, Vec3, f32)> {
        let to_center = self.position - point;
        match self.shape {
            Shape::Directional { to_light, height } => Some(if self.is_delta() {
                (to_light, f32::INFINITY, self.strength, f32::INFINITY)
            } else {
                // The beam's irradiance spread over the light's disc.
                let direction = Frame::new(to_light).to_world(uniform_cone(u, height));
                (direction, f32::INFINITY, self.strength, cone_pdf(height))
            }),
            Shape::Point | Shape::Spot { .. } => {
                let distance = to_center.length();
                if distance <= 0.0 {
                    return None;
                }
                let direction = to_center / distance;
                let cone = match self.shape {
                    Shape::Spot {
                        axis,
                        inner_cos,
                        outer_cos,
                    } => cone_attenuation(-direction.dot(axis), inner_cos, outer_cos),
                    _ => 1.0,
                };
                let weight = self.strength
                    * (cone * window(distance, self.range) / (distance * distance).max(1.0e-4));
                Some((direction, distance, weight, f32::INFINITY))
            }
            Shape::Flat {
                half_x,
                half_y,
                normal,
                area,
                round,
            } => {
                let (x, y) = if round {
                    uniform_disc(u)
                } else {
                    (2.0 * u.0 - 1.0, 2.0 * u.1 - 1.0)
                };
                let to_emitter = to_center + half_x * x + half_y * y;
                let distance = to_emitter.length();
                if distance <= 0.0 {
                    return None;
                }
                let direction = to_emitter / distance;
                let emitter_cos = normal.dot(direction).abs().max(MIN_EMITTER_COSINE);
                // Radiance times the area-to-solid-angle factor over the
                // area density reduces to intensity over distance squared:
                // the emitter cosine cancels, as it does in the rasterizer.
                let weight = self.strength
                    * (window(distance, self.range) / (distance * distance).max(1.0e-4));
                Some((
                    direction,
                    distance,
                    weight,
                    distance * distance / (area * emitter_cos),
                ))
            }
            Shape::Sphere { radius } => {
                let center_distance = to_center.length();
                if center_distance <= radius {
                    return None;
                }
                let height = sphere_cap_height(radius, center_distance);
                let direction =
                    Frame::new(to_center / center_distance).to_world(uniform_cone(u, height));
                let distance = sphere_distance(to_center, direction, radius)?;
                let pdf = cone_pdf(height);
                Some((
                    direction,
                    distance,
                    self.sphere_radiance(radius, distance) / pdf,
                    pdf,
                ))
            }
        }
    }

    /// A sphere light glows evenly with the radiance that gives it the
    /// intended intensity when seen from afar.
    fn sphere_radiance(&self, radius: f32, distance: f32) -> Vec3 {
        self.strength * (window(distance, self.range) / (PI * radius * radius))
    }

    /// Where a ray from `origin` meets the emitter, with the radiance it sees.
    fn intersect(&self, origin: Vec3, direction: Vec3) -> Option<(f32, Vec3, f32)> {
        let to_center = self.position - origin;
        match self.shape {
            Shape::Directional { .. } | Shape::Point | Shape::Spot { .. } => None,
            Shape::Flat {
                half_x,
                half_y,
                normal,
                area,
                round,
            } => {
                let facing = normal.dot(direction);
                if facing.abs() < 1.0e-6 {
                    return None;
                }
                let distance = normal.dot(to_center) / facing;
                if distance <= 0.0 {
                    return None;
                }
                let local = direction * distance - to_center;
                let (x, y) = (
                    local.dot(half_x) / half_x.length_squared(),
                    local.dot(half_y) / half_y.length_squared(),
                );
                let inside = if round {
                    x * x + y * y <= 1.0
                } else {
                    x.abs() <= 1.0 && y.abs() <= 1.0
                };
                let emitter_cos = facing.abs().max(MIN_EMITTER_COSINE);
                inside.then(|| {
                    (
                        distance,
                        self.strength * (window(distance, self.range) / (area * emitter_cos)),
                        distance * distance / (area * emitter_cos),
                    )
                })
            }
            Shape::Sphere { radius } => {
                let center_distance = to_center.length();
                if center_distance <= radius {
                    return None;
                }
                let distance = sphere_distance(to_center, direction, radius)?;
                let pdf = cone_pdf(sphere_cap_height(radius, center_distance));
                Some((distance, self.sphere_radiance(radius, distance), pdf))
            }
        }
    }

    /// Radiance of a directional light's disc along `direction`, with the
    /// density its sampler gives that direction.
    fn directional_radiance(&self, direction: Vec3) -> Option<(Vec3, f32)> {
        let Shape::Directional { to_light, height } = self.shape else {
            return None;
        };
        (!self.is_delta() && 1.0 - direction.dot(to_light) <= height).then(|| {
            let pdf = cone_pdf(height);
            (self.strength * pdf, pdf)
        })
    }
}

/// Distance along `direction` to the near side of a sphere at `to_center`.
fn sphere_distance(to_center: Vec3, direction: Vec3, radius: f32) -> Option<f32> {
    let along = to_center.dot(direction);
    let discriminant = radius * radius - (to_center.length_squared() - along * along);
    let distance = along - discriminant.max(0.0).sqrt();
    (along > 0.0 && discriminant >= -1.0e-6 * radius * radius && distance > 0.0).then_some(distance)
}

/// Chooses one light per shading point in proportion to its importance there.
pub(super) struct LightSampler<'a> {
    lights: &'a [TraceLight],
}

impl<'a> LightSampler<'a> {
    pub(super) fn new(lights: &'a [TraceLight]) -> Self {
        Self { lights }
    }

    fn total_importance(&self, point: Vec3) -> f32 {
        self.lights
            .iter()
            .map(|light| light.importance(point))
            .sum()
    }

    /// Probability that [`Self::choose`] picks `light` at `point`.
    pub(super) fn probability(&self, light: usize, point: Vec3) -> f32 {
        let total = self.total_importance(point);
        if total > 0.0 {
            self.lights[light].importance(point) / total
        } else {
            0.0
        }
    }

    /// Picks a light at `point`, returning it with the probability of the pick.
    pub(super) fn choose(&self, point: Vec3, u: f32) -> Option<(&'a TraceLight, f32)> {
        let total = self.total_importance(point);
        if !(total > 0.0 && total.is_finite()) {
            return None;
        }
        let mut remaining = u * total;
        let mut chosen = None;
        for light in self.lights {
            let importance = light.importance(point);
            if importance <= 0.0 {
                continue;
            }
            chosen = Some((light, importance / total));
            remaining -= importance;
            if remaining < 0.0 {
                break;
            }
        }
        chosen
    }

    /// Area lights a ray passes through before `max_distance`. Lights emit
    /// without blocking, so a ray can meet several.
    pub(super) fn hits(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> impl Iterator<Item = LightHit> + '_ {
        self.lights
            .iter()
            .enumerate()
            .filter_map(move |(index, light)| {
                let (distance, radiance, pdf) = light.intersect(origin, direction)?;
                (distance < max_distance).then_some(LightHit {
                    light: index,
                    radiance,
                    pdf,
                })
            })
    }

    /// Directional lights with a disc, seen along a ray that left the scene.
    pub(super) fn escaped(&self, direction: Vec3) -> impl Iterator<Item = (usize, Vec3, f32)> + '_ {
        self.lights
            .iter()
            .enumerate()
            .filter_map(move |(index, light)| {
                light
                    .directional_radiance(direction)
                    .map(|(radiance, pdf)| (index, radiance, pdf))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::path_trace::sampling::Rng;

    fn flat(round: bool) -> TraceLight {
        TraceLight {
            shape: Shape::Flat {
                half_x: Vec3::X * 0.6,
                half_y: Vec3::Y * 0.3,
                normal: Vec3::Z,
                area: if round { PI * 0.6 * 0.3 } else { 1.2 * 0.6 },
                round,
            },
            position: Vec3::new(0.2, 1.5, -0.4),
            strength: Vec3::new(30.0, 28.0, 25.0),
            range: Some(9.0),
            extent: 0.7,
            casts_shadows: true,
        }
    }

    fn sphere() -> TraceLight {
        TraceLight {
            shape: Shape::Sphere { radius: 0.25 },
            position: Vec3::new(-0.5, 1.2, 0.8),
            strength: Vec3::splat(12.0),
            range: None,
            extent: 0.25,
            casts_shadows: true,
        }
    }

    /// Irradiance on a horizontal receiver at the origin, estimated by
    /// sampling the light and by cosine-weighted rays that happen to hit it.
    fn irradiance_both_ways(light: &TraceLight) -> (Vec3, Vec3) {
        let mut rng = Rng::new(23, 6);
        let sampler = LightSampler::new(std::slice::from_ref(light));
        let samples = 400_000;
        let (mut direct, mut chance) = (Vec3::ZERO, Vec3::ZERO);
        for _ in 0..samples {
            if let Some(sample) = light.sample(Vec3::ZERO, rng.pair()) {
                direct += sample.weight * sample.direction.y.max(0.0);
            }
            let local = super::super::sampling::cosine_hemisphere(rng.pair());
            let direction = Vec3::new(local.x, local.z, local.y);
            for hit in sampler.hits(Vec3::ZERO, direction, f32::INFINITY) {
                // Cosine-weighted density is cos / pi, so the cosine cancels.
                chance += hit.radiance * PI;
            }
        }
        (direct / samples as f32, chance / samples as f32)
    }

    #[test]
    fn sampling_a_light_and_hitting_it_by_chance_measure_the_same_irradiance() {
        for (name, light) in [("rect", flat(false)), ("sphere", sphere())] {
            let (direct, chance) = irradiance_both_ways(&light);
            assert!(
                direct.max_element() > 0.5,
                "{name} delivers light: {direct:?}"
            );
            assert!(
                (direct - chance).abs().max_element() < 0.05 * direct.max_element(),
                "{name}: sampled {direct:?}, by chance {chance:?}"
            );
        }
    }

    #[test]
    fn sampled_directions_report_the_density_an_intersection_reports() {
        let mut rng = Rng::new(31, 8);
        for light in [flat(false), flat(true), sphere()] {
            for _ in 0..500 {
                let sample = light
                    .sample(Vec3::ZERO, rng.pair())
                    .expect("the receiver is outside the light");
                let (distance, _, pdf) = light
                    .intersect(Vec3::ZERO, sample.direction)
                    .expect("the sample lies on the light");
                assert!((distance - sample.distance).abs() < 1.0e-3 * sample.distance);
                assert!(
                    (pdf - sample.pdf).abs() < 1.0e-2 * sample.pdf,
                    "sampled {} met {pdf}",
                    sample.pdf
                );
            }
        }
    }

    #[test]
    fn a_distant_area_light_matches_a_point_light_of_the_same_intensity() {
        // Far away, the shape stops mattering: irradiance is I / d^2.
        let mut light = sphere();
        light.position = Vec3::new(0.0, 40.0, 0.0);
        let mut rng = Rng::new(2, 2);
        let samples = 20_000;
        let irradiance: f32 = (0..samples)
            .filter_map(|_| light.sample(Vec3::ZERO, rng.pair()))
            .map(|sample| sample.weight.x * sample.direction.y)
            .sum::<f32>()
            / samples as f32;
        let expected = 12.0 / (40.0 * 40.0);
        assert!(
            (irradiance - expected).abs() < 0.01 * expected,
            "{irradiance} against {expected}"
        );
    }

    #[test]
    fn the_range_window_and_the_spot_cone_cut_light_off() {
        let mut point = TraceLight {
            shape: Shape::Point,
            position: Vec3::new(0.0, 2.0, 0.0),
            strength: Vec3::splat(8.0),
            range: Some(1.5),
            extent: 0.0,
            casts_shadows: true,
        };
        assert_eq!(
            point.sample(Vec3::ZERO, (0.5, 0.5)).expect("sample").weight,
            Vec3::ZERO
        );
        assert_eq!(point.importance(Vec3::ZERO), 0.0);
        point.range = None;
        assert!(
            (point
                .sample(Vec3::ZERO, (0.5, 0.5))
                .expect("sample")
                .weight
                .x
                - 2.0)
                .abs()
                < 1.0e-5
        );
        let spot = TraceLight {
            shape: Shape::Spot {
                axis: Vec3::NEG_Y,
                inner_cos: 0.95,
                outer_cos: 0.9,
            },
            ..point
        };
        assert!(
            spot.sample(Vec3::ZERO, (0.5, 0.5))
                .expect("sample")
                .weight
                .x
                > 1.9
        );
        assert_eq!(
            spot.sample(Vec3::new(3.0, 0.0, 0.0), (0.5, 0.5))
                .expect("sample")
                .weight,
            Vec3::ZERO
        );
    }

    #[test]
    fn light_choice_follows_importance_and_reports_its_probability() {
        let lights = [flat(false), sphere()];
        let sampler = LightSampler::new(&lights);
        let point = Vec3::new(0.1, 0.0, 0.2);
        let (p0, p1) = (sampler.probability(0, point), sampler.probability(1, point));
        assert!((p0 + p1 - 1.0).abs() < 1.0e-5 && p0 > 0.0 && p1 > 0.0);
        let mut rng = Rng::new(4, 4);
        let picks = 40_000;
        let first = (0..picks)
            .filter(|_| {
                let (light, probability) = sampler
                    .choose(point, rng.next())
                    .expect("a light is chosen");
                let is_first = matches!(light.shape, Shape::Flat { .. });
                assert!((probability - if is_first { p0 } else { p1 }).abs() < 1.0e-5);
                is_first
            })
            .count();
        assert!((first as f32 / picks as f32 - p0).abs() < 0.01);
    }
}
