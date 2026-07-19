use crate::diagnostics::{AdapterLimitsReport, DevicePollStatus, GpuAdapterReport};

use super::GpuDeviceState;

#[cfg(any(target_arch = "wasm32", test))]
fn browser_poll_observation(result: &Result<wgpu::PollStatus, wgpu::PollError>) -> &'static str {
    match result {
        Ok(wgpu::PollStatus::QueueEmpty) => "queue-empty",
        Ok(wgpu::PollStatus::WaitSucceeded) => "wait-succeeded",
        Ok(wgpu::PollStatus::Poll) => "poll",
        Err(_) => "error",
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn browser_uses_automatic_resource_retirement(backend: wgpu::Backend) -> bool {
    matches!(backend, wgpu::Backend::Gl | wgpu::Backend::BrowserWebGpu)
}

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

    #[cfg(target_arch = "wasm32")]
    pub(in crate::render) fn last_poll_observation(&self) -> &'static str {
        self.last_poll_observation
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
        // Give browser backends an explicit, non-blocking opportunity to
        // maintain their resource lifetime trackers.
        let poll_result = self.device.poll(wgpu::PollType::Poll);
        self.last_poll_observation = browser_poll_observation(&poll_result);

        if browser_uses_automatic_resource_retirement(self.adapter.get_info().backend) {
            // Browser WebGPU's Device::poll is automatic/no-op and its JS
            // objects remain browser-owned after Rust releases its wrappers.
            // WebGL uses GlFenceBehavior::AutoFinish, with GL retaining deleted
            // objects referenced by in-flight commands. In both cases scena can
            // retire its logical records without claiming GPU completion.
            return match poll_result {
                Ok(wgpu::PollStatus::QueueEmpty | wgpu::PollStatus::WaitSucceeded) => {
                    let retired = self.pending_destructions;
                    self.pending_destructions = 0;
                    (retired, DevicePollStatus::Automatic)
                }
                Ok(wgpu::PollStatus::Poll) => (0, DevicePollStatus::Automatic),
                Err(_) => (0, DevicePollStatus::Unsupported),
            };
        }

        (0, DevicePollStatus::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::browser_poll_observation;

    #[test]
    fn browser_backends_use_automatic_resource_retirement() {
        assert!(super::browser_uses_automatic_resource_retirement(
            wgpu::Backend::Gl
        ));
        assert!(super::browser_uses_automatic_resource_retirement(
            wgpu::Backend::BrowserWebGpu
        ));
    }

    #[test]
    fn browser_poll_observation_preserves_raw_wgpu_status() {
        assert_eq!(
            browser_poll_observation(&Ok(wgpu::PollStatus::QueueEmpty)),
            "queue-empty"
        );
        assert_eq!(
            browser_poll_observation(&Ok(wgpu::PollStatus::WaitSucceeded)),
            "wait-succeeded"
        );
        assert_eq!(
            browser_poll_observation(&Ok(wgpu::PollStatus::Poll)),
            "poll"
        );
    }
}
