use super::*;

pub(super) fn hit_geometry<const PROFILE: bool>(
    node: NodeKey,
    geometry: &GeometryDesc,
    vertices: &[crate::geometry::GeometryVertex],
    deformed: bool,
    transform: Transform,
    ray: Ray,
    metrics: &mut PickingMetrics,
) -> Option<Hit> {
    if geometry.topology() != GeometryTopology::Triangles {
        return None;
    }
    let local_ray = inverse_transform_ray(ray, transform)?;
    let bvh = triangle_bvh::<PROFILE>(geometry, vertices, deformed, metrics);
    if !ray_hits_bvh_bounds::<PROFILE>(&bvh, local_ray, metrics) {
        return None;
    }
    let candidates = bvh.ray_candidates(local_ray.origin, local_ray.direction, f32::INFINITY);
    if PROFILE {
        metrics.bvh_node_bounds_tests = metrics
            .bvh_node_bounds_tests
            .saturating_add(candidates.node_bounds_tests);
    }
    let mut best = None;
    for triangle_index in candidates.triangles {
        if PROFILE {
            metrics.triangles_considered = metrics.triangles_considered.saturating_add(1);
        }
        let indices = geometry
            .indices()
            .get(triangle_index * 3..triangle_index * 3 + 3)?;
        let (Some(a), Some(b), Some(c)) = (
            vertices.get(indices[0] as usize),
            vertices.get(indices[1] as usize),
            vertices.get(indices[2] as usize),
        ) else {
            continue;
        };
        best = nearest_hit(
            best,
            hit_triangle::<PROFILE>(
                HitTarget::Node(node),
                transform_point(a.position, transform),
                transform_point(b.position, transform),
                transform_point(c.position, transform),
                ray,
                metrics,
            ),
        );
    }
    best
}

pub(super) struct GeometryInstanceHitInput<'a> {
    pub(super) node: NodeKey,
    pub(super) instance: InstanceId,
    pub(super) geometry: &'a GeometryDesc,
    pub(super) vertices: &'a [crate::geometry::GeometryVertex],
    pub(super) node_transform: Transform,
    pub(super) instance_transform: Transform,
    pub(super) ray: Ray,
}

pub(super) fn hit_geometry_instance<const PROFILE: bool>(
    input: GeometryInstanceHitInput<'_>,
    metrics: &mut PickingMetrics,
) -> Option<Hit> {
    let GeometryInstanceHitInput {
        node,
        instance,
        geometry,
        vertices,
        node_transform,
        instance_transform,
        ray,
    } = input;
    if geometry.topology() != GeometryTopology::Triangles {
        return None;
    }
    let node_local_ray = inverse_transform_ray(ray, node_transform)?;
    let local_ray = inverse_transform_ray(node_local_ray, instance_transform)?;
    let bvh = triangle_bvh::<PROFILE>(geometry, vertices, false, metrics);
    if !ray_hits_bvh_bounds::<PROFILE>(&bvh, local_ray, metrics) {
        return None;
    }
    let candidates = bvh.ray_candidates(local_ray.origin, local_ray.direction, f32::INFINITY);
    if PROFILE {
        metrics.bvh_node_bounds_tests = metrics
            .bvh_node_bounds_tests
            .saturating_add(candidates.node_bounds_tests);
    }
    let mut best = None;
    for triangle_index in candidates.triangles {
        if PROFILE {
            metrics.triangles_considered = metrics.triangles_considered.saturating_add(1);
        }
        let indices = geometry
            .indices()
            .get(triangle_index * 3..triangle_index * 3 + 3)?;
        let (Some(a), Some(b), Some(c)) = (
            vertices.get(indices[0] as usize),
            vertices.get(indices[1] as usize),
            vertices.get(indices[2] as usize),
        ) else {
            continue;
        };
        best = nearest_hit(
            best,
            hit_triangle::<PROFILE>(
                HitTarget::Instance { node, instance },
                transform_point(
                    transform_point(a.position, instance_transform),
                    node_transform,
                ),
                transform_point(
                    transform_point(b.position, instance_transform),
                    node_transform,
                ),
                transform_point(
                    transform_point(c.position, instance_transform),
                    node_transform,
                ),
                ray,
                metrics,
            ),
        );
    }
    best
}

