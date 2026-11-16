//! Path-traced delivery of the host's scene.

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, PathTraceOptions, PathTracedImage};

impl<F: AssetFetcher> SceneHostCore<F> {
    /// Path traces the scene through the active camera with the renderer's
    /// current environment, exposure, tone mapping and depth of field.
    ///
    /// `without` lists node handles to leave out: stand-ins that only a
    /// rasterized frame needs, such as the generated contact-shadow decals.
    /// The scene itself is not changed, so a capture taken before the trace
    /// still describes it.
    pub fn path_trace(
        &self,
        options: &PathTraceOptions,
        without: &[u64],
    ) -> Result<PathTracedImage, SceneHostError> {
        self.ensure_active_camera()?;
        let without = without
            .iter()
            .map(|handle| self.resolve_node(*handle))
            .collect::<Result<Vec<_>, _>>()?;
        self.renderer
            .path_trace_without(
                &self.scene,
                &self.assets,
                self.active_camera,
                options,
                &without,
                &|_, _| {},
            )
            .map_err(|error| SceneHostError::new(SceneHostErrorCode::Render, error.to_string()))
    }
}
