#![cfg(target_arch = "wasm32")]

use crate::diagnostics::Backend;
use crate::render::prepare::{
    PreparedDepthStats, PreparedEnvironmentLighting, PreparedGpuLightUniform, PreparedInstanceSet,
    PreparedLabelAtlas, PreparedLightingStats, PreparedMaterialSlot, PreparedPrimitive,
    PreparedStrokeSegment,
};

#[cfg(any(feature = "browser-probe", feature = "scene-host"))]
use super::browser_readback::{
    BrowserReadbackResourceDescriptor, create_browser_readback_resources,
};
use super::instancing::INSTANCE_BYTE_LEN;
use super::material_support::reject_unsupported_volume_texture_slots;
use super::materials::{create_material_bind_group_layout, create_material_resources};
use super::output::{create_output_bind_group_layout, create_output_uniform_buffer};
use super::pipeline::create_unlit_pipeline_set;
use super::resource_encoding::{
    encode_draw_resources, encode_retained_vertices, retained_draw_uniform_capacity,
    retained_instance_buffer_capacity,
};
use super::stats::GpuResourceStats;
use super::vertices::VERTEX_BYTE_LEN;
use super::{
    GpuDeviceState, GpuOutputPlan, GpuPrepareOutcome, GpuPreparedResources, depth, environment,
    light_assignment, material_texture_binding_mode, output, transmission,
};
use crate::render::prepare::TiledLightAssignment;
use crate::render::{PrepareWorkCounter, RasterTarget};

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
        tiled_light_assignment: &TiledLightAssignment,
        semantic_aov_capture_enabled: bool,
        output_plan: GpuOutputPlan,
        work: Option<&PrepareWorkCounter>,
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
        reject_unsupported_volume_texture_slots(target, material_slots)?;
        if output_plan.sample_count() > 1 {
            return Err(crate::PrepareError::UnsupportedSampleCount {
                backend: target.backend,
                requested: output_plan.sample_count(),
                maximum: 1,
            });
        }
        let semantic_attribution = semantic_aov_capture_enabled
            .then(|| {
                crate::render::semantic_aov::build_gpu_semantic_attribution(
                    draw_primitives,
                    draw_instances,
                    draw_strokes.len(),
                    draw_labels.quads().len(),
                )
            })
            .transpose()
            .map_err(|entries| crate::PrepareError::GpuResourceUpload {
                backend: target.backend,
                reason: format!(
                    "semantic AOV requires {entries} palette entries, exceeding the 24-bit limit"
                ),
            })?;
        let vertex_bytes = encode_retained_vertices(retained_primitives, retained_instances);
        let encoded_draw_resources = encode_draw_resources(
            draw_primitives,
            draw_instances,
            draw_strokes,
            semantic_attribution.as_ref(),
        );
        if let Some(work) = work {
            work.record_draw_uniform_indexing(
                encoded_draw_resources.draw_uniforms.len(),
                encoded_draw_resources
                    .draw_uniform_index_metrics
                    .lookup_probes,
                (encoded_draw_resources.draw_uniforms.len() as u64)
                    .saturating_mul(output::DRAW_UNIFORM_ENTRY_STRIDE),
            );
        }
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

        let include_tiled_light_storage = target.backend != Backend::WebGl2;
        let output_bind_group_layout =
            create_output_bind_group_layout(&self.device, include_tiled_light_storage);
        let texture_binding_mode = material_texture_binding_mode(target);
        let triangle_shader_lookup = self
            .triangle_shader_modules
            .get_or_create(&self.device, texture_binding_mode);
        let triangle_shader_cache_hit = triangle_shader_lookup.hit;
        if let Some(work) = work {
            work.record_gpu_triangle_shader_cache(triangle_shader_cache_hit);
        }
        let triangle_shader = triangle_shader_lookup.module;
        let material_bind_group_layout =
            create_material_bind_group_layout(&self.device, texture_binding_mode);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let light_assignment = light_assignment::create_light_assignment_resources(
            &self.device,
            &self.queue,
            tiled_light_assignment,
        );
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
                    output_plan.depth_color_enabled(),
                    1,
                )
            });
        let depth_compare = depth_prepass
            .as_ref()
            .map(|depth_prepass| depth_prepass.color_compare);
        let transmission = transmission::create_transmission_resources(
            &self.device,
            &triangle_shader,
            target,
            surface.config.format,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
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
            &light_assignment,
            include_tiled_light_storage,
            lighting_stats.directional_shadow_map_resolution,
            environment_lighting,
        );
        let surface_pipeline = create_unlit_pipeline_set(
            &self.device,
            &triangle_shader,
            surface.config.format,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            depth_compare,
            1,
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
        let semantic_aov = semantic_attribution.map(|attribution| {
            super::semantic_aov::create_resources(
                &self.device,
                super::semantic_aov::SemanticAovResourceDescriptor {
                    target,
                    output_layout: &output_bind_group_layout,
                    material_layout: &material_bind_group_layout,
                    draw_layout: &draw_bind_group_layout,
                    triangle_shader: &triangle_shader,
                    reversed_z: depth_stats.reversed_z,
                    attribution,
                    webgl2_surface_format: (target.backend == Backend::WebGl2)
                        .then_some(surface.config.format),
                },
            )
        });
        #[cfg(any(feature = "browser-probe", feature = "scene-host"))]
        let readback = (target.backend == Backend::WebGpu).then(|| {
            create_browser_readback_resources(
                &self.device,
                BrowserReadbackResourceDescriptor {
                    target,
                    surface_format: surface.config.format,
                    output_bind_group_layout: &output_bind_group_layout,
                    material_bind_group_layout: &material_bind_group_layout,
                    draw_bind_group_layout: &draw_bind_group_layout,
                    triangle_shader: &triangle_shader,
                    depth_compare,
                },
            )
        });
        #[cfg(not(any(feature = "browser-probe", feature = "scene-host")))]
        let readback = None;
        let post = output_plan.post_enabled().then(|| {
            super::post::create_resources(
                &self.device,
                &triangle_shader,
                target,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                depth_compare,
                Some(surface.config.format),
                depth_prepass
                    .as_ref()
                    .and_then(depth::DepthPrepassResources::color_view),
            )
        });
        let vertex_count = (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32;
        let mut stats = GpuResourceStats {
            buffers: 4,
            pipelines: 2,
            bind_groups: 1,
            shader_modules: 1,
            shader_module_creations: u64::from(!triangle_shader_cache_hit),
            approximate_gpu_memory_bytes: vertex_buffer_size
                .saturating_add(instance_buffer_size)
                .saturating_add(output::OUTPUT_UNIFORM_BYTE_LEN)
                .saturating_add(
                    output::DRAW_UNIFORM_ENTRY_STRIDE
                        .saturating_mul((draw_uniform_capacity as u64).max(1)),
                ),
            ..GpuResourceStats::default()
        };
        stats.add_assign(light_assignment::resource_stats(&light_assignment));
        stats.add_assign(super::materials::resource_stats(&material_resources));
        stats.add_assign(environment::resource_stats(
            lighting_stats.directional_shadow_map_resolution,
            environment_lighting,
        ));
        stats.add_assign(transmission::resource_stats(target));
        if let Some(resources) = &depth_prepass {
            stats.add_assign(depth::resource_stats(resources, target));
        }
        if let Some(resources) = &strokes {
            stats.add_assign(super::strokes::resource_stats(resources));
        }
        if let Some(resources) = &labels {
            stats.add_assign(super::labels::resource_stats(resources, retained_labels));
        }
        if let Some(resources) = &semantic_aov {
            stats.add_assign(super::semantic_aov::resource_stats(resources));
        }
        if let Some(resources) = &post {
            stats.add_assign(super::post::resource_stats(resources));
        }
        if let Some(resources) = &readback {
            stats.add_assign(super::browser_readback::resource_stats(resources, target));
        }

        self.resources = Some(GpuPreparedResources {
            target,
            vertex_buffer,
            instance_buffer,
            instance_buffer_capacity,
            output_uniform,
            output_bind_group,
            opaque_output_bind_group,
            light_uniform,
            light_assignment,
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
            semantic_aov,
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
            post,
            stats,
        });
        Ok(GpuPrepareOutcome::FullRebuild)
    }
}
