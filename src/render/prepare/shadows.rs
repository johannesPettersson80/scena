use crate::BakedAmbientOcclusionConfig;
use crate::assets::{Assets, MaterialHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::{Aabb, GeometryDesc, GeometryTopology, GeometryVertex, TriangleBvh};
use crate::scene::{NodeKey, Scene, Transform, Vec3};

use super::DeformationInputs;
use super::lighting::PreparedLights;
use super::materials::render_material_slot;
use super::transforms::{compose_transform, transform_position, transform_primitive};
use crate::render::PrepareWorkCounter;

mod cache;
mod math;
pub(in crate::render) use cache::ShadowVisibilityCache;
use cache::shadow_triangle_signature;
use math::{add_vec3, bounds_corners, cross_vec3, dot_vec3, scale_vec3, subtract_vec3};

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

    pub(super) fn triangles(&self) -> &[ShadowOccluder] {
        &self.triangles
    }
}

pub(super) fn collect_shadow_occluders<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<ShadowOccluderSet, PrepareError> {
    let mut occluders = Vec::new();

    for (renderable, transform) in scene.renderables() {
        for primitive in renderable.primitives() {
            let primitive = transform_primitive(primitive, transform, origin_shift);
            let [a, b, c] = primitive.vertices();
            occluders.push(ShadowOccluder {
                a: a.position,
                b: b.position,
                c: c.position,
            });
        }
    }

    let Some(assets) = assets else {
        return Ok(ShadowOccluderSet::new(occluders));
    };

    for (node, mesh, transform) in scene.mesh_nodes() {
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        let vertices = shadow_vertices(
            node,
            &geometry,
            DeformationInputs {
                morph_weights: scene.morph_weights(node),
                skin_matrices: scene.skin_matrices(node).as_deref(),
            },
        )?;
        append_shadow_geometry(
            &mut occluders,
            &geometry,
            &vertices,
            transform,
            origin_shift,
        );
    }

    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        let vertices = shadow_vertices(node, &geometry, DeformationInputs::default())?;
        for instance in instance_set.instances() {
            append_shadow_geometry(
                &mut occluders,
                &geometry,
                &vertices,
                compose_transform(node_transform, instance.transform()),
                origin_shift,
            );
        }
    }

    Ok(ShadowOccluderSet::new(occluders))
}

pub(super) fn cpu_shadow_visibility_required(
    scene: &Scene,
    backend_material_slots: &[MaterialHandle],
) -> bool {
    for (_node, mesh, _transform) in scene.mesh_nodes() {
        if render_material_slot(mesh.material(), backend_material_slots) == 0 {
            return true;
        }
    }
    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        if render_material_slot(instance_set.material(), backend_material_slots) == 0 {
            return true;
        }
    }
    false
}

pub(super) fn collect_shadow_projection_points<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    origin_shift: Vec3,
) -> Result<Vec<Vec3>, PrepareError> {
    let mut points = Vec::new();

    for (renderable, transform) in scene.renderables() {
        for primitive in renderable.primitives() {
            let primitive = transform_primitive(primitive, transform, origin_shift);
            for vertex in primitive.vertices() {
                points.push(vertex.position);
            }
        }
    }

    let Some(assets) = assets else {
        return Ok(points);
    };

    for (node, mesh, transform) in scene.mesh_nodes() {
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        let skin_matrices = scene.skin_matrices(node);
        let deformation = DeformationInputs {
            morph_weights: scene.morph_weights(node),
            skin_matrices: skin_matrices.as_deref(),
        };
        append_shadow_projection_points(
            &mut points,
            node,
            &geometry,
            deformation,
            transform,
            origin_shift,
        )?;
    }

    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        for instance in instance_set.instances() {
            append_shadow_projection_points(
                &mut points,
                node,
                &geometry,
                DeformationInputs::default(),
                compose_transform(node_transform, instance.transform()),
                origin_shift,
            )?;
        }
    }

    Ok(points)
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

