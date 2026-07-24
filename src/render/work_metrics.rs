use super::gpu;

/// Deterministic CPU work and byte counters collected by a profiled prepare.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrepareWorkMetrics {
    pub prepared_triangle_count: u64,
    pub prepared_model_vertex_buffer_count: u64,
    pub prepared_model_vertex_bytes: u64,
    pub prepared_unique_draw_transforms: u64,
    pub prepared_draw_transform_bytes: u64,
    pub prepared_triangle_reference_bytes: u64,
    pub prepared_list_copy_bytes: u64,
    pub asset_storage_lock_acquisitions: u64,
    pub generated_tangent_calls: u64,
    pub generated_tangent_triangles: u64,
    pub generated_tangent_vertices: u64,
    pub generated_tangent_cache_hits: u64,
    pub generated_tangent_cache_misses: u64,
    pub tangent_input_transform_bytes: u64,
    pub tangent_output_bytes: u64,
    pub deformed_vertex_bytes_materialized: u64,
    pub shadow_rays: u64,
    pub shadow_visibility_cache_hits: u64,
    pub shadow_visibility_cache_misses: u64,
    pub bvh_node_bounds_tests: u64,
    pub ray_triangle_intersection_tests: u64,
    pub area_light_samples: u64,
    pub cpu_bake_subdivided_triangles: u64,
    pub cpu_bake_shaded_vertices: u64,
    pub texture_samples: u64,
    pub cpu_bake_corner_bytes_copied: u64,
    pub gpu_buffer_creations: u64,
    pub gpu_texture_creations: u64,
    pub gpu_pipeline_creations: u64,
    pub gpu_bind_group_creations: u64,
    pub gpu_shader_module_creations: u64,
    pub gpu_triangle_shader_cache_hits: u64,
    pub gpu_triangle_shader_cache_misses: u64,
    pub gpu_nonblocking_polls: u64,
    pub gpu_blocking_polls: u64,
    pub draw_uniform_unique_values: u64,
    pub draw_uniform_lookup_probes: u64,
    pub draw_uniform_bytes_copied: u64,
}

impl PrepareWorkMetrics {
    pub const fn bytes_cloned_or_copied(self) -> u64 {
        self.prepared_model_vertex_bytes
            .saturating_add(self.prepared_list_copy_bytes)
            .saturating_add(self.tangent_input_transform_bytes)
            .saturating_add(self.tangent_output_bytes)
            .saturating_add(self.deformed_vertex_bytes_materialized)
            .saturating_add(self.cpu_bake_corner_bytes_copied)
            .saturating_add(self.draw_uniform_bytes_copied)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PrepareTelemetry {
    pub(super) full_prepares: u64,
    pub(super) prepared_primitive_collections: u64,
    pub(super) static_gpu_resource_rebuilds: u64,
    pub(super) dynamic_template_prepares: u64,
    pub(super) draw_uniform_only_updates: u64,
}

/// Controls whether a GPU render only presents or also synchronously copies
/// rendered pixels into the renderer's CPU frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderReadbackMode {
    /// Headless GPU rendering captures pixels. Attached native/browser
    /// surfaces present without a synchronous full-frame readback; native
    /// managed auto exposure uses a separate bounded asynchronous meter.
    #[default]
    Automatic,
    /// Submit/present without a texture-to-buffer copy, map, or blocking wait.
    PresentOnly,
    /// Copy and map the rendered pixels before returning.
    Synchronous,
}

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
    /// Bounded asynchronous exposure-meter copies submitted with the frame.
    pub auto_exposure_meter_submissions: u64,
    /// Individual surface pixels copied into the bounded exposure meter.
    pub auto_exposure_meter_samples: u64,
    pub gpu_buffer_creations: u64,
    pub gpu_texture_creations: u64,
    pub gpu_pipeline_creations: u64,
    pub gpu_bind_group_creations: u64,
    pub gpu_shader_module_creations: u64,
    /// Adapter format-feature cache misses observed during this render.
    pub gpu_format_feature_probes: u64,
    /// Native calls that encode the complete scene-color pass family.
    pub native_scene_color_passes: u64,
    pub gpu_queue_submissions: u64,
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
    /// Final linear-to-display encodes, counted per output pixel rather than
    /// per covered fragment.
    pub cpu_output_pixels_encoded: u64,
    /// Prepared-primitive entries inspected once to derive per-frame pass flags.
    pub cpu_primitive_flag_scan_items: u64,
}

impl RenderWorkMetrics {
    pub(super) fn add_gpu_result(&mut self, result: gpu::GpuRenderResult) {
        self.native_scene_color_passes = self
            .native_scene_color_passes
            .saturating_add(result.native_scene_color_passes);
        self.gpu_queue_submissions = self
            .gpu_queue_submissions
            .saturating_add(u64::from(result.submitted));
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
        self.auto_exposure_meter_submissions = self
            .auto_exposure_meter_submissions
            .saturating_add(result.auto_exposure_meter_submissions);
        self.auto_exposure_meter_samples = self
            .auto_exposure_meter_samples
            .saturating_add(result.auto_exposure_meter_samples);
    }
}
