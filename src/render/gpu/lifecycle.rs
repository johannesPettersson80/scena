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
        use std::sync::atomic::Ordering;

        // Give browser backends an explicit, non-blocking opportunity to
        // maintain their resource lifetime trackers.
        let poll_result = self.device.poll(wgpu::PollType::Poll);
        self.last_poll_observation = browser_poll_observation(&poll_result);

        if self.adapter.get_info().backend == wgpu::Backend::Gl {
            // WebGL uses GlFenceBehavior::AutoFinish. This is not a claim that
            // the GPU completed: GL retains deleted objects used by in-flight
            // commands, while wgpu automatically retires its logical records.
            // Report that distinction instead of fabricating Confirmed.
            return match poll_result {
                Ok(wgpu::PollStatus::QueueEmpty | wgpu::PollStatus::WaitSucceeded) => {
                    let retired = self.pending_destructions;
                    self.pending_destructions = 0;
                    self.submitted_destructions = 0;
                    self.confirmed_destructions.store(0, Ordering::Release);
                    (retired, DevicePollStatus::Automatic)
                }
                Ok(wgpu::PollStatus::Poll) => (0, DevicePollStatus::Automatic),
                Err(_) => (0, DevicePollStatus::Unsupported),
            };
        }

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
            // Register against the queue's previously submitted renderer work.
            // Creating an empty submission here is not a portable fence: ANGLE
            // with SwiftShader can leave that artificial submission active
            // indefinitely even though the real render work has completed.
            self.queue.on_submitted_work_done(move || {
                completion.fetch_add(unsubmitted, Ordering::Release);
            });
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

#[cfg(test)]
mod tests {
    use super::browser_poll_observation;

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
