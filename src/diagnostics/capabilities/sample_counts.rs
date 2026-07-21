use super::{Backend, CapabilityStatus};

pub(super) const fn single_sample_counts() -> [u32; 3] {
    [1, 0, 0]
}

pub(super) const fn explicit_msaa_default() -> CapabilityStatus {
    CapabilityStatus::ErrorIfRequired
}

pub(super) const fn renderer_sample_counts(backend: Backend) -> [u32; 3] {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface => [1, 4, 8],
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGpu | Backend::WebGl2 => {
            [1, 0, 0]
        }
    }
}

pub(super) const fn explicit_msaa_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface => CapabilityStatus::Supported,
        Backend::WebGpu | Backend::WebGl2 | Backend::SurfaceDescriptor => {
            CapabilityStatus::ErrorIfRequired
        }
        Backend::Headless => CapabilityStatus::FeatureDisabled,
    }
}
