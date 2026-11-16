//! Bounds-accurate hits for picks that have no `Assets` to read triangles from.

use super::*;
use crate::geometry::Aabb;

/// The twelve triangles of a box whose corner `i` takes its maximum on X, Y,
/// and Z when bits 0, 1, and 2 of `i` are set.
const BOX_TRIANGLES: [[usize; 3]; 12] = [
    [0, 2, 6],
    [0, 6, 4],
    [1, 3, 7],
    [1, 7, 5],
    [0, 1, 5],
    [0, 5, 4],
    [2, 3, 7],
    [2, 7, 6],
    [0, 1, 3],
    [0, 3, 2],
    [4, 5, 7],
    [4, 7, 6],
];

/// Picks every visible mesh and instance-set node through the local bounds
/// the scene recorded for it, carried through the node's exact world matrix.
///
/// # Errors
///
/// Returns [`LookupError::PickRequiresAssets`] for the first visible node the
/// scene cannot bound, so an asset-backed node is never reported as a miss.
pub(super) fn pick_recorded_bounds(scene: &Scene, ray: Ray) -> Result<Option<Hit>, LookupError> {
    let mut best = None;
    for (node, mesh, world) in scene.mesh_nodes() {
        let Some(bounds) = scene.recorded_pick_surface(node, mesh.geometry(), mesh.material())?
        else {
            continue;
        };
        best = nearest_hit(best, hit_bounds(HitTarget::Node(node), bounds, world, ray));
    }
    for (node, instance_set, node_world) in scene.instance_set_nodes() {
        if !instance_set.instances().any(|instance| instance.visible()) {
            continue;
        }
        let Some(bounds) =
            scene.recorded_pick_surface(node, instance_set.geometry(), instance_set.material())?
        else {
            continue;
        };
        for instance in instance_set
            .instances()
            .filter(|instance| instance.visible())
        {
            let target = HitTarget::Instance {
                node,
                instance: instance.id(),
            };
            let world = node_world.child(instance.transform());
            best = nearest_hit(best, hit_bounds(target, bounds, world, ray));
        }
    }
    Ok(best)
}

/// Intersects the box that `bounds` becomes under `world`.
///
/// The box is tested as world-space triangles instead of inverting `world`,
/// so a sheared box is exact and a flattening transform still leaves its
/// visible faces hittable. The hit has no normal: the bounds are not the
/// surface.
fn hit_bounds(target: HitTarget, bounds: Aabb, world: WorldTransform, ray: Ray) -> Option<Hit> {
    let corners: [Vec3; 8] = std::array::from_fn(|corner| {
        world.transform_point(Vec3::new(
            if corner & 1 == 0 {
                bounds.min.x
            } else {
                bounds.max.x
            },
            if corner & 2 == 0 {
                bounds.min.y
            } else {
                bounds.max.y
            },
            if corner & 4 == 0 {
                bounds.min.z
            } else {
                bounds.max.z
            },
        ))
    });
    let distance = BOX_TRIANGLES
        .iter()
        .filter_map(|[a, b, c]| {
            ray_triangle_intersection(ray, corners[*a], corners[*b], corners[*c])
        })
        .map(|(distance, _u, _v)| distance)
        .min_by(f32::total_cmp)?;
    Some(Hit {
        target,
        distance,
        world_position: add_vec3(ray.origin, scale_vec3(ray.direction, distance)),
        normal: None,
    })
}
