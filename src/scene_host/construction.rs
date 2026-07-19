use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AssetFetcher, Assets, DefaultAssetFetcher, HeadlessBackendSelectionReport, Renderer,
    SurfaceViewport,
};

impl SceneHostCore<DefaultAssetFetcher> {
    pub fn headless(width: u32, height: u32) -> Result<Self, SceneHostError> {
        Self::headless_with_fetcher(DefaultAssetFetcher::default(), width, height)
    }

    /// Builds a strict GPU-backed headless host.
    ///
    /// GPU adapter/device failures are returned as [`SceneHostErrorCode::Build`];
    /// this constructor never returns a CPU-backed host.
    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, SceneHostError> {
        Self::headless_gpu_with_fetcher(DefaultAssetFetcher::default(), width, height)
    }

    /// Requests a GPU host and explicitly permits a CPU fallback.
    ///
    /// The typed report records the requested and selected backends plus the
    /// original GPU build error whenever fallback was used.
    pub fn headless_prefer_gpu(
        width: u32,
        height: u32,
    ) -> Result<(Self, HeadlessBackendSelectionReport), SceneHostError> {
        Self::headless_prefer_gpu_with_fetcher(DefaultAssetFetcher::default(), width, height)
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    /// Returns explicit GPU-selection evidence when construction permitted a
    /// fallback. Strict and CPU-only construction return `None`.
    pub const fn backend_selection_report(&self) -> Option<&HeadlessBackendSelectionReport> {
        self.backend_selection_report.as_ref()
    }

    pub fn headless_with_fetcher(
        fetcher: F,
        width: u32,
        height: u32,
    ) -> Result<Self, SceneHostError> {
        let viewport = headless_viewport(width, height)?;
        Self::from_renderer(
            Assets::with_fetcher(fetcher),
            Renderer::headless(width, height)?,
            viewport,
        )
    }

    /// Builds a strict GPU-backed headless host with a caller-owned fetcher.
    ///
    /// Unlike `headless_prefer_gpu_with_fetcher`, this method propagates GPU
    /// construction failure and can never succeed with [`crate::Backend::Headless`].
    pub fn headless_gpu_with_fetcher(
        fetcher: F,
        width: u32,
        height: u32,
    ) -> Result<Self, SceneHostError> {
        Self::headless_gpu_with_fetcher_using(fetcher, width, height, Renderer::headless_gpu)
    }

    /// Requests a GPU-backed host with a caller-owned fetcher and explicitly
    /// permits CPU fallback, returning a typed selection report.
    pub fn headless_prefer_gpu_with_fetcher(
        fetcher: F,
        width: u32,
        height: u32,
    ) -> Result<(Self, HeadlessBackendSelectionReport), SceneHostError> {
        Self::headless_prefer_gpu_with_fetcher_using(fetcher, width, height, Renderer::headless_gpu)
    }

    pub(super) fn headless_gpu_with_fetcher_using<G>(
        fetcher: F,
        width: u32,
        height: u32,
        build_gpu: G,
    ) -> Result<Self, SceneHostError>
    where
        G: FnOnce(u32, u32) -> Result<Renderer, crate::BuildError>,
    {
        let viewport = headless_viewport(width, height)?;
        let renderer = build_gpu(width, height)?;
        Self::from_renderer(Assets::with_fetcher(fetcher), renderer, viewport)
    }

    pub(super) fn headless_prefer_gpu_with_fetcher_using<G>(
        fetcher: F,
        width: u32,
        height: u32,
        build_gpu: G,
    ) -> Result<(Self, HeadlessBackendSelectionReport), SceneHostError>
    where
        G: FnOnce(u32, u32) -> Result<Renderer, crate::BuildError>,
    {
        let viewport = headless_viewport(width, height)?;
        let (renderer, report) = match build_gpu(width, height) {
            Ok(renderer) => (renderer, HeadlessBackendSelectionReport::gpu()),
            Err(gpu_error) => (
                Renderer::headless(width, height)?,
                HeadlessBackendSelectionReport::cpu_fallback(gpu_error),
            ),
        };
        let host = Self::from_renderer(Assets::with_fetcher(fetcher), renderer, viewport)?;
        Ok((host, report))
    }
}

fn headless_viewport(width: u32, height: u32) -> Result<SurfaceViewport, SceneHostError> {
    SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidViewport,
            format!("invalid viewport {width}x{height} at DPR 1"),
        )
    })
}
