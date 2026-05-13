use super::GpuDeviceState;

impl core::fmt::Debug for GpuDeviceState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GpuDeviceState")
            .field("instance", &self.instance)
            .field("adapter", &self.adapter)
            .field("device", &self.device)
            .field("queue", &self.queue)
            .field("surface", &self.surface)
            .field("pending_destructions", &self.pending_destructions)
            .field("resources", &self.resources)
            .field("browser_canvas_prepared", &self.browser_canvas.is_some())
            .field(
                "webgl2_render_cache_prepared",
                &self.webgl2_render_cache.is_some(),
            )
            .finish()
    }
}
