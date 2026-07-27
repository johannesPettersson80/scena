use crate::Assets;
use crate::diagnostics::LookupError;
use crate::picking::{
    CursorPosition, Hit, HitTarget, InteractionContext, PickingMetrics, Viewport,
};

use super::{CameraKey, NodeKey, NodeKind, RenderableNode, Scene, Transform};

impl Scene {
    #[cfg(feature = "scene-host")]
    pub(crate) fn raycast_with_assets<F>(
        &self,
        origin: super::Vec3,
        direction: super::Vec3,
        assets: &Assets<F>,
    ) -> Result<Option<Hit>, LookupError> {
        crate::picking::raycast_scene_with_assets(self, assets, origin, direction)
    }

    pub fn pick(
        &self,
        camera: CameraKey,
        cursor: CursorPosition,
        viewport: Viewport,
    ) -> Result<Option<Hit>, LookupError> {
        crate::picking::pick_scene(self, camera, cursor, viewport)
    }

    /// Intersects asset-backed mesh triangles in their current rendered pose.
    ///
    /// Morph targets are evaluated before skinning, matching render and shadow
    /// preparation. Node transforms and instance-root transforms are then
    /// applied in scene-graph order. Hits report world-space distance, position,
    /// and the transformed geometric winding normal; singular transforms only
    /// remain hittable where they leave a nondegenerate triangle. Missing or
    /// invalid deformation inputs return a structured
    /// [`LookupError::InvalidSkinBinding`] instead of using base vertices.
    pub fn pick_with_assets<F>(
        &self,
        camera: CameraKey,
        cursor: CursorPosition,
        viewport: Viewport,
        assets: &Assets<F>,
    ) -> Result<Option<Hit>, LookupError> {
        crate::picking::pick_scene_with_assets(self, assets, camera, cursor, viewport)
    }

    /// Runs an asset-aware pick while returning deterministic work counters.
    ///
    /// Use this inspection path for benchmark and diagnostic evidence. Normal
    /// interactive picking should use [`Scene::pick_with_assets`] so it does
    /// not collect profiling counters.
    pub fn pick_with_assets_profiled<F>(
        &self,
        camera: CameraKey,
        cursor: CursorPosition,
        viewport: Viewport,
        assets: &Assets<F>,
    ) -> Result<(Option<Hit>, PickingMetrics), LookupError> {
        crate::picking::pick_scene_with_assets_profiled(self, assets, camera, cursor, viewport)
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

    pub fn pick_and_select_with_assets<F>(
        &mut self,
        camera: CameraKey,
        cursor: CursorPosition,
        viewport: Viewport,
        assets: &Assets<F>,
    ) -> Result<Option<Hit>, LookupError> {
        let hit = self.pick_with_assets(camera, cursor, viewport, assets)?;
        let target = hit.map(|hit| hit.target());
        self.interaction.set_hover(target);
        self.interaction.set_primary_selection(target);
        Ok(hit)
    }

    pub fn pick_and_hover(
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
        self.set_hover_target(hit.map(|hit| hit.target()));
        Ok(hit)
    }

    pub fn pick_and_hover_with_assets<F>(
        &mut self,
        camera: CameraKey,
        cursor: CursorPosition,
        viewport: Viewport,
        assets: &Assets<F>,
    ) -> Result<Option<Hit>, LookupError> {
        let hit = self.pick_with_assets(camera, cursor, viewport, assets)?;
        self.set_hover_target(hit.map(|hit| hit.target()));
        Ok(hit)
    }

    pub fn set_hover_target(&mut self, target: Option<HitTarget>) {
        self.interaction.set_hover(target);
    }

    pub fn set_primary_selection_target(&mut self, target: Option<HitTarget>) {
        self.interaction.set_primary_selection(target);
    }

    pub(crate) fn pickable_renderables(
        &self,
    ) -> impl Iterator<Item = (NodeKey, &RenderableNode, Transform)> {
        self.nodes
            .iter()
            .filter_map(|(key, node)| match &node.kind {
                NodeKind::Renderable(renderable) if self.visible_for_active_camera(key) => self
                    .world_transform(key)
                    .map(|transform| (key, renderable, transform)),
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

    pub fn interaction(&self) -> &InteractionContext {
        &self.interaction
    }

    pub fn interaction_mut(&mut self) -> &mut InteractionContext {
        &mut self.interaction
    }
}
