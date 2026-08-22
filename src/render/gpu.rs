#[cfg(target_arch = "wasm32")]
mod browser_color_space;
#[cfg(target_arch = "wasm32")]
mod browser_meter;
mod browser_readback;
#[cfg(target_arch = "wasm32")]
mod browser_readback_trace;
mod build;
#[cfg(target_arch = "wasm32")]
mod debug;
mod depth;
#[cfg(not(target_arch = "wasm32"))]
mod draw;
mod draw_common;
#[cfg(not(target_arch = "wasm32"))]
mod draw_overlays;
#[cfg(target_arch = "wasm32")]
mod draw_surface;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
mod draw_surface_probe;
#[cfg(target_arch = "wasm32")]
mod draw_surface_support;
mod draw_uniform;
mod dynamic_draw_state;
mod environment;
#[cfg(not(target_arch = "wasm32"))]
mod headless_target;
mod instancing;
mod labels;
mod lifecycle;
mod light_assignment;
mod material_batched;
mod material_bindings;
mod material_mips;
mod material_support;
mod material_uniform;
mod material_upload;
mod materials;
mod msaa;
mod output;
mod overlays;
mod pipeline;
mod pipeline_requirements;
mod post;
#[cfg(not(target_arch = "wasm32"))]
mod prepare_resources;
mod prepare_resources_support;
#[cfg(target_arch = "wasm32")]
mod prepare_resources_wasm;
#[cfg(target_arch = "wasm32")]
mod prepare_resources_wasm_support;
#[cfg(not(target_arch = "wasm32"))]
mod readback;
mod resource_encoding;
mod scene_color;
#[cfg_attr(not(feature = "scene-host"), allow(dead_code))]
mod semantic_aov;
mod shader_manifest;
pub(crate) mod shading_tables;
mod shadow;
mod stats;
mod strokes;
mod surface_config;
mod surface_frame;
mod transmission;
mod vertices;

#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;
use crate::diagnostics::OutputColorSpace;
use crate::platform::SurfaceSize;

#[cfg(target_arch = "wasm32")]
use self::browser_readback::BrowserReadbackResources;
pub(in crate::render) use self::dynamic_draw_state::DynamicDrawStateUpdate;
use self::instancing::InstanceDrawBatch;
use self::labels::LabelResources;
use self::light_assignment::LightAssignmentResources;
use self::material_bindings::MaterialTextureBindingMode;
use self::pipeline::MeshPipelineSet;
pub(super) use self::post::{GpuOutputPlan, GpuPostPassCounts, GpuPostSettings};
use self::shadow::ShadowCasterResources;
pub(super) use self::stats::GpuResourceStats;
use self::strokes::StrokeResources;
pub(in crate::render) use self::surface_frame::SurfaceFrameSkipReason;
use self::vertices::{DrawUniformValue, PrimitiveDrawBatch};
use super::RasterTarget;
use super::prepare::PreparedGpuLightUniform;

#[allow(dead_code)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub(super) struct GpuDeviceState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<GpuSurfaceState>,
    runtime_fault: surface_frame::GpuRuntimeFaultState,
    // Enables the measured V3DV headless upload and target-alignment
    // workarounds. Attached surfaces and every other adapter stay untouched.
    #[cfg(not(target_arch = "wasm32"))]
    unstable_v3d_headless: bool,
    pending_destructions: u64,
    triangle_shader_modules: pipeline::TriangleShaderModuleCache,
    sample_count_capabilities: msaa::SampleCountCapabilityCache,
    #[cfg(not(target_arch = "wasm32"))]
    auto_exposure_meter: readback::GpuAutoExposureMeter,
    #[cfg(target_arch = "wasm32")]
    browser_auto_exposure_meter: browser_meter::BrowserAutoExposureMeter,
    #[cfg(target_arch = "wasm32")]
    #[cfg_attr(not(feature = "browser-probe"), allow(dead_code))]
    last_poll_observation: &'static str,
    resources: Option<GpuPreparedResources>,
    output_color_space: OutputColorSpace,
    display_p3_canvas_configured: bool,
    #[cfg(target_arch = "wasm32")]
    browser_canvas: Option<web_sys::HtmlCanvasElement>,
}

