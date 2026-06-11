use super::capabilities::{Backend, CapabilityStatus, HardwareTier};

pub(in crate::diagnostics) const fn forward_pbr_status(
    backend: Backend,
    gpu_device: bool,
) -> CapabilityStatus {
    match (backend, gpu_device) {
        (
            Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2,
            true,
        ) => CapabilityStatus::Supported,
        (
            Backend::Headless
            | Backend::HeadlessGpu
            | Backend::SurfaceDescriptor
            | Backend::NativeSurface
            | Backend::WebGpu
            | Backend::WebGl2,
            false,
        )
        | (Backend::Headless | Backend::SurfaceDescriptor, true) => CapabilityStatus::Degraded,
    }
}

pub(in crate::diagnostics) const fn directional_shadow_status(
    _backend: Backend,
) -> CapabilityStatus {
    CapabilityStatus::Degraded
}

pub(in crate::diagnostics) const fn punctual_shadow_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::FeatureDisabled
}

pub(in crate::diagnostics) const fn hardware_tier(
    backend: Backend,
    gpu_device: bool,
) -> HardwareTier {
    match (backend, gpu_device) {
        (Backend::NativeSurface, true) => HardwareTier::High,
        (Backend::HeadlessGpu | Backend::WebGpu, true) => HardwareTier::Medium,
        (
            Backend::Headless
            | Backend::HeadlessGpu
            | Backend::SurfaceDescriptor
            | Backend::NativeSurface
            | Backend::WebGpu
            | Backend::WebGl2,
            false,
        )
        | (Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2, true) => {
            HardwareTier::Low
        }
    }
}

pub(in crate::diagnostics) const fn reversed_z_depth_status(backend: Backend) -> CapabilityStatus {
    gpu_only_status(backend)
}

pub(in crate::diagnostics) const fn directional_shadow_map_default_size(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 1024,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 2048,
    }
}

pub(in crate::diagnostics) const fn directional_shadow_map_max_size(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 2048,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 4096,
    }
}

pub(in crate::diagnostics) const fn ibl_default_size(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 128,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 256,
    }
}

pub(in crate::diagnostics) const fn bloom_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::Supported
}

pub(in crate::diagnostics) const fn ambient_occlusion_status(
    _backend: Backend,
) -> CapabilityStatus {
    CapabilityStatus::Supported
}

pub(in crate::diagnostics) const fn order_independent_transparency_status(
    backend: Backend,
) -> CapabilityStatus {
    match backend {
        Backend::Headless | Backend::SurfaceDescriptor => CapabilityStatus::Supported,
        _ => CapabilityStatus::FeatureDisabled,
    }
}

pub(in crate::diagnostics) const fn physical_glass_transmission_status(
    backend: Backend,
    gpu_device: bool,
) -> CapabilityStatus {
    match (backend, gpu_device) {
        (
            Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2,
            true,
        ) => CapabilityStatus::Supported,
        (
            Backend::Headless
            | Backend::HeadlessGpu
            | Backend::SurfaceDescriptor
            | Backend::NativeSurface
            | Backend::WebGpu
            | Backend::WebGl2,
            false,
        )
        | (Backend::Headless | Backend::SurfaceDescriptor, true) => CapabilityStatus::Degraded,
    }
}

pub(in crate::diagnostics) const fn wide_gamut_output_status(
    backend: Backend,
    surface_attached: bool,
) -> CapabilityStatus {
    match (backend, surface_attached) {
        (Backend::WebGpu | Backend::WebGl2, true) => CapabilityStatus::Degraded,
        _ => CapabilityStatus::FeatureDisabled,
    }
}

pub(in crate::diagnostics) const fn hardware_instancing_status(
    backend: Backend,
) -> CapabilityStatus {
    gpu_only_status(backend)
}

/// Phase 1F: backend-level support for sampling `texture_2d_array<f32>` /
/// `sampler2DArray`. WebGPU mandates 256+ layer support; WebGL2 GLES 3.0+
/// also exposes array textures. CPU-rasterizer backends keep the per-
/// material bind path.
pub(in crate::diagnostics) const fn texture_arrays_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2 => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor => CapabilityStatus::FeatureDisabled,
    }
}

/// Phase 1F: minimum guaranteed `max_texture_array_layers` per backend.
/// Both the WebGPU `Limits::default()` and the WebGL2 `GLES 3.0`
/// specification mandate at least 256 layers; runtime adapter probes can
/// report higher values via `Capabilities::with_adapter_limits`.
pub(in crate::diagnostics) const fn max_texture_array_layers(backend: Backend) -> u32 {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2 => 256,
        Backend::Headless | Backend::SurfaceDescriptor => 0,
    }
}

pub(in crate::diagnostics) const fn fragment_high_precision_status(
    backend: Backend,
) -> CapabilityStatus {
    gpu_only_status(backend)
}

pub(in crate::diagnostics) const fn uniform_buffer_status(backend: Backend) -> CapabilityStatus {
    gpu_only_status(backend)
}

pub(in crate::diagnostics) const fn uniform_buffer_max_bytes(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 16_384,
        Backend::Headless | Backend::SurfaceDescriptor => 0,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => 65_536,
    }
}

pub(in crate::diagnostics) const fn default_clipping_planes(backend: Backend) -> u8 {
    match backend {
        Backend::WebGl2 => 4,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 8,
    }
}

pub(in crate::diagnostics) const fn max_clipping_planes(backend: Backend) -> u8 {
    match backend {
        Backend::WebGl2 => 8,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 16,
    }
}

pub(in crate::diagnostics) const fn gpu_frustum_culling_status(
    backend: Backend,
) -> CapabilityStatus {
    match backend {
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu
        | Backend::WebGl2 => CapabilityStatus::FeatureDisabled,
    }
}

pub(in crate::diagnostics) const fn per_instance_culling_status(
    backend: Backend,
) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::Degraded
        }
    }
}

pub(in crate::diagnostics) const fn compute_shader_status(backend: Backend) -> CapabilityStatus {
    gpu_only_status(backend)
}

pub(in crate::diagnostics) const fn storage_buffer_status(backend: Backend) -> CapabilityStatus {
    gpu_only_status(backend)
}

const fn gpu_only_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

pub(in crate::diagnostics) const fn readback_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::WebGl2 => CapabilityStatus::Degraded,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => CapabilityStatus::Supported,
    }
}
