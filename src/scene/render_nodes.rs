use super::{
    Aabb, Camera, CameraKey, InstanceSet, LabelDesc, LabelKey, Light, LightKey, MeshNode, NodeKey,
    NodeKind, RenderableNode, Scene, Transform,
};

impl Scene {
    pub(crate) fn renderables(&self) -> impl Iterator<Item = (&RenderableNode, Transform)> {
        self.renderable_nodes()
            .map(|(_node, renderable, transform)| (renderable, transform))
    }

    pub(crate) fn renderable_nodes(
        &self,
    ) -> impl Iterator<Item = (super::NodeKey, &RenderableNode, Transform)> {
        self.nodes
            .iter()
            .filter_map(|(node_key, node)| match &node.kind {
                NodeKind::Renderable(renderable) if self.visible_for_active_camera(node_key) => {
                    self.world_transform(node_key)
                        .map(|transform| (node_key, renderable, transform))
                }
                NodeKind::Empty
                | NodeKind::Renderable(_)
                | NodeKind::Mesh(_)
                | NodeKind::Model(_)
                | NodeKind::InstanceSet(_)
                | NodeKind::ParticleSet(_)
                | NodeKind::Label(_)
                | NodeKind::Camera(_)
                | NodeKind::Light(_) => None,
            })
    }

    pub(crate) fn visible_drawable_count(&self) -> usize {
        self.renderables().count()
            + self.mesh_nodes().count()
            + self.model_nodes().count()
            + self.instance_set_nodes().count()
            + self.particle_set_nodes().count()
            + self.label_nodes().count()
    }

    pub(crate) fn mesh_nodes(&self) -> impl Iterator<Item = (NodeKey, MeshNode, Transform)> + '_ {
        self.nodes.iter().filter_map(|(key, node)| {
            let NodeKind::Mesh(mesh) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(key) {
                return None;
            }
            self.world_transform(key)
                .map(|transform| (key, mesh, transform))
        })
    }

    pub(crate) fn instance_set_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, &InstanceSet, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::InstanceSet(instance_set) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
            self.instance_sets
                .get(instance_set)
                .and_then(|instance_set| {
                    self.world_transform(node_key)
                        .map(|transform| (node_key, instance_set, transform))
                })
        })
    }

    pub(crate) fn model_nodes(&self) -> impl Iterator<Item = NodeKey> + '_ {
        self.nodes.iter().filter_map(|(key, node)| {
            if !matches!(node.kind, NodeKind::Model(_)) || !self.visible_for_active_camera(key) {
                return None;
            }
            Some(key)
        })
    }

    pub(crate) fn label_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, LabelKey, &LabelDesc, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Label(label) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
            self.labels.get(label).and_then(|label_desc| {
                self.world_transform(node_key)
                    .map(|transform| (node_key, label, label_desc, transform))
            })
        })
    }

    pub(crate) fn light_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, LightKey, Light, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Light(light_key) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
            self.lights.get(light_key).copied().and_then(|light| {
                self.world_transform(node_key)
                    .map(|transform| (node_key, light_key, light, transform))
            })
        })
    }

    pub(crate) fn node_transforms(&self) -> impl Iterator<Item = (NodeKey, Transform)> + '_ {
        self.nodes.iter().map(|(key, node)| (key, node.transform))
    }

    pub(crate) fn mesh_bounds_nodes(&self) -> impl Iterator<Item = (NodeKey, Aabb)> + '_ {
        self.node_bounds
            .iter()
            .filter(|(node, _)| self.visible_for_active_camera(**node))
            .map(|(node, bounds)| (*node, *bounds))
    }

    pub(crate) fn camera_nodes(&self) -> impl Iterator<Item = (NodeKey, CameraKey, &Camera)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Camera(camera_key) = node.kind else {
                return None;
            };
            self.cameras
                .get(camera_key)
                .map(|camera| (node_key, camera_key, camera))
        })
    }
}
