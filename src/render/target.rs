use crate::diagnostics::{Backend, RenderError};
use crate::platform::SurfaceKind;

pub(super) const MAX_FULL_FRAME_SUPERSAMPLE_FACTOR: u32 = 8;
const MAX_FULL_FRAME_SUPERSAMPLE_DIMENSION: u32 = 16_384;
const MAX_FULL_FRAME_SUPERSAMPLE_PIXELS: u64 = 134_217_728;

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

pub(super) fn validate_supersample_target(
    target: RasterTarget,
    factor: u32,
) -> Result<RasterTarget, RenderError> {
    let scaled = target
        .scaled(factor)
        .ok_or_else(|| RenderError::InvalidSurfaceSize {
            width: target.width.saturating_mul(factor),
            height: target.height.saturating_mul(factor),
        })?;
    let pixels = u64::from(scaled.width) * u64::from(scaled.height);
    if !matches!(factor, 1 | 2 | 3 | 4 | MAX_FULL_FRAME_SUPERSAMPLE_FACTOR)
        || scaled.width > MAX_FULL_FRAME_SUPERSAMPLE_DIMENSION
        || scaled.height > MAX_FULL_FRAME_SUPERSAMPLE_DIMENSION
        || pixels > MAX_FULL_FRAME_SUPERSAMPLE_PIXELS
    {
        return Err(RenderError::UnsupportedSupersampleFactor {
            factor,
            width: target.width,
            height: target.height,
            scaled_width: scaled.width,
            scaled_height: scaled.height,
            maximum_dimension: MAX_FULL_FRAME_SUPERSAMPLE_DIMENSION,
            maximum_pixels: MAX_FULL_FRAME_SUPERSAMPLE_PIXELS,
        });
    }
    Ok(scaled)
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
