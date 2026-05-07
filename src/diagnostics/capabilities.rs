use super::{Diagnostic, DiagnosticCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Headless,
    HeadlessGpu,
    SurfaceDescriptor,
    NativeSurface,
    WebGpu,
    WebGl2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputStageStatus {
    AcesSrgb,
    BackendPassthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlphaPipelineStatus {
    LinearSourceOver,
    BackendPassthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapabilityStatus {
    Supported,
    Degraded,
    FeatureDisabled,
    ErrorIfRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HardwareTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Capabilities {
    pub backend: Backend,
    pub color_target_format: &'static str,
    pub gpu_device: bool,
    pub surface_attached: bool,
    pub hardware_tier: HardwareTier,
    pub output_stage: OutputStageStatus,
    pub alpha_pipeline: AlphaPipelineStatus,
    pub forward_pbr: CapabilityStatus,
    pub directional_shadow_map_default_size: u32,
    pub directional_shadow_map_max_size: u32,
    pub directional_shadow_pcf_kernel: u8,
    pub ibl_cubemap_default_size: u32,
    pub ibl_brdf_lut_default_size: u32,
    pub default_clipping_planes: u8,
    pub max_clipping_planes: u8,
    pub gpu_frustum_culling: CapabilityStatus,
    pub per_instance_culling: CapabilityStatus,
    pub compute_shaders: CapabilityStatus,
    pub storage_buffers: CapabilityStatus,
    pub readback_headless_screenshots: CapabilityStatus,
    pub reversed_z_depth: CapabilityStatus,
}

impl Capabilities {
    pub const fn headless() -> Self {
        Self::for_backend(Backend::Headless)
    }

    pub const fn for_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: false,
            surface_attached: false,
            hardware_tier: hardware_tier(backend, false),
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            forward_pbr: CapabilityStatus::Supported,
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_default_size(backend),
            default_clipping_planes: default_clipping_planes(backend),
            max_clipping_planes: max_clipping_planes(backend),
            gpu_frustum_culling: gpu_frustum_culling_status(backend),
            per_instance_culling: per_instance_culling_status(backend),
            compute_shaders: compute_shader_status(backend),
            storage_buffers: storage_buffer_status(backend),
            readback_headless_screenshots: readback_status(backend),
            reversed_z_depth: reversed_z_depth_status(backend),
        }
    }

    pub const fn for_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: false,
            hardware_tier: hardware_tier(backend, true),
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            forward_pbr: CapabilityStatus::Supported,
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_default_size(backend),
            default_clipping_planes: default_clipping_planes(backend),
            max_clipping_planes: max_clipping_planes(backend),
            gpu_frustum_culling: gpu_frustum_culling_status(backend),
            per_instance_culling: per_instance_culling_status(backend),
            compute_shaders: compute_shader_status(backend),
            storage_buffers: storage_buffer_status(backend),
            readback_headless_screenshots: readback_status(backend),
            reversed_z_depth: reversed_z_depth_status(backend),
        }
    }

    pub const fn for_attached_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: true,
            hardware_tier: hardware_tier(backend, true),
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            forward_pbr: CapabilityStatus::Supported,
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_default_size(backend),
            default_clipping_planes: default_clipping_planes(backend),
            max_clipping_planes: max_clipping_planes(backend),
            gpu_frustum_culling: gpu_frustum_culling_status(backend),
            per_instance_culling: per_instance_culling_status(backend),
            compute_shaders: compute_shader_status(backend),
            storage_buffers: storage_buffer_status(backend),
            readback_headless_screenshots: readback_status(backend),
            reversed_z_depth: reversed_z_depth_status(backend),
        }
    }

    pub fn diagnostics(self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.backend == Backend::WebGl2
            && self.reversed_z_depth == CapabilityStatus::FeatureDisabled
        {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::WebGl2DepthCompatibility,
                "WebGL2 uses the compatibility depth profile without reversed-Z depth",
                "tighten camera near/far ranges and keep large scenes camera-relative when targeting WebGL2",
            ));
        }
        diagnostics
    }
}

const fn hardware_tier(backend: Backend, gpu_device: bool) -> HardwareTier {
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

const fn reversed_z_depth_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

const fn directional_shadow_map_default_size(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 1024,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 2048,
    }
}

const fn directional_shadow_map_max_size(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 2048,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 4096,
    }
}

const fn ibl_default_size(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 128,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 256,
    }
}

const fn default_clipping_planes(backend: Backend) -> u8 {
    match backend {
        Backend::WebGl2 => 4,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 8,
    }
}

const fn max_clipping_planes(backend: Backend) -> u8 {
    match backend {
        Backend::WebGl2 => 8,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => 16,
    }
}

const fn gpu_frustum_culling_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

const fn per_instance_culling_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::Degraded
        }
    }
}

const fn compute_shader_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

const fn storage_buffer_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

const fn readback_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::WebGl2 => CapabilityStatus::Degraded,
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu => CapabilityStatus::Supported,
    }
}
