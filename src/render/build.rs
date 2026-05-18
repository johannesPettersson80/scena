use std::cell::Cell;
use std::marker::PhantomData;

use crate::diagnostics::{
    Backend, BuildError, Capabilities, DebugOverlay, HardwareTier, RendererStats,
};
use crate::material::Color;
use crate::picking::InteractionStyle;
use crate::platform::{PlatformSurface, PlatformSurfaceAttachment};

use super::gpu;
use super::gpu::GpuDeviceState;
use super::{
    OutputTransform, Profile, Quality, RasterTarget, RenderMode, Renderer, RendererOptions,
    backend_for_attached_surface, validate_target_size,
};

impl Renderer {
    pub fn headless(width: u32, height: u32) -> Result<Self, BuildError> {
        Self::headless_with_options(width, height, RendererOptions::default())
    }

    /// Builds a CPU-headless renderer at the canonical first-render size
    /// (800x600). This is the renderer-as-library analog of the Three.js
    /// `new THREE.WebGLRenderer()` one-liner: callers who do not care about
    /// the exact target dimensions can drop the explicit `(width, height)`
    /// pair and lean on the `Renderer::headless_default()` constant. Closes
    /// scena-api-ergonomics-reviewer Phase 6 finding F1.
    pub fn headless_default() -> Result<Self, BuildError> {
        Self::headless(800, 600)
    }

