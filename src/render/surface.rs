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
        let (backend, gpu, attached) = match attachment {
            PlatformSurfaceAttachment::Descriptor => (Backend::SurfaceDescriptor, None, false),
            #[cfg(not(target_arch = "wasm32"))]
            PlatformSurfaceAttachment::NativeWindow(window) => {
                let backend = backend_for_attached_surface(kind);
                let gpu =
                    pollster::block_on(gpu::request_native_surface_gpu(backend, size, window))?;
                (backend, Some(gpu), true)
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
        if let Some(linear_frame) = &mut self.linear_frame {
            linear_frame.resize(self.target.pixel_len(), Color::BLACK);
        }
        if gpu.is_some() && self.linear_frame.is_some() {
            self.linear_frame = None;
        } else if gpu.is_none() && self.linear_frame.is_none() {
            self.linear_frame = Some(vec![Color::BLACK; self.target.pixel_len()]);
        }
        self.gpu = gpu;
        self.capabilities = if attached {
            Capabilities::for_attached_gpu_backend(backend)
        } else if self.gpu.is_some() {
            Capabilities::for_gpu_backend(backend)
        } else {
            Capabilities::for_backend(backend)
        };
        self.stats.target_width = size.width;
        self.stats.target_height = size.height;
        self.surface_lost = None;
        self.target_revision = self.target_revision.saturating_add(1);
        self.prepared = None;
        self.last_rendered_generation = None;
        Ok(())
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
        self.target.width = width;
        self.target.height = height;
        self.frame.resize(self.target.byte_len(), 0);
        self.fxaa_scratch.resize(self.target.byte_len(), 0);
        if let Some(linear_frame) = &mut self.linear_frame {
            linear_frame.resize(self.target.pixel_len(), Color::BLACK);
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