fn shadow_vertices(
    node: NodeKey,
    geometry: &GeometryDesc,
    deformation: DeformationInputs<'_>,
) -> Result<Vec<GeometryVertex>, PrepareError> {
    geometry
        .deformed_vertices(deformation.morph_weights, deformation.skin_matrices)
        .map(|vertices| vertices.into_owned())
        .map_err(|error| PrepareError::InvalidSkinGeometry {
            node,
            reason: format!("{error:?}"),
        })
}

fn append_shadow_geometry(
    occluders: &mut Vec<ShadowOccluder>,
    geometry: &GeometryDesc,
    vertices: &[GeometryVertex],
    transform: Transform,
    origin_shift: Vec3,
) {
    if geometry.topology() != GeometryTopology::Triangles {
        return;
    }
    for triangle in geometry.indices().chunks_exact(3) {
        occluders.push(ShadowOccluder {
            a: transform_position(
                vertices[triangle[0] as usize].position,
                transform,
                origin_shift,
            ),
            b: transform_position(
                vertices[triangle[1] as usize].position,
                transform,
                origin_shift,
            ),
            c: transform_position(
                vertices[triangle[2] as usize].position,
                transform,
                origin_shift,
            ),
        });
    }
}

/// Computes the light-space view-projection matrix for a directional light.
/// Returns an orthographic `light_from_world` whose frustum tightly encloses
/// the world-space AABB of `occluders` along `light_direction`. The shader
/// projects fragment world positions through this matrix and samples the
/// shadow texture written by the shadow caster pass; closes the
/// `LightCamera` portion of scena-wgpu-architect F3 (Phase 1B).
///
/// Returned matrix is column-major 4x4 to match the WGSL `mat4x4<f32>` upload
/// layout used elsewhere in the renderer.
pub(super) fn directional_light_view_projection(
    light_direction: Vec3,
    occluders: &[ShadowOccluder],
) -> [f32; 16] {
    directional_light_view_projection_from_points(
        light_direction,
        occluders
            .iter()
            .flat_map(|occluder| [occluder.a, occluder.b, occluder.c]),
    )
}

pub(in crate::render) fn directional_light_view_projection_from_points(
    light_direction: Vec3,
    points: impl IntoIterator<Item = Vec3>,
) -> [f32; 16] {
    // Light forward = direction the light travels. Build an orthonormal basis.
    let forward = normalize_or(light_direction, Vec3::new(0.0, 0.0, -1.0));
    let world_up = if forward.y.abs() > 0.99 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let right = normalize_or(cross_vec3(world_up, forward), Vec3::new(1.0, 0.0, 0.0));
    let up = cross_vec3(forward, right);

    // Project all occluder vertices into the light-view basis, then build an
    // orthographic projection that fits the resulting AABB. min/max is the
    // tight light-space bounding box.
    let mut any = false;
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for vertex in points {
        any = true;
        let lx = dot_vec3(right, vertex);
        let ly = dot_vec3(up, vertex);
        let lz = dot_vec3(forward, vertex);
        min = Vec3::new(min.x.min(lx), min.y.min(ly), min.z.min(lz));
        max = Vec3::new(max.x.max(lx), max.y.max(ly), max.z.max(lz));
    }
    if !any {
        return identity_matrix4();
    }

    // Pad slightly so receivers near the AABB edges aren't clipped, and so
    // self-shadow acne lands inside the [near, far] window the projection
    // covers.
    let pad = ((max.x - min.x) + (max.y - min.y) + (max.z - min.z)) * 0.05;
    min = Vec3::new(min.x - pad, min.y - pad, min.z - pad);
    max = Vec3::new(max.x + pad, max.y + pad, max.z + pad);

    // Orthographic projection mapping light-view [min..max] → clip [-1..1] x
    // [-1..1] x [0..1] (WebGPU/wgpu Z range). Composed with the view rotation
    // expressing world → light-view-space.
    let inv_x = 1.0 / (max.x - min.x).max(f32::EPSILON);
    let inv_y = 1.0 / (max.y - min.y).max(f32::EPSILON);
    let inv_z = 1.0 / (max.z - min.z).max(f32::EPSILON);

    // light_from_world = ortho * view. Compose by hand so the column-major
    // upload layout matches WGSL.
    //
    // view matrix (world → light-view-space, column-major):
    //   col0 = (right.x, up.x, forward.x, 0)
    //   col1 = (right.y, up.y, forward.y, 0)
    //   col2 = (right.z, up.z, forward.z, 0)
    //   col3 = (0, 0, 0, 1)
    //
    // ortho matrix (light-view → clip, column-major, depth in [0..1]):
    //   col0 = (2 * inv_x, 0, 0, 0)
    //   col1 = (0, 2 * inv_y, 0, 0)
    //   col2 = (0, 0, inv_z, 0)
    //   col3 = (-(max.x + min.x) * inv_x,
    //           -(max.y + min.y) * inv_y,
    //           -min.z * inv_z,
    //           1)
    //
    // Combined column-major light_from_world:
    let cm = |x: f32, y: f32, z: f32, w: f32| [x, y, z, w];
    let col0 = cm(
        2.0 * right.x * inv_x,
        2.0 * up.x * inv_y,
        forward.x * inv_z,
        0.0,
    );
    let col1 = cm(
        2.0 * right.y * inv_x,
        2.0 * up.y * inv_y,
        forward.y * inv_z,
        0.0,
    );
    let col2 = cm(
        2.0 * right.z * inv_x,
        2.0 * up.z * inv_y,
        forward.z * inv_z,
        0.0,
    );
    let col3 = cm(
        -(max.x + min.x) * inv_x,
        -(max.y + min.y) * inv_y,
        -min.z * inv_z,
        1.0,
    );
    [
        col0[0], col0[1], col0[2], col0[3], col1[0], col1[1], col1[2], col1[3], col2[0], col2[1],
        col2[2], col2[3], col3[0], col3[1], col3[2], col3[3],
    ]
}

