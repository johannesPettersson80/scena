use super::instancing::INSTANCE_BYTE_LEN;
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
    /// Plan line 778 commit 2: distinct material bind groups consumed by
    /// the unlit pass. Equals 1 when the renderer chose the batched
    /// `texture_2d_array<f32>` path (single shared bind group serviced via
    /// dynamic-offset uniforms) and equals the per-material slot count
    /// otherwise (one bind group per slot, including the synthetic
    /// fallback at index 0).
    pub(in crate::render) material_bind_groups: u32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PreparedResourceEstimateInput {
    pub(super) target: RasterTarget,
    pub(super) vertex_count: usize,
    pub(super) instance_capacity: usize,
    pub(super) has_surface_pipeline: bool,
    pub(super) shadow_maps: u64,
    pub(super) shadow_map_resolution: Option<u32>,
    pub(super) depth_prepass_passes: u64,
    pub(super) material_texture_count: u64,
    pub(super) material_texture_bytes: u64,
    pub(super) light_assignment_bytes: u64,
    /// Plan line 778 commit 2: distinct material bind groups in the
    /// prepared resource set. The estimator records this so the
    /// observable `RendererStats::material_bind_groups` reflects the
    /// actual GPU shape rather than the per-material count.
    pub(super) material_bind_groups: u32,
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
        instance_capacity,
        has_surface_pipeline,
        shadow_maps,
        shadow_map_resolution,
        depth_prepass_passes,
        material_texture_count,
        material_texture_bytes,
        light_assignment_bytes,
        material_bind_groups,
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
    let instance_bytes = (instance_capacity * INSTANCE_BYTE_LEN).max(4) as u64;
    let uniform_bytes = output::OUTPUT_UNIFORM_BYTE_LEN;
    let transmission_textures = 2;
    let transmission_render_targets = 1;
    let mesh_pipeline_sets = 1 + u64::from(has_surface_pipeline);
    let mesh_pipelines = mesh_pipeline_sets * 2;
    let transmission_pipelines = 2;
    let shadow_caster_pipelines = 1;
    let depth_prepass_pipelines = depth_prepass_passes.saturating_mul(2);
    let pipelines =
        mesh_pipelines + transmission_pipelines + shadow_caster_pipelines + depth_prepass_pipelines;
    let mesh_shader_modules = mesh_pipelines;
    let transmission_shader_modules = transmission_pipelines;
    let shadow_caster_shader_modules = 1;
    let depth_prepass_shader_modules = u64::from(depth_prepass_passes > 0);
    let shader_modules = mesh_shader_modules
        + transmission_shader_modules
        + shadow_caster_shader_modules
        + depth_prepass_shader_modules;
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
        buffers: 7,
        #[cfg(target_arch = "wasm32")]
        buffers: 6,
        // textures: 1 + material_texture_count + shadow_maps + depth_prepass_passes + transmission_textures
        #[cfg(not(target_arch = "wasm32"))]
        textures: 1
            + material_texture_count
            + shadow_maps
            + depth_prepass_passes
            + transmission_textures,
        #[cfg(target_arch = "wasm32")]
        textures: material_texture_count + transmission_textures,
        // render_targets: 1 + shadow_maps + depth_prepass_passes + transmission_render_targets
        #[cfg(not(target_arch = "wasm32"))]
        render_targets: 1 + shadow_maps + depth_prepass_passes + transmission_render_targets,
        #[cfg(target_arch = "wasm32")]
        render_targets: 1 + transmission_render_targets,
        pipelines,
        // Plan line 778 commit 2: the unlit pass binds 1 output bind group
        // + N material bind groups (1 when batched, slot count otherwise).
        // Adding `material_bind_groups` keeps the resource estimate
        // consistent with the actual GPU shape.
        bind_groups: 2 + u64::from(material_bind_groups),
        shader_modules,
        material_bind_groups,
        #[cfg(not(target_arch = "wasm32"))]
        approximate_gpu_memory_bytes: texture_bytes
            + readback_bytes
            + vertex_bytes
            + instance_bytes
            + uniform_bytes
            + light_assignment_bytes
            + material_texture_bytes
            + shadow_map_bytes
            + depth_prepass_bytes
            + texture_bytes,
        #[cfg(target_arch = "wasm32")]
        approximate_gpu_memory_bytes: vertex_bytes
            + instance_bytes
            + uniform_bytes
            + light_assignment_bytes
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

        assert_eq!(stats.buffers, 7);
        assert_eq!(stats.textures, 4);
        assert_eq!(stats.render_targets, 2);
        assert_eq!(stats.pipelines, 5);
        assert_eq!(stats.bind_groups, 3);
        assert_eq!(stats.shader_modules, 5);
        assert_eq!(stats.destruction_records(), 26);
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

        assert_eq!(stats.textures, 5);
        assert_eq!(stats.render_targets, 3);
        assert_eq!(stats.destruction_records(), 28);
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

        assert_eq!(stats.textures, 5);
        assert_eq!(stats.render_targets, 3);
        assert_eq!(stats.pipelines, 7);
        assert_eq!(stats.shader_modules, 6);
        assert_eq!(stats.destruction_records(), 31);
        assert!(stats.approximate_gpu_memory_bytes >= 4 * 4 * 4);
    }

    fn estimate_input(target: RasterTarget, vertex_count: usize) -> PreparedResourceEstimateInput {
        PreparedResourceEstimateInput {
            target,
            vertex_count,
            instance_capacity: 1,
            has_surface_pipeline: false,
            shadow_maps: 0,
            shadow_map_resolution: None,
            depth_prepass_passes: 0,
            material_texture_count: 1,
            material_texture_bytes: 4,
            light_assignment_bytes: 84,
            material_bind_groups: 1,
        }
    }
}
