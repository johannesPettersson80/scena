use super::{FirstRender, HeadlessGltfViewer};
use crate::diagnostics::RenderOutcome;

impl FirstRender {
    /// Observed wall time spent in the explicit first prepare stage.
    pub const fn prepare_duration(&self) -> std::time::Duration {
        self.prepare_duration
    }

    /// Observed wall time spent in the explicit first render stage.
    pub const fn render_duration(&self) -> std::time::Duration {
        self.render_duration
    }
}

impl HeadlessGltfViewer {
    /// Re-runs the explicit prepare step after scene, asset, renderer, or environment changes.
    pub fn prepare(&mut self) -> crate::Result<()> {
        let started = std::time::Instant::now();
        self.renderer
            .prepare_with_assets(&mut self.scene, &self.assets)?;
        self.last_prepare_duration = started.elapsed();
        Ok(())
    }

    /// Renders the next frame using the active camera.
    pub fn render_next_frame(&mut self) -> crate::Result<RenderOutcome> {
        let started = std::time::Instant::now();
        let outcome = self.renderer.render_active(&self.scene)?;
        self.last_render_duration = Some(started.elapsed());
        Ok(outcome)
    }
}
