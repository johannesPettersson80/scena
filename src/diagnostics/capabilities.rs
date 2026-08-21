use super::capability_status::*;
use super::post_processing::PostProcessingReportV1;
use super::{Diagnostic, DiagnosticCode};
use serde::{Deserialize, Deserializer, Serialize};

mod capability_probe;
mod capability_types;
mod color_formats;
mod sample_counts;
pub use capability_probe::{
    CapabilityConstraintProbeV1, CapabilityConstraintStatusV1, CapabilityProbeModeV1,
    CapabilityProbeStatusV1, CapabilityProbeUnavailableV1, CapabilityProbeV1,
    CapabilityTargetProbeV1, GpuDeviceReport,
};
pub use capability_types::{
    AlphaPipelineStatus, Backend, CapabilityStatus, HardwareTier, OutputColorSpace,
    OutputStageStatus,
};
use color_formats::static_color_target_format;
#[cfg(not(target_arch = "wasm32"))]
use sample_counts::measured_sample_counts;
use sample_counts::{
    explicit_msaa_default, explicit_msaa_status, renderer_sample_counts, single_sample_counts,
};

pub const CAPABILITY_REPORT_SCHEMA_V1: &str = "scena.capability_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterLimitsReport {
    pub max_texture_dimension_2d: u32,
    pub max_bind_groups: u32,
    pub max_uniform_buffer_binding_size: u64,
    pub max_vertex_attributes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    capabilities: Capabilities,
    adapter: Option<GpuAdapterReport>,
    post_processing: Option<PostProcessingReportV1>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReportV1 {
    pub schema: String,
    pub capabilities: Capabilities,
    pub adapter: Option<GpuAdapterReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_processing: Option<PostProcessingReportV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe: Option<CapabilityProbeV1>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Capabilities {
    pub backend: Backend,
    pub color_target_format: &'static str,
    /// Renderer-owned color attachment sample counts. Zero entries are unused.
    pub render_sample_counts: [u32; 3],
    /// Renderer-owned depth attachment sample counts. Zero entries are unused.
    pub depth_sample_counts: [u32; 3],
    /// Whether an exact caller-selected multisample count can be honored.
    pub explicit_msaa: CapabilityStatus,
    pub gpu_device: bool,
    pub surface_attached: bool,
    pub hardware_tier: HardwareTier,
    pub output_stage: OutputStageStatus,
    pub alpha_pipeline: AlphaPipelineStatus,
    pub forward_pbr: CapabilityStatus,
    /// Whether an explicit `photo.quality = "final"` request can run without
    /// backend degradation. Unsupported backends must return
    /// `final_photo_unsupported`.
    pub final_photo: CapabilityStatus,
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
    pub order_independent_transparency: CapabilityStatus,
    pub physical_glass_transmission: CapabilityStatus,
    pub wide_gamut_output: CapabilityStatus,
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
    pub subject_visible_mask: CapabilityStatus,
    pub auto_exposure_metering_average: CapabilityStatus,
    pub auto_exposure_metering_center_weighted: CapabilityStatus,
    pub auto_exposure_metering_highlight_weighted: CapabilityStatus,
    pub auto_exposure_metering_subject: CapabilityStatus,
    pub auto_exposure_metering_spot: CapabilityStatus,
    pub compute_shaders: CapabilityStatus,
    pub storage_buffers: CapabilityStatus,
    pub readback_headless_screenshots: CapabilityStatus,
    pub reversed_z_depth: CapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CapabilitiesDeserialize {
    backend: Backend,
    color_target_format: String,
    #[serde(default = "single_sample_counts")]
    render_sample_counts: [u32; 3],
    #[serde(default = "single_sample_counts")]
    depth_sample_counts: [u32; 3],
    #[serde(default = "explicit_msaa_default")]
    explicit_msaa: CapabilityStatus,
    gpu_device: bool,
    surface_attached: bool,
    hardware_tier: HardwareTier,
    output_stage: OutputStageStatus,
    alpha_pipeline: AlphaPipelineStatus,
    forward_pbr: CapabilityStatus,
    #[serde(default = "final_photo_error_if_required_default")]
    final_photo: CapabilityStatus,
    directional_shadows: CapabilityStatus,
    point_shadows: CapabilityStatus,
    spot_shadows: CapabilityStatus,
    directional_shadow_map_default_size: u32,
    directional_shadow_map_max_size: u32,
    directional_shadow_pcf_kernel: u8,
    ibl_cubemap_default_size: u32,
    ibl_brdf_lut_default_size: u32,
    bloom: CapabilityStatus,
    screen_space_ambient_occlusion: CapabilityStatus,
    order_independent_transparency: CapabilityStatus,
    physical_glass_transmission: CapabilityStatus,
    wide_gamut_output: CapabilityStatus,
    texture_compression_basisu: CapabilityStatus,
    hardware_instancing: CapabilityStatus,
    texture_arrays: CapabilityStatus,
    max_texture_array_layers: u32,
    fragment_high_precision: CapabilityStatus,
    uniform_buffers: CapabilityStatus,
    uniform_buffer_max_bytes: u32,
    default_clipping_planes: u8,
    max_clipping_planes: u8,
    gpu_frustum_culling: CapabilityStatus,
    per_instance_culling: CapabilityStatus,
    #[serde(default = "subject_visible_mask_default")]
    subject_visible_mask: CapabilityStatus,
    #[serde(default = "metering_supported_default")]
    auto_exposure_metering_average: CapabilityStatus,
    #[serde(default = "metering_feature_disabled_default")]
    auto_exposure_metering_center_weighted: CapabilityStatus,
    #[serde(default = "metering_feature_disabled_default")]
    auto_exposure_metering_highlight_weighted: CapabilityStatus,
    #[serde(default = "metering_feature_disabled_default")]
    auto_exposure_metering_subject: CapabilityStatus,
    #[serde(default = "metering_feature_disabled_default")]
    auto_exposure_metering_spot: CapabilityStatus,
    compute_shaders: CapabilityStatus,
    storage_buffers: CapabilityStatus,
    readback_headless_screenshots: CapabilityStatus,
    reversed_z_depth: CapabilityStatus,
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = CapabilitiesDeserialize::deserialize(deserializer)?;
        Ok(Self {
            backend: value.backend,
            color_target_format: static_color_target_format(&value.color_target_format)?,
            render_sample_counts: value.render_sample_counts,
            depth_sample_counts: value.depth_sample_counts,
            explicit_msaa: value.explicit_msaa,
            gpu_device: value.gpu_device,
            surface_attached: value.surface_attached,
            hardware_tier: value.hardware_tier,
            output_stage: value.output_stage,
            alpha_pipeline: value.alpha_pipeline,
            forward_pbr: value.forward_pbr,
            final_photo: value.final_photo,
            directional_shadows: value.directional_shadows,
            point_shadows: value.point_shadows,
            spot_shadows: value.spot_shadows,
            directional_shadow_map_default_size: value.directional_shadow_map_default_size,
            directional_shadow_map_max_size: value.directional_shadow_map_max_size,
            directional_shadow_pcf_kernel: value.directional_shadow_pcf_kernel,
            ibl_cubemap_default_size: value.ibl_cubemap_default_size,
            ibl_brdf_lut_default_size: value.ibl_brdf_lut_default_size,
            bloom: value.bloom,
            screen_space_ambient_occlusion: value.screen_space_ambient_occlusion,
            order_independent_transparency: value.order_independent_transparency,
            physical_glass_transmission: value.physical_glass_transmission,
            wide_gamut_output: value.wide_gamut_output,
            texture_compression_basisu: value.texture_compression_basisu,
            hardware_instancing: value.hardware_instancing,
            texture_arrays: value.texture_arrays,
            max_texture_array_layers: value.max_texture_array_layers,
            fragment_high_precision: value.fragment_high_precision,
            uniform_buffers: value.uniform_buffers,
            uniform_buffer_max_bytes: value.uniform_buffer_max_bytes,
            default_clipping_planes: value.default_clipping_planes,
            max_clipping_planes: value.max_clipping_planes,
            gpu_frustum_culling: value.gpu_frustum_culling,
            per_instance_culling: value.per_instance_culling,
            subject_visible_mask: value.subject_visible_mask,
            auto_exposure_metering_average: value.auto_exposure_metering_average,
            auto_exposure_metering_center_weighted: value.auto_exposure_metering_center_weighted,
            auto_exposure_metering_highlight_weighted: value
                .auto_exposure_metering_highlight_weighted,
            auto_exposure_metering_subject: value.auto_exposure_metering_subject,
            auto_exposure_metering_spot: value.auto_exposure_metering_spot,
            compute_shaders: value.compute_shaders,
            storage_buffers: value.storage_buffers,
            readback_headless_screenshots: value.readback_headless_screenshots,
            reversed_z_depth: value.reversed_z_depth,
        })
    }
}

fn subject_visible_mask_default() -> CapabilityStatus {
    CapabilityStatus::FeatureDisabled
}

fn metering_supported_default() -> CapabilityStatus {
    CapabilityStatus::Supported
}

fn metering_feature_disabled_default() -> CapabilityStatus {
    CapabilityStatus::FeatureDisabled
}

fn final_photo_error_if_required_default() -> CapabilityStatus {
    CapabilityStatus::ErrorIfRequired
}

impl Capabilities {
    pub const fn headless() -> Self {
        Self::for_backend(Backend::Headless)
    }

    pub const fn for_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            render_sample_counts: renderer_sample_counts(backend),
            depth_sample_counts: renderer_sample_counts(backend),
            explicit_msaa: explicit_msaa_status(backend),
            gpu_device: false,
            surface_attached: false,
            hardware_tier: hardware_tier(backend, false),
            output_stage: OutputStageStatus::PbrNeutralSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            forward_pbr: forward_pbr_status(backend, false),
            final_photo: final_photo_status(backend, false),
            directional_shadows: directional_shadow_status(backend, false),
            point_shadows: punctual_shadow_status(backend),
            spot_shadows: punctual_shadow_status(backend),
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_brdf_lut_size(backend),
            bloom: bloom_status(backend),
            screen_space_ambient_occlusion: ambient_occlusion_status(backend),
            order_independent_transparency: order_independent_transparency_status(backend),
            physical_glass_transmission: physical_glass_transmission_status(backend, false),
            wide_gamut_output: wide_gamut_output_status(backend, false),
            texture_compression_basisu: CapabilityStatus::FeatureDisabled,
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
            subject_visible_mask: subject_visible_mask_status(backend),
            auto_exposure_metering_average: auto_exposure_metering_average_status(backend),
            auto_exposure_metering_center_weighted: auto_exposure_metering_unimplemented_status(
                backend,
            ),
            auto_exposure_metering_highlight_weighted: auto_exposure_metering_unimplemented_status(
                backend,
            ),
            auto_exposure_metering_subject: auto_exposure_metering_subject_status(backend),
            auto_exposure_metering_spot: auto_exposure_metering_unimplemented_status(backend),
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
            render_sample_counts: renderer_sample_counts(backend),
            depth_sample_counts: renderer_sample_counts(backend),
            explicit_msaa: explicit_msaa_status(backend),
            gpu_device: true,
            surface_attached: false,
            hardware_tier: hardware_tier(backend, true),
            output_stage: OutputStageStatus::PbrNeutralSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            forward_pbr: forward_pbr_status(backend, true),
            final_photo: final_photo_status(backend, true),
            directional_shadows: directional_shadow_status(backend, true),
            point_shadows: punctual_shadow_status(backend),
            spot_shadows: punctual_shadow_status(backend),
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_brdf_lut_size(backend),
            bloom: bloom_status(backend),
            screen_space_ambient_occlusion: ambient_occlusion_status(backend),
            order_independent_transparency: order_independent_transparency_status(backend),
            physical_glass_transmission: physical_glass_transmission_status(backend, true),
            wide_gamut_output: wide_gamut_output_status(backend, false),
            texture_compression_basisu: CapabilityStatus::FeatureDisabled,
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
            subject_visible_mask: subject_visible_mask_status(backend),
            auto_exposure_metering_average: auto_exposure_metering_average_status(backend),
            auto_exposure_metering_center_weighted: auto_exposure_metering_unimplemented_status(
                backend,
            ),
            auto_exposure_metering_highlight_weighted: auto_exposure_metering_unimplemented_status(
                backend,
            ),
            auto_exposure_metering_subject: auto_exposure_metering_subject_status(backend),
            auto_exposure_metering_spot: auto_exposure_metering_unimplemented_status(backend),
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
            render_sample_counts: renderer_sample_counts(backend),
            depth_sample_counts: renderer_sample_counts(backend),
            explicit_msaa: explicit_msaa_status(backend),
            gpu_device: true,
            surface_attached: true,
            hardware_tier: hardware_tier(backend, true),
            output_stage: OutputStageStatus::PbrNeutralSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
            forward_pbr: forward_pbr_status(backend, true),
            final_photo: final_photo_status(backend, true),
            directional_shadows: directional_shadow_status(backend, true),
            point_shadows: punctual_shadow_status(backend),
            spot_shadows: punctual_shadow_status(backend),
            directional_shadow_map_default_size: directional_shadow_map_default_size(backend),
            directional_shadow_map_max_size: directional_shadow_map_max_size(backend),
            directional_shadow_pcf_kernel: 3,
            ibl_cubemap_default_size: ibl_default_size(backend),
            ibl_brdf_lut_default_size: ibl_brdf_lut_size(backend),
            bloom: bloom_status(backend),
            screen_space_ambient_occlusion: ambient_occlusion_status(backend),
            order_independent_transparency: order_independent_transparency_status(backend),
            physical_glass_transmission: physical_glass_transmission_status(backend, true),
            wide_gamut_output: wide_gamut_output_status(backend, true),
            texture_compression_basisu: CapabilityStatus::FeatureDisabled,
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
            subject_visible_mask: subject_visible_mask_status(backend),
            auto_exposure_metering_average: auto_exposure_metering_average_status(backend),
            auto_exposure_metering_center_weighted: auto_exposure_metering_unimplemented_status(
                backend,
            ),
            auto_exposure_metering_highlight_weighted: auto_exposure_metering_unimplemented_status(
                backend,
            ),
            auto_exposure_metering_subject: auto_exposure_metering_subject_status(backend),
            auto_exposure_metering_spot: auto_exposure_metering_unimplemented_status(backend),
            compute_shaders: compute_shader_status(backend),
            storage_buffers: storage_buffer_status(backend),
            readback_headless_screenshots: readback_status(backend),
            reversed_z_depth: reversed_z_depth_status(backend),
        }
    }

    pub const fn with_display_p3_output(mut self, canvas_configured: bool) -> Self {
        if canvas_configured {
            match (self.backend, self.surface_attached) {
                (Backend::WebGpu | Backend::WebGl2, true) => {}
                _ => return self,
            }
            self.color_target_format = "Rgba8UnormSrgb+DisplayP3Canvas";
            self.output_stage = OutputStageStatus::PbrNeutralDisplayP3;
            self.wide_gamut_output = CapabilityStatus::Supported;
        }
        self
    }

    pub(crate) const fn with_color_target_format(
        mut self,
        color_target_format: &'static str,
    ) -> Self {
        self.color_target_format = color_target_format;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const fn with_measured_sample_count_maxima(
        mut self,
        render_maximum: u32,
        depth_maximum: u32,
    ) -> Self {
        self.render_sample_counts = measured_sample_counts(render_maximum);
        self.depth_sample_counts = measured_sample_counts(depth_maximum);
        if matches!(self.backend, Backend::HeadlessGpu | Backend::NativeSurface) {
            self.explicit_msaa = if render_maximum >= 4 && depth_maximum >= 4 {
                CapabilityStatus::Supported
            } else {
                CapabilityStatus::ErrorIfRequired
            };
        }
        self
    }

    pub fn diagnostics(self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.forward_pbr == CapabilityStatus::Degraded {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::ForwardPbrDegraded,
                "PBR is reported as degraded until GPU material, texture, and IBL shading are proven",
                "treat metallic-roughness output as a compatibility preview until the PBR visual gate closes",
            ));
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::MaterialPresetFallback,
                "Complete real-world material presets are in fallback on this lane: \
                 chrome, brushed_steel, clearcoat_plastic, clear_glass, \
                 frosted_glass, leather, rubber, and satin require approved \
                 material proof before they can be claimed as complete",
                "use MaterialDesc::* as scalar preview shortcuts, or claim \
                 Assets::material_presets() only for lanes with Round E \
                 reference, browser, and capability row proof artifacts",
            ));
        }
        if self.directional_shadows == CapabilityStatus::Degraded {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::DirectionalShadowsDegraded,
                "Directional shadows are degraded on CPU/reference lanes without GPU shadow-map receiver proof",
                "query the active backend before enabling shadow-dependent presentation; GPU-device WebGPU/WebGL2/native lanes carry receiver-darkening proof",
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
        if self.order_independent_transparency == CapabilityStatus::FeatureDisabled {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::OrderIndependentTransparencyDisabled,
                "Order-independent transparency is disabled until a backend has overlap order-invariance proof",
                "sort alpha-blended surfaces back-to-front or use opaque/masked fallbacks until the OIT gate closes for this backend",
            ));
        }
        if self.physical_glass_transmission != CapabilityStatus::Supported {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::PhysicalGlassTransmissionDegraded,
                "Physical glass transmission is degraded until the lane proves scene-color transmission, IOR/thickness refraction, rough-transmission blur, and transparency ordering",
                "do not claim clear_glass/frosted_glass as complete real-world materials until the Round E glass and OIT proof artifacts pass for this backend",
            ));
        }
        if self.wide_gamut_output != CapabilityStatus::Supported {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::WideGamutOutputUnavailable,
                "Wide-gamut output needs a browser surface color-space probe before Display P3 can be claimed",
                "treat output as sRGB unless a recorded canvas probe reports Display P3 for the active backend",
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
            post_processing: None,
            diagnostics: capabilities.diagnostics(),
        }
    }

    pub fn new_with_post_processing(
        capabilities: Capabilities,
        adapter: Option<GpuAdapterReport>,
        post_processing: PostProcessingReportV1,
    ) -> Self {
        Self {
            capabilities,
            adapter,
            post_processing: Some(post_processing),
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

    pub fn post_processing(&self) -> Option<&PostProcessingReportV1> {
        self.post_processing.as_ref()
    }

    pub fn to_schema_report(&self) -> CapabilityReportV1 {
        CapabilityReportV1 {
            schema: CAPABILITY_REPORT_SCHEMA_V1.to_owned(),
            capabilities: self.capabilities,
            adapter: self.adapter.clone(),
            post_processing: self.post_processing.clone(),
            probe: None,
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self.to_schema_report())
            .expect("capability report schema contains only serializable fields")
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_gpu_sample_counts_are_conservative_until_a_device_is_measured() {
        let unprobed = Capabilities::for_gpu_backend(Backend::HeadlessGpu);
        assert_eq!(unprobed.render_sample_counts, [1, 0, 0]);
        assert_eq!(unprobed.depth_sample_counts, [1, 0, 0]);
        assert_eq!(unprobed.explicit_msaa, CapabilityStatus::ErrorIfRequired);

        let measured = unprobed.with_measured_sample_count_maxima(4, 4);
        assert_eq!(measured.render_sample_counts, [1, 4, 0]);
        assert_eq!(measured.depth_sample_counts, [1, 4, 0]);
        assert_eq!(measured.explicit_msaa, CapabilityStatus::Supported);

        let mismatched = unprobed.with_measured_sample_count_maxima(8, 4);
        assert_eq!(mismatched.render_sample_counts, [1, 4, 8]);
        assert_eq!(mismatched.depth_sample_counts, [1, 4, 0]);
        assert_eq!(mismatched.explicit_msaa, CapabilityStatus::Supported);
    }
}