    pub fn headless_with_options(
        width: u32,
        height: u32,
        options: RendererOptions,
    ) -> Result<Self, BuildError> {
        Self::from_raster_target(width, height, Backend::Headless, None, false, options)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        let gpu = pollster::block_on(gpu::request_headless_gpu(Backend::HeadlessGpu))?;
        Self::from_raster_target(
            width,
            height,
            Backend::HeadlessGpu,
            Some(gpu),
            false,
            RendererOptions::default(),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        Err(BuildError::UnsupportedBackend {
            backend: Backend::HeadlessGpu,
        })
    }

    pub fn from_surface(surface: PlatformSurface) -> Result<Self, BuildError> {
        Self::from_surface_with_options(surface, RendererOptions::default())
    }

    pub fn from_surface_with_options(
        surface: PlatformSurface,
        options: RendererOptions,
    ) -> Result<Self, BuildError> {
        let (kind, size, attachment) = surface.into_parts();
        match attachment {
            PlatformSurfaceAttachment::Descriptor => Self::from_raster_target(
                size.width,
                size.height,
                Backend::SurfaceDescriptor,
                None,
                false,
                options,
            ),
            #[cfg(not(target_arch = "wasm32"))]
            PlatformSurfaceAttachment::NativeWindow(window) => {
                let backend = backend_for_attached_surface(kind);
                let gpu =
                    pollster::block_on(gpu::request_native_surface_gpu(backend, size, window))?;
                Self::from_raster_target(size.width, size.height, backend, Some(gpu), true, options)
            }
            #[cfg(target_arch = "wasm32")]
            PlatformSurfaceAttachment::BrowserWebGpuCanvas(_)
            | PlatformSurfaceAttachment::BrowserWebGl2Canvas(_) => {
                let backend = backend_for_attached_surface(kind);
                Err(BuildError::AsyncSurfaceRequired { backend })
            }
        }
    }

    pub async fn from_surface_async(surface: PlatformSurface) -> Result<Self, BuildError> {
        Self::from_surface_async_with_options(surface, RendererOptions::default()).await
    }

    pub async fn from_surface_async_with_options(
        surface: PlatformSurface,
        options: RendererOptions,
    ) -> Result<Self, BuildError> {
        let (kind, size, attachment) = surface.into_parts();

        #[cfg(target_arch = "wasm32")]
        {
            match attachment {
                PlatformSurfaceAttachment::Descriptor => {
                    return Self::from_raster_target(
                        size.width,
                        size.height,
                        Backend::SurfaceDescriptor,
                        None,
                        false,
                        options,
                    );
                }
                PlatformSurfaceAttachment::BrowserWebGpuCanvas(canvas) => {
                    let backend = backend_for_attached_surface(kind);
                    let gpu = gpu::request_browser_surface_gpu(backend, size, canvas).await?;
                    return Self::from_raster_target(
                        size.width,
                        size.height,
                        backend,
                        Some(gpu),
                        true,
                        options,
                    );
                }
                PlatformSurfaceAttachment::BrowserWebGl2Canvas(canvas) => {
                    let backend = backend_for_attached_surface(kind);
                    let gpu = gpu::request_browser_surface_gpu(backend, size, canvas).await?;
                    return Self::from_raster_target(
                        size.width,
                        size.height,
                        backend,
                        Some(gpu),
                        true,
                        options,
                    );
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let gpu = match attachment {
                PlatformSurfaceAttachment::Descriptor => {
                    return Self::from_raster_target(
                        size.width,
                        size.height,
                        Backend::SurfaceDescriptor,
                        None,
                        false,
                        options,
                    );
                }
                PlatformSurfaceAttachment::NativeWindow(window) => {
                    let backend = backend_for_attached_surface(kind);
                    gpu::request_native_surface_gpu(backend, size, window).await?
                }
            };
            let backend = backend_for_attached_surface(kind);
            Self::from_raster_target(size.width, size.height, backend, Some(gpu), true, options)
        }
    }

    pub(super) fn from_raster_target(
        width: u32,
        height: u32,
        backend: Backend,
        gpu: Option<GpuDeviceState>,
        surface_attached: bool,
        options: RendererOptions,
    ) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        let has_gpu = gpu.is_some();
        let capabilities = if surface_attached {
            Capabilities::for_attached_gpu_backend(backend)
        } else if has_gpu {
            Capabilities::for_gpu_backend(backend)
        } else {
            Capabilities::for_backend(backend)
        };
        let target = RasterTarget {
            width,
            height,
            backend,
        };
        let profile = options.profile();
        let quality = resolve_quality(options, capabilities);
        let render_mode = resolve_render_mode(options, profile);
        Ok(Self {
            target,
            prepared: None,
            frame: vec![0; target.byte_len()],
            fxaa_scratch: vec![0; target.byte_len()],
            linear_frame: (!has_gpu).then(|| vec![Color::BLACK; target.pixel_len()]),
            depth_frame: (!has_gpu).then(|| vec![f32::INFINITY; target.pixel_len()]),
            stats: RendererStats {
                target_width: width,
                target_height: height,
                ..RendererStats::default()
            },
            diagnostics: Vec::new(),
            capabilities,
            gpu,
            output: OutputTransform::default(),
            profile,
            quality,
            render_mode,
            render_generation: 0,
            last_rendered_generation: None,
            debug_overlay: DebugOverlay::None,
            debug_revision: 0,
            surface_lost: None,
            context_lost: None,
            device_lost: None,
            hover_style: InteractionStyle::default(),
            selection_style: InteractionStyle::default(),
            environment: None,
            environment_lighting_cache: None,
            background_color: Color::BLACK,
            auto_exposure: None,
            last_auto_exposure: None,
            environment_revision: 0,
            target_revision: 0,
            prepare_telemetry: Default::default(),
            not_sync: PhantomData::<Cell<()>>,
        })
    }
}

fn resolve_quality(options: RendererOptions, capabilities: Capabilities) -> Quality {
    if let Some(quality) = options.explicit_quality() {
        return quality;
    }
    match options.profile() {
        Profile::Quality => Quality::High,
        Profile::Compatibility | Profile::Industrial => Quality::Low,
        Profile::Balanced => Quality::Medium,
        Profile::Auto => match capabilities.hardware_tier {
            HardwareTier::High => Quality::High,
            HardwareTier::Medium => Quality::Medium,
            HardwareTier::Low => Quality::Low,
        },
    }
}

fn resolve_render_mode(options: RendererOptions, profile: Profile) -> RenderMode {
    if let Some(render_mode) = options.explicit_render_mode() {
        return render_mode;
    }
    match profile {
        Profile::Industrial => RenderMode::OnChange,
        Profile::Auto | Profile::Quality | Profile::Balanced | Profile::Compatibility => {
            RenderMode::Manual
        }
    }
}
