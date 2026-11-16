//! Writing instance placements that may shear.
//!
//! Only an instanced scene import does this: it flattens an asset hierarchy
//! into instances, and a rotated node under a non-uniformly scaled one
//! flattens to a placement that a `Transform` cannot hold.

use glam::Affine3A;

use crate::diagnostics::LookupError;

use super::super::{InstanceSetKey, Scene, WorldTransform};
use super::InstanceId;

impl Scene {
    /// Adds an instance placed by `placement`, which may be sheared.
    pub(crate) fn push_instance_placement(
        &mut self,
        instance_set: InstanceSetKey,
        placement: WorldTransform,
    ) -> Result<InstanceId, LookupError> {
        if let Some(transform) = placement.exact_trs() {
            return self.push_instance(instance_set, transform);
        }
        let matrix = validate_placement(placement)?;
        let id = self.push_instance(instance_set, placement.trs())?;
        self.instance_set_mut(instance_set)?
            .sheared
            .insert(id, matrix);
        Ok(id)
    }

    /// Replaces an instance's placement with `placement`, which may be
    /// sheared.
    pub(crate) fn set_instance_placement(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        placement: WorldTransform,
    ) -> Result<(), LookupError> {
        if let Some(transform) = placement.exact_trs() {
            return self.set_instance_transform(instance_set, instance, transform);
        }
        let matrix = validate_placement(placement)?;
        let set = self.instance_set_mut(instance_set)?;
        let moved = set.set_transform(instance_set, instance, placement.trs())?;
        let sheared = set.sheared.insert(instance, matrix) != Some(matrix);
        if moved || sheared {
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }
}

fn validate_placement(placement: WorldTransform) -> Result<Affine3A, LookupError> {
    let matrix = placement.matrix();
    if matrix.is_finite() {
        Ok(matrix)
    } else {
        Err(LookupError::InvalidTransform {
            reason: "instance placement must contain only finite values",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::view_math::transform_aabb;
    use crate::scene::{PerspectiveCamera, Transform, Vec3};
    use crate::{
        Aabb, Assets, Color, CursorPosition, GeometryDesc, HitTarget, MaterialDesc, Viewport,
    };

    fn sheared() -> WorldTransform {
        WorldTransform::from(Transform::IDENTITY.with_scale(Vec3::new(3.0, 1.0, 1.0)))
            .child(Transform::at(Vec3::new(0.5, 0.0, 0.0)).rotate_z_deg(40.0))
    }

    fn scene_with_set() -> (Scene, InstanceSetKey) {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.1, 0.1, 0.1));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let set = scene
            .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
            .expect("instance set inserts");
        (scene, set)
    }

    fn placement_of(scene: &Scene, set: InstanceSetKey, id: InstanceId) -> WorldTransform {
        let set = scene.instance_set(set).expect("instance set exists");
        let instance = set
            .instances()
            .find(|instance| instance.id() == id)
            .expect("instance exists");
        set.placement(instance)
    }

    #[test]
    fn a_sheared_placement_is_kept_exactly_and_its_transform_is_the_decomposition() {
        let (mut scene, set) = scene_with_set();
        let placement = sheared();
        assert!(placement.exact_trs().is_none());

        let id = scene
            .push_instance_placement(set, placement)
            .expect("sheared instance inserts");

        assert_eq!(placement_of(&scene, set, id).matrix(), placement.matrix());
        let instances = scene.instance_set(set).expect("instance set exists");
        assert!(instances.has_sheared_instance());
        assert_eq!(
            instances
                .instances()
                .next()
                .expect("one instance")
                .transform(),
            placement.trs()
        );
    }

    #[test]
    fn a_trs_transform_replaces_a_sheared_placement() {
        let (mut scene, set) = scene_with_set();
        let id = scene
            .push_instance_placement(set, sheared())
            .expect("sheared instance inserts");
        let plain = Transform::at(Vec3::new(1.0, 2.0, 3.0));
        let revision = scene.transform_revision;

        scene
            .set_instance_transform(set, id, plain)
            .expect("instance transform updates");

        assert_eq!(placement_of(&scene, set, id).exact_trs(), Some(plain));
        assert!(
            !scene
                .instance_set(set)
                .expect("set exists")
                .has_sheared_instance()
        );
        assert!(scene.transform_revision > revision);
    }

    #[test]
    fn replacing_a_placement_marks_transforms_dirty_only_when_it_changes() {
        let (mut scene, set) = scene_with_set();
        let id = scene
            .push_instance(set, Transform::IDENTITY)
            .expect("instance inserts");
        let revision = scene.transform_revision;

        scene
            .set_instance_placement(set, id, sheared())
            .expect("placement updates");
        let after_first = scene.transform_revision;
        scene
            .set_instance_placement(set, id, sheared())
            .expect("placement repeats");

        assert!(after_first > revision);
        assert_eq!(scene.transform_revision, after_first);
        assert_eq!(placement_of(&scene, set, id).matrix(), sheared().matrix());
    }

    #[test]
    fn removed_and_hidden_instances_stop_counting_as_sheared() {
        let (mut scene, set) = scene_with_set();
        let id = scene
            .push_instance_placement(set, sheared())
            .expect("sheared instance inserts");

        scene
            .set_instance_visible(set, id, false)
            .expect("instance hides");
        assert!(
            !scene
                .instance_set(set)
                .expect("set exists")
                .has_sheared_instance()
        );

        scene.remove_instance(set, id).expect("instance removes");
        let replacement = scene
            .push_instance(set, Transform::IDENTITY)
            .expect("instance inserts");
        assert!(
            placement_of(&scene, set, replacement).exact_trs().is_some(),
            "a removed instance must not leave its matrix behind"
        );
    }

    /// One sheared instance of a 1.0 x 0.2 x 0.2 bar under a rotated set
    /// node, with the exact world placement the pair gives it.
    fn scene_with_sheared_bar() -> (
        Scene,
        Assets,
        crate::scene::NodeKey,
        InstanceId,
        WorldTransform,
    ) {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 0.2, 0.2));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let node_local = Transform::at(Vec3::new(0.2, 0.1, 0.0)).rotate_y_deg(15.0);
        let (node, set) = scene
            .add_instance_set_node(scene.root(), geometry, material, node_local)
            .expect("instance set inserts");
        let id = scene
            .push_instance_placement(set, sheared())
            .expect("sheared instance inserts");
        let world = WorldTransform::from(node_local).then(sheared());
        (scene, assets, node, id, world)
    }

