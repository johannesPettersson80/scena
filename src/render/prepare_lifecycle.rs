use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::scene::Scene;

use super::camera;
use super::{Renderer, culling, gpu, prepare, validate_target_size};

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
        self.prepare_inner::<()>(scene, None)
    }

    pub fn prepare_with_assets<F>(
        &mut self,
        scene: &mut Scene,
        assets: &Assets<F>,
    ) -> Result<(), PrepareError> {
        self.prepare_inner(scene, Some(assets))
    }

    fn prepare_inner<F>(
        &mut self,
        scene: &mut Scene,
        assets: Option<&Assets<F>>,
    ) -> Result<(), PrepareError> {
        let total_start = prepare_now_ms();
        let mut step_start = total_start;
        self.poll_device();
        self.diagnostics.clear();
        validate_target_size(self.target.width, self.target.height).map_err(|()| {
            PrepareError::InvalidTargetSize {
                width: self.target.width,
                height: self.target.height,
            }
        })?;
        let mut diagnostics = prepare::collect_precision_diagnostics(scene, self.target.backend);
        diagnostics.extend(prepare::collect_camera_visibility_diagnostics(
            scene,
            self.target,
        ));
        if let Some(assets) = assets {
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
        let environment_prepare_stats =
            prepare::collect_environment_prepare_stats(environment_desc.as_ref());
        let environment_count = u64::from(environment_desc.is_some());
        let lighting_stats = prepare::collect_lighting_stats(scene, self.target.backend)?;
        let environment_lighting = self.environment_lighting_for_prepare(environment_desc.as_ref());
        let gpu_light_uniform =
            prepare::collect_gpu_light_uniform(scene, scene.origin_shift(), &environment_lighting);
        step_start = log_prepare_step("environment + lights", step_start);
        let active_camera_projection = scene.active_camera().and_then(|camera| {
            camera::CameraProjection::from_scene(scene, camera, self.target).ok()
        });
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
            } else {
                let draw_uniform_pairs = dynamic_draw_uniform_pairs(scene);
                match prepare::collect_dynamic_light_from_world(scene, assets) {
                    Ok(light_from_world) => {
                        if let Some(gpu) = &mut self.gpu {
                            match gpu.update_dynamic_draw_uniforms(
                                self.target,
                                gpu_light_uniform,
                                light_from_world,
                                &draw_uniform_pairs,
                            ) {
                                Ok(()) => {
                                    if let Some(prepared) = self.prepared.as_mut() {
                                        prepared.transform_revision = scene.transform_revision();
                                    }
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
                                    self.last_rendered_generation = None;
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
            }
        }
        let backend_sampled_base_color_textures = backend_material_slots
            .iter()
            .filter_map(|slot| slot.base_color.as_ref().map(|texture| texture.handle))
            .collect::<Vec<_>>();
        let prepared_scene = prepare::collect_prepared_primitives(
            self.target,
            scene,
            assets,
            active_camera_projection.as_ref(),
            &backend_sampled_base_color_textures,
            &backend_material_handles,
            environment_lighting.clone(),
        )?;
        self.prepare_telemetry.prepared_primitive_collections = self
            .prepare_telemetry
            .prepared_primitive_collections
            .saturating_add(1);
        self.prepare_telemetry.full_prepares =
            self.prepare_telemetry.full_prepares.saturating_add(1);
        step_start = log_prepare_step("collect_prepared_primitives", step_start);
        let light_from_world = prepared_scene.light_from_world;
        let culled_primitives = culling::cull_prepared_primitives(
            prepared_scene.primitives,
            active_camera_projection.as_ref(),
            self.gpu.is_some(),
        );
        let primitives = culled_primitives.visible;
        let depth_stats = prepare::collect_depth_prepass_stats(&primitives, self.target.backend);
        self.apply_prepare_stats(
            logical_stats,
            environment_prepare_stats,
            lighting_stats,
            depth_stats,
            culled_primitives.culled,
            &backend_material_slots,
        );
        step_start = log_prepare_step("cull + stats", step_start);
        if let Some(gpu) = &mut self.gpu {
            gpu.prepare(
                self.target,
                &primitives,
                lighting_stats,
                gpu_light_uniform,
                light_from_world,
                depth_stats,
                &backend_material_slots,
                &environment_lighting,
            )?;
            let stats = gpu.prepared_resource_stats();
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
            environment_revision: self.environment_revision,
            target_revision: self.target_revision,
            debug_revision: self.debug_revision,
            primitives,
            clipping_planes: scene.active_clipping_plane_values().collect(),
        });
        self.render_generation = self.render_generation.saturating_add(1);
        self.last_rendered_generation = None;
        self.diagnostics = diagnostics;
        log_prepare_step("prepare_inner tail", step_start);
        log_prepare_step("prepare_inner total", total_start);
        Ok(())
    }

    fn dynamic_gpu_prepare_rejection_reason(
        &self,
        scene: &Scene,
        backend_material_handles: &[crate::assets::MaterialHandle],
    ) -> Option<&'static str> {
        let Some(prepared) = self.prepared.as_ref() else {
            return Some("no prepared template");
        };
        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Some("scene identity changed");
        }
        if prepared.structure_revision != scene.structure_revision() {
            return Some("structure revision changed");
        }
        if prepared.environment_revision != self.environment_revision {
            return Some("environment revision changed");
        }
        if prepared.target_revision != self.target_revision {
            return Some("target revision changed");
        }
        if prepared.debug_revision != self.debug_revision {
            return Some("debug revision changed");
        }
        if prepared.transform_revision == scene.transform_revision() {
            return None;
        }
        if scene.model_nodes().next().is_some() {
            return Some("model nodes present");
        }
        if scene.instance_set_nodes().next().is_some() {
            return Some("instance set nodes present");
        }
        if scene.label_nodes().next().is_some() {
            return Some("label nodes present");
        }
        if scene
            .mesh_nodes()
            .any(|(_node, mesh, _transform)| !backend_material_handles.contains(&mesh.material()))
        {
            return Some("moving mesh missing GPU material slot");
        }
        if !prepared
            .primitives
            .iter()
            .all(crate::geometry::Primitive::depth_prepass_eligible)
        {
            return Some("non-opaque primitive present");
        }
        None
    }

    fn apply_prepare_stats(
        &mut self,
        logical_stats: prepare::PreparedLogicalResourceStats,
        environment_prepare_stats: prepare::PreparedEnvironmentStats,
        lighting_stats: prepare::PreparedLightingStats,
        depth_stats: prepare::PreparedDepthStats,
        culled_objects: u64,
        backend_material_slots: &[prepare::PreparedMaterialSlot],
    ) {
        self.stats.materials = logical_stats.materials;
        self.stats.material_bindings = logical_stats.material_bindings;
        self.stats.material_texture_bindings = logical_stats.material_texture_bindings;
        self.stats.material_sampler_bindings = logical_stats.material_sampler_bindings;
        self.stats.material_textures_missing_decoded_pixels =
            logical_stats.material_textures_missing_decoded_pixels;
        self.stats.material_batch_layers =
            prepare::compute_material_batch_plan(backend_material_slots).layer_count;
        self.stats.environments = logical_stats.environments;
        self.stats.environment_cubemaps = environment_prepare_stats.cubemaps;
        self.stats.environment_prefilter_passes = environment_prepare_stats.prefilter_passes;
        self.stats.environment_brdf_luts = environment_prepare_stats.brdf_luts;
        self.stats.live_logical_handles = logical_stats.live_logical_handles;
        self.stats.shadow_maps = lighting_stats.shadow_maps;
        self.stats.depth_prepass_passes = depth_stats.passes;
        self.stats.depth_prepass_draws = depth_stats.draws;
        self.stats.directional_shadow_map_resolution =
            lighting_stats.directional_shadow_map_resolution;
        self.stats.directional_shadow_pcf_kernel = lighting_stats.directional_shadow_pcf_kernel;
        self.stats.culled_objects = culled_objects;
    }

    fn apply_gpu_resource_stats(
        &mut self,
        stats: gpu::GpuResourceStats,
        pending_destructions: u64,
        logical_texture_count: u64,
    ) {
        self.stats.buffers = stats.buffers;
        self.stats.textures = logical_texture_count;
        self.stats.render_targets = stats.render_targets;
        self.stats.pipelines = stats.pipelines;
        self.stats.bind_groups = stats.bind_groups;
        self.stats.shader_modules = stats.shader_modules;
        self.stats.pending_destructions = pending_destructions;
        self.stats.material_bind_groups = stats.material_bind_groups;
        self.stats.approximate_gpu_memory_bytes =
            (stats.approximate_gpu_memory_bytes > 0).then_some(stats.approximate_gpu_memory_bytes);
    }
}

