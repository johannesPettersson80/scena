use crate::diagnostics::LookupError;
use crate::picking::{CursorPosition, Hit, InteractionContext, Viewport};

use super::{CameraKey, NodeKey, NodeKind, RenderableNode, Scene, Transform};

impl Scene {
    pub fn pick(
        &self,
        camera: CameraKey,
        cursor: CursorPosition,
        viewport: Viewport,
    ) -> Result<Option<Hit>, LookupError> {
        crate::picking::pick_scene(self, camera, cursor, viewport)
    }

    pub(crate) fn pickable_renderables(
        &self,
    ) -> impl Iterator<Item = (NodeKey, &RenderableNode, Transform)> {
        self.nodes
            .iter()
            .filter_map(|(key, node)| match &node.kind {
                NodeKind::Renderable(renderable) => Some((key, renderable, node.transform)),
                NodeKind::Empty
                | NodeKind::Mesh(_)
                | NodeKind::Model(_)
                | NodeKind::InstanceSet(_)
                | NodeKind::Label(_)
                | NodeKind::Camera(_)
                | NodeKind::Light(_) => None,
            })
    }

    pub fn interaction(&self) -> &InteractionContext {
        &self.interaction
    }

    pub fn interaction_mut(&mut self) -> &mut InteractionContext {
        &mut self.interaction
    }
}
