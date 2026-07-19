use crate::assets::DefaultAssetFetcher;
use crate::scene_host::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{Assets, HeadlessBackendSelectionReport, Renderer, SurfaceViewport};

impl<F: crate::AssetFetcher> super::SceneHostRecipeBuild<F> {
    /// Returns explicit GPU-selection evidence for preferred-GPU recipe
    /// construction without changing this result type's public field shape.
    pub const fn backend_selection_report(&self) -> Option<&HeadlessBackendSelectionReport> {
        self.host.backend_selection_report()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecipeBackendPolicy {
    Cpu,
    StrictGpu,
    PreferGpu,
}

pub(super) fn recipe_headless_host(
    width: u32,
    height: u32,
    options: crate::RendererOptions,
    backend_policy: RecipeBackendPolicy,
) -> Result<SceneHostCore<DefaultAssetFetcher>, SceneHostError> {
    let viewport = SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidViewport,
            format!("invalid viewport {width}x{height} at DPR 1"),
        )
    })?;
    let (renderer, report) = match backend_policy {
        RecipeBackendPolicy::Cpu => (
            Renderer::headless_with_options(width, height, options)?,
            None,
        ),
        RecipeBackendPolicy::StrictGpu => (
            Renderer::headless_gpu_with_options(width, height, options)?,
            None,
        ),
        RecipeBackendPolicy::PreferGpu => {
            match Renderer::headless_gpu_with_options(width, height, options) {
                Ok(renderer) => (renderer, Some(HeadlessBackendSelectionReport::gpu())),
                Err(gpu_error) => (
                    Renderer::headless_with_options(width, height, options)?,
                    Some(HeadlessBackendSelectionReport::cpu_fallback(gpu_error)),
                ),
            }
        }
    };
    let mut host = SceneHostCore::from_renderer(Assets::new(), renderer, viewport)?;
    host.backend_selection_report = report;
    Ok(host)
}

pub(super) fn recipe_manifest_host(
    width: u32,
    height: u32,
) -> Result<SceneHostCore<DefaultAssetFetcher>, SceneHostError> {
    let viewport = SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidViewport,
            format!("invalid viewport {width}x{height} at DPR 1"),
        )
    })?;
    SceneHostCore::for_manifest_build(Assets::new(), viewport)
}
