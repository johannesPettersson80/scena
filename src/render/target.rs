use crate::diagnostics::Backend;
use crate::platform::SurfaceKind;

/// Row-major render target dimensions used for CPU frame and accumulator indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RasterTarget {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) backend: Backend,
}

impl RasterTarget {
    pub(super) fn pixel_len(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    pub(super) fn byte_len(self) -> usize {
        self.pixel_len() * 4
    }

    pub(super) fn pixel_index(self, x: u32, y: u32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }

    pub(super) fn scaled(self, scale: u32) -> Option<Self> {
        Some(Self {
            width: self.width.checked_mul(scale)?,
            height: self.height.checked_mul(scale)?,
            backend: self.backend,
        })
    }
}

pub(super) fn backend_for_attached_surface(kind: SurfaceKind) -> Backend {
    match kind {
        SurfaceKind::NativeWindow => Backend::NativeSurface,
        SurfaceKind::BrowserWebGpuCanvas => Backend::WebGpu,
        SurfaceKind::BrowserWebGl2Canvas => Backend::WebGl2,
    }
}

pub(super) fn validate_target_size(width: u32, height: u32) -> Result<(), ()> {
    if width == 0 || height == 0 {
        Err(())
    } else {
        Ok(())
    }
}
