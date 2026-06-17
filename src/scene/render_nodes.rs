use super::{NodeKind, RenderableNode, Scene, Transform};

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
}
