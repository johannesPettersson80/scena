use crate::assets::DefaultAssetFetcher;
use crate::scene_host::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{Assets, Renderer, SurfaceViewport};

pub(super) fn recipe_headless_host(
    width: u32,
    height: u32,
    options: crate::RendererOptions,
    prefer_gpu: bool,
) -> Result<SceneHostCore<DefaultAssetFetcher>, SceneHostError> {
    let viewport = SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidViewport,
            format!("invalid viewport {width}x{height} at DPR 1"),
        )
    })?;
    let renderer = if prefer_gpu {
        Renderer::headless_gpu_with_options(width, height, options)
            .or_else(|_gpu_error| Renderer::headless_with_options(width, height, options))?
    } else {
        Renderer::headless_with_options(width, height, options)?
    };
    SceneHostCore::from_renderer(Assets::new(), renderer, viewport)
}
