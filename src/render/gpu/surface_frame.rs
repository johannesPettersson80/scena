#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SurfaceAcquireStatus {
    Success,
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
    /// wgpu 29 reports acquisition validation directly but reports device
    /// out-of-memory through the device error channel. Keeping it in the same
    /// policy makes both hard-failure paths injectable and consistent.
    #[allow(dead_code)]
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SurfaceAcquireAction {
    Present,
    PresentThenReconfigure,
    SkipTimeout,
    SkipOccluded,
    ReconfigureAndRetry,
    FailAfterRetry(SurfaceAcquireStatus),
    FailLost,
    FailValidation,
    FailOutOfMemory,
}

#[derive(Debug, Default)]
pub(super) struct SurfaceAcquisitionPolicy {
    retry_consumed: bool,
}

impl SurfaceAcquisitionPolicy {
    pub(super) fn action(&mut self, status: SurfaceAcquireStatus) -> SurfaceAcquireAction {
        match status {
            SurfaceAcquireStatus::Success => SurfaceAcquireAction::Present,
            SurfaceAcquireStatus::Suboptimal => SurfaceAcquireAction::PresentThenReconfigure,
            SurfaceAcquireStatus::Timeout => SurfaceAcquireAction::SkipTimeout,
            SurfaceAcquireStatus::Occluded => SurfaceAcquireAction::SkipOccluded,
            SurfaceAcquireStatus::Outdated => {
                if self.retry_consumed {
                    SurfaceAcquireAction::FailAfterRetry(status)
                } else {
                    self.retry_consumed = true;
                    SurfaceAcquireAction::ReconfigureAndRetry
                }
            }
            SurfaceAcquireStatus::Lost => SurfaceAcquireAction::FailLost,
            SurfaceAcquireStatus::Validation => SurfaceAcquireAction::FailValidation,
            SurfaceAcquireStatus::OutOfMemory => SurfaceAcquireAction::FailOutOfMemory,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) enum SurfaceFrameSkipReason {
    Timeout,
    Occluded,
}

pub(super) struct SurfaceFrameAcquisition {
    pub(super) output: Option<wgpu::SurfaceTexture>,
    pub(super) skip: Option<SurfaceFrameSkipReason>,
    pub(super) reconfigure_after_present: bool,
    pub(super) reconfigurations: u64,
    pub(super) retries: u64,
}

impl SurfaceFrameAcquisition {
    fn detached() -> Self {
        Self {
            output: None,
            skip: None,
            reconfigure_after_present: false,
            reconfigurations: 0,
            retries: 0,
        }
    }

    fn skipped(reason: SurfaceFrameSkipReason, reconfigurations: u64, retries: u64) -> Self {
        Self {
            output: None,
            skip: Some(reason),
            reconfigure_after_present: false,
            reconfigurations,
            retries,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SurfaceConfigurationChange {
    pub(super) format_changed: bool,
    pub(super) present_mode_changed: bool,
    pub(super) size_changed: bool,
}

pub(super) fn refresh_surface_configuration(
    surface: &mut GpuSurfaceState,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    target: RasterTarget,
) -> SurfaceConfigurationChange {
    let size = super::build::clamp_surface_size_to_adapter_limits(
        crate::platform::SurfaceSize {
            width: target.width,
            height: target.height,
        },
        device.limits().max_texture_dimension_2d,
    );
    let previous = surface.config.clone();
    let capabilities = surface.surface.get_capabilities(adapter);
    let mut next = surface
        .surface
        .get_default_config(adapter, size.width, size.height)
        .unwrap_or_else(|| {
            let mut retained = previous.clone();
            retained.width = size.width;
            retained.height = size.height;
            retained
        });
    if capabilities
        .alpha_modes
        .contains(&wgpu::CompositeAlphaMode::Opaque)
    {
        next.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
    }
    build::enable_scene_host_surface_readback(&mut next, &capabilities);
    let change = SurfaceConfigurationChange {
        format_changed: next.format != previous.format,
        present_mode_changed: next.present_mode != previous.present_mode,
        size_changed: next.width != previous.width || next.height != previous.height,
    };
    surface.surface.configure(device, &next);
    surface.config = next;
    change
}

pub(super) fn reconfigure_existing_surface(surface: &mut GpuSurfaceState, device: &wgpu::Device) {
    surface.surface.configure(device, &surface.config);
}

pub(super) fn acquire_surface_frame(
    surface: Option<&mut GpuSurfaceState>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    target: RasterTarget,
) -> Result<SurfaceFrameAcquisition, RenderError> {
    let Some(surface) = surface else {
        return Ok(SurfaceFrameAcquisition::detached());
    };
    let mut policy = SurfaceAcquisitionPolicy::default();
    let mut reconfigurations = 0;
    let mut retries = 0;
    let mut configuration_changed = false;
    loop {
        let (status, output) = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output) => {
                (SurfaceAcquireStatus::Success, Some(output))
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(output) => {
                (SurfaceAcquireStatus::Suboptimal, Some(output))
            }
            wgpu::CurrentSurfaceTexture::Timeout => (SurfaceAcquireStatus::Timeout, None),
            wgpu::CurrentSurfaceTexture::Occluded => (SurfaceAcquireStatus::Occluded, None),
            wgpu::CurrentSurfaceTexture::Outdated => (SurfaceAcquireStatus::Outdated, None),
            wgpu::CurrentSurfaceTexture::Lost => (SurfaceAcquireStatus::Lost, None),
            wgpu::CurrentSurfaceTexture::Validation => (SurfaceAcquireStatus::Validation, None),
        };
        match policy.action(status) {
            SurfaceAcquireAction::Present | SurfaceAcquireAction::PresentThenReconfigure => {
                if configuration_changed {
                    drop(output);
                    return Err(RenderError::SurfaceConfigurationChanged {
                        backend: target.backend,
                    });
                }
                return Ok(SurfaceFrameAcquisition {
                    output,
                    skip: None,
                    reconfigure_after_present: matches!(status, SurfaceAcquireStatus::Suboptimal),
                    reconfigurations,
                    retries,
                });
            }
            SurfaceAcquireAction::SkipTimeout => {
                return Ok(SurfaceFrameAcquisition::skipped(
                    SurfaceFrameSkipReason::Timeout,
                    reconfigurations,
                    retries,
                ));
            }
            SurfaceAcquireAction::SkipOccluded => {
                return Ok(SurfaceFrameAcquisition::skipped(
                    SurfaceFrameSkipReason::Occluded,
                    reconfigurations,
                    retries,
                ));
            }
            SurfaceAcquireAction::ReconfigureAndRetry => {
                let change = refresh_surface_configuration(surface, adapter, device, target);
                configuration_changed |= change.format_changed || change.present_mode_changed;
                reconfigurations += 1;
                retries += 1;
            }
            SurfaceAcquireAction::FailLost => {
                return Err(RenderError::SurfaceLost { recoverable: true });
            }
            SurfaceAcquireAction::FailAfterRetry(SurfaceAcquireStatus::Outdated) => {
                return Err(RenderError::SurfaceOutdated {
                    backend: target.backend,
                    retry_attempted: true,
                });
            }
            SurfaceAcquireAction::FailAfterRetry(_) => unreachable!("only outdated is retried"),
            SurfaceAcquireAction::FailValidation => {
                return Err(RenderError::GpuValidation {
                    backend: target.backend,
                });
            }
            SurfaceAcquireAction::FailOutOfMemory => {
                return Err(RenderError::GpuOutOfMemory {
                    backend: target.backend,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SurfaceAcquireAction, SurfaceAcquireStatus, SurfaceAcquisitionPolicy};

    #[test]
    fn outdated_surface_reconfigures_and_retries_exactly_once() {
        let mut policy = SurfaceAcquisitionPolicy::default();
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Outdated),
            SurfaceAcquireAction::ReconfigureAndRetry
        );
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Outdated),
            SurfaceAcquireAction::FailAfterRetry(SurfaceAcquireStatus::Outdated)
        );
    }

    #[test]
    fn lost_surface_requires_host_recreation_without_fake_retry() {
        let mut policy = SurfaceAcquisitionPolicy::default();
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Lost),
            SurfaceAcquireAction::FailLost
        );
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Lost),
            SurfaceAcquireAction::FailLost
        );
    }

