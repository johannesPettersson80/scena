#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RendererStats {
    pub buffers: u64,
    /// Live texture allocations owned by the active prepared GPU resource set.
    /// Unlike `textures`, this is a physical backend allocation count.
    pub gpu_textures: u64,
    pub textures: u64,
    pub materials: u64,
    pub material_bindings: u64,
    pub material_texture_bindings: u64,
    pub material_sampler_bindings: u64,
    pub material_textures_missing_decoded_pixels: u64,
    /// Number of layers a `texture_2d_array` carries when prepared materials
    /// share `(sampler, format, dimensions)` for every populated role.
    pub material_batch_layers: u32,
    /// Actual material bind-group count consumed by the GPU pipeline.
    pub material_bind_groups: u32,
    pub render_targets: u64,
    pub pipelines: u64,
    pub bind_groups: u64,
    pub shader_modules: u64,
    pub environments: u64,
    pub environment_cubemaps: u64,
    pub environment_prefilter_passes: u64,
    pub environment_brdf_luts: u64,
    pub scene_imports: u64,
    pub shadow_maps: u64,
    pub depth_prepass_passes: u64,
    pub depth_prepass_draws: u64,
    pub ambient_occlusion_passes: u64,
    pub screen_space_reflection_passes: u64,
    pub order_independent_transparency_passes: u64,
    pub bloom_passes: u64,
    pub depth_of_field_passes: u64,
    pub fxaa_passes: u64,
    pub live_logical_handles: u64,
    pub pending_destructions: u64,
    pub frames_rendered: u64,
    /// Deprecated alias of `triangles`; kept stable until 2.0.
    pub draw_calls: u64,
    pub triangles: u64,
    pub gpu_draw_submissions: u64,
    pub instances: u64,
    pub culled_objects: u64,
    pub gpu_culling_dispatches: u64,
    pub skipped_frames: u64,
    /// Frames skipped because surface acquisition timed out.
    pub surface_timeout_skips: u64,
    /// Frames skipped because the host surface was occluded.
    pub surface_occluded_skips: u64,
    /// Surface configurations refreshed after outdated, lost, or suboptimal acquisition.
    pub surface_reconfigurations: u64,
    /// Surface acquisition retries performed after a configuration refresh.
    pub surface_acquire_retries: u64,
    pub gpu_submissions: u64,
    pub approximate_gpu_memory_bytes: Option<u64>,
    pub cpu_frame_ms: f32,
    pub gpu_frame_ms: Option<f32>,
    pub primitives: u64,
    pub target_width: u32,
    pub target_height: u32,
    pub directional_shadow_map_resolution: Option<u32>,
    pub directional_shadow_pcf_kernel: Option<u8>,
}
