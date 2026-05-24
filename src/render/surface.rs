use crate::assets::{Assets, RetainPolicy};
use crate::diagnostics::{Backend, BuildError, Capabilities, PrepareError, RenderError};
use crate::material::Color;
use crate::platform::{PlatformSurface, PlatformSurfaceAttachment, SurfaceEvent};
use crate::scene::Scene;

#[cfg(not(target_arch = "wasm32"))]
use super::gpu;
use super::{RasterTarget, Renderer, backend_for_attached_surface, validate_target_size};

impl Renderer {
    pub fn handle_surface_event(&mut self, event: SurfaceEvent) -> Result<(), RenderError> {
        match event {
            SurfaceEvent::Resize { width, height } => {
                self.resize_target(width, height)?;
            }
            SurfaceEvent::ViewportChanged(viewport) => {
                let size = viewport.physical_size();
                self.resize_target(size.width, size.height)?;
            }
            SurfaceEvent::ScaleFactorChanged { .. } | SurfaceEvent::Occluded { .. } => {
                self.target_revision = self.target_revision.saturating_add(1);
            }
            SurfaceEvent::Hidden | SurfaceEvent::Shown => {}
            SurfaceEvent::Lost => {
                self.surface_lost = Some(true);
                self.target_revision = self.target_revision.saturating_add(1);
            }
            SurfaceEvent::ContextLost { recoverable } => {
                self.context_lost = Some(recoverable);
                self.target_revision = self.target_revision.saturating_add(1);
            }
            SurfaceEvent::ContextRestored => {
                if self.context_lost == Some(true) {
                    self.context_lost = None;
                    self.target_revision = self.target_revision.saturating_add(1);
                }
            }
            SurfaceEvent::DeviceLost { recoverable } => {
                self.device_lost = Some(recoverable);
                self.target_revision = self.target_revision.saturating_add(1);
            }
        }
        Ok(())
    }

