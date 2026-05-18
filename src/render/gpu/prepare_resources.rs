use crate::geometry::Primitive;

#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;

use super::super::RasterTarget;
use super::super::prepare::{
    PreparedDepthStats, PreparedEnvironmentLighting, PreparedGpuLightUniform,
    PreparedLightingStats, PreparedMaterialSlot,
};
#[cfg(target_arch = "wasm32")]
use super::browser_readback::create_browser_readback_resources;
use super::materials::{
    create_material_bind_group_layout, create_material_resources, material_bind_group_count,
    material_texture_byte_len, material_texture_count,
};
use super::output::{create_output_bind_group_layout, create_output_uniform_buffer};
use super::pipeline::create_unlit_pipeline;
#[cfg(not(target_arch = "wasm32"))]
use super::pipeline::{BYTES_PER_PIXEL, GPU_COLOR_FORMAT};
#[cfg(not(target_arch = "wasm32"))]
use super::stats::align_to;
use super::stats::{PreparedResourceEstimateInput, estimate_prepared_resource_stats};
use super::vertices::{DrawUniformValue, VERTEX_BYTE_LEN, encode_vertices};
use super::{
    GpuDeviceState, GpuPrepareOutcome, GpuPreparedResources, depth, environment,
    material_texture_binding_mode, output,
};

impl GpuDeviceState {
    pub(in crate::render) fn update_dynamic_draw_uniforms(
        &mut self,
        target: RasterTarget,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        draw_uniform_pairs: &[([f32; 16], [f32; 16])],
    ) -> Result<(), &'static str> {
        let Some(resources) = self.resources.as_mut() else {
            return Err("no GPU resources");
        };
        if resources.target != target {
            return Err("target changed");
        }
        if resources.draw_uniforms.len() != draw_uniform_pairs.len() {
            return Err("draw uniform shape changed");
        }
        self.queue.write_buffer(
            &resources.draw_uniform_buffer,
            0,
            &output::encode_draw_uniform_bytes(draw_uniform_pairs),
        );
        resources.draw_uniforms = draw_uniform_pairs
            .iter()
            .map(|(world_from_model, normal_from_model)| DrawUniformValue {
                world_from_model: *world_from_model,
                normal_from_model: *normal_from_model,
            })
            .collect();
        resources.light_uniform = light_uniform;
        resources.light_from_world = light_from_world;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::render) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[Primitive],
        lighting_stats: PreparedLightingStats,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        depth_stats: PreparedDepthStats,
        material_slots: &[PreparedMaterialSlot],
        environment_lighting: &PreparedEnvironmentLighting,
    ) -> Result<GpuPrepareOutcome, crate::PrepareError> {
        self.configure_surface(target);
        self.release_prepared_resources();
        if primitives.is_empty() {
            return Ok(GpuPrepareOutcome::NoResources);
        }

        let vertex_bytes = encode_vertices(primitives);
        let (draw_batches, draw_uniforms) = super::vertices::encode_draw_batches(primitives);
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
        let texture_binding_mode = material_texture_binding_mode(target);
        let material_bind_group_layout =
            create_material_bind_group_layout(&self.device, texture_binding_mode);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let material_resources = create_material_resources(
            &self.device,
            &self.queue,
            &material_bind_group_layout,
            material_slots,
            texture_binding_mode,
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
            texture_binding_mode,
            depth_compare,
        );
        let surface_pipeline = self.surface.as_ref().map(|surface| {
            create_unlit_pipeline(
                &self.device,
                surface.config.format,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                texture_binding_mode,
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
        Ok(GpuPrepareOutcome::FullRebuild)
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::render) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[Primitive],
        lighting_stats: PreparedLightingStats,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        depth_stats: PreparedDepthStats,
        material_slots: &[PreparedMaterialSlot],
        environment_lighting: &PreparedEnvironmentLighting,
    ) -> Result<GpuPrepareOutcome, crate::PrepareError> {
        self.configure_surface(target);
        self.release_prepared_resources();
        let Some(surface) = self.surface.as_ref() else {
            return Ok(GpuPrepareOutcome::NoResources);
        };
        if primitives.is_empty() {
            return Ok(GpuPrepareOutcome::NoResources);
        }
        let vertex_bytes = encode_vertices(primitives);
        let (draw_batches, draw_uniforms) = super::vertices::encode_draw_batches(primitives);
        let vertex_buffer_size = vertex_bytes.len().max(4) as u64;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.browser.scene_vertices"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !vertex_bytes.is_empty() {
            self.queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
        }

        let output_bind_group_layout = create_output_bind_group_layout(&self.device);
        let texture_binding_mode = material_texture_binding_mode(target);
        let material_bind_group_layout =
            create_material_bind_group_layout(&self.device, texture_binding_mode);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let material_resources = create_material_resources(
            &self.device,
            &self.queue,
            &material_bind_group_layout,
            material_slots,
            texture_binding_mode,
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
        let depth_prepass = (matches!(target.backend, Backend::WebGpu | Backend::WebGl2)
            && depth_stats.passes > 0)
            .then(|| {
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
            texture_binding_mode,
            depth_compare,
        );
        let readback = (target.backend == Backend::WebGpu).then(|| {
            create_browser_readback_resources(
                &self.device,
                target,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                texture_binding_mode,
                depth_compare,
            )
        });
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
            readback,
            vertex_count,
            draw_batches,
            draw_uniforms,
            draw_uniform_buffer,
            draw_bind_group,
            stats,
        });
        Ok(GpuPrepareOutcome::FullRebuild)
    }
}