    #[test]
    fn timeout_and_occlusion_are_diagnostic_skips() {
        let mut policy = SurfaceAcquisitionPolicy::default();
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Timeout),
            SurfaceAcquireAction::SkipTimeout
        );
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Occluded),
            SurfaceAcquireAction::SkipOccluded
        );
    }

    #[test]
    fn validation_and_out_of_memory_are_hard_failures() {
        let mut policy = SurfaceAcquisitionPolicy::default();
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Validation),
            SurfaceAcquireAction::FailValidation
        );
        assert_eq!(
            policy.action(SurfaceAcquireStatus::OutOfMemory),
            SurfaceAcquireAction::FailOutOfMemory
        );
    }

    #[test]
    fn runtime_fault_channel_preserves_validation_and_oom() {
        let state = super::GpuRuntimeFaultState::default();
        *state.fault.lock().expect("fault lock") = Some(super::GpuRuntimeFault::Validation);
        assert!(matches!(
            state.render_error(crate::Backend::HeadlessGpu),
            Some(crate::RenderError::GpuValidation {
                backend: crate::Backend::HeadlessGpu
            })
        ));
        *state.fault.lock().expect("fault lock") = Some(super::GpuRuntimeFault::OutOfMemory);
        assert!(matches!(
            state.render_error(crate::Backend::HeadlessGpu),
            Some(crate::RenderError::GpuOutOfMemory {
                backend: crate::Backend::HeadlessGpu
            })
        ));
    }

    #[test]
    fn suboptimal_frame_is_presented_then_reconfigured() {
        let mut policy = SurfaceAcquisitionPolicy::default();
        assert_eq!(
            policy.action(SurfaceAcquireStatus::Suboptimal),
            SurfaceAcquireAction::PresentThenReconfigure
        );
    }
}
use crate::diagnostics::{Backend, RenderError};

