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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterLimitsReport {
    pub max_texture_dimension_2d: u32,
    pub max_bind_groups: u32,
    pub max_uniform_buffer_binding_size: u64,
    pub max_vertex_attributes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterReport {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub vendor: u32,
    pub device: u32,
    pub driver: String,
    pub driver_info: String,
    pub features: String,
    pub limits: AdapterLimitsReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReport {
    capabilities: Capabilities,
    adapter: Option<GpuAdapterReport>,
    diagnostics: Vec<Diagnostic>,
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
    pub directional_shadows: CapabilityStatus,
    pub point_shadows: CapabilityStatus,
    pub spot_shadows: CapabilityStatus,
    pub directional_shadow_map_default_size: u32,
    pub directional_shadow_map_max_size: u32,
    pub directional_shadow_pcf_kernel: u8,
    pub ibl_cubemap_default_size: u32,
    pub ibl_brdf_lut_default_size: u32,
    pub bloom: CapabilityStatus,
    pub screen_space_ambient_occlusion: CapabilityStatus,
    pub texture_compression_basisu: CapabilityStatus,
    pub hardware_instancing: CapabilityStatus,
    /// Phase 1F: whether the backend can sample from `texture_2d_array<f32>`
    /// (or `sampler2DArray` on WebGL2 GLES 3.0+). When `Supported`, the
    /// renderer can pack multiple per-material textures of the same role,
    /// sampler, format, and dimensions into a single array texture and
    /// drop per-draw bind-group changes between materials. When
    /// `FeatureDisabled` the renderer keeps the per-material bind path.
    pub texture_arrays: CapabilityStatus,
    /// Maximum array-texture layer count this backend exposes. Reflects the
    /// WebGPU `Limits::max_texture_array_layers` field (or the WebGL2
    /// `MAX_ARRAY_TEXTURE_LAYERS` query). Zero on CPU-rasterizer backends
    /// where array textures are not used.
    pub max_texture_array_layers: u32,
    pub fragment_high_precision: CapabilityStatus,
    pub uniform_buffers: CapabilityStatus,
    pub uniform_buffer_max_bytes: u32,
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
            forward_pbr: forward_pbr_status(backend),
            directional_shadows: directional_shadow_status(backend),
            point_shadows: punctual_shadow_status(backend),
            spot_shadows: punctual_shadow_status(backend),
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_default_size(backend),
            bloom: postprocess_status(backend),
            screen_space_ambient_occlusion: postprocess_status(backend),
            texture_compression_basisu: texture_compression_basisu_status(backend),
            hardware_instancing: hardware_instancing_status(backend),
            texture_arrays: texture_arrays_status(backend),
            max_texture_array_layers: max_texture_array_layers(backend),
            fragment_high_precision: fragment_high_precision_status(backend),
            uniform_buffers: uniform_buffer_status(backend),
            uniform_buffer_max_bytes: uniform_buffer_max_bytes(backend),
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
            forward_pbr: forward_pbr_status(backend),
            directional_shadows: directional_shadow_status(backend),
            point_shadows: punctual_shadow_status(backend),
            spot_shadows: punctual_shadow_status(backend),
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_default_size(backend),
            bloom: postprocess_status(backend),
            screen_space_ambient_occlusion: postprocess_status(backend),
            texture_compression_basisu: texture_compression_basisu_status(backend),
            hardware_instancing: hardware_instancing_status(backend),
            texture_arrays: texture_arrays_status(backend),
            max_texture_array_layers: max_texture_array_layers(backend),
            fragment_high_precision: fragment_high_precision_status(backend),
            uniform_buffers: uniform_buffer_status(backend),
            uniform_buffer_max_bytes: uniform_buffer_max_bytes(backend),
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
            forward_pbr: forward_pbr_status(backend),
            directional_shadows: directional_shadow_status(backend),
            point_shadows: punctual_shadow_status(backend),
            spot_shadows: punctual_shadow_status(backend),
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_default_size(backend),
            bloom: postprocess_status(backend),
            screen_space_ambient_occlusion: postprocess_status(backend),
            texture_compression_basisu: texture_compression_basisu_status(backend),
            hardware_instancing: hardware_instancing_status(backend),
            texture_arrays: texture_arrays_status(backend),
            max_texture_array_layers: max_texture_array_layers(backend),
            fragment_high_precision: fragment_high_precision_status(backend),
            uniform_buffers: uniform_buffer_status(backend),
            uniform_buffer_max_bytes: uniform_buffer_max_bytes(backend),
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
        if self.forward_pbr == CapabilityStatus::Degraded {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::ForwardPbrDegraded,
                "PBR is reported as degraded until GPU material, texture, and IBL shading are proven",
                "treat metallic-roughness output as a compatibility preview until the PBR visual gate closes",
            ));
        }
        if self.directional_shadows == CapabilityStatus::Degraded {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::DirectionalShadowsDegraded,
                "Directional shadows are degraded until shadow maps are rendered and sampled into visible receiver pixels",
                "treat shadow-map counters as allocation metadata until the shadow visual gate closes",
            ));
        }
        if self.point_shadows == CapabilityStatus::FeatureDisabled {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::PointShadowsDisabled,
                "Point shadows are disabled until cube-map shadow rendering and receiver sampling are implemented",
                "use unshadowed point lights or bake shadowing into assets until the point-shadow gate closes",
            ));
        }
        if self.spot_shadows == CapabilityStatus::FeatureDisabled {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::SpotShadowsDisabled,
                "Spot shadows are disabled until projected spot shadow maps and receiver sampling are implemented",
                "use unshadowed spot lights or bake shadowing into assets until the spot-shadow gate closes",
            ));
        }
        if self.bloom == CapabilityStatus::FeatureDisabled {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::BloomDisabled,
                "Bloom is disabled until the postprocessing pipeline has threshold, blur, and compositing proof",
                "do not market bloom; use the ACES output stage plus FXAA until the bloom gate closes",
            ));
        }
        if self.screen_space_ambient_occlusion == CapabilityStatus::FeatureDisabled {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::AmbientOcclusionDisabled,
                "Screen-space ambient occlusion is disabled until SSAO or GTAO has depth-aware visual proof",
                "do not market SSAO/GTAO; use authored occlusion textures or baked lighting until the ambient-occlusion gate closes",
            ));
        }
        if self.gpu_frustum_culling == CapabilityStatus::FeatureDisabled {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::GpuCullingDisabled,
                "GPU culling is disabled until the compute path writes real culling decisions",
                "use CPU culling diagnostics and draw statistics until the GPU culling gate closes",
            ));
        }
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

