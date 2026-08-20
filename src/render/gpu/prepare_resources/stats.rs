use crate::render::RasterTarget;

use super::super::MsaaColorResources;
use super::super::output;
use super::super::stats::GpuResourceStats;

/// The fixed part of a preparation allocation report. Optional subsystem
/// resources are added by the prepare owner after each subsystem is built.
#[allow(clippy::too_many_arguments)]
pub(super) fn base_resource_stats(
    target: RasterTarget,
    vertex_buffer_size: u64,
    instance_buffer_size: u64,
    padded_bytes_per_row: u32,
    has_surface_output_uniform: bool,
    has_msaa8_pipelines: bool,
    has_surface_pipeline: bool,
    triangle_shader_cache_hit: bool,
    draw_uniform_capacity: usize,
) -> GpuResourceStats {
    let texture_bytes = GpuResourceStats::target_bytes(target, 4, 1);
    GpuResourceStats {
        buffers: 6 + u64::from(has_surface_output_uniform),
        textures: 1,
        render_targets: 1,
        pipelines: 4 + u64::from(has_msaa8_pipelines) * 2 + u64::from(has_surface_pipeline) * 2,
        bind_groups: 1 + u64::from(has_surface_output_uniform) * 2,
        shader_modules: 1,
        shader_module_creations: u64::from(!triangle_shader_cache_hit),
        approximate_gpu_memory_bytes: vertex_buffer_size
            .saturating_add(instance_buffer_size)
            .saturating_add(
                u64::from(padded_bytes_per_row)
                    .saturating_mul(u64::from(target.height))
                    .saturating_mul(2),
            )
            .saturating_add(output::OUTPUT_UNIFORM_BYTE_LEN)
            .saturating_add(
                u64::from(has_surface_output_uniform)
                    .saturating_mul(output::OUTPUT_UNIFORM_BYTE_LEN),
            )
            .saturating_add(
                output::DRAW_UNIFORM_ENTRY_STRIDE
                    .saturating_mul((draw_uniform_capacity as u64).max(1)),
            )
            .saturating_add(texture_bytes),
        ..GpuResourceStats::default()
    }
}

pub(super) fn add_msaa_resource_stats(
    stats: &mut GpuResourceStats,
    resources: &MsaaColorResources,
) {
    stats.add_assign(GpuResourceStats {
        textures: 1,
        render_targets: 1,
        approximate_gpu_memory_bytes: GpuResourceStats::target_bytes(
            resources.target,
            4,
            resources.sample_count,
        ),
        ..GpuResourceStats::default()
    });
}
