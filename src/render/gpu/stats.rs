#[cfg(not(target_arch = "wasm32"))]
use super::{BYTES_PER_PIXEL, VERTEX_BYTE_LEN, align_to, output};

#[cfg(not(target_arch = "wasm32"))]
use super::super::RasterTarget;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct GpuResourceStats {
    pub(in crate::render) buffers: u64,
    pub(in crate::render) textures: u64,
    pub(in crate::render) render_targets: u64,
    pub(in crate::render) pipelines: u64,
    pub(in crate::render) bind_groups: u64,
    pub(in crate::render) shader_modules: u64,
    pub(in crate::render) approximate_gpu_memory_bytes: u64,
}

impl GpuResourceStats {
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::render) fn destruction_records(self) -> u64 {
        self.buffers
            + self.textures
            + self.render_targets
            + self.pipelines
            + self.bind_groups
            + self.shader_modules
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn estimate_prepared_resource_stats(
    target: RasterTarget,
    vertex_count: usize,
    has_surface_pipeline: bool,
) -> GpuResourceStats {
    if vertex_count == 0 {
        return GpuResourceStats::default();
    }

    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let texture_bytes = u64::from(unpadded_bytes_per_row) * u64::from(target.height);
    let readback_bytes = u64::from(padded_bytes_per_row) * u64::from(target.height);
    let vertex_bytes = (vertex_count * VERTEX_BYTE_LEN).max(4) as u64;
    let uniform_bytes = output::OUTPUT_UNIFORM_BYTE_LEN;
    let pipelines = 1 + u64::from(has_surface_pipeline);

    GpuResourceStats {
        buffers: 3,
        textures: 1,
        render_targets: 1,
        pipelines,
        bind_groups: 1,
        shader_modules: pipelines,
        approximate_gpu_memory_bytes: texture_bytes + readback_bytes + vertex_bytes + uniform_bytes,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;

    #[test]
    fn estimates_prepared_headless_gpu_resource_counters() {
        let target = RasterTarget {
            width: 4,
            height: 4,
            backend: Backend::HeadlessGpu,
        };

        let stats = estimate_prepared_resource_stats(target, 3, false);

        assert_eq!(stats.buffers, 3);
        assert_eq!(stats.textures, 1);
        assert_eq!(stats.render_targets, 1);
        assert_eq!(stats.pipelines, 1);
        assert_eq!(stats.bind_groups, 1);
        assert_eq!(stats.shader_modules, 1);
        assert_eq!(stats.destruction_records(), 8);
        assert!(stats.approximate_gpu_memory_bytes > 0);
    }

    #[test]
    fn estimates_empty_headless_gpu_resource_counters_at_baseline() {
        let target = RasterTarget {
            width: 4,
            height: 4,
            backend: Backend::HeadlessGpu,
        };

        let stats = estimate_prepared_resource_stats(target, 0, false);

        assert_eq!(stats, GpuResourceStats::default());
        assert_eq!(stats.destruction_records(), 0);
    }
}