#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
impl GpuDeviceState {
    pub(in crate::render) async fn wait_for_submitted_browser_work(&self) -> Result<(), ()> {
        let window = web_sys::window().ok_or(())?;
        let completion = js_sys::Promise::new(&mut |resolve, _reject| {
            self.queue.on_submitted_work_done(move || {
                let _ = resolve.call0(&wasm_bindgen::JsValue::UNDEFINED);
            });
        });
        let timeout = js_sys::Promise::new(&mut move |_resolve, reject| {
            if let Err(error) =
                window.set_timeout_with_callback_and_timeout_and_arguments_0(&reject, 10_000)
            {
                let _ = reject.call1(&wasm_bindgen::JsValue::UNDEFINED, &error);
            }
        });
        let contenders = js_sys::Array::of2(&completion, &timeout);
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::race(&contenders))
            .await
            .map(|_| ())
            .map_err(|_| ())
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) use build::request_browser_surface_gpu;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use build::{request_headless_gpu, request_native_surface_gpu};

#[derive(Debug)]
pub(super) struct GpuSurfaceState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

fn material_texture_binding_mode(target: RasterTarget) -> MaterialTextureBindingMode {
    #[cfg(target_arch = "wasm32")]
    {
        if target.backend == Backend::WebGl2 {
            return MaterialTextureBindingMode::Texture2d;
        }
    }
    let _ = target;
    MaterialTextureBindingMode::Texture2dArray
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GpuPrepareOutcome {
    NoResources,
    FullRebuild,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct GpuRenderResult {
    pub(in crate::render) submitted: bool,
    pub(in crate::render) post_counts: GpuPostPassCounts,
    pub(in crate::render) draw_submissions: u64,
    pub(in crate::render) native_scene_color_passes: u64,
    pub(in crate::render) readback_copies: u64,
    pub(in crate::render) readback_bytes_copied: u64,
    pub(in crate::render) map_requests: u64,
    pub(in crate::render) blocking_polls: u64,
    pub(in crate::render) blocking_waits: u64,
    pub(in crate::render) cpu_frame_copy_bytes: u64,
    pub(in crate::render) auto_exposure_meter_submissions: u64,
    pub(in crate::render) auto_exposure_meter_samples: u64,
    pub(in crate::render) surface_skip: Option<surface_frame::SurfaceFrameSkipReason>,
    pub(in crate::render) surface_reconfigurations: u64,
    pub(in crate::render) surface_acquire_retries: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct GpuPreparedResources {
    target: RasterTarget,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: [wgpu::Buffer; 2],
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_buffer_capacity: usize,
    output_uniform: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
    opaque_output_bind_group: wgpu::BindGroup,
    reflection_probe_output_bind_groups: Vec<wgpu::BindGroup>,
    reflection_probe_opaque_output_bind_groups: Vec<wgpu::BindGroup>,
    surface_output_uniform: Option<wgpu::Buffer>,
    surface_output_bind_group: Option<wgpu::BindGroup>,
    surface_opaque_output_bind_group: Option<wgpu::BindGroup>,
    surface_reflection_probe_output_bind_groups: Vec<wgpu::BindGroup>,
    surface_reflection_probe_opaque_output_bind_groups: Vec<wgpu::BindGroup>,
    light_uniform: PreparedGpuLightUniform,
    #[allow(dead_code)]
    light_assignment: LightAssignmentResources,
    /// Phase 1B: directional-light view-projection. See `prepare/shadows.rs`.
    light_from_world: [f32; 16],
    material_resources: materials::MaterialResources,
    // Phase 1B/1C: directional shadow caster + env cubemap; always allocated
    // (1x1 placeholder when feature absent), gated by lighting uniform flags.
    shadow_caster: ShadowCasterResources,
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    environment_cubemap: wgpu::Texture,
    #[allow(dead_code)]
    reflection_probe_cubemaps: Vec<wgpu::Texture>,
    #[allow(dead_code)]
    environment_sampler: wgpu::Sampler,
    /// Kept alive for the lifetime of the output bind groups that reference it.
    #[allow(dead_code)]
    ltc_tables: wgpu::Buffer,
    /// Likewise. Replaces the `brdf_lut_texture` that used to be baked,
    /// uploaded and never bound because no texture unit was free for it.
    #[allow(dead_code)]
    brdf_table: wgpu::Buffer,
    transmission: transmission::TransmissionResources,
    depth_prepass: Option<depth::DepthPrepassResources>,
    overlay_depth_prepass: Option<depth::DepthPrepassResources>,
    strokes: Option<StrokeResources>,
    labels: Option<LabelResources>,
    semantic_aov: Option<semantic_aov::SemanticAovResources>,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    instance_batches: Vec<InstanceDrawBatch>,
    instance_count: usize,
    identity_instance: u32,
    // Phase 1A.2: per-draw uniforms via draw_uniform_buffer + draw_bind_group
    // with dynamic offsets. Vertex stream carries model-space positions; the
    // shader applies draw.world_from_model. Closes wgpu-architect F2.
    #[allow(dead_code)]
    draw_uniforms: Vec<DrawUniformValue>,
    draw_uniform_capacity: usize,
    #[allow(dead_code)]
    draw_uniform_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    post: Option<post::PostResources>,
    offscreen_pipelines: MeshPipelineSet,
    offscreen_msaa4_pipelines: MeshPipelineSet,
    offscreen_msaa8_pipelines: Option<MeshPipelineSet>,
    msaa_color: Option<MsaaColorResources>,
    surface_msaa_color: Option<MsaaColorResources>,
    surface_pipeline: Option<MeshPipelineSet>,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    stats: GpuResourceStats,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct MsaaColorResources {
    target: RasterTarget,
    format: wgpu::TextureFormat,
    sample_count: u32,
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct GpuPreparedResources {
    target: RasterTarget,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_buffer_capacity: usize,
    output_uniform: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
    opaque_output_bind_group: wgpu::BindGroup,
    reflection_probe_output_bind_groups: Vec<wgpu::BindGroup>,
    reflection_probe_opaque_output_bind_groups: Vec<wgpu::BindGroup>,
    light_uniform: PreparedGpuLightUniform,
    #[allow(dead_code)]
    light_assignment: LightAssignmentResources,
    /// Phase 1B: directional-light view-projection matrix; mirrors the
    /// native variant. Uploaded into the camera uniform's light_from_world
    /// slot.
    light_from_world: [f32; 16],
    material_resources: materials::MaterialResources,
    // Phase 1B/1C (wasm32 mirror): shadow caster + env cubemap, always
    // allocated; same gating as the native variant.
    shadow_caster: ShadowCasterResources,
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    environment_cubemap: wgpu::Texture,
    #[allow(dead_code)]
    reflection_probe_cubemaps: Vec<wgpu::Texture>,
    #[allow(dead_code)]
    environment_sampler: wgpu::Sampler,
    /// Kept alive for the lifetime of the output bind groups that reference it.
    #[allow(dead_code)]
    ltc_tables: wgpu::Buffer,
    /// Likewise. Replaces the `brdf_lut_texture` that used to be baked,
    /// uploaded and never bound because no texture unit was free for it.
    #[allow(dead_code)]
    brdf_table: wgpu::Buffer,
    transmission: transmission::TransmissionResources,
    depth_prepass: Option<depth::DepthPrepassResources>,
    strokes: Option<StrokeResources>,
    labels: Option<LabelResources>,
    semantic_aov: Option<semantic_aov::SemanticAovResources>,
    surface_pipeline: MeshPipelineSet,
    readback: Option<BrowserReadbackResources>,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    instance_batches: Vec<InstanceDrawBatch>,
    instance_count: usize,
    identity_instance: u32,
    // Phase 1A.2: per-draw uniforms uploaded through draw_uniform_buffer +
    // draw_bind_group with dynamic offsets, mirroring the native variant.
    #[allow(dead_code)]
    draw_uniforms: Vec<DrawUniformValue>,
    draw_uniform_capacity: usize,
    #[allow(dead_code)]
    draw_uniform_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    post: Option<post::PostResources>,
    stats: GpuResourceStats,
}

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn max_supported_sample_count_cached(&self, formats: &[wgpu::TextureFormat]) -> u32 {
        self.sample_count_capabilities
            .maximum_for_device(&self.device, &self.adapter, formats)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn measured_sample_count_maxima(&self) -> (u32, u32) {
        let render_maximum = self.max_supported_sample_count_cached(&[
            pipeline::GPU_COLOR_FORMAT,
            post::scene_color_format(),
            self.color_target_format(),
        ]);
        let depth_maximum =
            self.max_supported_sample_count_cached(&[wgpu::TextureFormat::Depth32Float]);
        (render_maximum, depth_maximum)
    }

    pub(super) fn sample_count_capability_probe_count(&self) -> u64 {
        self.sample_count_capabilities.probe_count()
    }

    pub(in crate::render) fn color_target_format(&self) -> wgpu::TextureFormat {
        self.surface
            .as_ref()
            .map_or(pipeline::GPU_COLOR_FORMAT, |surface| surface.config.format)
    }

    pub(in crate::render) fn color_target_format_name(&self) -> &'static str {
        match self.color_target_format() {
            wgpu::TextureFormat::Rgba8Unorm => "Rgba8Unorm",
            wgpu::TextureFormat::Rgba8UnormSrgb => "Rgba8UnormSrgb",
            wgpu::TextureFormat::Bgra8Unorm => "Bgra8Unorm",
            wgpu::TextureFormat::Bgra8UnormSrgb => "Bgra8UnormSrgb",
            _ => "Rgba8UnormSrgb",
        }
    }

    pub(super) const fn display_p3_canvas_configured(&self) -> bool {
        self.display_p3_canvas_configured
    }

    pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats {
        self.resources
            .as_ref()
            .map(|resources| resources.stats)
            .unwrap_or_default()
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(in crate::render) fn depth_prepass_has_color_target(&self) -> Option<bool> {
        self.resources
            .as_ref()
            .and_then(|resources| resources.depth_prepass.as_ref())
            .map(depth::DepthPrepassResources::depth_color_enabled)
    }

    pub(super) fn clamp_surface_size_to_device_limits(&self, size: SurfaceSize) -> SurfaceSize {
        build::clamp_surface_size_to_adapter_limits(
            size,
            self.device.limits().max_texture_dimension_2d,
        )
    }

    pub(super) fn surface_size(&self) -> Option<SurfaceSize> {
        self.surface.as_ref().map(|surface| SurfaceSize {
            width: surface.config.width,
            height: surface.config.height,
        })
    }

    pub(super) const fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(in crate::render) fn draw_vertex_ranges_for_test(&self) -> Vec<(u32, u32)> {
        self.resources
            .as_ref()
            .map(|resources| {
                resources
                    .draw_batches
                    .iter()
                    .map(|batch| (batch.start_vertex, batch.vertex_count))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(in crate::render) fn vertex_buffer_bytes_for_test(&self) -> Option<u64> {
        self.resources
            .as_ref()
            .map(|resources| u64::from(resources.vertex_count) * vertices::VERTEX_BYTE_LEN as u64)
    }
}
