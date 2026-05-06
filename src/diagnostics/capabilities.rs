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