impl CapabilityReport {
    pub fn new(capabilities: Capabilities, adapter: Option<GpuAdapterReport>) -> Self {
        Self {
            capabilities,
            adapter,
            diagnostics: capabilities.diagnostics(),
        }
    }

    pub const fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub const fn backend(&self) -> Backend {
        self.capabilities.backend
    }

    pub fn adapter(&self) -> Option<&GpuAdapterReport> {
        self.adapter.as_ref()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

const fn forward_pbr_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::Degraded
}

const fn directional_shadow_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::Degraded
}

const fn punctual_shadow_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::FeatureDisabled
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

const fn postprocess_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::FeatureDisabled
}

const fn texture_compression_basisu_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::FeatureDisabled
}

const fn hardware_instancing_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

/// Phase 1F: backend-level support for sampling `texture_2d_array<f32>` /
/// `sampler2DArray`. WebGPU mandates 256+ layer support; WebGL2 GLES 3.0+
/// also exposes array textures. CPU-rasterizer backends keep the per-
/// material bind path.
const fn texture_arrays_status(backend: Backend) -> CapabilityStatus {
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
const fn max_texture_array_layers(backend: Backend) -> u32 {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2 => 256,
        Backend::Headless | Backend::SurfaceDescriptor => 0,
    }
}

const fn fragment_high_precision_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

const fn uniform_buffer_status(backend: Backend) -> CapabilityStatus {
    match backend {
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => {
            CapabilityStatus::Supported
        }
        Backend::Headless | Backend::SurfaceDescriptor | Backend::WebGl2 => {
            CapabilityStatus::FeatureDisabled
        }
    }
}

const fn uniform_buffer_max_bytes(backend: Backend) -> u32 {
    match backend {
        Backend::WebGl2 => 16_384,
        Backend::Headless | Backend::SurfaceDescriptor => 0,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu => 65_536,
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
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface
        | Backend::WebGpu
        | Backend::WebGl2 => CapabilityStatus::FeatureDisabled,
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