    pub fn recover_surface(&mut self, surface: PlatformSurface) -> Result<(), BuildError> {
        let (kind, size, attachment) = surface.into_parts();
        let (backend, gpu, attached, size) = match attachment {
            PlatformSurfaceAttachment::Descriptor => {
                (Backend::SurfaceDescriptor, None, false, size)
            }
            #[cfg(not(target_arch = "wasm32"))]
            PlatformSurfaceAttachment::NativeWindow(window) => {
                let backend = backend_for_attached_surface(kind);
                let gpu =
                    pollster::block_on(gpu::request_native_surface_gpu(backend, size, window))?;
                let size = gpu.surface_size().unwrap_or(size);
                (backend, Some(gpu), true, size)
            }
            #[cfg(target_arch = "wasm32")]
            PlatformSurfaceAttachment::BrowserWebGpuCanvas(_)
            | PlatformSurfaceAttachment::BrowserWebGl2Canvas(_) => {
                let backend = backend_for_attached_surface(kind);
                return Err(BuildError::AsyncSurfaceRequired { backend });
            }
        };
        validate_target_size(size.width, size.height).map_err(|()| {
            BuildError::InvalidTargetSize {
                width: size.width,
                height: size.height,
            }
        })?;

        self.target = RasterTarget {
            width: size.width,
            height: size.height,
            backend,
        };
        self.frame.resize(self.target.byte_len(), 0);
        self.fxaa_scratch.resize(self.target.byte_len(), 0);
        self.bloom_scratch.resize(self.target.byte_len(), 0);
        self.oit_scratch.resize(
            self.target.pixel_len(),
            super::cpu::OitAccumPixel::default(),
        );
        if let Some(linear_frame) = &mut self.linear_frame {
            linear_frame.resize(self.target.pixel_len(), Color::BLACK);
        }
        if let Some(depth_frame) = &mut self.depth_frame {
            depth_frame.resize(self.target.pixel_len(), f32::INFINITY);
        }
        if gpu.is_some() && self.linear_frame.is_some() {
            self.linear_frame = None;
            self.depth_frame = None;
        } else if gpu.is_none() && self.linear_frame.is_none() {
            self.linear_frame = Some(vec![Color::BLACK; self.target.pixel_len()]);
            self.depth_frame = Some(vec![f32::INFINITY; self.target.pixel_len()]);
        }
        self.gpu = gpu;
        self.capabilities = if attached {
            Capabilities::for_attached_gpu_backend(backend)
        } else if self.gpu.is_some() {
            Capabilities::for_gpu_backend(backend)
        } else {
            Capabilities::for_backend(backend)
        }
        .with_display_p3_output(false);
        self.stats.target_width = size.width;
        self.stats.target_height = size.height;
        self.surface_lost = None;
        self.target_revision = self.target_revision.saturating_add(1);
        self.prepared = None;
        self.last_rendered_generation = None;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn attach_surface_async(
        &mut self,
        surface: PlatformSurface,
    ) -> Result<(), BuildError> {
        let (kind, size, attachment) = surface.into_parts();
        match attachment {
            PlatformSurfaceAttachment::BrowserWebGpuCanvas(canvas)
            | PlatformSurfaceAttachment::BrowserWebGl2Canvas(canvas) => {
                let backend = backend_for_attached_surface(kind);
                if self.target.backend != backend {
                    return Err(BuildError::UnsupportedBackend { backend });
                }
                let gpu = self
                    .gpu
                    .as_mut()
                    .ok_or(BuildError::UnsupportedBackend { backend })?;
                let size = gpu.attach_browser_surface(backend, size, canvas)?;
                let display_p3_canvas_configured = gpu.display_p3_canvas_configured();
                if self.target.width != size.width || self.target.height != size.height {
                    self.resize_target(size.width, size.height).map_err(|_| {
                        BuildError::InvalidTargetSize {
                            width: size.width,
                            height: size.height,
                        }
                    })?;
                }
                self.capabilities = Capabilities::for_attached_gpu_backend(backend)
                    .with_display_p3_output(display_p3_canvas_configured);
                self.surface_lost = None;
                self.last_rendered_generation = None;
                Ok(())
            }
            PlatformSurfaceAttachment::Descriptor => self.recover_surface(
                PlatformSurface::browser_webgl2_canvas(size.width, size.height),
            ),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn attach_surface_async(
        &mut self,
        surface: PlatformSurface,
    ) -> Result<(), BuildError> {
        self.recover_surface(surface)
    }

    pub fn release_surface(&mut self) {
        if let Some(gpu) = &mut self.gpu {
            gpu.release_surface();
        }
        self.last_rendered_generation = None;
    }

    pub fn recover_context<F>(
        &mut self,
        assets: &Assets<F>,
        _scene: &mut Scene,
    ) -> Result<(), PrepareError> {
        if assets.retain_policy() == RetainPolicy::Never {
            return Err(PrepareError::BackendCapabilityMismatch {
                feature: "context recovery",
                backend: self.target.backend,
                help: "Assets uses RetainPolicy::Never; recreate assets or retain CPU data for recovery"
                    .to_string(),
            });
        }
        match self.context_lost.or(self.device_lost) {
            Some(false) => Err(PrepareError::BackendCapabilityMismatch {
                feature: "context recovery",
                backend: self.target.backend,
                help: "the host reported the GPU context as unrecoverable; rebuild Renderer"
                    .to_string(),
            }),
            Some(true) | None => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.clear_prepared_resources_for_context_recovery();
                }
                self.context_lost = None;
                self.device_lost = None;
                self.target_revision = self.target_revision.saturating_add(1);
                self.prepared = None;
                self.last_rendered_generation = None;
                Ok(())
            }
        }
    }

    pub(super) fn resize_target(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        validate_target_size(width, height)
            .map_err(|()| RenderError::InvalidSurfaceSize { width, height })?;
        let size = if let Some(gpu) = &self.gpu {
            gpu.clamp_surface_size_to_device_limits(crate::platform::SurfaceSize { width, height })
        } else {
            crate::platform::SurfaceSize { width, height }
        };
        let width = size.width;
        let height = size.height;
        if self.target.width == width && self.target.height == height {
            return Ok(());
        }
        self.target.width = width;
        self.target.height = height;
        self.frame.resize(self.target.byte_len(), 0);
        self.fxaa_scratch.resize(self.target.byte_len(), 0);
        self.bloom_scratch.resize(self.target.byte_len(), 0);
        self.oit_scratch.resize(
            self.target.pixel_len(),
            super::cpu::OitAccumPixel::default(),
        );
        if let Some(linear_frame) = &mut self.linear_frame {
            linear_frame.resize(self.target.pixel_len(), Color::BLACK);
        }
        if let Some(depth_frame) = &mut self.depth_frame {
            depth_frame.resize(self.target.pixel_len(), f32::INFINITY);
        }
        self.stats.target_width = width;
        self.stats.target_height = height;
        self.target_revision = self.target_revision.saturating_add(1);
        Ok(())
    }

    pub(super) fn loss_error(&self) -> Result<(), RenderError> {
        if let Some(recoverable) = self.surface_lost {
            return Err(RenderError::SurfaceLost { recoverable });
        }
        if let Some(recoverable) = self.context_lost {
            return Err(RenderError::ContextLost { recoverable });
        }
        if let Some(recoverable) = self.device_lost {
            return Err(RenderError::GpuDeviceLost { recoverable });
        }
        Ok(())
    }
}
