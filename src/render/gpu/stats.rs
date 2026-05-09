use super::output;
#[cfg(not(target_arch = "wasm32"))]
use super::pipeline::BYTES_PER_PIXEL;
use super::vertices::VERTEX_BYTE_LEN;

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

#[derive(Debug, Clone, Copy)]
pub(super) struct PreparedResourceEstimateInput {
    pub(super) target: RasterTarget,
    pub(super) vertex_count: usize,
    pub(super) has_surface_pipeline: bool,
    pub(super) shadow_maps: u64,
    pub(super) shadow_map_resolution: Option<u32>,
    pub(super) depth_prepass_passes: u64,
    pub(super) material_texture_count: u64,
    pub(super) material_texture_bytes: u64,
}

impl GpuResourceStats {
    pub(in crate::render) fn destruction_records(self) -> u64 {
        self.buffers
            + self.textures
            + self.render_targets
            + self.pipelines
            + self.bind_groups
            + self.shader_modules
    }
}

pub(super) fn estimate_prepared_resource_stats(
    input: PreparedResourceEstimateInput,
) -> GpuResourceStats {
    let PreparedResourceEstimateInput {
        target,
        vertex_count,
        has_surface_pipeline,
        shadow_maps,
        shadow_map_resolution,
        depth_prepass_passes,
        material_texture_count,
        material_texture_bytes,
    } = input;

    if vertex_count == 0 {
        return GpuResourceStats::default();
    }

    #[cfg(not(target_arch = "wasm32"))]
    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    #[cfg(not(target_arch = "wasm32"))]
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    #[cfg(not(target_arch = "wasm32"))]
    let texture_bytes = u64::from(unpadded_bytes_per_row) * u64::from(target.height);
    #[cfg(not(target_arch = "wasm32"))]
    let readback_bytes = u64::from(padded_bytes_per_row) * u64::from(target.height);
    let vertex_bytes = (vertex_count * VERTEX_BYTE_LEN).max(4) as u64;
    let uniform_bytes = output::OUTPUT_UNIFORM_BYTE_LEN;
    let pipelines = 1 + u64::from(has_surface_pipeline) + depth_prepass_passes;
    #[cfg(not(target_arch = "wasm32"))]
    let shadow_map_bytes = shadow_map_resolution
        .map(|resolution| {
            let edge = u64::from(resolution);
            shadow_maps.saturating_mul(edge.saturating_mul(edge).saturating_mul(4))
        })
        .unwrap_or(0);
    #[cfg(target_arch = "wasm32")]
    let shadow_map_bytes = {
        let _ = shadow_map_resolution;
        let _ = shadow_maps;
        0
    };
    #[cfg(not(target_arch = "wasm32"))]
    let depth_prepass_bytes = u64::from(target.width)
        .saturating_mul(u64::from(target.height))
        .saturating_mul(4)
        .saturating_mul(depth_prepass_passes);
    #[cfg(target_arch = "wasm32")]
    let depth_prepass_bytes = {
        let _ = target;
        0
    };

    GpuResourceStats {
        #[cfg(not(target_arch = "wasm32"))]
        buffers: 3,
        #[cfg(target_arch = "wasm32")]
        buffers: 2,
        #[cfg(not(target_arch = "wasm32"))]
        textures: 1 + material_texture_count + shadow_maps + depth_prepass_passes,
        #[cfg(target_arch = "wasm32")]
        textures: material_texture_count,
        #[cfg(not(target_arch = "wasm32"))]
        render_targets: 1 + shadow_maps + depth_prepass_passes,
        #[cfg(target_arch = "wasm32")]
        render_targets: 1,
        pipelines,
        bind_groups: 1 + material_texture_count,
        shader_modules: pipelines,
        #[cfg(not(target_arch = "wasm32"))]
        approximate_gpu_memory_bytes: texture_bytes
            + readback_bytes
            + vertex_bytes
            + uniform_bytes
            + material_texture_bytes
            + shadow_map_bytes
            + depth_prepass_bytes,
        #[cfg(target_arch = "wasm32")]
        approximate_gpu_memory_bytes: vertex_bytes
            + uniform_bytes
            + material_texture_bytes
            + shadow_map_bytes
            + depth_prepass_bytes,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
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

        let stats = estimate_prepared_resource_stats(estimate_input(target, 3));

        assert_eq!(stats.buffers, 3);
        assert_eq!(stats.textures, 2);
        assert_eq!(stats.render_targets, 1);
        assert_eq!(stats.pipelines, 1);
        assert_eq!(stats.bind_groups, 2);
        assert_eq!(stats.shader_modules, 1);
        assert_eq!(stats.destruction_records(), 10);
        assert!(stats.approximate_gpu_memory_bytes > 0);
    }

    #[test]
    fn estimates_empty_headless_gpu_resource_counters_at_baseline() {
        let target = RasterTarget {
            width: 4,
            height: 4,
            backend: Backend::HeadlessGpu,
        };

        let stats = estimate_prepared_resource_stats(estimate_input(target, 0));

        assert_eq!(stats, GpuResourceStats::default());
        assert_eq!(stats.destruction_records(), 0);
    }

    #[test]
    fn estimates_single_shadow_map_resource_counters() {
        let target = RasterTarget {
            width: 4,
            height: 4,
            backend: Backend::HeadlessGpu,
        };

        let stats = estimate_prepared_resource_stats(PreparedResourceEstimateInput {
            shadow_maps: 1,
            shadow_map_resolution: Some(2048),
            ..estimate_input(target, 3)
        });

        assert_eq!(stats.textures, 3);
        assert_eq!(stats.render_targets, 2);
        assert_eq!(stats.destruction_records(), 12);
        assert!(stats.approximate_gpu_memory_bytes >= 2048 * 2048 * 4);
    }

    #[test]
    fn estimates_depth_prepass_resource_counters() {
        let target = RasterTarget {
            width: 4,
            height: 4,
            backend: Backend::HeadlessGpu,
        };

        let stats = estimate_prepared_resource_stats(PreparedResourceEstimateInput {
            depth_prepass_passes: 1,
            ..estimate_input(target, 3)
        });

        assert_eq!(stats.textures, 3);
        assert_eq!(stats.render_targets, 2);
        assert_eq!(stats.pipelines, 2);
        assert_eq!(stats.shader_modules, 2);
        assert_eq!(stats.destruction_records(), 14);
        assert!(stats.approximate_gpu_memory_bytes >= 4 * 4 * 4);
    }

    fn estimate_input(target: RasterTarget, vertex_count: usize) -> PreparedResourceEstimateInput {
        PreparedResourceEstimateInput {
            target,
            vertex_count,
            has_surface_pipeline: false,
            shadow_maps: 0,
            shadow_map_resolution: None,
            depth_prepass_passes: 0,
            material_texture_count: 1,
            material_texture_bytes: 4,
        }
    }
}