fn triangle_bvh<const PROFILE: bool>(
    geometry: &GeometryDesc,
    vertices: &[crate::geometry::GeometryVertex],
    deformed: bool,
    metrics: &mut PickingMetrics,
) -> std::sync::Arc<TriangleBvh> {
    if deformed {
        if PROFILE {
            metrics.deformed_bvh_builds = metrics.deformed_bvh_builds.saturating_add(1);
        }
        return std::sync::Arc::new(TriangleBvh::from_indexed(vertices, geometry.indices()));
    }
    let (bvh, hit) = geometry.cached_triangle_bvh();
    if PROFILE {
        if hit {
            metrics.static_bvh_cache_hits = metrics.static_bvh_cache_hits.saturating_add(1);
        } else {
            metrics.static_bvh_cache_misses = metrics.static_bvh_cache_misses.saturating_add(1);
        }
    }
    bvh
}

fn ray_hits_bvh_bounds<const PROFILE: bool>(
    bvh: &TriangleBvh,
    ray: Ray,
    metrics: &mut PickingMetrics,
) -> bool {
    if PROFILE {
        metrics.mesh_bounds_tests = metrics.mesh_bounds_tests.saturating_add(1);
    }
    let hit = bvh
        .bounds()
        .is_some_and(|bounds| ray_hits_bounds(ray, bounds.min, bounds.max));
    if PROFILE && !hit {
        metrics.mesh_bounds_rejections = metrics.mesh_bounds_rejections.saturating_add(1);
    }
    hit
}

fn inverse_transform_ray(ray: Ray, transform: Transform) -> Option<Ray> {
    const MIN_SCALE: f32 = 1.0e-8;
    if !transform.scale.x.is_finite()
        || !transform.scale.y.is_finite()
        || !transform.scale.z.is_finite()
        || transform.scale.x.abs() <= MIN_SCALE
        || transform.scale.y.abs() <= MIN_SCALE
        || transform.scale.z.abs() <= MIN_SCALE
    {
        return None;
    }
    let inverse_rotation = crate::scene::Quat::from_xyzw(
        -transform.rotation.x,
        -transform.rotation.y,
        -transform.rotation.z,
        transform.rotation.w,
    );
    let origin = rotate_vec3(
        inverse_rotation,
        subtract_vec3(ray.origin, transform.translation),
    );
    let direction = rotate_vec3(inverse_rotation, ray.direction);
    Some(Ray {
        origin: Vec3::new(
            origin.x / transform.scale.x,
            origin.y / transform.scale.y,
            origin.z / transform.scale.z,
        ),
        direction: Vec3::new(
            direction.x / transform.scale.x,
            direction.y / transform.scale.y,
            direction.z / transform.scale.z,
        ),
    })
}

pub(super) fn hit_triangle<const PROFILE: bool>(
    target: HitTarget,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    ray: Ray,
    metrics: &mut PickingMetrics,
) -> Option<Hit> {
    if PROFILE {
        metrics.triangle_bounds_tests = metrics.triangle_bounds_tests.saturating_add(1);
    }
    let (min, max) = triangle_bounds(a, b, c);
    if !ray_hits_bounds(ray, min, max) {
        return None;
    }
    if PROFILE {
        metrics.ray_triangle_intersection_tests =
            metrics.ray_triangle_intersection_tests.saturating_add(1);
    }
    let (distance, _u, _v) = ray_triangle_intersection(ray, a, b, c)?;
    Some(Hit {
        target,
        distance,
        world_position: add_vec3(ray.origin, scale_vec3(ray.direction, distance)),
        normal: normalize_optional(cross(subtract_vec3(b, a), subtract_vec3(c, a))),
    })
}
