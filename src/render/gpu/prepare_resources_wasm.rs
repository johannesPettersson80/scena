#![cfg(target_arch = "wasm32")]

use crate::diagnostics::Backend;
use crate::render::prepare::{
    PreparedDepthStats, PreparedEnvironmentLighting, PreparedGpuLightUniform, PreparedInstanceSet,
    PreparedLabelAtlas, PreparedLightingStats, PreparedMaterialSlot, PreparedPrimitive,
    PreparedStrokeSegment,
};

use super::browser_readback::create_browser_readback_resources;
use super::instancing::INSTANCE_BYTE_LEN;
use super::materials::{
    create_material_bind_group_layout, create_material_resources, material_bind_group_count,
    material_texture_byte_len, material_texture_count,
};
use super::output::{create_output_bind_group_layout, create_output_uniform_buffer};
use super::pipeline::create_unlit_pipeline_set;
use super::resource_encoding::{
    encode_draw_resources, encode_retained_vertices, retained_draw_uniform_capacity,
    retained_instance_buffer_capacity,
};
use super::stats::{PreparedResourceEstimateInput, estimate_prepared_resource_stats};
use super::vertices::VERTEX_BYTE_LEN;
use super::{
    GpuDeviceState, GpuPrepareOutcome, GpuPreparedResources, depth, environment,
    material_texture_binding_mode, output, transmission,
};
use crate::render::RasterTarget;

impl GpuDeviceState {
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
        let Some(surface) = self.surface.as_ref() else {
            return Ok(GpuPrepareOutcome::NoResources);
        };
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
            label: Some("scena.browser.scene_vertices"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !vertex_bytes.is_empty() {
            self.queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
        }
        let instance_buffer_capacity = retained_instance_buffer_capacity(retained_instances);
        let instance_buffer_size = (instance_buffer_capacity * INSTANCE_BYTE_LEN).max(4) as u64;
        let instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.m4.browser_scene_instances"),
            size: instance_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&instance_buffer, 0, instance_bytes);

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
        let depth_prepass = (matches!(target.backend, Backend::WebGpu | Backend::WebGl2)
            && depth_stats.passes > 0)
            .then(|| {
                depth::create_depth_prepass_resources(
                    &self.device,
                    target,
                    depth_stats.reversed_z,
                    &output_bind_group_layout,
                    &draw_bind_group_layout,
                    false,
                )
            });
        let depth_compare = depth_prepass
            .as_ref()
            .map(|depth_prepass| depth_prepass.color_compare);
        let transmission = transmission::create_transmission_resources(
            &self.device,
            target,
            surface.config.format,
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
        let surface_pipeline = create_unlit_pipeline_set(
            &self.device,
            surface.config.format,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            texture_binding_mode,
            depth_compare,
        );
        let strokes = (!retained_strokes.is_empty()).then(|| {
            super::strokes::create_resources(
                &self.device,
                super::strokes::StrokeResourceDescriptor {
                    target_format: surface.config.format,
                    surface_format: Some(surface.config.format),
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
                    target_format: surface.config.format,
                    surface_format: Some(surface.config.format),
                    output_bind_group_layout: &output_bind_group_layout,
                    depth_compare,
                    labels: draw_labels,
                },
            )
        });
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
            instance_capacity: instance_buffer_capacity,
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
            surface_pipeline,
            readback,
            vertex_count,
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
            stats,
        });
        Ok(GpuPrepareOutcome::FullRebuild)
    }
}
