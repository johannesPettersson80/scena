use crate::diagnostics::{
    AdapterLimitsReport, Backend, CapabilityConstraintProbeV1, CapabilityConstraintStatusV1,
    CapabilityProbeModeV1, CapabilityProbeStatusV1, CapabilityProbeV1, CapabilityTargetProbeV1,
    DevicePollStatus, GpuAdapterReport, GpuDeviceReport,
};

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

    pub(in crate::render) fn live_capability_probe(
        &self,
        backend: Backend,
        probed_at_unix_ms: u64,
    ) -> CapabilityProbeV1 {
        let device_limits = self.device.limits();
        CapabilityProbeV1 {
            mode: CapabilityProbeModeV1::LiveAdapter,
            status: CapabilityProbeStatusV1::Measured,
            source: "live_wgpu_adapter".to_owned(),
            probed_at_unix_ms: Some(probed_at_unix_ms),
            requested_backend: backend,
            selected_backend: Some(backend),
            device: Some(GpuDeviceReport {
                features: format!("{:?}", self.device.features()),
                limits: AdapterLimitsReport {
                    max_texture_dimension_2d: device_limits.max_texture_dimension_2d,
                    max_bind_groups: device_limits.max_bind_groups,
                    max_uniform_buffer_binding_size: device_limits.max_uniform_buffer_binding_size,
                    max_vertex_attributes: device_limits.max_vertex_attributes,
                },
            }),
            color_target: self.target_probe(self.color_target_format()),
            depth_target: self.target_probe(wgpu::TextureFormat::Depth32Float),
            readback: CapabilityConstraintProbeV1 {
                status: if self.surface.is_none() {
                    CapabilityConstraintStatusV1::Supported
                } else {
                    CapabilityConstraintStatusV1::NotProbed
                },
                detail: if self.surface.is_none() {
                    "headless target is configured for COPY_SRC readback"
                } else {
                    "surface presentation probe does not exercise screenshot readback"
                }
                .to_owned(),
            },
            presentation: CapabilityConstraintProbeV1 {
                status: if self.surface.is_some() {
                    CapabilityConstraintStatusV1::Supported
                } else {
                    CapabilityConstraintStatusV1::NotProbed
                },
                detail: if self.surface.is_some() {
                    "attached surface configuration was selected by wgpu"
                } else {
                    "headless probe has no presentation surface"
                }
                .to_owned(),
            },
            unavailable: None,
        }
    }

    fn target_probe(&self, format: wgpu::TextureFormat) -> CapabilityTargetProbeV1 {
        let features = self.adapter.get_texture_format_features(format);
        CapabilityTargetProbeV1 {
            format: format!("{format:?}"),
            source: "adapter_format_features".to_owned(),
            measured: true,
            allowed_usages: Some(format!("{:?}", features.allowed_usages)),
            sample_counts: [1, 2, 4, 8, 16]
                .into_iter()
                .filter(|sample_count| {
                    super::msaa::texture_format_supports_sample_count(
                        &self.device,
                        &self.adapter,
                        format,
                        *sample_count,
                    )
                })
                .collect(),
        }
    }

    pub(in crate::render) fn pending_destructions(&self) -> u64 {
        self.pending_destructions
    }

    #[cfg(target_arch = "wasm32")]
    #[cfg_attr(not(feature = "browser-probe"), allow(dead_code))]
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::render) fn poll_device_nonblocking(&mut self) -> (u64, DevicePollStatus) {
        let pending = self.pending_destructions;
        match self.device.poll(wgpu::PollType::Poll) {
            Ok(wgpu::PollStatus::QueueEmpty | wgpu::PollStatus::WaitSucceeded) => {
                self.pending_destructions = 0;
                (pending, DevicePollStatus::Confirmed)
            }
            Ok(wgpu::PollStatus::Poll) => (0, DevicePollStatus::Submitted),
            Err(_) => (0, DevicePollStatus::Unsupported),
        }
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

    #[cfg(target_arch = "wasm32")]
    pub(in crate::render) fn poll_device_nonblocking(&mut self) -> (u64, DevicePollStatus) {
        self.poll_device()
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