fn dynamic_draw_uniform_pairs(scene: &Scene) -> Vec<([f32; 16], [f32; 16])> {
    let mut values = Vec::new();
    let origin_shift = scene.origin_shift();
    for (_renderable, transform) in scene.renderables() {
        push_dynamic_draw_uniform(&mut values, transform, origin_shift);
    }
    for (_node, _mesh, transform) in scene.mesh_nodes() {
        push_dynamic_draw_uniform(&mut values, transform, origin_shift);
    }
    if values.is_empty() {
        values.push((identity_matrix4(), identity_matrix4()));
    }
    values
}

fn push_dynamic_draw_uniform(
    values: &mut Vec<([f32; 16], [f32; 16])>,
    transform: crate::scene::Transform,
    origin_shift: crate::scene::Vec3,
) {
    let raw_world_from_model =
        prepare::transforms::world_from_model_matrix(transform, origin_shift);
    let raw_normal_from_model = prepare::transforms::normal_from_model_matrix(transform);
    let world_from_model = if prepare::transforms::invert_matrix4(&raw_world_from_model).is_some() {
        raw_world_from_model
    } else {
        identity_matrix4()
    };
    let normal_from_model = if prepare::transforms::invert_matrix4(&raw_normal_from_model).is_some()
    {
        raw_normal_from_model
    } else {
        identity_matrix4()
    };
    if values
        .iter()
        .any(|(existing_world, _)| *existing_world == world_from_model)
    {
        return;
    }
    values.push((world_from_model, normal_from_model));
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}
