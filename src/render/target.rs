use crate::diagnostics::{Backend, RenderError};
use crate::platform::SurfaceKind;

pub(super) const MAX_FULL_FRAME_SUPERSAMPLE_FACTOR: u32 = 8;
const MAX_FULL_FRAME_SUPERSAMPLE_DIMENSION: u32 = 16_384;
const MAX_FULL_FRAME_SUPERSAMPLE_PIXELS: u64 = 134_217_728;
const V3D_HEADLESS_TARGET_WIDTH_ALIGNMENT: u32 = 64;
const V3D_HEADLESS_TARGET_ALIGNMENT_MIN_WIDTH: u32 = 1024;

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

pub(super) fn v3d_headless_render_target(
    target: RasterTarget,
    workaround_required: bool,
) -> RasterTarget {
    if !workaround_required
        || target.backend != Backend::HeadlessGpu
        || target.width < V3D_HEADLESS_TARGET_ALIGNMENT_MIN_WIDTH
        || target
            .width
            .is_multiple_of(V3D_HEADLESS_TARGET_WIDTH_ALIGNMENT)
    {
        return target;
    }
    let width = target.width - target.width % V3D_HEADLESS_TARGET_WIDTH_ALIGNMENT;
    let height = ((u64::from(target.height) * u64::from(width) + u64::from(target.width) / 2)
        / u64::from(target.width))
    .max(1) as u32;
    RasterTarget {
        width,
        height,
        backend: target.backend,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v3d_headless_render_target_aligns_large_unaligned_width_preserving_aspect() {
        let logical = RasterTarget {
            width: 2700,
            height: 1725,
            backend: Backend::HeadlessGpu,
        };
        assert_eq!(
            v3d_headless_render_target(logical, true),
            RasterTarget {
                width: 2688,
                height: 1717,
                backend: Backend::HeadlessGpu,
            }
        );
    }

    #[test]
    fn v3d_headless_render_target_leaves_safe_or_unaffected_targets_unchanged() {
        for (target, required) in [
            (
                RasterTarget {
                    width: 2560,
                    height: 1680,
                    backend: Backend::HeadlessGpu,
                },
                true,
            ),
            (
                RasterTarget {
                    width: 160,
                    height: 105,
                    backend: Backend::HeadlessGpu,
                },
                true,
            ),
            (
                RasterTarget {
                    width: 2700,
                    height: 1725,
                    backend: Backend::HeadlessGpu,
                },
                false,
            ),
        ] {
            assert_eq!(v3d_headless_render_target(target, required), target);
        }
    }
}
