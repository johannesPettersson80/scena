use super::{Backend, CapabilityStatus};

pub(super) const fn single_sample_counts() -> [u32; 3] {
    [1, 0, 0]
}

pub(super) const fn explicit_msaa_default() -> CapabilityStatus {
    CapabilityStatus::ErrorIfRequired
}

pub(super) const fn renderer_sample_counts(backend: Backend) -> [u32; 3] {
    match backend {
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::NativeSurface
        | Backend::SurfaceDescriptor
        | Backend::WebGpu
        | Backend::WebGl2 => [1, 0, 0],
    }
}

pub(super) const fn explicit_msaa_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu
        | Backend::NativeSurface
        | Backend::WebGpu
        | Backend::WebGl2
        | Backend::SurfaceDescriptor => CapabilityStatus::ErrorIfRequired,
        Backend::Headless => CapabilityStatus::FeatureDisabled,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) const fn measured_sample_counts(maximum: u32) -> [u32; 3] {
    [
        1,
        if maximum >= 4 { 4 } else { 0 },
        if maximum >= 8 { 8 } else { 0 },
    ]
}
