use super::super::RasterTarget;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct GpuResourceStats {
    pub(in crate::render) buffers: u64,
    pub(in crate::render) textures: u64,
    pub(in crate::render) render_targets: u64,
    pub(in crate::render) pipelines: u64,
    pub(in crate::render) bind_groups: u64,
    pub(in crate::render) shader_modules: u64,
    pub(in crate::render) shader_module_creations: u64,
    pub(in crate::render) approximate_gpu_memory_bytes: u64,
    /// Plan line 778 commit 2: distinct material bind groups consumed by
    /// the unlit pass. Equals 1 when the renderer chose the batched
    /// `texture_2d_array<f32>` path (single shared bind group serviced via
    /// dynamic-offset uniforms) and equals the per-material slot count
    /// otherwise (one bind group per slot, including the synthetic
    /// fallback at index 0).
    pub(in crate::render) material_bind_groups: u32,
}

impl GpuResourceStats {
    pub(in crate::render) fn destruction_records(self) -> u64 {
        self.buffers + self.textures + self.pipelines + self.bind_groups
    }

    pub(super) fn add_assign(&mut self, other: Self) {
        self.buffers = self.buffers.saturating_add(other.buffers);
        self.textures = self.textures.saturating_add(other.textures);
        self.render_targets = self.render_targets.saturating_add(other.render_targets);
        self.pipelines = self.pipelines.saturating_add(other.pipelines);
        self.bind_groups = self.bind_groups.saturating_add(other.bind_groups);
        self.shader_modules = self.shader_modules.saturating_add(other.shader_modules);
        self.shader_module_creations = self
            .shader_module_creations
            .saturating_add(other.shader_module_creations);
        self.approximate_gpu_memory_bytes = self
            .approximate_gpu_memory_bytes
            .saturating_add(other.approximate_gpu_memory_bytes);
        self.material_bind_groups = self
            .material_bind_groups
            .saturating_add(other.material_bind_groups);
    }

    pub(super) fn target_bytes(
        target: RasterTarget,
        bytes_per_pixel: u64,
        sample_count: u32,
    ) -> u64 {
        u64::from(target.width)
            .saturating_mul(u64::from(target.height))
            .saturating_mul(bytes_per_pixel)
            .saturating_mul(u64::from(sample_count))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}
