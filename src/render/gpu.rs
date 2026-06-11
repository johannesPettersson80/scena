#[cfg(target_arch = "wasm32")]
mod browser_color_space;
#[cfg(target_arch = "wasm32")]
mod browser_exposure;
mod browser_readback;
mod build;
#[cfg(target_arch = "wasm32")]
mod debug;
mod depth;
mod draw;
mod draw_uniform;
mod environment;
mod lifecycle;
mod material_batched;
mod material_bindings;
mod material_mips;
mod material_uniform;
mod material_upload;
mod materials;
mod output;
mod pipeline;
mod prepare_resources;
mod scene_color;
mod shadow;
mod stats;
mod surface_config;
mod transmission;
mod vertices;

#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;
use crate::diagnostics::OutputColorSpace;
use crate::platform::SurfaceSize;

#[cfg(target_arch = "wasm32")]
use self::browser_readback::BrowserReadbackResources;
use self::material_bindings::MaterialTextureBindingMode;
use self::shadow::ShadowCasterResources;
pub(super) use self::stats::GpuResourceStats;
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
    pending_destructions: u64,
    resources: Option<GpuPreparedResources>,
    output_color_space: OutputColorSpace,
    display_p3_canvas_configured: bool,
    #[cfg(target_arch = "wasm32")]
    browser_canvas: Option<web_sys::HtmlCanvasElement>,
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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct GpuPreparedResources {
    target: RasterTarget,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    output_uniform: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
    opaque_output_bind_group: wgpu::BindGroup,
    light_uniform: PreparedGpuLightUniform,
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
    environment_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    brdf_lut_texture: wgpu::Texture,
    transmission: transmission::TransmissionResources,
    depth_prepass: Option<depth::DepthPrepassResources>,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    // Phase 1A.2: per-draw uniforms via draw_uniform_buffer + draw_bind_group
    // with dynamic offsets. Vertex stream carries model-space positions; the
    // shader applies draw.world_from_model. Closes wgpu-architect F2.
    #[allow(dead_code)]
    draw_uniforms: Vec<DrawUniformValue>,
    draw_uniform_capacity: usize,
    #[allow(dead_code)]
    draw_uniform_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    offscreen_pipeline: wgpu::RenderPipeline,
    surface_pipeline: Option<wgpu::RenderPipeline>,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    stats: GpuResourceStats,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct GpuPreparedResources {
    target: RasterTarget,
    vertex_buffer: wgpu::Buffer,
    output_uniform: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
    opaque_output_bind_group: wgpu::BindGroup,
    light_uniform: PreparedGpuLightUniform,
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
    environment_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    brdf_lut_texture: wgpu::Texture,
    transmission: transmission::TransmissionResources,
    depth_prepass: Option<depth::DepthPrepassResources>,
    surface_pipeline: wgpu::RenderPipeline,
    readback: Option<BrowserReadbackResources>,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    // Phase 1A.2: per-draw uniforms uploaded through draw_uniform_buffer +
    // draw_bind_group with dynamic offsets, mirroring the native variant.
    #[allow(dead_code)]
    draw_uniforms: Vec<DrawUniformValue>,
    draw_uniform_capacity: usize,
    #[allow(dead_code)]
    draw_uniform_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    stats: GpuResourceStats,
}

impl GpuDeviceState {
    pub(super) const fn display_p3_canvas_configured(&self) -> bool {
        self.display_p3_canvas_configured
    }

    pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats {
        self.resources
            .as_ref()
            .map(|resources| resources.stats)
            .unwrap_or_default()
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
}