fn append_shadow_projection_points(
    points: &mut Vec<Vec3>,
    node: NodeKey,
    geometry: &GeometryDesc,
    deformation: DeformationInputs<'_>,
    transform: Transform,
    origin_shift: Vec3,
) -> Result<(), PrepareError> {
    if geometry.topology() != GeometryTopology::Triangles {
        return Ok(());
    }
    if deformation.morph_weights.is_none()
        && deformation.skin_matrices.is_none()
        && geometry.skin().is_none()
    {
        append_transformed_bounds_points(points, geometry.bounds(), transform, origin_shift);
        return Ok(());
    }

    let vertices = shadow_vertices(node, geometry, deformation)?;
    for vertex in vertices {
        points.push(transform_position(vertex.position, transform, origin_shift));
    }
    Ok(())
}

fn append_transformed_bounds_points(
    points: &mut Vec<Vec3>,
    bounds: Aabb,
    transform: Transform,
    origin_shift: Vec3,
) {
    for corner in bounds_corners(bounds) {
        points.push(transform_position(corner, transform, origin_shift));
    }
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length = dot_vec3(value, value).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return fallback;
    }
    Vec3::new(value.x / length, value.y / length, value.z / length)
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
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
    fn shadow_projection_from_points_matches_triangle_vertices() {
        let points = bounds_corners(Aabb::new(
            Vec3::new(-1.0, -0.5, -0.25),
            Vec3::new(1.0, 0.5, 0.25),
        ));
        let occluders = [
            ShadowOccluder {
                a: points[0],
                b: points[1],
                c: points[2],
            },
            ShadowOccluder {
                a: points[3],
                b: points[4],
                c: points[5],
            },
            ShadowOccluder {
                a: points[6],
                b: points[7],
                c: points[0],
            },
        ];
        let light_direction = Vec3::new(-0.2, -1.0, -0.35);

        let from_points = directional_light_view_projection_from_points(light_direction, points);
        let from_triangles = directional_light_view_projection(light_direction, &occluders);

        for (left, right) in from_points.iter().zip(from_triangles.iter()) {
            assert!((left - right).abs() < 1e-6);
        }
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
