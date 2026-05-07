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

    pub fn pick_pointer(
        &self,
        camera: CameraKey,
        physical_x: f32,
        physical_y: f32,
        physical_width: u32,
        physical_height: u32,
        device_pixel_ratio: f32,
    ) -> Result<Option<Hit>, LookupError> {
        let viewport = Viewport::new(physical_width, physical_height, device_pixel_ratio).ok_or(
            LookupError::InvalidViewport {
                width: physical_width,
                height: physical_height,
            },
        )?;
        self.pick(
            camera,
            CursorPosition::physical(physical_x, physical_y),
            viewport,
        )
    }

    pub fn pick_and_select(
        &mut self,
        camera: CameraKey,
        physical_x: f32,
        physical_y: f32,
        physical_width: u32,
        physical_height: u32,
        device_pixel_ratio: f32,
    ) -> Result<Option<Hit>, LookupError> {
        let hit = self.pick_pointer(
            camera,
            physical_x,
            physical_y,
            physical_width,
            physical_height,
            device_pixel_ratio,
        )?;
        let target = hit.map(|hit| hit.target());
        self.interaction.set_hover(target);
        self.interaction.set_primary_selection(target);
        Ok(hit)
    }

    pub(crate) fn pickable_renderables(
        &self,
    ) -> impl Iterator<Item = (NodeKey, &RenderableNode, Transform)> {
        self.nodes
            .iter()
            .filter_map(|(key, node)| match &node.kind {
                NodeKind::Renderable(renderable) if self.visible_for_active_camera(key) => {
                    Some((key, renderable, node.transform))
                }
                NodeKind::Empty
                | NodeKind::Renderable(_)
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
