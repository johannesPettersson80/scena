use crate::diagnostics::LookupError;
use crate::picking::{CursorPosition, Hit, Viewport};

use super::InteractiveGltfViewer;

impl InteractiveGltfViewer {
    /// Registers a callback fired by [`Self::click_at`].
    ///
    /// The callback receives the same typed result returned by `click_at`:
    /// `Ok(Some(hit))` for a hit, `Ok(None)` for an empty click, and `Err(_)`
    /// for picking failures. This keeps application glue on the viewer API
    /// without bypassing the existing asset-aware picking path.
    pub fn on_click<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(std::result::Result<Option<Hit>, LookupError>) + 'static,
    {
        self.click_callback = Some(Box::new(callback));
        self
    }

    /// Registers a callback fired by [`Self::hover_at`].
    pub fn on_hover<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(std::result::Result<Option<Hit>, LookupError>) + 'static,
    {
        self.hover_callback = Some(Box::new(callback));
        self
    }

    /// Clears the callback registered with [`Self::on_click`].
    pub fn clear_click_callback(&mut self) -> &mut Self {
        self.click_callback = None;
        self
    }

    /// Clears the callback registered with [`Self::on_hover`].
    pub fn clear_hover_callback(&mut self) -> &mut Self {
        self.hover_callback = None;
        self
    }

    /// Ray-picks the active scene at the given physical pointer coordinates
    /// using the renderer's current target dimensions as the viewport. The
    /// pick is asset-aware so glTF-imported mesh and instance-set nodes
    /// participate alongside scene-owned renderables. `device_pixel_ratio` is
    /// fixed at 1.0; high-DPR consumers should use
    /// `viewer.scene.pick_with_assets(...)` directly.
    pub fn pick_at(&self, physical_x: f32, physical_y: f32) -> Result<Option<Hit>, LookupError> {
        let viewport = self.viewport_for_pick()?;
        self.scene.pick_with_assets(
            self.camera,
            CursorPosition::physical(physical_x, physical_y),
            viewport,
            &self.assets,
        )
    }

    /// Convenience for `pick_at` followed by promoting the hit to the scene's
    /// primary selection and hover target. Mirrors
    /// `Scene::pick_and_select_with_assets` against the active camera and
    /// renderer dimensions.
    pub fn pick_and_select_at(
        &mut self,
        physical_x: f32,
        physical_y: f32,
    ) -> Result<Option<Hit>, LookupError> {
        let viewport = self.viewport_for_pick()?;
        self.scene.pick_and_select_with_assets(
            self.camera,
            CursorPosition::physical(physical_x, physical_y),
            viewport,
            &self.assets,
        )
    }

    /// Picks and selects at physical pointer coordinates, then fires the
    /// registered click callback with the same result.
    pub fn click_at(
        &mut self,
        physical_x: f32,
        physical_y: f32,
    ) -> Result<Option<Hit>, LookupError> {
        let result = self.pick_and_select_at(physical_x, physical_y);
        if let Some(callback) = &mut self.click_callback {
            callback(result.clone());
        }
        result
    }

    /// Convenience for `pick_at` followed by setting the hovered hit. Mirrors
    /// `Scene::pick_and_hover_with_assets` against the active camera and
    /// renderer dimensions.
    pub fn pick_and_hover_at(
        &mut self,
        physical_x: f32,
        physical_y: f32,
    ) -> Result<Option<Hit>, LookupError> {
        let viewport = self.viewport_for_pick()?;
        self.scene.pick_and_hover_with_assets(
            self.camera,
            CursorPosition::physical(physical_x, physical_y),
            viewport,
            &self.assets,
        )
    }

    /// Picks and updates hover at physical pointer coordinates, then fires the
    /// registered hover callback with the same result.
    pub fn hover_at(
        &mut self,
        physical_x: f32,
        physical_y: f32,
    ) -> Result<Option<Hit>, LookupError> {
        let result = self.pick_and_hover_at(physical_x, physical_y);
        if let Some(callback) = &mut self.hover_callback {
            callback(result.clone());
        }
        result
    }

    fn viewport_for_pick(&self) -> Result<Viewport, LookupError> {
        let stats = self.renderer.stats();
        Viewport::new(stats.target_width, stats.target_height, 1.0).ok_or(
            LookupError::InvalidViewport {
                width: stats.target_width,
                height: stats.target_height,
            },
        )
    }
}