use super::super::RasterTarget;
use super::{GpuSurfaceState, build};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuRuntimeFault {
    Validation,
    OutOfMemory,
}

#[derive(Debug, Clone, Default)]
pub(super) struct GpuRuntimeFaultState {
    fault: std::sync::Arc<std::sync::Mutex<Option<GpuRuntimeFault>>>,
}

impl GpuRuntimeFaultState {
    fn record(&self, error: &wgpu::Error) {
        let fault = match error {
            wgpu::Error::OutOfMemory { .. } => GpuRuntimeFault::OutOfMemory,
            wgpu::Error::Validation { .. } | wgpu::Error::Internal { .. } => {
                GpuRuntimeFault::Validation
            }
        };
        if let Ok(mut slot) = self.fault.lock() {
            *slot = Some(fault);
        }
    }

    pub(super) fn render_error(&self, backend: Backend) -> Option<RenderError> {
        let fault = self.fault.lock().ok().and_then(|slot| *slot)?;
        Some(match fault {
            GpuRuntimeFault::Validation => RenderError::GpuValidation { backend },
            GpuRuntimeFault::OutOfMemory => RenderError::GpuOutOfMemory { backend },
        })
    }
}

pub(super) fn install_gpu_error_callback(device: &wgpu::Device, state: GpuRuntimeFaultState) {
    device.on_uncaptured_error(std::sync::Arc::new(move |error| {
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("scena wgpu uncaptured error: {error:?}");
        #[cfg(target_arch = "wasm32")]
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena wgpu uncaptured error: {error:?}"
        )));
        state.record(&error);
    }));
}
