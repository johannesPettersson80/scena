use std::cell::Cell;
use std::marker::PhantomData;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::diagnostics::{
    Backend, BuildError, Capabilities, HardwareTier, OutputColorSpace, RendererStats,
};
use crate::material::Color;
use crate::platform::{PlatformSurface, PlatformSurfaceAttachment};

use super::gpu;
use super::gpu::GpuDeviceState;
use super::{
    AntiAliasing, OutputTransform, Profile, Quality, RasterTarget, RenderMode, Renderer,
    RendererOptions, backend_for_attached_surface, validate_target_size,
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
        Self::headless_gpu_with_options(width, height, RendererOptions::default())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn headless_gpu_with_options(
        width: u32,
        height: u32,
        options: RendererOptions,
    ) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        let headless_gpu_test_guard = Some(HeadlessGpuTestSupportGuard::acquire());
        let gpu = pollster::block_on(gpu::request_headless_gpu(Backend::HeadlessGpu))?;
        let mut renderer = Self::from_raster_target(
            width,
            height,
            Backend::HeadlessGpu,
            Some(gpu),
            false,
            options,
        )?;
        renderer._headless_gpu_test_guard = headless_gpu_test_guard;
        Ok(renderer)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        Err(BuildError::UnsupportedBackend {
            backend: Backend::HeadlessGpu,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn headless_gpu_with_options(
        width: u32,
        height: u32,
        _options: RendererOptions,
    ) -> Result<Self, BuildError> {
        Self::headless_gpu(width, height)
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
                let size = gpu.surface_size().unwrap_or(size);
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
                    let gpu = gpu::request_browser_surface_gpu(
                        backend,
                        size,
                        canvas,
                        options.output_color_space(),
                    )
                    .await?;
                    let size = gpu.surface_size().unwrap_or(size);
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
                    let gpu = gpu::request_browser_surface_gpu(
                        backend,
                        size,
                        canvas,
                        options.output_color_space(),
                    )
                    .await?;
                    let size = gpu.surface_size().unwrap_or(size);
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
            let size = gpu.surface_size().unwrap_or(size);
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
        let output_color_space = options.output_color_space();
        let display_p3_configured = output_color_space == OutputColorSpace::DisplayP3
            && gpu
                .as_ref()
                .is_some_and(GpuDeviceState::display_p3_canvas_configured);
        let capabilities = if surface_attached {
            Capabilities::for_attached_gpu_backend(backend)
        } else if has_gpu {
            Capabilities::for_gpu_backend(backend)
        } else {
            Capabilities::for_backend(backend)
        }
        .with_display_p3_output(display_p3_configured);
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
            bloom_scratch: vec![0; target.byte_len()],
            oit_scratch: vec![super::cpu::OitAccumPixel::default(); target.pixel_len()],
            cpu_supersample_frame: Vec::new(),
            cpu_supersample_oit_scratch: Vec::new(),
            linear_frame: (!has_gpu).then(|| vec![Color::BLACK; target.pixel_len()]),
            depth_frame: (!has_gpu).then(|| vec![f32::INFINITY; target.pixel_len()]),
            cpu_supersample_linear_frame: Vec::new(),
            cpu_supersample_depth_frame: Vec::new(),
            cpu_material_reflection_scratch: Vec::new(),
            cpu_effect_rgba8_scratch: Vec::new(),
            cpu_row_band_bins: super::cpu_render::CpuRowBandBins::default(),
            gpu_supersample_frame: Vec::new(),
            stats: RendererStats {
                target_width: width,
                target_height: height,
                ..RendererStats::default()
            },
            diagnostics: Vec::new(),
            capabilities,
            gpu,
            output: OutputTransform::default(),
            anti_aliasing: anti_aliasing_for_quality(quality),
            cpu_occlusion_culling: true,
            semantic_aov_capture_enabled: options.semantic_aov_capture(),
            supersample_factor: 1,
            reconstruction_filter: super::ReconstructionFilter::Box,
            order_independent_transparency: None,
            screen_space_ambient_occlusion: None,
            screen_space_reflections: None,
            depth_of_field: None,
            bloom: None,
            profile,
            quality,
            render_mode,
            output_color_space,
            render_generation: 0,
            last_rendered_generation: None,
            last_rendered_frame: None,
            last_readback_frame: None,
            surface_lost: None,
            context_lost: None,
            device_lost: None,
            environment: None,
            environment_lighting_cache: Default::default(),
            background_color: Color::BLACK,
            auto_exposure: None,
            last_auto_exposure: None,
            environment_revision: 0,
            target_revision: 0,
            output_resources_revision: 0,
            prepare_telemetry: Default::default(),
            last_render_work_metrics: super::RenderWorkMetrics::default(),
            #[cfg(test)]
            depth_prepass_enabled_for_test: true,
            #[cfg(not(target_arch = "wasm32"))]
            _headless_gpu_test_guard: None,
            not_sync: PhantomData::<Cell<()>>,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
static HEADLESS_GPU_TEST_SUPPORT_SLOT: AtomicBool = AtomicBool::new(false);

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static HEADLESS_GPU_TEST_SUPPORT_DEPTH: Cell<usize> = const { Cell::new(0) };
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(super) struct HeadlessGpuTestSupportGuard {
    owns_slot: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl HeadlessGpuTestSupportGuard {
    fn acquire() -> Self {
        if HEADLESS_GPU_TEST_SUPPORT_DEPTH.with(|depth| {
            let current = depth.get();
            if current > 0 {
                depth.set(current + 1);
                true
            } else {
                false
            }
        }) {
            return Self { owns_slot: false };
        }

        loop {
            if HEADLESS_GPU_TEST_SUPPORT_SLOT
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                HEADLESS_GPU_TEST_SUPPORT_DEPTH.with(|depth| depth.set(1));
                return Self { owns_slot: true };
            }
            std::thread::yield_now();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for HeadlessGpuTestSupportGuard {
    fn drop(&mut self) {
        HEADLESS_GPU_TEST_SUPPORT_DEPTH.with(|depth| {
            let current = depth.get();
            if current > 0 {
                depth.set(current - 1);
            }
        });
        if self.owns_slot {
            HEADLESS_GPU_TEST_SUPPORT_SLOT.store(false, Ordering::Release);
        }
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

fn anti_aliasing_for_quality(quality: Quality) -> AntiAliasing {
    match quality {
        Quality::Low => AntiAliasing::None,
        Quality::Medium => AntiAliasing::Fxaa,
        Quality::High => AntiAliasing::Msaa4,
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
