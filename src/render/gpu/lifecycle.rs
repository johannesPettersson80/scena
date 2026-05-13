use crate::diagnostics::{AdapterLimitsReport, GpuAdapterReport};

use super::GpuDeviceState;

impl GpuDeviceState {
    pub(in crate::render) fn adapter_report(&self) -> GpuAdapterReport {
        let info = self.adapter.get_info();
        let limits = self.adapter.limits();
        GpuAdapterReport {
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            vendor: info.vendor,
            device: info.device,
            driver: info.driver,
            driver_info: info.driver_info,
            features: format!("{:?}", self.adapter.features()),
            limits: AdapterLimitsReport {
                max_texture_dimension_2d: limits.max_texture_dimension_2d,
                max_bind_groups: limits.max_bind_groups,
                max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
                max_vertex_attributes: limits.max_vertex_attributes,
            },
        }
    }

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
        {
            self.webgl2_render_cache = None;
        }
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
