use super::gpu;

/// Deterministic GPU synchronization and copy counters from the last render.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderWorkMetrics {
    pub prepared_primitive_list_clones: u64,
    pub prepared_stroke_list_clones: u64,
    pub prepared_label_list_clones: u64,
    pub prepared_list_clone_bytes: u64,
    pub readback_copies: u64,
    pub readback_bytes_copied: u64,
    pub map_requests: u64,
    pub blocking_polls: u64,
    pub blocking_waits: u64,
    pub cpu_frame_copy_bytes: u64,
    pub gpu_buffer_creations: u64,
    pub gpu_texture_creations: u64,
    pub gpu_pipeline_creations: u64,
    pub gpu_bind_group_creations: u64,
    pub gpu_shader_module_creations: u64,
    pub async_readback_submissions: u64,
    pub peak_readbacks_in_flight: u64,
    /// Native workers selected for the CPU raster pass; one means serial.
    pub cpu_parallel_workers: u64,
    /// Candidate triangle visits after screen-row binning.
    pub cpu_raster_candidate_triangles: u64,
    /// Triangle visits the old all-triangles-per-band scan would perform.
    pub cpu_raster_full_rescan_triangles: u64,
    /// Capacity growth while rebuilding retained row-bin scratch.
    pub cpu_raster_bin_storage_growth_bytes: u64,
}

impl RenderWorkMetrics {
    pub(super) fn add_gpu_result(&mut self, result: gpu::GpuRenderResult) {
        self.readback_copies = self.readback_copies.saturating_add(result.readback_copies);
        self.readback_bytes_copied = self
            .readback_bytes_copied
            .saturating_add(result.readback_bytes_copied);
        self.map_requests = self.map_requests.saturating_add(result.map_requests);
        self.blocking_polls = self.blocking_polls.saturating_add(result.blocking_polls);
        self.blocking_waits = self.blocking_waits.saturating_add(result.blocking_waits);
        self.cpu_frame_copy_bytes = self
            .cpu_frame_copy_bytes
            .saturating_add(result.cpu_frame_copy_bytes);
    }
}
