#[cfg(target_arch = "wasm32")]
use crate::diagnostics::Backend;

use super::super::RasterTarget;
use super::super::prepare::{
    PreparedDepthStats, PreparedEnvironmentLighting, PreparedGpuLightUniform,
    PreparedLightingStats, PreparedMaterialSlot,
};
#[cfg(target_arch = "wasm32")]
use super::browser_readback::create_browser_readback_resources;
use super::material_support::reject_unsupported_volume_texture_slots;
use super::materials::{create_material_bind_group_layout, create_material_resources};
use super::output::{create_output_bind_group_layout, create_output_uniform_buffer};
use super::pipeline::GPU_COLOR_FORMAT;
use super::prepare_resources_support::{
    build_semantic_attribution, create_geometry_buffers, validate_sample_count,
};
mod pipelines;
use super::resource_encoding::{
    encode_draw_resources, encode_retained_vertices, retained_draw_uniform_capacity,
};
use super::stats::GpuResourceStats;
use super::vertices::VERTEX_BYTE_LEN;
use super::{
    GpuDeviceState, GpuOutputPlan, GpuPrepareOutcome, GpuPreparedResources, depth, environment,
    light_assignment, material_texture_binding_mode, output, transmission,
};
use crate::render::PrepareWorkCounter;
use crate::render::prepare::{
    PreparedInstanceSet, PreparedLabelAtlas, PreparedPrimitive, PreparedStrokeSegment,
    TiledLightAssignment,
};
use pipelines::create_pipeline_resources;

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
        tiled_light_assignment: &TiledLightAssignment,
        semantic_aov_capture_enabled: bool,
        output_plan: GpuOutputPlan,
        work: Option<&PrepareWorkCounter>,
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
        reject_unsupported_volume_texture_slots(target, material_slots)?;

        let (sample_count, scene_format) = validate_sample_count(self, target, output_plan)?;
        let semantic_attribution = build_semantic_attribution(
            target,
            semantic_aov_capture_enabled,
            draw_primitives,
            draw_instances,
            draw_strokes,
            draw_labels,
        )?;
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
        let geometry_buffers = create_geometry_buffers(
            &self.device,
            &vertex_bytes,
            &encoded_draw_resources.instance_bytes,
            retained_instances,
        );
        let vertex_buffer_size = geometry_buffers.vertex_buffer_size;
        let instance_buffer_size = geometry_buffers.instance_buffer_size;
        let instance_buffer_capacity = geometry_buffers.instance_buffer_capacity;
        let vertex_buffer = geometry_buffers.vertex_buffer;
        let instance_buffer = geometry_buffers.instance_buffer;

        let super::headless_target::HeadlessTargetResources {
            texture,
            view,
            readback,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        } = super::headless_target::create(&self.device, target);
        let output_bind_group_layout = create_output_bind_group_layout(&self.device, true);
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
        let depth_prepass = (depth_stats.passes > 0).then(|| {
            depth::create_depth_prepass_resources(
                &self.device,
                target,
                depth_stats.reversed_z,
                &output_bind_group_layout,
                &draw_bind_group_layout,
                output_plan.depth_color_enabled(),
                sample_count,
            )
        });
        let depth_compare = depth_prepass
            .as_ref()
            .map(|depth_prepass| depth_prepass.color_compare);
        let transmission = transmission::create_transmission_resources(
            &self.device,
            &triangle_shader,
            target,
            GPU_COLOR_FORMAT,
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
            true,
            lighting_stats.directional_shadow_map_resolution,
            environment_lighting,
        );
        let pipeline_resources = create_pipeline_resources(
            &self.device,
            self.surface.as_ref().map(|surface| surface.config.format),
            output_plan,
            sample_count,
            &triangle_shader,
            &output_bind_group_layout,
            &material_bind_group_layout,
            &draw_bind_group_layout,
            depth_compare,
            &shadow_caster.view,
            &shadow_sampler,
            &environment_cubemap,
            &environment_sampler,
            &transmission.view,
            &transmission.placeholder_view,
            &transmission.sampler,
            &light_assignment,
        );
        let surface_output_uniform = pipeline_resources.surface_output_uniform;
        let surface_output_bind_group = pipeline_resources.surface_output_bind_group;
        let surface_opaque_output_bind_group = pipeline_resources.surface_opaque_output_bind_group;
        let offscreen_pipelines = pipeline_resources.offscreen_pipelines;
        let offscreen_msaa4_pipelines = pipeline_resources.offscreen_msaa4_pipelines;
        let offscreen_msaa8_pipelines = pipeline_resources.offscreen_msaa8_pipelines;
        let surface_pipeline = pipeline_resources.surface_pipeline;
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
                },
            )
        });
        let mut post = output_plan.post_enabled().then(|| {
            super::post::create_resources(
                &self.device,
                &triangle_shader,
                target,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                depth_compare,
                self.surface.as_ref().map(|surface| surface.config.format),
                depth_prepass
                    .as_ref()
                    .and_then(depth::DepthPrepassResources::color_view),
            )
        });
        if sample_count == 8
            && let Some(post_resources) = post.as_mut()
        {
            super::post::ensure_scene_msaa8_pipelines(
                &self.adapter,
                &self.device,
                &triangle_shader,
                post_resources,
                target,
                &output_bind_group_layout,
                &material_bind_group_layout,
                &draw_bind_group_layout,
                depth_compare,
            )
            .map_err(|error| match error {
                crate::RenderError::UnsupportedSampleCount {
                    backend,
                    requested,
                    maximum,
                } => crate::PrepareError::UnsupportedSampleCount {
                    backend,
                    requested,
                    maximum,
                },
                other => crate::PrepareError::GpuResourceUpload {
                    backend: target.backend,
                    reason: other.to_string(),
                },
            })?;
        }
        let msaa_color = (sample_count > 1).then(|| {
            super::msaa::create_msaa_color_resources(
                &self.device,
                target,
                scene_format,
                sample_count,
            )
        });
        let surface_msaa_color = if sample_count > 1 && !output_plan.post_enabled() {
            self.surface.as_ref().map(|surface| {
                super::msaa::create_msaa_color_resources(
                    &self.device,
                    target,
                    surface.config.format,
                    sample_count,
                )
            })
        } else {
            None
        };
        let overlay_depth_prepass = depth_prepass.as_ref().and_then(|depth_prepass| {
            (sample_count > 1).then(|| {
                depth::create_depth_prepass_resources(
                    &self.device,
                    target,
                    depth_prepass.reversed_z(),
                    &output_bind_group_layout,
                    &draw_bind_group_layout,
                    false,
                    1,
                )
            })
        });
        let texture_bytes = GpuResourceStats::target_bytes(target, 4, 1);
        let mut stats = GpuResourceStats {
            buffers: 6 + u64::from(surface_output_uniform.is_some()),
            textures: 1,
            render_targets: 1,
            pipelines: 4
                + u64::from(offscreen_msaa8_pipelines.is_some()) * 2
                + u64::from(surface_pipeline.is_some()) * 2,
            bind_groups: 1 + u64::from(surface_output_uniform.is_some()) * 2,
            shader_modules: 1,
            shader_module_creations: u64::from(!triangle_shader_cache_hit),
            approximate_gpu_memory_bytes: vertex_buffer_size
                .saturating_add(instance_buffer_size)
                .saturating_add(
                    u64::from(padded_bytes_per_row)
                        .saturating_mul(u64::from(target.height))
                        .saturating_mul(2),
                )
                .saturating_add(output::OUTPUT_UNIFORM_BYTE_LEN)
                .saturating_add(
                    u64::from(surface_output_uniform.is_some())
                        .saturating_mul(output::OUTPUT_UNIFORM_BYTE_LEN),
                )
                .saturating_add(
                    output::DRAW_UNIFORM_ENTRY_STRIDE
                        .saturating_mul((draw_uniform_capacity as u64).max(1)),
                )
                .saturating_add(texture_bytes),
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
        if let Some(resources) = &overlay_depth_prepass {
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
        if let Some(resources) = &msaa_color {
            stats.add_assign(GpuResourceStats {
                textures: 1,
                render_targets: 1,
                approximate_gpu_memory_bytes: GpuResourceStats::target_bytes(
                    resources.target,
                    4,
                    resources.sample_count,
                ),
                ..GpuResourceStats::default()
            });
        }
        if let Some(resources) = &surface_msaa_color {
            stats.add_assign(GpuResourceStats {
                textures: 1,
                render_targets: 1,
                approximate_gpu_memory_bytes: GpuResourceStats::target_bytes(
                    resources.target,
                    4,
                    resources.sample_count,
                ),
                ..GpuResourceStats::default()
            });
        }

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
            surface_output_uniform,
            surface_output_bind_group,
            surface_opaque_output_bind_group,
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
            overlay_depth_prepass,
            strokes,
            labels,
            semantic_aov,
            vertex_count: (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32,
            draw_batches: encoded_draw_resources.draw_batches,
            instance_batches: encoded_draw_resources.instance_batches,
            instance_count: encoded_draw_resources.instance_count,
            identity_instance: encoded_draw_resources.identity_instance,
            draw_uniforms: encoded_draw_resources.draw_uniforms,
            draw_uniform_capacity,
            draw_uniform_buffer,
            draw_bind_group,
            post,
            offscreen_pipelines,
            offscreen_msaa4_pipelines,
            offscreen_msaa8_pipelines,
            msaa_color,
            surface_msaa_color,
            surface_pipeline,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            stats,
        });
        Ok(GpuPrepareOutcome::FullRebuild)
    }
}
