#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;

use super::super::RasterTarget;
use super::super::prepare::{
    PreparedDepthStats, PreparedEnvironmentLighting, PreparedGpuLightUniform,
    PreparedLightingStats, PreparedMaterialSlot,
};
#[cfg(target_arch = "wasm32")]
use super::browser_readback::create_browser_readback_resources;
use super::instancing::INSTANCE_BYTE_LEN;
use super::materials::{
    create_material_bind_group_layout, create_material_resources, material_bind_group_count,
    material_texture_byte_len, material_texture_count,
};
use super::output::{create_output_bind_group_layout, create_output_uniform_buffer};
use super::pipeline::create_unlit_pipeline_set;
#[cfg(not(target_arch = "wasm32"))]
use super::pipeline::{BYTES_PER_PIXEL, GPU_COLOR_FORMAT};
use super::resource_encoding::{
    encode_draw_resources, encode_retained_vertices, retained_draw_uniform_capacity,
    retained_instance_buffer_capacity,
};
#[cfg(not(target_arch = "wasm32"))]
use super::stats::align_to;
use super::stats::{PreparedResourceEstimateInput, estimate_prepared_resource_stats};
use super::vertices::VERTEX_BYTE_LEN;
use super::{
    GpuDeviceState, GpuPrepareOutcome, GpuPreparedResources, depth, environment,
    material_texture_binding_mode, output, transmission,
};
use crate::render::prepare::{
    PreparedInstanceSet, PreparedLabelAtlas, PreparedPrimitive, PreparedStrokeSegment,
};

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::render) fn prepare(
        &mut self,
        target: RasterTarget,
        retained_primitives: &[PreparedPrimitive],
        draw_primitives: &[PreparedPrimitive],
        retained_instances: &[PreparedInstanceSet],
        draw_instances: &[PreparedInstanceSet],
        retained_strokes: &[PreparedStrokeSegment],
        draw_strokes: &[PreparedStrokeSegment],
        retained_labels: &PreparedLabelAtlas,
        draw_labels: &PreparedLabelAtlas,
        lighting_stats: PreparedLightingStats,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        depth_stats: PreparedDepthStats,
        material_slots: &[PreparedMaterialSlot],
        environment_lighting: &PreparedEnvironmentLighting,
    ) -> Result<GpuPrepareOutcome, crate::PrepareError> {
        self.configure_surface(target);
        self.release_prepared_resources();
        if retained_primitives.is_empty()
            && retained_instances.is_empty()
            && retained_strokes.is_empty()
            && retained_labels.is_empty()
        {
            return Ok(GpuPrepareOutcome::NoResources);
        }

        let vertex_bytes = encode_retained_vertices(retained_primitives, retained_instances);
        let encoded_draw_resources =
            encode_draw_resources(draw_primitives, draw_instances, draw_strokes);
        let instance_bytes = &encoded_draw_resources.instance_bytes;
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
        let instance_buffer_capacity = retained_instance_buffer_capacity(retained_instances);
        let instance_buffer_size = (instance_buffer_capacity * INSTANCE_BYTE_LEN).max(4) as u64;
        let instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.m4.scene_instances"),
            size: instance_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        {
            let mut initial_instance_bytes = instance_bytes.clone();
            initial_instance_bytes.resize(instance_buffer_size as usize, 0);
            let mut mapped = instance_buffer.slice(..).get_mapped_range_mut();
            mapped.copy_from_slice(&initial_instance_bytes);
        }
        instance_buffer.unmap();

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
        let draw_uniform_capacity = retained_draw_uniform_capacity(
            retained_primitives,
            retained_instances,
            retained_strokes.len(),
            encoded_draw_resources.draw_uniforms.len(),
        );
        let draw_uniform_buffer =
            output::create_draw_uniform_buffer(&self.device, draw_uniform_capacity as u64);
        self.queue.write_buffer(
            &draw_uniform_buffer,
            0,
            &output::encode_draw_uniform_bytes(&encoded_draw_resources.draw_uniforms),
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
                false,
                1,
            )
        });
        let depth_compare = depth_prepass
            .as_ref()
            .map(|depth_prepass| depth_prepass.color_compare);
        let transmission = transmission::create_transmission_resources(
            &self.device,
            target,
            GPU_COLOR_FORMAT,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            texture_binding_mode,
            depth_compare,
        );
        let environment::OutputResources {
            shadow_caster,
            shadow_sampler,
            environment_cubemap,
            environment_sampler,
            brdf_lut_texture,
            output_bind_group,
            opaque_output_bind_group,
        } = environment::build_output_resources(
            &self.device,
            &self.queue,
            &output_bind_group_layout,
            &draw_bind_group_layout,
            &output_uniform,
            &transmission.view,
            &transmission.placeholder_view,
            &transmission.sampler,
            lighting_stats.directional_shadow_map_resolution,
            environment_lighting,
        );
        let offscreen_pipelines = create_unlit_pipeline_set(
            &self.device,
            GPU_COLOR_FORMAT,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            texture_binding_mode,
            depth_compare,
            1,
        );
        let offscreen_msaa4_pipelines = create_unlit_pipeline_set(
            &self.device,
            GPU_COLOR_FORMAT,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            texture_binding_mode,
            depth_compare,
            4,
        );
        let surface_pipeline = self.surface.as_ref().map(|surface| {
            create_unlit_pipeline_set(
                &self.device,
                surface.config.format,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                texture_binding_mode,
                depth_compare,
                1,
            )
        });
        let strokes = (!retained_strokes.is_empty()).then(|| {
            super::strokes::create_resources(
                &self.device,
                super::strokes::StrokeResourceDescriptor {
                    target_format: GPU_COLOR_FORMAT,
                    surface_format: self.surface.as_ref().map(|surface| surface.config.format),
                    output_bind_group_layout: &output_bind_group_layout,
                    draw_bind_group_layout: &draw_bind_group_layout,
                    depth_compare,
                    retained_strokes,
                    batches: encoded_draw_resources.stroke_batches,
                },
            )
        });
        let labels = (!retained_labels.is_empty()).then(|| {
            super::labels::create_resources(
                &self.device,
                &self.queue,
                super::labels::LabelResourceDescriptor {
                    target_format: GPU_COLOR_FORMAT,
                    surface_format: self.surface.as_ref().map(|surface| surface.config.format),
                    output_bind_group_layout: &output_bind_group_layout,
                    depth_compare,
                    labels: draw_labels,
                },
            )
        });
        let stats = estimate_prepared_resource_stats(PreparedResourceEstimateInput {
            target,
            vertex_count: vertex_bytes.len() / VERTEX_BYTE_LEN,
            instance_capacity: instance_buffer_capacity,
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
            instance_buffer,
            instance_buffer_capacity,
            output_uniform,
            output_bind_group,
            opaque_output_bind_group,
            light_uniform,
            light_from_world,
            material_resources,
            shadow_caster,
            shadow_sampler,
            environment_cubemap,
            environment_sampler,
            brdf_lut_texture,
            transmission,
            depth_prepass,
            strokes,
            labels,
            vertex_count: (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32,
            draw_batches: encoded_draw_resources.draw_batches,
            instance_batches: encoded_draw_resources.instance_batches,
            instance_count: encoded_draw_resources.instance_count,
            identity_instance: encoded_draw_resources.identity_instance,
            draw_uniforms: encoded_draw_resources.draw_uniforms,
            draw_uniform_capacity,
            draw_uniform_buffer,
            draw_bind_group,
            output_bind_group_layout,
            material_bind_group_layout,
            draw_bind_group_layout,
            texture_binding_mode,
            depth_compare,
            post: None,
            offscreen_pipelines,
            offscreen_msaa4_pipelines,
            offscreen_msaa8_pipelines: None,
            msaa_color: None,
            surface_pipeline,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            stats,
        });
        Ok(GpuPrepareOutcome::FullRebuild)
    }
}
