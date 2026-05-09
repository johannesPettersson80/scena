mod build;
mod depth;
mod draw;
mod lifecycle;
mod material_mips;
mod material_uniform;
mod materials;
mod output;
mod pipeline;
#[cfg(not(target_arch = "wasm32"))]
mod shadow;
mod stats;
mod vertices;
#[cfg(target_arch = "wasm32")]
mod webgl2;
#[cfg(target_arch = "wasm32")]
mod webgl2_camera;
#[cfg(target_arch = "wasm32")]
mod webgl2_lighting;
#[cfg(target_arch = "wasm32")]
mod webgl2_materials;
#[cfg(target_arch = "wasm32")]
mod webgl2_program;
#[cfg(target_arch = "wasm32")]
mod webgl2_texture_set;
#[cfg(target_arch = "wasm32")]
mod webgl2_vertices;

#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;
use crate::diagnostics::{AdapterLimitsReport, GpuAdapterReport};
#[cfg(not(target_arch = "wasm32"))]
use crate::geometry::Primitive;

use self::materials::{
    create_material_bind_group_layout, create_material_resources, material_texture_byte_len,
};
use self::output::{
    create_output_bind_group, create_output_bind_group_layout, create_output_uniform_buffer,
};
use self::pipeline::create_unlit_pipeline;
#[cfg(not(target_arch = "wasm32"))]
use self::pipeline::{BYTES_PER_PIXEL, GPU_COLOR_FORMAT};
#[cfg(not(target_arch = "wasm32"))]
use self::shadow::create_shadow_texture;
pub(super) use self::stats::GpuResourceStats;
#[cfg(not(target_arch = "wasm32"))]
use self::stats::align_to;
use self::stats::{PreparedResourceEstimateInput, estimate_prepared_resource_stats};
use self::vertices::{
    DrawUniformValue, PrimitiveDrawBatch, VERTEX_BYTE_LEN, encode_draw_batches, encode_vertices,
};
use super::RasterTarget;
use super::prepare::{
    PreparedDepthStats, PreparedGpuLightUniform, PreparedLightingStats, PreparedMaterialSlot,
};

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct GpuDeviceState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<GpuSurfaceState>,
    pending_destructions: u64,
    resources: Option<GpuPreparedResources>,
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
    light_uniform: PreparedGpuLightUniform,
    /// Phase 1B: directional-light view-projection matrix (orthographic) that
    /// maps world-space → light-clip-space. Computed from the active
    /// directional light direction + the AABB of shadow occluders in
    /// `prepare/shadows.rs::directional_light_view_projection`. Consumed by
    /// the shadow caster pass + fragment-shader shadow sampling.
    light_from_world: [f32; 16],
    material_resources: Vec<materials::MaterialTextureResources>,
    // ARCH-SHADOW-MAP: M2 allocates shadow resources before the shadow render pass is
    // wired; the explicit fields keep the deferred binding visible to reviews and doctor.
    #[allow(dead_code)]
    shadow_texture: Option<wgpu::Texture>,
    #[allow(dead_code)]
    shadow_view: Option<wgpu::TextureView>,
    depth_prepass: Option<depth::DepthPrepassResources>,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    // Phase 1A.2: per-draw world_from_model / normal_from_model uniforms uploaded
    // through draw_uniform_buffer + draw_bind_group with dynamic offsets. The GPU
    // vertex stream carries MODEL-SPACE positions (encode_vertices applies the
    // matrix inverse on CPU); the shader applies draw.world_from_model to recover
    // world space. The buffer + Vec are retained on the resources struct so the
    // GPU object lifetime tracks the prepared scene; the bind group is the live
    // consumer at draw time. Closes scena-wgpu-architect Phase 6 finding F2.
    #[allow(dead_code)]
    draw_uniforms: Vec<DrawUniformValue>,
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
    light_uniform: PreparedGpuLightUniform,
    /// Phase 1B: directional-light view-projection matrix; mirrors the
    /// native variant. Uploaded into the camera uniform's light_from_world
    /// slot.
    light_from_world: [f32; 16],
    material_resources: Vec<materials::MaterialTextureResources>,
    depth_prepass: Option<depth::DepthPrepassResources>,
    surface_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    // Phase 1A.2: per-draw uniforms uploaded through draw_uniform_buffer +
    // draw_bind_group with dynamic offsets, mirroring the native variant.
    #[allow(dead_code)]
    draw_uniforms: Vec<DrawUniformValue>,
    #[allow(dead_code)]
    draw_uniform_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    webgl2_vertices: Vec<f32>,
    stats: GpuResourceStats,
}

