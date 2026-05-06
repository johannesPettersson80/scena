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
pub struct Capabilities {
    pub backend: Backend,
    pub color_target_format: &'static str,
    pub gpu_device: bool,
    pub surface_attached: bool,
    pub output_stage: OutputStageStatus,
    pub alpha_pipeline: AlphaPipelineStatus,
    pub directional_shadow_map_default_size: u32,
    pub directional_shadow_map_max_size: u32,
    pub directional_shadow_pcf_kernel: u8,
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
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            reversed_z_depth: reversed_z_depth_status(backend),
        }
    }

    pub const fn for_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: false,
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            reversed_z_depth: reversed_z_depth_status(backend),
        }
    }

    pub const fn for_attached_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: true,
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
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
