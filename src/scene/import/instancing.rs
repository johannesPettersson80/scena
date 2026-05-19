use crate::assets::SceneAssetMesh;
use crate::geometry::Aabb;

use super::ImportOptions;
use super::bounds::{transform_aabb, union_optional};
use crate::scene::{InstanceSetKey, NodeKey, NodeKind, Scene, Transform};

pub(super) fn instanced_bounds(
    bounds: Aabb,
    instances: &[Transform],
    options: ImportOptions,
) -> Aabb {
    instances
        .iter()
        .map(|instance| transform_aabb(bounds, options.convert_transform(*instance)))
        .reduce(|left, right| union_optional(Some(left), right))
        .unwrap_or(bounds)
}

impl Scene {
    pub(super) fn instantiate_single_import_instance_set(
        &mut self,
        parent: NodeKey,
        mesh: &SceneAssetMesh,
        transform: Transform,
        instances: &[Transform],
        options: ImportOptions,
    ) -> NodeKey {
        let (node, instance_set) = self.add_import_instance_set(parent, mesh, transform);
        self.push_import_instances(instance_set, instances, options);
        node
    }

    pub(super) fn instantiate_multi_import_instance_sets(
        &mut self,
        parent: NodeKey,
        meshes: &[SceneAssetMesh],
        transform: Transform,
        instances: &[Transform],
        options: ImportOptions,
    ) -> NodeKey {
        let node = self
            .insert_node(parent, NodeKind::Empty, transform)
            .expect("import parent was inserted by this scene");
        for mesh in meshes {
            let (child, instance_set) =
                self.add_import_instance_set(node, mesh, Transform::IDENTITY);
            self.push_import_instances(instance_set, instances, options);
            self.node_bounds
                .insert(child, instanced_bounds(mesh.bounds(), instances, options));
        }
        node
    }

    pub(super) fn add_import_instance_set(
        &mut self,
        parent: NodeKey,
        mesh: &SceneAssetMesh,
        transform: Transform,
    ) -> (NodeKey, InstanceSetKey) {
        let instance_set = self
            .add_instance_set(parent, mesh.geometry(), mesh.material(), transform)
            .expect("import parent was inserted by this scene");
        let node = self
            .nodes
            .iter()
            .find_map(|(node, node_data)| match node_data.kind {
                NodeKind::InstanceSet(candidate) if candidate == instance_set => Some(node),
                _ => None,
            })
            .expect("inserted instance set node is present in this scene");
        (node, instance_set)
    }

    pub(super) fn push_import_instances(
        &mut self,
        instance_set: InstanceSetKey,
        instances: &[Transform],
        options: ImportOptions,
    ) {
        for instance in instances {
            self.push_instance(instance_set, options.convert_transform(*instance))
                .expect("import-created instance set accepts instances");
        }
    }
}
