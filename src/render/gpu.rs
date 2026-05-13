mod build;
#[cfg(target_arch = "wasm32")]
mod debug;
mod depth;
mod draw;
mod draw_uniform;
mod environment;
mod lifecycle;
mod material_batched;
mod material_mips;
mod material_uniform;
mod material_upload;
mod materials;
mod output;
mod pipeline;
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
#[cfg(not(target_arch = "wasm32"))]
use crate::geometry::Primitive;

use self::materials::{
    create_material_bind_group_layout, create_material_resources, material_bind_group_count,
    material_texture_byte_len, material_texture_count,
};
use self::output::{create_output_bind_group_layout, create_output_uniform_buffer};
use self::pipeline::create_unlit_pipeline;
#[cfg(not(target_arch = "wasm32"))]
use self::pipeline::{BYTES_PER_PIXEL, GPU_COLOR_FORMAT};
use self::shadow::ShadowCasterResources;
pub(super) use self::stats::GpuResourceStats;
#[cfg(not(target_arch = "wasm32"))]
use self::stats::align_to;
use self::stats::{PreparedResourceEstimateInput, estimate_prepared_resource_stats};
use self::vertices::{
    DrawUniformValue, PrimitiveDrawBatch, VERTEX_BYTE_LEN, encode_draw_batches, encode_vertices,
};
use super::RasterTarget;
use super::prepare::{
    PreparedDepthStats, PreparedEnvironmentLighting, PreparedGpuLightUniform,
    PreparedLightingStats, PreparedMaterialSlot,
};

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
    #[cfg(target_arch = "wasm32")]
    browser_canvas: Option<web_sys::HtmlCanvasElement>,
    #[cfg(target_arch = "wasm32")]
    webgl2_render_cache: Option<webgl2::WebGl2RenderCache>,
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
    /// Phase 1B: directional-light view-projection. See `prepare/shadows.rs`.
    light_from_world: [f32; 16],
    material_resources: materials::MaterialResources,
    // Phase 1B/1C: directional shadow caster + env cubemap; always allocated
    // (1×1 placeholder when feature absent), gated by lighting uniform flags.
    shadow_caster: ShadowCasterResources,
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    environment_cubemap: wgpu::Texture,
    #[allow(dead_code)]
    environment_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    brdf_lut_texture: wgpu::Texture,
    depth_prepass: Option<depth::DepthPrepassResources>,
    #[allow(dead_code)]
    vertex_count: u32,
    draw_batches: Vec<PrimitiveDrawBatch>,
    // Phase 1A.2: per-draw uniforms via draw_uniform_buffer + draw_bind_group
    // with dynamic offsets. Vertex stream carries model-space positions; the
    // shader applies draw.world_from_model. Closes wgpu-architect F2.
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
        environment_lighting: &PreparedEnvironmentLighting,
    ) -> Result<(), crate::PrepareError> {
        self.configure_surface(target);
        self.release_prepared_resources();
        if primitives.is_empty() {
            return Ok(());
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
        let environment::OutputResources {
            shadow_caster,
            shadow_sampler,
            environment_cubemap,
            environment_sampler,
            brdf_lut_texture,
            output_bind_group,
        } = environment::build_output_resources(
            &self.device,
            &self.queue,
            &output_bind_group_layout,
            &draw_bind_group_layout,
            &output_uniform,
            lighting_stats.directional_shadow_map_resolution,
            environment_lighting,
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
            material_texture_count: material_texture_count(&material_resources),
            material_texture_bytes: material_texture_byte_len(&material_resources),
            material_bind_groups: material_bind_group_count(&material_resources),
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
            shadow_caster,
            shadow_sampler,
            environment_cubemap,
            environment_sampler,
            brdf_lut_texture,
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
        Ok(())
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
        environment_lighting: &PreparedEnvironmentLighting,
    ) -> Result<(), crate::PrepareError> {
        self.configure_surface(target);
        self.release_prepared_resources();
        let Some(surface) = self.surface.as_ref() else {
            return Ok(());
        };
        if primitives.is_empty() {
            return Ok(());
        }
        let vertex_bytes = encode_vertices(primitives);
        let (draw_batches, draw_uniforms) = encode_draw_batches(primitives);
        let webgl2_vertices = webgl2::encode_vertices(primitives);
        if target.backend == Backend::WebGl2 {
            let Some(canvas) = self.browser_canvas.as_ref() else {
                return Err(crate::PrepareError::GpuResourceUpload {
                    backend: target.backend,
                    reason: "WebGL2 target has no attached browser canvas".to_string(),
                });
            };
            webgl2::prepare_canvas_vertices(
                &mut self.webgl2_render_cache,
                canvas,
                &webgl2_vertices,
                &draw_batches,
                material_slots,
            )
            .map_err(|error| crate::PrepareError::GpuResourceUpload {
                backend: target.backend,
                reason: error
                    .as_string()
                    .unwrap_or_else(|| "WebGL2 resource preparation failed".to_string()),
            })?;
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
        // Phase 1B/1C (wasm32 mirror of native): bundle group-0 GPU resources
        // — shadow caster + sampler, environment cubemap + sampler, and the
        // output bind group — through `environment::build_output_resources`.
        let environment::OutputResources {
            shadow_caster,
            shadow_sampler,
            environment_cubemap,
            environment_sampler,
            brdf_lut_texture,
            output_bind_group,
        } = environment::build_output_resources(
            &self.device,
            &self.queue,
            &output_bind_group_layout,
            &draw_bind_group_layout,
            &output_uniform,
            lighting_stats.directional_shadow_map_resolution,
            environment_lighting,
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
            shadow_maps: lighting_stats.shadow_maps,
            shadow_map_resolution: lighting_stats.directional_shadow_map_resolution,
            depth_prepass_passes: u64::from(depth_prepass.is_some()),
            material_texture_count: material_texture_count(&material_resources),
            material_texture_bytes: material_texture_byte_len(&material_resources),
            material_bind_groups: material_bind_group_count(&material_resources),
        });

        self.resources = Some(GpuPreparedResources {
            target,
            vertex_buffer,
            output_uniform,
            output_bind_group,
            light_uniform,
            light_from_world,
            material_resources,
            shadow_caster,
            shadow_sampler,
            environment_cubemap,
            environment_sampler,
            brdf_lut_texture,
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
        Ok(())
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

#[cfg(test)]
mod tests {
    const WEBGL2_PROGRAM_SOURCE: &str = include_str!("gpu/webgl2_program.rs");

    #[test]
    fn host_tests_guard_webgl2_khronos_pbr_neutral_source() {
        assert!(
            WEBGL2_PROGRAM_SOURCE.contains("pbrNeutralTonemap")
                && WEBGL2_PROGRAM_SOURCE.contains("startCompression")
                && WEBGL2_PROGRAM_SOURCE.contains("desaturation")
                && WEBGL2_PROGRAM_SOURCE.contains("color_management.x > 1.5"),
            "native CI must still guard the WebGL2 source for the Khronos PBR Neutral \
             tone-mapping branch even though the WebGL2 module is wasm32-gated"
        );
    }
}
