use super::{NodeKind, RenderableNode, Scene, Transform};

impl Scene {
    pub(crate) fn renderables(&self) -> impl Iterator<Item = (&RenderableNode, Transform)> {
        self.nodes.values().filter_map(|node| match &node.kind {
            NodeKind::Renderable(renderable) => Some((renderable, node.transform)),
            NodeKind::Empty
            | NodeKind::Mesh(_)
            | NodeKind::Model(_)
            | NodeKind::InstanceSet(_)
            | NodeKind::Label(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        })
    }
}
