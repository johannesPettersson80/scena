use super::*;

impl Renderer {
    pub fn gpu_adapter_report(&self) -> Option<GpuAdapterReport> {
        self.gpu.as_ref().map(GpuDeviceState::adapter_report)
    }

    pub const fn last_render_work_metrics(&self) -> RenderWorkMetrics {
        self.last_render_work_metrics
    }

    pub fn render_active(&mut self, scene: &Scene) -> Result<RenderOutcome, RenderError> {
        self.prepared_state(scene)?;
        let camera = scene.active_camera().ok_or(RenderError::NoActiveCamera)?;
        self.render(scene, camera)
    }

    pub fn frame_rgba8(&self) -> &[u8] {
        &self.frame
    }

    pub fn poll_device(&mut self) -> DevicePoll {
        let before = self.stats.pending_destructions;
        let (destroyed_resources, status) = self
            .gpu
            .as_mut()
            .map(|gpu| gpu.poll_device())
            .unwrap_or((0, DevicePollStatus::Unsupported));
        let after = self
            .gpu
            .as_ref()
            .map(|gpu| gpu.pending_destructions())
            .unwrap_or(0);
        self.stats.pending_destructions = after;
        DevicePoll {
            pending_destructions_before: before,
            pending_destructions_after: after,
            destroyed_resources,
            status,
            gpu_polled: status == DevicePollStatus::Confirmed,
        }
    }

    #[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
    pub(crate) fn browser_device_poll_observation(&self) -> &'static str {
        self.gpu
            .as_ref()
            .map(GpuDeviceState::last_poll_observation)
            .unwrap_or("no-gpu")
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub(crate) fn rendered_frame_state(&self) -> Option<RenderedFrameState> {
        self.last_rendered_frame
    }

    pub(crate) fn readback_frame_state(&self) -> Option<RenderedFrameState> {
        self.last_readback_frame
    }

    pub(crate) fn clear_rendered_frame(&mut self) {
        self.last_rendered_generation = None;
        self.last_rendered_frame = None;
        self.last_readback_frame = None;
    }

    pub fn has_gpu_device(&self) -> bool {
        self.gpu.is_some()
    }
}
