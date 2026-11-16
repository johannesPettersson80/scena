use crate::BakedAmbientOcclusionConfig;
use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::geometry::{Aabb, TriangleBvh};
use crate::scene::{Scene, Vec3};

use super::DeformationInputs;
use super::lighting::PreparedLights;
use crate::render::PrepareWorkCounter;

mod cache;
mod collection;
mod math;
mod projection;
pub(in crate::render) use cache::ShadowVisibilityCache;
use cache::shadow_triangle_signature;
pub(super) use collection::cpu_shadow_visibility_required;
use math::{add_vec3, cross_vec3, dot_vec3, scale_vec3, subtract_vec3};
pub(super) use projection::{collect_shadow_projection_groups, focused_light_view_projection};

const SHADOW_OCCLUDED_VISIBILITY: f32 = 0.0;
const SHADOW_RAY_RELATIVE_BIAS: f32 = 0.000_01;
const SHADOW_RAY_MIN_BIAS: f32 = 0.000_01;
const SHADOW_RAY_MAX_BIAS: f32 = 0.000_5;
const SHADOW_RAY_MIN_HIT_DISTANCE: f32 = 0.000_001;

#[derive(Clone, Copy)]
pub(super) struct ShadowOccluder {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

pub(super) struct ShadowOccluderSet {
    triangles: Vec<ShadowOccluder>,
    bvh: TriangleBvh,
    state_signature: u64,
}

impl Default for ShadowOccluderSet {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl ShadowOccluderSet {
    pub(super) fn new(triangles: Vec<ShadowOccluder>) -> Self {
        let positions = triangles
            .iter()
            .map(|triangle| [triangle.a, triangle.b, triangle.c])
            .collect::<Vec<_>>();
        Self {
            triangles,
            bvh: TriangleBvh::from_triangles(&positions),
            state_signature: shadow_triangle_signature(&positions),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}

/// The directional-shadow contract deliberately keeps this entry point with
/// the shadow owner while collection mechanics live in `collection`.
pub(super) fn collect_shadow_occluders<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<ShadowOccluderSet, PrepareError> {
    collection::collect_occluders(scene, assets, origin_shift)
}

pub(super) fn directional_shadow_factor_profiled(
    position: Vec3,
    lights: &PreparedLights,
    occluders: &ShadowOccluderSet,
    work: Option<&PrepareWorkCounter>,
) -> f32 {
    let Some(ray_direction) = lights.primary_shadow_ray_direction() else {
        return 1.0;
    };
    if let Some(work) = work {
        work.record_shadow_ray();
    }
    let origin = add_vec3(position, scale_vec3(ray_direction, SHADOW_RAY_MAX_BIAS));
    let traversal =
        occluders
            .bvh
            .any_ray_candidate(origin, ray_direction, f32::INFINITY, |triangle_index| {
                if let Some(work) = work {
                    work.record_ray_triangle_intersection_test();
                }
                ray_intersects_triangle(origin, ray_direction, occluders.triangles[triangle_index])
            });
    if let Some(work) = work {
        work.record_bvh_node_bounds_tests(traversal.node_bounds_tests);
    }
    if traversal.hit {
        return SHADOW_OCCLUDED_VISIBILITY;
    }
    1.0
}

#[cfg(test)]
pub(super) fn area_shadow_factor(
    position: Vec3,
    lights: &PreparedLights,
    occluders: &[ShadowOccluder],
) -> f32 {
    area_shadow_factor_profiled(
        position,
        lights,
        &ShadowOccluderSet::new(occluders.to_vec()),
        None,
    )
}

pub(super) fn area_shadow_factor_profiled(
    position: Vec3,
    lights: &PreparedLights,
    occluders: &ShadowOccluderSet,
    work: Option<&PrepareWorkCounter>,
) -> f32 {
    if occluders.is_empty() || !lights.has_area_lights() {
        return 1.0;
    }
    let mut incident_weight_sum = 0.0f32;
    let mut visibility_sum = 0.0f32;
    for (sample_position, incident_weight) in lights.area_shadow_samples(position) {
        if !incident_weight.is_finite() || incident_weight <= 0.0 {
            continue;
        }
        let to_light = subtract_vec3(sample_position, position);
        let distance = dot_vec3(to_light, to_light).sqrt();
        if !distance.is_finite() {
            continue;
        }
        let direction = scale_vec3(to_light, distance.recip());
        let bias =
            (distance * SHADOW_RAY_RELATIVE_BIAS).clamp(SHADOW_RAY_MIN_BIAS, SHADOW_RAY_MAX_BIAS);
        if distance <= bias * 2.0 {
            continue;
        }
        let origin = add_vec3(position, scale_vec3(direction, bias));
        let max_distance = distance - bias * 2.0;
        if let Some(work) = work {
            work.record_area_light_sample();
            work.record_shadow_ray();
        }
        let traversal =
            occluders
                .bvh
                .any_ray_candidate(origin, direction, max_distance, |triangle_index| {
                    if let Some(work) = work {
                        work.record_ray_triangle_intersection_test();
                    }
                    ray_intersects_triangle_before(
                        origin,
                        direction,
                        max_distance,
                        occluders.triangles[triangle_index],
                    )
                });
        if let Some(work) = work {
            work.record_bvh_node_bounds_tests(traversal.node_bounds_tests);
        }
        visibility_sum += incident_weight
            * if traversal.hit {
                SHADOW_OCCLUDED_VISIBILITY
            } else {
                1.0
            };
        incident_weight_sum += incident_weight;
    }
    if incident_weight_sum <= f32::EPSILON {
        1.0
    } else {
        visibility_sum / incident_weight_sum
    }
}

pub(super) fn ambient_visibility_factor_profiled(
    position: Vec3,
    normal: Vec3,
    occluders: &ShadowOccluderSet,
    config: BakedAmbientOcclusionConfig,
    work: Option<&PrepareWorkCounter>,
) -> f32 {
    if occluders.is_empty() || config.intensity() <= 0.0 {
        return 1.0;
    }
    let Some(bounds) = occluders.bvh.bounds() else {
        return 1.0;
    };
    let max_distance = bounds.bounding_sphere_radius() * config.radius_fraction();
    if !max_distance.is_finite() || max_distance <= SHADOW_RAY_MIN_HIT_DISTANCE {
        return 1.0;
    }

    let normal = normalize_or(normal, Vec3::new(0.0, 1.0, 0.0));
    let reference = if normal.z.abs() < 0.999 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let tangent = normalize_or(cross_vec3(reference, normal), Vec3::new(1.0, 0.0, 0.0));
    let bitangent = cross_vec3(normal, tangent);
    let bias = (max_distance * 0.001).clamp(SHADOW_RAY_MIN_BIAS, SHADOW_RAY_MAX_BIAS);
    let origin = add_vec3(position, scale_vec3(normal, bias));
    let sample_count = u32::from(config.sample_count());
    let mut occluded = 0_u32;

    for sample in 0..sample_count {
        let unit = (sample as f32 + 0.5) / sample_count as f32;
        let radial = unit.sqrt();
        let normal_component = (1.0 - unit).sqrt();
        let angle = (sample as f32 + 0.5) * 2.399_963_1;
        let (sin_angle, cos_angle) = angle.sin_cos();
        let direction = normalize_or(
            add_vec3(
                add_vec3(
                    scale_vec3(tangent, cos_angle * radial),
                    scale_vec3(bitangent, sin_angle * radial),
                ),
                scale_vec3(normal, normal_component),
            ),
            normal,
        );
        if let Some(work) = work {
            work.record_shadow_ray();
        }
        let traversal =
            occluders
                .bvh
                .any_ray_candidate(origin, direction, max_distance, |triangle_index| {
                    if let Some(work) = work {
                        work.record_ray_triangle_intersection_test();
                    }
                    ray_intersects_triangle_before(
                        origin,
                        direction,
                        max_distance,
                        occluders.triangles[triangle_index],
                    )
                });
        if let Some(work) = work {
            work.record_bvh_node_bounds_tests(traversal.node_bounds_tests);
        }
        occluded += u32::from(traversal.hit);
    }

    let occluded_fraction = occluded as f32 / sample_count as f32;
    (1.0 - occluded_fraction * config.intensity()).clamp(0.0, 1.0)
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length = dot_vec3(value, value).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return fallback;
    }
    Vec3::new(value.x / length, value.y / length, value.z / length)
}

fn ray_intersects_triangle(origin: Vec3, direction: Vec3, triangle: ShadowOccluder) -> bool {
    ray_intersects_triangle_before(origin, direction, f32::INFINITY, triangle)
}

fn ray_intersects_triangle_before(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    triangle: ShadowOccluder,
) -> bool {
    let edge1 = subtract_vec3(triangle.b, triangle.a);
    let edge2 = subtract_vec3(triangle.c, triangle.a);
    let h = cross_vec3(direction, edge2);
    let determinant = dot_vec3(edge1, h);
    if determinant.abs() <= 0.000_001 {
        return false;
    }
    let inverse_determinant = determinant.recip();
    let s = subtract_vec3(origin, triangle.a);
    let u = inverse_determinant * dot_vec3(s, h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross_vec3(s, edge1);
    let v = inverse_determinant * dot_vec3(direction, q);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = inverse_determinant * dot_vec3(edge2, q);
    t > SHADOW_RAY_MIN_HIT_DISTANCE && t < max_distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BakedAmbientOcclusionConfig;
    use crate::material::Color;
    use crate::scene::{AreaLight, AreaLightShape, Scene, Transform, Vec3};

    #[test]
    fn pf06_shadow_bvh_reduces_exact_triangle_tests_without_changing_visibility() {
        let mut triangles = Vec::new();
        for index in 0..4096 {
            let x = 10.0 + index as f32 * 0.01;
            triangles.push(ShadowOccluder {
                a: Vec3::new(x, -0.1, 1.0),
                b: Vec3::new(x + 0.005, -0.1, 1.0),
                c: Vec3::new(x, 0.1, 1.0),
            });
        }
        triangles.push(ShadowOccluder {
            a: Vec3::new(-1.0, -1.0, 1.0),
            b: Vec3::new(1.0, -1.0, 1.0),
            c: Vec3::new(0.0, 1.0, 1.0),
        });
        let occluders = ShadowOccluderSet::new(triangles);
        let mut scene = Scene::new();
        scene
            .directional_light(crate::DirectionalLight::default().with_shadows(true))
            .add()
            .expect("light");
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
        let work = PrepareWorkCounter::default();

        let visibility =
            directional_shadow_factor_profiled(Vec3::ZERO, &lights, &occluders, Some(&work));
        let metrics = work.snapshot();

        assert_eq!(visibility, SHADOW_OCCLUDED_VISIBILITY);
        assert!(metrics.bvh_node_bounds_tests > 0);
        assert!(metrics.ray_triangle_intersection_tests < 128, "{metrics:?}");
    }

    #[test]
    fn area_shadow_visibility_is_partial_when_occluder_covers_part_of_emitter() {
        let mut scene = Scene::new();
        scene
            .area_light(
                AreaLight::default()
                    .with_color(Color::WHITE)
                    .with_luminous_flux_lumens(800.0)
                    .with_shape(AreaLightShape::rect(2.0, 2.0)),
            )
            .transform(Transform {
                translation: Vec3::new(0.0, 0.0, 2.0),
                ..Transform::default()
            })
            .add()
            .expect("area light inserts");
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
        let occluders = [
            ShadowOccluder {
                a: Vec3::new(-10.0, -10.0, 1.0),
                b: Vec3::new(0.0, -10.0, 1.0),
                c: Vec3::new(-10.0, 10.0, 1.0),
            },
            ShadowOccluder {
                a: Vec3::new(0.0, -10.0, 1.0),
                b: Vec3::new(0.0, 10.0, 1.0),
                c: Vec3::new(-10.0, 10.0, 1.0),
            },
        ];

        let visibility = area_shadow_factor(Vec3::ZERO, &lights, &occluders);

        assert!(
            visibility > 0.4 && visibility < 0.8,
            "half-occluded area light should produce a soft partial shadow, got {visibility}"
        );
    }

    #[test]
    fn area_shadow_visibility_uses_dense_emitter_samples() {
        let mut scene = Scene::new();
        scene
            .area_light(
                AreaLight::default()
                    .with_color(Color::WHITE)
                    .with_luminous_flux_lumens(800.0)
                    .with_shape(AreaLightShape::rect(2.0, 2.0)),
            )
            .transform(Transform {
                translation: Vec3::new(0.0, 0.0, 2.0),
                ..Transform::default()
            })
            .add()
            .expect("area light inserts");
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);

        assert!(
            lights.area_shadow_sample_positions().count() >= 16,
            "finite area-light shadows need a dense emitter sample set so penumbrae are not limited to four hard visibility bands"
        );
    }

    #[test]
    fn area_shadow_visibility_preserves_millimetre_scale_contact() {
        let mut scene = Scene::new();
        scene
            .area_light(
                AreaLight::default()
                    .with_color(Color::WHITE)
                    .with_luminous_flux_lumens(800.0)
                    .with_shape(AreaLightShape::rect(0.02, 0.02)),
            )
            .transform(Transform {
                translation: Vec3::new(0.0, 0.0, 1.0),
                ..Transform::default()
            })
            .add()
            .expect("area light inserts");
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
        let occluders = [
            ShadowOccluder {
                a: Vec3::new(-1.0, -1.0, 0.005),
                b: Vec3::new(1.0, -1.0, 0.005),
                c: Vec3::new(-1.0, 1.0, 0.005),
            },
            ShadowOccluder {
                a: Vec3::new(1.0, -1.0, 0.005),
                b: Vec3::new(1.0, 1.0, 0.005),
                c: Vec3::new(-1.0, 1.0, 0.005),
            },
        ];

        let visibility = area_shadow_factor(Vec3::ZERO, &lights, &occluders);

        assert!(
            (visibility - SHADOW_OCCLUDED_VISIBILITY).abs() < 0.000_001,
            "a receiver-to-occluder gap below one centimetre is contact, not a reason to skip the occluder"
        );
    }

    #[test]
    fn fully_occluded_area_light_has_zero_direct_visibility() {
        let mut scene = Scene::new();
        scene
            .area_light(
                AreaLight::default()
                    .with_color(Color::WHITE)
                    .with_luminous_flux_lumens(800.0)
                    .with_shape(AreaLightShape::rect(0.02, 0.02)),
            )
            .transform(Transform {
                translation: Vec3::new(0.0, 0.0, 1.0),
                ..Transform::default()
            })
            .add()
            .expect("area light inserts");
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
        let occluders = [
            ShadowOccluder {
                a: Vec3::new(-1.0, -1.0, 0.5),
                b: Vec3::new(1.0, -1.0, 0.5),
                c: Vec3::new(-1.0, 1.0, 0.5),
            },
            ShadowOccluder {
                a: Vec3::new(1.0, -1.0, 0.5),
                b: Vec3::new(1.0, 1.0, 0.5),
                c: Vec3::new(-1.0, 1.0, 0.5),
            },
        ];

        let visibility = area_shadow_factor(Vec3::ZERO, &lights, &occluders);

        assert!(
            visibility <= f32::EPSILON,
            "opaque geometry blocks direct light; ambient/environment lighting owns bounce, got {visibility}"
        );
    }

    #[test]
    fn area_shadow_visibility_is_weighted_by_incident_light_energy() {
        let mut scene = Scene::new();
        for (position, flux) in [
            (Vec3::new(-1.0, 0.0, 2.0), 1_000.0),
            (Vec3::new(1.0, 0.0, 2.0), 10.0),
        ] {
            scene
                .area_light(
                    AreaLight::default()
                        .with_color(Color::WHITE)
                        .with_luminous_flux_lumens(flux)
                        .with_shape(AreaLightShape::rect(0.02, 0.02)),
                )
                .transform(Transform {
                    translation: position,
                    ..Transform::default()
                })
                .add()
                .expect("area light inserts");
        }
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
        let occluders = [
            ShadowOccluder {
                a: Vec3::new(-10.0, -10.0, 1.0),
                b: Vec3::new(0.0, -10.0, 1.0),
                c: Vec3::new(-10.0, 10.0, 1.0),
            },
            ShadowOccluder {
                a: Vec3::new(0.0, -10.0, 1.0),
                b: Vec3::new(0.0, 10.0, 1.0),
                c: Vec3::new(-10.0, 10.0, 1.0),
            },
        ];

        let visibility = area_shadow_factor(Vec3::ZERO, &lights, &occluders);

        assert!(
            visibility < 0.05,
            "blocking the key while leaving a 100x dimmer fill visible must not retain half of all direct lighting, got {visibility}"
        );
    }

    #[test]
    fn baked_ambient_visibility_resolves_near_contact_without_dimming_distant_geometry() {
        let config = BakedAmbientOcclusionConfig::new(16, 0.20, 0.80);
        let nearby = horizontal_quad_occluders(0.01);
        let distant = horizontal_quad_occluders(0.20);

        let nearby_visibility = ambient_visibility_factor_profiled(
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            &nearby,
            config,
            None,
        );
        let distant_visibility = ambient_visibility_factor_profiled(
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            &distant,
            config,
            None,
        );

        assert!(
            nearby_visibility < 0.60,
            "nearby geometry must occlude HDR/environment light at contact; visibility={nearby_visibility}"
        );
        assert!(
            distant_visibility > 0.95,
            "finite-radius ambient visibility must not turn into global scene darkening; visibility={distant_visibility}"
        );
    }

    #[test]
    fn baked_ambient_visibility_is_normal_aware_at_shared_hard_edges() {
        let config = BakedAmbientOcclusionConfig::new(16, 0.20, 0.80);
        let wall = vertical_quad_occluders(0.01);

        let toward_wall = ambient_visibility_factor_profiled(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            &wall,
            config,
            None,
        );
        let away_from_wall = ambient_visibility_factor_profiled(
            Vec3::ZERO,
            Vec3::new(-1.0, 0.0, 0.0),
            &wall,
            config,
            None,
        );

        assert!(toward_wall < 0.60);
        assert!(away_from_wall > 0.95);
    }

    fn horizontal_quad_occluders(height: f32) -> ShadowOccluderSet {
        ShadowOccluderSet::new(vec![
            ShadowOccluder {
                a: Vec3::new(-0.25, height, -0.25),
                b: Vec3::new(0.25, height, -0.25),
                c: Vec3::new(0.25, height, 0.25),
            },
            ShadowOccluder {
                a: Vec3::new(-0.25, height, -0.25),
                b: Vec3::new(0.25, height, 0.25),
                c: Vec3::new(-0.25, height, 0.25),
            },
        ])
    }

    fn vertical_quad_occluders(distance: f32) -> ShadowOccluderSet {
        ShadowOccluderSet::new(vec![
            ShadowOccluder {
                a: Vec3::new(distance, -0.25, -0.25),
                b: Vec3::new(distance, 0.25, -0.25),
                c: Vec3::new(distance, 0.25, 0.25),
            },
            ShadowOccluder {
                a: Vec3::new(distance, -0.25, -0.25),
                b: Vec3::new(distance, 0.25, 0.25),
                c: Vec3::new(distance, -0.25, 0.25),
            },
        ])
    }
}
