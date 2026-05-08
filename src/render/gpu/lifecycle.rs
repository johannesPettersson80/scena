use super::GpuDeviceState;
#[cfg(target_arch = "wasm32")]
use super::webgl2;

impl GpuDeviceState {
    pub(in crate::render) fn pending_destructions(&self) -> u64 {
        self.pending_destructions
    }

    pub(in crate::render) fn release_prepared_resources(&mut self) {
        if let Some(resources) = self.resources.take() {
            self.pending_destructions = self
                .pending_destructions
                .saturating_add(resources.stats.destruction_records());
        }
    }

    pub(in crate::render) fn clear_prepared_resources_for_context_recovery(&mut self) {
        self.release_prepared_resources();
        #[cfg(target_arch = "wasm32")]
        webgl2::clear_render_cache();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::render) fn poll_device(&mut self) -> (u64, bool) {
        let pending = self.pending_destructions;
        let gpu_polled = self
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .is_ok();
        if gpu_polled {
            self.pending_destructions = 0;
        }
        (
            pending.saturating_sub(self.pending_destructions),
            gpu_polled,
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::render) fn poll_device(&mut self) -> (u64, bool) {
        let pending = self.pending_destructions;
        self.pending_destructions = 0;
        (pending, true)
    }
}
