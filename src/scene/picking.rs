use crate::Assets;
use crate::diagnostics::LookupError;
use crate::picking::{
    CursorPosition, Hit, HitTarget, InteractionContext, PickingMetrics, Viewport,
};

use super::{CameraKey, NodeKey, NodeKind, RenderableNode, Scene, WorldTransform};

impl Scene {
    /// Picks the nearest hit under `cursor` without an `Assets` store.
    ///
    /// Primitives added with [`Scene::add_renderable`] are intersected
    /// triangle by triangle. Mesh and instance-set nodes are
    /// **bounds-accurate**: a scene holds handles, not geometry, so it
    /// intersects the local bounds it recorded for each node, carried through
    /// the node's exact world matrix. Bounds are recorded when a glTF scene is
    /// instantiated and whenever
    /// [`Renderer::prepare_with_assets`](crate::Renderer::prepare_with_assets)
    /// runs. A bounds hit reports the point where the ray enters the bounds as
    /// `world_position`, has no `normal`, and can land on the empty space a
    /// concave or hollow mesh leaves inside its bounds. Line geometry and
    /// stroke materials are not pickable, as in [`Scene::pick_with_assets`],
    /// which is the **triangle-accurate** pick.
    ///
    /// # Errors
    ///
    /// A plain pick never reports a visible asset-backed node as a miss. It
    /// returns [`LookupError::PickRequiresAssets`] when a visible mesh or
    /// instance-set node has no usable recorded bounds: the node was never
    /// prepared or imported, its geometry or material handle changed since, or
    /// it is morphed or skinned and can have left the bounds of its undeformed
    /// mesh. Call [`Scene::pick_with_assets`] for such a scene, or prepare it
    /// first. [`LookupError::CameraNotFound`] is returned for a missing camera.
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
    /// This is the triangle-accurate pick; [`Scene::pick`] only sees mesh
    /// nodes through their recorded bounds.
    ///
    /// Morph targets are evaluated before skinning, matching render and shadow
    /// preparation. Each node's exact world matrix and each instance transform
    /// are then applied in scene-graph order, so sheared and mirrored
    /// hierarchies are hit where they are drawn. Hits report world-space
    /// distance, position, and the geometric normal of the rendered front
    /// face; singular transforms only remain hittable where they leave a
    /// nondegenerate triangle. Missing or invalid deformation inputs return a
    /// structured [`LookupError::InvalidSkinBinding`] instead of using base
    /// vertices.
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

    /// [`Scene::pick`] for a pointer position in physical pixels.
    ///
    /// Mesh and instance-set nodes are bounds-accurate here too; use
    /// [`Scene::pick_with_assets`] for triangle accuracy.
    ///
    /// # Errors
    ///
    /// Returns [`LookupError::InvalidViewport`] for a zero-sized viewport and
    /// every error of [`Scene::pick`], including
    /// [`LookupError::PickRequiresAssets`] for a visible asset-backed node
    /// with no usable recorded bounds.
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

    /// Runs [`Scene::pick_pointer`] and makes the result the hover and
    /// primary selection target.
    ///
    /// Mesh and instance-set nodes are bounds-accurate; use
    /// [`Scene::pick_and_select_with_assets`] for triangle accuracy.
    ///
    /// # Errors
    ///
    /// Returns every error of [`Scene::pick_pointer`], including
    /// [`LookupError::PickRequiresAssets`]. A failed pick leaves the hover and
    /// selection untouched.
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

    /// Runs the triangle-accurate [`Scene::pick_with_assets`] and makes the
    /// result the hover and primary selection target.
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

    /// Runs [`Scene::pick_pointer`] and makes the result the hover target.
    ///
    /// Mesh and instance-set nodes are bounds-accurate; use
    /// [`Scene::pick_and_hover_with_assets`] for triangle accuracy.
    ///
    /// # Errors
    ///
    /// Returns every error of [`Scene::pick_pointer`], including
    /// [`LookupError::PickRequiresAssets`]. A failed pick leaves the hover
    /// untouched.
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

    /// Runs the triangle-accurate [`Scene::pick_with_assets`] and makes the
    /// result the hover target.
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
    ) -> impl Iterator<Item = (NodeKey, &RenderableNode, WorldTransform)> {
        self.nodes
            .iter()
            .filter_map(|(key, node)| match &node.kind {
                NodeKind::Renderable(renderable) if self.visible_for_active_camera(key) => self
                    .resolved_world(key)
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