    #[test]
    fn a_set_with_a_sheared_instance_is_baked_per_instance() {
        let (scene, _assets, node, _id, _world) = scene_with_sheared_bar();
        let set = scene
            .instance_set_nodes()
            .find(|(candidate, _, _)| *candidate == node)
            .map(|(_, set, _)| set)
            .expect("instance set node is visible");
        assert!(set.needs_per_instance_bake());
    }

    #[test]
    fn world_bounds_place_a_sheared_instance_with_its_matrix() {
        let (scene, assets, node, _id, world) = scene_with_sheared_bar();
        let half = Vec3::new(0.5, 0.1, 0.1);
        let expected = transform_aabb(Aabb::new(-half, half), world);

        let bounds = scene
            .node_world_bounds(node, &assets)
            .expect("bounds resolve")
            .expect("the set has bounds");

        assert!(
            (bounds.min - expected.min).abs().max_element() <= 1.0e-5
                && (bounds.max - expected.max).abs().max_element() <= 1.0e-5,
            "expected {expected:?}, got {bounds:?}"
        );
    }

    #[test]
    fn picks_hit_a_sheared_instance_on_its_exact_surface() {
        let (mut scene, assets, node, id, world) = scene_with_sheared_bar();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, 6.0)),
            )
            .expect("camera inserts");
        scene.set_active_camera(camera).expect("camera activates");
        // Aim at a point inside the bar, towards one end, where the sheared
        // bar and its decomposition no longer overlap.
        let inside = world.transform_point(Vec3::new(0.4, 0.0, 0.0));
        let cursor = scene
            .project_world_point(camera, inside, 200, 200)
            .expect("camera resolves")
            .expect("the bar is in front of the camera");
        let cursor = CursorPosition::physical(cursor.x, cursor.y);
        let viewport = Viewport::new(200, 200, 1.0).expect("viewport is valid");

        let hit = scene
            .pick_with_assets(camera, cursor, viewport, &assets)
            .expect("pick succeeds")
            .expect("the ray passes through the bar");

        assert_eq!(hit.target(), HitTarget::Instance { node, instance: id });
        let local = world
            .inverse()
            .expect("placement inverts")
            .transform_point(hit.world_position);
        let half = Vec3::new(0.5, 0.1, 0.1);
        assert!(
            (local.abs() - half).max_element() <= 1.0e-3,
            "the hit must lie on the bar: local {local:?}"
        );
        assert!(
            ((local.abs() - half).abs().min_element()) <= 1.0e-3,
            "the hit must lie on a face of the bar: local {local:?}"
        );
    }
}
