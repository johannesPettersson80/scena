use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::scene::{ClippingPlane, Scene};

use super::camera;
use super::prepare_retained::{
    assign_original_instance_vertex_offsets, assign_original_label_quad_indices,
    assign_original_stroke_indices, assign_original_vertex_offsets,
    filter_retained_instances_for_scene, filter_retained_labels_for_scene,
    filter_retained_primitives_for_scene, filter_retained_strokes_for_scene,
    next_gpu_vertex_offset, prepared_instance_count, retained_template_covers_visible_sources,
};
use super::{
    PrepareWorkCounter, PrepareWorkMetrics, Renderer, culling, gpu, prepare, validate_target_size,
};

const GPU_PREPARE_DESTRUCTION_PRESSURE_LIMIT: u64 = 16_384;

mod support;

fn gpu_triangle_primitives<'a>(
    primitives: &'a [prepare::PreparedPrimitive],
    work: Option<&PrepareWorkCounter>,
) -> Cow<'a, [prepare::PreparedPrimitive]> {
    if primitives
        .iter()
        .all(prepare::PreparedPrimitive::gpu_triangle_path)
    {
        return Cow::Borrowed(primitives);
    }
    let filtered = primitives
        .iter()
        .filter(|primitive| primitive.gpu_triangle_path())
        .cloned()
        .collect::<Vec<_>>();
    if let Some(work) = work {
        work.record_prepared_list_copy_bytes(
            (filtered.len() as u64)
                .saturating_mul(std::mem::size_of::<prepare::PreparedPrimitive>() as u64),
        );
    }
    Cow::Owned(filtered)
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn prepare_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(not(all(target_arch = "wasm32", feature = "demo-page")))]
fn prepare_now_ms() -> f64 {
    0.0
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_prepare_step(label: &str, start_ms: f64) -> f64 {
    let now = prepare_now_ms();
    if prepare_logging_enabled() {
        let elapsed_ms = now - start_ms;
        web_sys::console::log_1(&format!("[scena-prepare] {label}: {elapsed_ms:.1}ms").into());
    }
    now
}

#[cfg(not(all(target_arch = "wasm32", feature = "demo-page")))]
fn log_prepare_step(_label: &str, _start_ms: f64) -> f64 {
    0.0
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_dynamic_reject(reason: &str) {
    if prepare_logging_enabled() {
        web_sys::console::log_1(&format!("[scena-prepare] dynamic reject: {reason}").into());
    }
}

#[cfg(not(all(target_arch = "wasm32", feature = "demo-page")))]
fn log_dynamic_reject(_reason: &str) {}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn prepare_logging_enabled() -> bool {
    web_sys::window()
        .and_then(|window| {
            js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("location")).ok()
        })
        .and_then(|location| {
            js_sys::Reflect::get(&location, &wasm_bindgen::JsValue::from_str("search")).ok()
        })
        .and_then(|search| search.as_string())
        .is_some_and(|search| search.contains("perf=1") || search.contains("timing=1"))
}

impl Renderer {
    pub fn prepare(&mut self, scene: &mut Scene) -> Result<(), PrepareError> {
        self.prepare_inner::<()>(scene, None, None)
    }

    /// Prepares a scene while collecting deterministic CPU/GPU work counters.
    pub fn prepare_profiled(
        &mut self,
        scene: &mut Scene,
    ) -> Result<PrepareWorkMetrics, PrepareError> {
        let work = PrepareWorkCounter::default();
        self.prepare_inner::<()>(scene, None, Some(&work))?;
        Ok(work.snapshot())
    }

    pub fn prepare_with_assets<F>(
        &mut self,
        scene: &mut Scene,
        assets: &Assets<F>,
    ) -> Result<(), PrepareError> {
        self.prepare_inner(scene, Some(assets), None)
    }

    /// Prepares with assets while collecting deterministic CPU work counters.
    pub fn prepare_with_assets_profiled<F>(
        &mut self,
        scene: &mut Scene,
        assets: &Assets<F>,
    ) -> Result<PrepareWorkMetrics, PrepareError> {
        let work = PrepareWorkCounter::default();
        let lock_count_before = assets.storage_lock_acquisitions();
        self.prepare_inner(scene, Some(assets), Some(&work))?;
        work.record_asset_storage_locks(
            assets
                .storage_lock_acquisitions()
                .saturating_sub(lock_count_before),
        );
        Ok(work.snapshot())
    }

    fn prepare_inner<F>(
        &mut self,
        scene: &mut Scene,
        assets: Option<&Assets<F>>,
        work: Option<&PrepareWorkCounter>,
    ) -> Result<(), PrepareError> {
        let total_start = prepare_now_ms();
        let mut step_start = total_start;
        self.prepare_device_ready()?;
        if let Some(gpu) = self.gpu.as_mut() {
            let blocking = gpu.pending_destructions() > GPU_PREPARE_DESTRUCTION_PRESSURE_LIMIT;
            if blocking {
                let _ = gpu.poll_device();
            } else {
                let _ = gpu.poll_device_nonblocking();
            }
            self.stats.pending_destructions = gpu.pending_destructions();
            if let Some(work) = work {
                work.record_gpu_prepare_poll(blocking);
            }
        }
        self.diagnostics.clear();
        let target = self
            .prepare_target()
            .map_err(|()| PrepareError::InvalidTargetSize {
                width: self.target.width,
                height: self.target.height,
            })?;
        validate_target_size(target.width, target.height).map_err(|()| {
            PrepareError::InvalidTargetSize {
                width: target.width,
                height: target.height,
            }
        })?;
        let screen_space_scale = self.screen_space_prepare_scale();
        let mut diagnostics = self.configuration_diagnostics.clone();
        diagnostics.extend(prepare::collect_precision_diagnostics(
            scene,
            target.backend,
        ));
        diagnostics.extend(prepare::collect_camera_visibility_diagnostics(
            scene, target,
        ));
        if let Some(assets) = assets {
            diagnostics.extend(prepare::collect_asset_camera_visibility_diagnostics(
                scene, target, assets,
            ));
            diagnostics.extend(prepare::collect_material_texture_diagnostics(scene, assets));
        }
        step_start = log_prepare_step("diagnostics", step_start);
        let environment_desc = match self.environment {
            Some(environment) => {
                let Some(assets) = assets else {
                    return Err(PrepareError::EnvironmentAssetsRequired { environment });
                };
                Some(
                    assets
                        .environment(environment)
                        .ok_or(PrepareError::EnvironmentNotFound { environment })?,
                )
            }
            None => None,
        };
        let environment_prepare_stats = prepare::collect_environment_prepare_stats(
            environment_desc.as_ref(),
            self.target.backend,
        );
        let environment_count = u64::from(environment_desc.is_some());
        let output_plan = gpu::GpuOutputPlan::new(
            self.anti_aliasing,
            self.bloom.is_some(),
            self.screen_space_ambient_occlusion.is_some(),
            self.screen_space_reflections.is_some(),
            self.depth_of_field.is_some(),
            self.auto_exposure.is_some(),
        );
        let active_camera_projection = scene
            .active_camera()
            .and_then(|camera| camera::CameraProjection::from_scene(scene, camera, target).ok());
        let lighting_stats = prepare::collect_lighting_stats(scene, self.target.backend)?;
        let environment_lighting = self.environment_lighting_for_prepare(environment_desc.as_ref());
        let tiled_light_assignment = prepare::collect_gpu_tiled_light_assignment(
            scene,
            scene.origin_shift(),
            target,
            active_camera_projection.as_ref(),
        )?;
        let gpu_light_uniform = prepare::collect_gpu_light_uniform(
            scene,
            scene.origin_shift(),
            &environment_lighting,
            tiled_light_assignment.is_active(),
        );
        step_start = log_prepare_step("environment + lights", step_start);
        let backend_material_slots = if self.gpu.is_some() {
            prepare::collect_backend_material_slots(scene, assets)
        } else {
            Vec::new()
        };
        let backend_material_handles = backend_material_slots
            .iter()
            .map(|slot| slot.handle)
            .collect::<Vec<_>>();
        let logical_stats =
            prepare::collect_logical_resource_stats(scene, assets, environment_count);
        step_start = log_prepare_step("camera + backend material slots", step_start);
        if self.gpu.is_some() {
            if let Some(reason) =
                self.dynamic_gpu_prepare_rejection_reason(scene, &backend_material_handles)
            {
                log_dynamic_reject(reason);
            } else if let Some((dynamic_primitives, dynamic_strokes, dynamic_instances)) =
                self.reencode_retained_draws(scene)
            {
                let culled_primitives = culling::cull_prepared_primitives(
                    dynamic_primitives,
                    active_camera_projection.as_ref(),
                    target,
                    self.cpu_occlusion_culling
                        && scene.clipping_planes().planes().is_empty()
                        && scene.section_box().is_none(),
                    true,
                );
                let dynamic_culled_objects = culled_primitives.culled;
                let dynamic_primitives = culled_primitives.visible;
                match prepare::collect_dynamic_light_from_world(scene, assets) {
                    Ok(light_from_world) => {
                        let semantic_label_quad_count = self
                            .prepared
                            .as_ref()
                            .map(|prepared| prepared.labels.quads().len())
                            .unwrap_or(0);
                        let semantic_aov_capture_enabled = self.semantic_aov_capture_enabled;
                        if let Some(gpu) = &mut self.gpu {
                            match gpu.update_dynamic_draw_state(gpu::DynamicDrawStateUpdate {
                                target,
                                light_uniform: gpu_light_uniform,
                                light_from_world,
                                primitives: &dynamic_primitives,
                                instances: &dynamic_instances,
                                strokes: &dynamic_strokes,
                                semantic_aov_capture_enabled,
                                label_quad_count: semantic_label_quad_count,
                                work,
                            }) {
                                Ok(()) => {
                                    if let Some(prepared) = self.prepared.as_mut() {
                                        prepared.transform_revision = scene.transform_revision();
                                        prepared.camera_revision = scene.camera_revision();
                                        prepared.appearance_revision = scene.appearance_revision();
                                        prepared.visibility_revision = scene.visibility_revision();
                                        prepared.primitives = Arc::from(dynamic_primitives);
                                        prepared.strokes = Arc::from(dynamic_strokes);
                                        prepared.instances = Arc::from(dynamic_instances);
                                    }
                                    self.stats.instances = self
                                        .prepared
                                        .as_ref()
                                        .map(prepared_instance_count)
                                        .unwrap_or(0);
                                    self.stats.culled_objects = dynamic_culled_objects;
                                    self.stats.textures = logical_stats.textures;
                                    self.prepare_telemetry.dynamic_template_prepares = self
                                        .prepare_telemetry
                                        .dynamic_template_prepares
                                        .saturating_add(1);
                                    self.prepare_telemetry.draw_uniform_only_updates = self
                                        .prepare_telemetry
                                        .draw_uniform_only_updates
                                        .saturating_add(1);
                                    self.render_generation =
                                        self.render_generation.saturating_add(1);
                                    self.clear_rendered_frame();
                                    self.diagnostics = diagnostics;
                                    log_prepare_step("dynamic draw-uniform update", step_start);
                                    log_prepare_step("prepare_inner total", total_start);
                                    return Ok(());
                                }
                                Err(reason) => log_dynamic_reject(reason),
                            }
                        }
                    }
                    Err(_error) => log_dynamic_reject("dynamic shadow projection failed"),
                }
            } else {
                log_dynamic_reject("visible source missing from retained template");
            }
        }
        let backend_sampled_base_color_textures = backend_material_slots
            .iter()
            .filter_map(|slot| slot.base_color.as_ref().map(|texture| texture.handle))
            .collect::<Vec<_>>();
        let prepared_scene = prepare::collect_prepared_primitives_profiled(
            target,
            screen_space_scale,
            scene,
            assets,
            active_camera_projection.as_ref(),
            &backend_sampled_base_color_textures,
            &backend_material_handles,
            environment_lighting.clone(),
            work,
            Some(&mut self.shadow_visibility_cache),
        )?;
        self.prepare_telemetry.prepared_primitive_collections = self
            .prepare_telemetry
            .prepared_primitive_collections
            .saturating_add(1);
        self.prepare_telemetry.full_prepares =
            self.prepare_telemetry.full_prepares.saturating_add(1);
        step_start = log_prepare_step("collect_prepared_primitives", step_start);
        let light_from_world = prepared_scene.light_from_world;
        let mut retained_primitives = assign_original_vertex_offsets(prepared_scene.primitives);
        if let Some(work) = work {
            work.record_prepared_geometry_storage(prepare::share_model_space_vertex_buffer(
                &mut retained_primitives,
            ));
        } else {
            prepare::share_model_space_vertex_buffer(&mut retained_primitives);
        }
        let culled_primitives = culling::cull_prepared_primitives(
            retained_primitives.clone(),
            active_camera_projection.as_ref(),
            target,
            self.cpu_occlusion_culling
                && scene.clipping_planes().planes().is_empty()
                && scene.section_box().is_none(),
            self.gpu.is_some(),
        );
        let retained_primitives: Arc<[prepare::PreparedPrimitive]> = Arc::from(retained_primitives);
        let primitives: Arc<[prepare::PreparedPrimitive]> = Arc::from(culled_primitives.visible);
        let retained_strokes: Arc<[prepare::PreparedStrokeSegment]> =
            Arc::from(assign_original_stroke_indices(prepared_scene.strokes));
        let strokes = Arc::clone(&retained_strokes);
        let retained_labels = Arc::new(assign_original_label_quad_indices(prepared_scene.labels));
        let labels = Arc::clone(&retained_labels);
        let mut retained_instances = assign_original_instance_vertex_offsets(
            prepared_scene.instances,
            next_gpu_vertex_offset(&retained_primitives),
        );
        for set in &mut retained_instances {
            let storage = prepare::share_model_space_vertex_buffer(set.primitives_mut());
            if let Some(work) = work {
                work.record_prepared_geometry_storage(storage);
            }
        }
        let retained_instances: Arc<[prepare::PreparedInstanceSet]> = Arc::from(retained_instances);
        let instances = Arc::clone(&retained_instances);
        let gpu_retained_primitives = gpu_triangle_primitives(&retained_primitives, work);
        let gpu_primitives = gpu_triangle_primitives(&primitives, work);
        let depth_stats = prepare::collect_depth_prepass_stats_iter(
            primitives
                .iter()
                .chain(instances.iter().flat_map(|set| set.primitives().iter())),
            target.backend,
        );
        #[cfg(test)]
        let depth_stats = if self.depth_prepass_enabled_for_test {
            depth_stats
        } else {
            prepare::PreparedDepthStats::default()
        };
        self.apply_prepare_stats(
            logical_stats,
            environment_prepare_stats,
            lighting_stats,
            depth_stats,
            culled_primitives.culled,
            &backend_material_slots,
        );
        step_start = log_prepare_step("cull + stats", step_start);
        let semantic_aov_capture_enabled = self.semantic_aov_capture_enabled;
        if let Some(gpu) = &mut self.gpu {
            gpu.prepare(
                target,
                gpu_retained_primitives.as_ref(),
                gpu_primitives.as_ref(),
                &retained_instances,
                &instances,
                &retained_strokes,
                &strokes,
                &retained_labels,
                &labels,
                lighting_stats,
                gpu_light_uniform,
                light_from_world,
                depth_stats,
                &backend_material_slots,
                &environment_lighting,
                &tiled_light_assignment,
                semantic_aov_capture_enabled,
                output_plan,
                work,
            )?;
            let stats = gpu.prepared_resource_stats();
            if let Some(work) = work {
                work.record_gpu_resource_creations(
                    stats.buffers,
                    stats.textures,
                    stats.pipelines,
                    stats.bind_groups,
                    stats.shader_module_creations,
                );
            }
            let pending_destructions = gpu.pending_destructions();
            self.apply_gpu_resource_stats(stats, pending_destructions, logical_stats.textures);
            self.prepare_telemetry.static_gpu_resource_rebuilds = self
                .prepare_telemetry
                .static_gpu_resource_rebuilds
                .saturating_add(1);
            step_start = log_prepare_step("gpu.prepare", step_start);
        } else {
            self.stats.textures = logical_stats.textures;
            self.stats.material_bind_groups = 0;
        }
        self.prepared = Some(super::PreparedSceneState {
            scene: scene.identity(),
            structure_revision: scene.structure_revision(),
            transform_revision: scene.transform_revision(),
            camera_revision: scene.camera_revision(),
            appearance_revision: scene.appearance_revision(),
            visibility_revision: scene.visibility_revision(),
            environment_revision: self.environment_revision,
            target_revision: self.target_revision,
            output_resources_revision: self.output_resources_revision,
            retained_primitives,
            primitives,
            retained_strokes,
            strokes,
            retained_labels,
            labels,
            retained_instances,
            instances,
            clipping_planes: Arc::from(
                scene
                    .active_clipping_plane_values()
                    .collect::<Vec<ClippingPlane>>(),
            ),
            section_box: scene.section_box(),
        });
        self.stats.instances = self
            .prepared
            .as_ref()
            .map(prepared_instance_count)
            .unwrap_or(0);
        self.render_generation = self.render_generation.saturating_add(1);
        self.clear_rendered_frame();
        self.diagnostics = diagnostics;
        log_prepare_step("prepare_inner tail", step_start);
        log_prepare_step("prepare_inner total", total_start);
        Ok(())
    }
}
use std::borrow::Cow;
use std::sync::Arc;