impl GpuDeviceState {
    pub(super) fn adapter_report(&self) -> GpuAdapterReport {
        let info = self.adapter.get_info();
        let limits = self.adapter.limits();
        GpuAdapterReport {
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            vendor: info.vendor,
            device: info.device,
            driver: info.driver,
            driver_info: info.driver_info,
            features: format!("{:?}", self.adapter.features()),
            limits: AdapterLimitsReport {
                max_texture_dimension_2d: limits.max_texture_dimension_2d,
                max_bind_groups: limits.max_bind_groups,
                max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
                max_vertex_attributes: limits.max_vertex_attributes,
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[Primitive],
        lighting_stats: PreparedLightingStats,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        depth_stats: PreparedDepthStats,
        material_slots: &[PreparedMaterialSlot],
    ) {
        self.configure_surface(target);
        self.release_prepared_resources();
        if primitives.is_empty() {
            return;
        }

        let vertex_bytes = encode_vertices(primitives);
        let (draw_batches, draw_uniforms) = encode_draw_batches(primitives);
        let vertex_buffer_size = vertex_bytes.len().max(4) as u64;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.m0.scene_vertices"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        if !vertex_bytes.is_empty() {
            let mut mapped = vertex_buffer.slice(..).get_mapped_range_mut();
            mapped.copy_from_slice(&vertex_bytes);
        }
        vertex_buffer.unmap();

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scena.headless_gpu.target"),
            size: wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: GPU_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
        let padded_bytes_per_row =
            align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.headless_gpu.readback"),
            size: u64::from(padded_bytes_per_row) * u64::from(target.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let output_bind_group_layout = create_output_bind_group_layout(&self.device);
        let material_bind_group_layout = create_material_bind_group_layout(&self.device);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let output_bind_group =
            create_output_bind_group(&self.device, &output_bind_group_layout, &output_uniform);
        let material_resources = create_material_resources(
            &self.device,
            &self.queue,
            &material_bind_group_layout,
            material_slots,
        );
        let shadow_texture = create_shadow_texture(
            &self.device,
            lighting_stats.directional_shadow_map_resolution,
        );
        let shadow_view = shadow_texture
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let draw_bind_group_layout = output::create_draw_bind_group_layout(&self.device);
        let draw_uniform_buffer =
            output::create_draw_uniform_buffer(&self.device, draw_uniforms.len() as u64);
        let draw_uniform_pairs: Vec<([f32; 16], [f32; 16])> = draw_uniforms
            .iter()
            .map(|value| (value.world_from_model, value.normal_from_model))
            .collect();
        self.queue.write_buffer(
            &draw_uniform_buffer,
            0,
            &output::encode_draw_uniform_bytes(&draw_uniform_pairs),
        );
        let draw_bind_group = output::create_draw_bind_group(
            &self.device,
            &draw_bind_group_layout,
            &draw_uniform_buffer,
        );
        let depth_prepass = (depth_stats.passes > 0).then(|| {
            depth::create_depth_prepass_resources(
                &self.device,
                target,
                depth_stats.reversed_z,
                &output_bind_group_layout,
                &draw_bind_group_layout,
            )
        });
        let depth_compare = depth_prepass
            .as_ref()
            .map(|depth_prepass| depth_prepass.color_compare);
        let offscreen_pipeline = create_unlit_pipeline(
            &self.device,
            GPU_COLOR_FORMAT,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            depth_compare,
        );
        let surface_pipeline = self.surface.as_ref().map(|surface| {
            create_unlit_pipeline(
                &self.device,
                surface.config.format,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                depth_compare,
            )
        });
        let stats = estimate_prepared_resource_stats(PreparedResourceEstimateInput {
            target,
            vertex_count: vertex_bytes.len() / VERTEX_BYTE_LEN,
            has_surface_pipeline: surface_pipeline.is_some(),
            shadow_maps: lighting_stats.shadow_maps,
            shadow_map_resolution: lighting_stats.directional_shadow_map_resolution,
            depth_prepass_passes: depth_stats.passes,
            material_texture_count: material_resources.len() as u64,
            material_texture_bytes: material_texture_byte_len(&material_resources),
        });

        self.resources = Some(GpuPreparedResources {
            target,
            texture,
            view,
            readback,
            vertex_buffer,
            output_uniform,
            output_bind_group,
            light_uniform,
            light_from_world,
            material_resources,
            shadow_texture,
            shadow_view,
            depth_prepass,
            vertex_count: (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32,
            draw_batches,
            draw_uniforms,
            draw_uniform_buffer,
            draw_bind_group,
            offscreen_pipeline,
            surface_pipeline,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            stats,
        });
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[crate::geometry::Primitive],
        lighting_stats: PreparedLightingStats,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        depth_stats: PreparedDepthStats,
        material_slots: &[PreparedMaterialSlot],
    ) {
        let _ = lighting_stats;
        self.configure_surface(target);
        self.release_prepared_resources();
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        if primitives.is_empty() {
            return;
        }
        let vertex_bytes = encode_vertices(primitives);
        let (draw_batches, draw_uniforms) = encode_draw_batches(primitives);
        let webgl2_vertices = webgl2::encode_vertices(primitives);
        if target.backend == Backend::WebGl2 {
            let Some(canvas) = self.browser_canvas.as_ref() else {
                return;
            };
            if webgl2::prepare_canvas_vertices(
                canvas,
                &webgl2_vertices,
                &draw_batches,
                material_slots,
            )
            .is_err()
            {
                return;
            }
        }
        let vertex_buffer_size = vertex_bytes.len().max(4) as u64;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.browser.scene_vertices"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        if !vertex_bytes.is_empty() {
            let mut mapped = vertex_buffer.slice(..).get_mapped_range_mut();
            mapped.copy_from_slice(&vertex_bytes);
        }
        vertex_buffer.unmap();

        let output_bind_group_layout = create_output_bind_group_layout(&self.device);
        let material_bind_group_layout = create_material_bind_group_layout(&self.device);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let output_bind_group =
            create_output_bind_group(&self.device, &output_bind_group_layout, &output_uniform);
        let material_resources = create_material_resources(
            &self.device,
            &self.queue,
            &material_bind_group_layout,
            material_slots,
        );
        let draw_bind_group_layout = output::create_draw_bind_group_layout(&self.device);
        let draw_uniform_buffer =
            output::create_draw_uniform_buffer(&self.device, draw_uniforms.len() as u64);
        let draw_uniform_pairs: Vec<([f32; 16], [f32; 16])> = draw_uniforms
            .iter()
            .map(|value| (value.world_from_model, value.normal_from_model))
            .collect();
        self.queue.write_buffer(
            &draw_uniform_buffer,
            0,
            &output::encode_draw_uniform_bytes(&draw_uniform_pairs),
        );
        let draw_bind_group = output::create_draw_bind_group(
            &self.device,
            &draw_bind_group_layout,
            &draw_uniform_buffer,
        );
        let depth_prepass =
            (target.backend == Backend::WebGpu && depth_stats.passes > 0).then(|| {
                depth::create_depth_prepass_resources(
                    &self.device,
                    target,
                    depth_stats.reversed_z,
                    &output_bind_group_layout,
                    &draw_bind_group_layout,
                )
            });
        let depth_compare = depth_prepass
            .as_ref()
            .map(|depth_prepass| depth_prepass.color_compare);
        let surface_pipeline = create_unlit_pipeline(
            &self.device,
            surface.config.format,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            depth_compare,
        );
        let vertex_count = (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32;
        let stats = estimate_prepared_resource_stats(PreparedResourceEstimateInput {
            target,
            vertex_count: vertex_count as usize,
            has_surface_pipeline: true,
            shadow_maps: 0,
            shadow_map_resolution: None,
            depth_prepass_passes: u64::from(depth_prepass.is_some()),
            material_texture_count: material_resources.len() as u64,
            material_texture_bytes: material_texture_byte_len(&material_resources),
        });

        self.resources = Some(GpuPreparedResources {
            target,
            vertex_buffer,
            output_uniform,
            output_bind_group,
            light_uniform,
            light_from_world,
            material_resources,
            depth_prepass,
            surface_pipeline,
            vertex_count,
            draw_batches,
            draw_uniforms,
            draw_uniform_buffer,
            draw_bind_group,
            webgl2_vertices,
            stats,
        });
    }

    pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats {
        self.resources
            .as_ref()
            .map(|resources| resources.stats)
            .unwrap_or_default()
    }

    fn configure_surface(&mut self, target: RasterTarget) {
        if let Some(surface) = &mut self.surface {
            if surface.config.width != target.width || surface.config.height != target.height {
                surface.config.width = target.width;
                surface.config.height = target.height;
            }
            surface.surface.configure(&self.device, &surface.config);
        }
    }
}
