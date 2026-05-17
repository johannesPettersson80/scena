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
        let logical_stats =
            prepare::collect_logical_resource_stats(scene, assets, environment_count);
        step_start = log_prepare_step("camera + backend material slots", step_start);
        let backend_sampled_base_color_textures = backend_material_slots
            .iter()
            .filter_map(|slot| slot.base_color.as_ref().map(|texture| texture.handle))
            .collect::<Vec<_>>();
        let backend_material_handles = backend_material_slots
            .iter()
            .map(|slot| slot.handle)
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
