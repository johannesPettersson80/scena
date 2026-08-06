use super::*;

const CPU_PARALLEL_MIN_PIXELS: usize = 512 * 512;
const CPU_PARALLEL_MIN_PRIMITIVES: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct CpuPrimitiveFlags {
    pub(super) has_physical_transmission: bool,
}

impl CpuPrimitiveFlags {
    pub(super) fn scan(primitives: &[PreparedPrimitive]) -> Self {
        Self {
            has_physical_transmission: primitives
                .iter()
                .any(cpu::primitive_needs_physical_transmission),
        }
    }
}

pub(super) fn should_parallelize_cpu_geometry_pass(
    input: &CpuGeometryPass<'_>,
    primitive_flags: CpuPrimitiveFlags,
) -> bool {
    // Screen-space reflections used to force this to false. They no longer do:
    // recording is row-separable and the resolve happens once, after the bands
    // are joined. Physical transmission still needs the finished frame *during*
    // rasterization, so it stays serial.
    !primitive_flags.has_physical_transmission
        && input.primitives.len() >= CPU_PARALLEL_MIN_PRIMITIVES
        && input.target.pixel_len() >= CPU_PARALLEL_MIN_PIXELS
        && cpu_geometry_worker_count(input.target) > 1
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cpu_geometry_worker_count(target: RasterTarget) -> usize {
    super::super::parallel::worker_count(target.height as usize)
        .min(target.height as usize)
        .max(1)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn cpu_geometry_worker_count(_target: RasterTarget) -> usize {
    1
}
