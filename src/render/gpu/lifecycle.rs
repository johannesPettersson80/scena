use crate::diagnostics::{AdapterLimitsReport, DevicePollStatus, GpuAdapterReport};

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

    pub(in crate::render) fn release_surface(&mut self) {
        self.surface = None;
        #[cfg(target_arch = "wasm32")]
        {
            self.browser_canvas = None;
            self.display_p3_canvas_configured = false;
        }
    }

    pub(in crate::render) fn clear_prepared_resources_for_context_recovery(&mut self) {
        self.release_prepared_resources();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::render) fn poll_device(&mut self) -> (u64, DevicePollStatus) {
        let pending = self.pending_destructions;
        let confirmed = self
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .is_ok();
        if confirmed {
            self.pending_destructions = 0;
        }
        (
            pending.saturating_sub(self.pending_destructions),
            if confirmed {
                DevicePollStatus::Confirmed
            } else {
                DevicePollStatus::Unsupported
            },
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::render) fn poll_device(&mut self) -> (u64, DevicePollStatus) {
        use std::sync::atomic::Ordering;

        // wgpu-core's WebGL backend dispatches queue-completion callbacks from
        // an explicit poll/submit boundary. WebGPU's browser backend may
        // resolve the callback from its Promise instead, so this non-blocking
        // poll is safe for both paths and never fabricates completion.
        let _ = self.device.poll(wgpu::PollType::Poll);
        let confirmed = self
            .confirmed_destructions
            .swap(0, Ordering::AcqRel)
            .min(self.pending_destructions)
            .min(self.submitted_destructions);
        self.pending_destructions = self.pending_destructions.saturating_sub(confirmed);
        self.submitted_destructions = self.submitted_destructions.saturating_sub(confirmed);

        let unsubmitted = self
            .pending_destructions
            .saturating_sub(self.submitted_destructions);
        if unsubmitted > 0 {
            let completion = std::sync::Arc::clone(&self.confirmed_destructions);
            self.submitted_destructions = self.submitted_destructions.saturating_add(unsubmitted);
            let encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("scena.resource_destruction_completion"),
                });
            let command_buffer = encoder.finish();
            command_buffer.on_submitted_work_done(move || {
                completion.fetch_add(unsubmitted, Ordering::Release);
            });
            // A concrete empty command buffer creates a real submission/fence
            // without render work or retained resources. Attaching the
            // callback to that submission makes completion backend-agnostic.
            self.queue.submit(std::iter::once(command_buffer));
        }

        let status = if confirmed > 0 {
            DevicePollStatus::Confirmed
        } else if self.submitted_destructions > 0 {
            DevicePollStatus::Submitted
        } else {
            DevicePollStatus::Automatic
        };
        (confirmed, status)
    }
}
