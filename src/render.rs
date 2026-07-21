//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

#[cfg(not(target_arch = "wasm32"))]
use std::collections::VecDeque;
use std::{cell::Cell, marker::PhantomData};

#[cfg(feature = "inspection")]
pub mod animation_introspection;
#[cfg(feature = "inspection")]
pub mod appearance;
mod area_ltc;
mod backend_selection;
mod background;
mod build;
mod camera;
mod color_contract;
mod cpu;
mod cpu_geometry;
mod cpu_labels;
mod cpu_overlay;
mod cpu_reflections;
mod cpu_render;
mod cpu_resolve;
mod cpu_strokes;
mod cpu_transmission;
mod culling;
#[cfg(test)]
mod depth_prepass_tests;
mod environment_cache;
mod exposure;
mod frame;
mod gpu;
#[cfg(feature = "inspection")]
pub mod introspection;
mod offscreen;
mod output;
mod parallel;
mod pbr_brdf;
mod physical_transmission;
mod prepare;
mod prepare_lifecycle;
mod prepare_retained;
#[cfg(feature = "inspection")]
pub mod quality;
mod reporting;
#[cfg(feature = "inspection")]
mod screen_bounds;
mod screen_space_reflections;
#[cfg_attr(not(feature = "scene-host"), allow(dead_code))]
pub(crate) mod semantic_aov;
mod settings;
// PreparedSceneState stores clipping_planes: Vec<ClippingPlane> in state.rs.
mod state;
mod surface;
mod target;
#[cfg(feature = "inspection")]
pub mod visibility_diagnosis;
#[cfg(feature = "inspection")]
pub mod visual_repair;
mod work_metrics;

use crate::assets::EnvironmentHandle;
use crate::diagnostics::{
    Capabilities, ChangeKind, DevicePoll, DevicePollStatus, Diagnostic, GpuAdapterReport,
    NotPreparedReason, OutputColorSpace, RenderError, RenderOutcome, RendererStats,
};
use crate::material::Color;
use crate::scene::{CameraKey, ClippingPlane, Scene, SectionBox};

pub use self::backend_selection::HeadlessBackendSelectionReport;
pub use self::background::Background;
pub use self::exposure::{
    AutoExposureConfig, AutoExposureResult, estimate_auto_exposure_from_linear_colors,
    estimate_auto_exposure_from_srgb8,
};
use self::frame::depth_of_field_post_config;
use self::gpu::GpuDeviceState;
pub use self::offscreen::{OffscreenTarget, PixelReadback};
use self::output::OutputTransform;
pub use self::output::{
    AntiAliasing, DepthOfFieldConfig, OrderIndependentTransparencyConfig, PostBloomConfig,
    ReconstructionFilter, ScreenSpaceAmbientOcclusionConfig, Tonemapper,
};
#[doc(hidden)]
pub use self::prepare::{
    EnvironmentBakeMetrics, precompute_environment_sidecar, precompute_environment_sidecar_profiled,
};
pub use self::screen_space_reflections::ScreenSpaceReflectionConfig;
pub use self::settings::{Profile, Quality, RenderMode, RendererOptions};
use self::state::{PreparedSceneState, RenderedFrameState};
use self::target::{RasterTarget, backend_for_attached_surface, validate_target_size};
use self::work_metrics::PrepareTelemetry;
pub use self::work_metrics::{PrepareWorkMetrics, RenderReadbackMode, RenderWorkMetrics};

#[derive(Debug)]
pub struct Renderer {
    target: RasterTarget,
    prepared: Option<PreparedSceneState>,
    frame: Vec<u8>,
    fxaa_scratch: Vec<u8>,
    bloom_scratch: Vec<u8>,
    oit_scratch: Vec<cpu::OitAccumPixel>,
    cpu_supersample_frame: Vec<u8>,
    cpu_supersample_oit_scratch: Vec<cpu::OitAccumPixel>,
    // CPU-only linear scene-referred straight-alpha accumulator. Stores the source of truth
    // before every pixel is ACES+sRGB encoded into `frame`.
    linear_frame: Option<Vec<Color>>,
    // CPU-only camera-space depth buffer. Lower positive values are closer to the active camera.
    depth_frame: Option<Vec<f32>>,
    cpu_supersample_linear_frame: Vec<Color>,
    cpu_supersample_depth_frame: Vec<f32>,
    cpu_material_reflection_scratch: Vec<screen_space_reflections::MaterialReflectionPixel>,
    cpu_effect_rgba8_scratch: Vec<u8>,
    cpu_row_band_bins: cpu_render::CpuRowBandBins,
    gpu_supersample_frame: Vec<u8>,
    stats: RendererStats,
    diagnostics: Vec<Diagnostic>,
    configuration_diagnostics: Vec<Diagnostic>,
    capabilities: Capabilities,
    gpu: Option<GpuDeviceState>,
    output: OutputTransform,
    anti_aliasing: AntiAliasing,
    cpu_occlusion_culling: bool,
    semantic_aov_capture_enabled: bool,
    supersample_factor: u32,
    reconstruction_filter: ReconstructionFilter,
    order_independent_transparency: Option<OrderIndependentTransparencyConfig>,
    screen_space_ambient_occlusion: Option<ScreenSpaceAmbientOcclusionConfig>,
    screen_space_reflections: Option<ScreenSpaceReflectionConfig>,
    depth_of_field: Option<DepthOfFieldConfig>,
    bloom: Option<PostBloomConfig>,
    profile: Profile,
    quality: Quality,
    render_mode: RenderMode,
    output_color_space: OutputColorSpace,
    render_generation: u64,
    last_rendered_generation: Option<u64>,
    last_rendered_frame: Option<RenderedFrameState>,
    last_readback_frame: Option<RenderedFrameState>,
    surface_lost: Option<bool>,
    context_lost: Option<bool>,
    device_lost: Option<bool>,
    environment: Option<EnvironmentHandle>,
    environment_lighting_cache: environment_cache::EnvironmentLightingCache,
    background_color: Color,
    auto_exposure: Option<AutoExposureConfig>,
    last_auto_exposure: Option<AutoExposureResult>,
    environment_revision: u64,
    target_revision: u64,
    output_resources_revision: u64,
    prepare_telemetry: PrepareTelemetry,
    last_render_work_metrics: RenderWorkMetrics,
    #[cfg(test)]
    depth_prepass_enabled_for_test: bool,
    #[cfg(not(target_arch = "wasm32"))]
    _headless_gpu_test_guard: Option<build::HeadlessGpuTestSupportGuard>,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Default)]
pub(in crate::render) struct PrepareWorkCounter {
    prepared_triangle_count: Cell<u64>,
    prepared_model_vertex_buffer_count: Cell<u64>,
    prepared_model_vertex_bytes: Cell<u64>,
    prepared_unique_draw_transforms: Cell<u64>,
    prepared_draw_transform_bytes: Cell<u64>,
    prepared_triangle_reference_bytes: Cell<u64>,
    prepared_list_copy_bytes: Cell<u64>,
    asset_storage_lock_acquisitions: Cell<u64>,
    generated_tangent_calls: Cell<u64>,
    generated_tangent_triangles: Cell<u64>,
    generated_tangent_vertices: Cell<u64>,
    generated_tangent_cache_hits: Cell<u64>,
    generated_tangent_cache_misses: Cell<u64>,
    tangent_input_transform_bytes: Cell<u64>,
    tangent_output_bytes: Cell<u64>,
    deformed_vertex_bytes_materialized: Cell<u64>,
    shadow_rays: Cell<u64>,
    shadow_visibility_cache_hits: Cell<u64>,
    shadow_visibility_cache_misses: Cell<u64>,
    bvh_node_bounds_tests: Cell<u64>,
    ray_triangle_intersection_tests: Cell<u64>,
    area_light_samples: Cell<u64>,
    cpu_bake_subdivided_triangles: Cell<u64>,
    cpu_bake_shaded_vertices: Cell<u64>,
    texture_samples: Cell<u64>,
    cpu_bake_corner_bytes_copied: Cell<u64>,
    gpu_buffer_creations: Cell<u64>,
    gpu_texture_creations: Cell<u64>,
    gpu_pipeline_creations: Cell<u64>,
    gpu_bind_group_creations: Cell<u64>,
    gpu_shader_module_creations: Cell<u64>,
    gpu_triangle_shader_cache_hits: Cell<u64>,
    gpu_triangle_shader_cache_misses: Cell<u64>,
    gpu_nonblocking_polls: Cell<u64>,
    gpu_blocking_polls: Cell<u64>,
    draw_uniform_unique_values: Cell<u64>,
    draw_uniform_lookup_probes: Cell<u64>,
    draw_uniform_bytes_copied: Cell<u64>,
}

impl PrepareWorkCounter {
    pub(in crate::render) fn record_prepared_geometry_storage(
        &self,
        metrics: prepare::PreparedGeometryStorageMetrics,
    ) {
        self.prepared_triangle_count.set(
            self.prepared_triangle_count
                .get()
                .saturating_add(metrics.triangle_count),
        );
        self.prepared_model_vertex_buffer_count.set(
            self.prepared_model_vertex_buffer_count
                .get()
                .saturating_add(metrics.model_vertex_buffer_count),
        );
        self.prepared_model_vertex_bytes.set(
            self.prepared_model_vertex_bytes
                .get()
                .saturating_add(metrics.model_vertex_bytes),
        );
        self.prepared_unique_draw_transforms.set(
            self.prepared_unique_draw_transforms
                .get()
                .saturating_add(metrics.unique_draw_transforms),
        );
        self.prepared_draw_transform_bytes.set(
            self.prepared_draw_transform_bytes
                .get()
                .saturating_add(metrics.draw_transform_bytes),
        );
        self.prepared_triangle_reference_bytes.set(
            self.prepared_triangle_reference_bytes
                .get()
                .saturating_add(metrics.triangle_reference_bytes),
        );
    }

    pub(in crate::render) fn record_prepared_list_copy_bytes(&self, bytes: u64) {
        self.prepared_list_copy_bytes
            .set(self.prepared_list_copy_bytes.get().saturating_add(bytes));
    }

    pub(in crate::render) fn record_asset_storage_locks(&self, acquisitions: u64) {
        self.asset_storage_lock_acquisitions.set(
            self.asset_storage_lock_acquisitions
                .get()
                .saturating_add(acquisitions),
        );
    }

    pub(in crate::render) fn record_generated_tangents(
        &self,
        triangles: usize,
        vertices: usize,
        input_transform_bytes: u64,
        output_bytes: u64,
    ) {
        self.generated_tangent_calls
            .set(self.generated_tangent_calls.get().saturating_add(1));
        self.generated_tangent_triangles.set(
            self.generated_tangent_triangles
                .get()
                .saturating_add(triangles as u64),
        );
        self.generated_tangent_vertices.set(
            self.generated_tangent_vertices
                .get()
                .saturating_add(vertices as u64),
        );
        self.tangent_input_transform_bytes.set(
            self.tangent_input_transform_bytes
                .get()
                .saturating_add(input_transform_bytes),
        );
        self.tangent_output_bytes
            .set(self.tangent_output_bytes.get().saturating_add(output_bytes));
    }

    pub(in crate::render) fn record_generated_tangent_cache(&self, hit: bool) {
        let counter = if hit {
            &self.generated_tangent_cache_hits
        } else {
            &self.generated_tangent_cache_misses
        };
        counter.set(counter.get().saturating_add(1));
    }

    pub(in crate::render) fn record_tangent_output_bytes(&self, output_bytes: u64) {
        self.tangent_output_bytes
            .set(self.tangent_output_bytes.get().saturating_add(output_bytes));
    }

    pub(in crate::render) fn record_deformed_vertex_bytes(&self, bytes: u64) {
        self.deformed_vertex_bytes_materialized.set(
            self.deformed_vertex_bytes_materialized
                .get()
                .saturating_add(bytes),
        );
    }

    pub(in crate::render) fn record_shadow_ray(&self) {
        self.shadow_rays
            .set(self.shadow_rays.get().saturating_add(1));
    }

    pub(in crate::render) fn record_bvh_node_bounds_tests(&self, tests: u64) {
        self.bvh_node_bounds_tests
            .set(self.bvh_node_bounds_tests.get().saturating_add(tests));
    }

    pub(in crate::render) fn record_shadow_visibility_cache(&self, hit: bool) {
        let counter = if hit {
            &self.shadow_visibility_cache_hits
        } else {
            &self.shadow_visibility_cache_misses
        };
        counter.set(counter.get().saturating_add(1));
    }

    pub(in crate::render) fn record_ray_triangle_intersection_test(&self) {
        self.ray_triangle_intersection_tests
            .set(self.ray_triangle_intersection_tests.get().saturating_add(1));
    }

    pub(in crate::render) fn record_area_light_sample(&self) {
        self.area_light_samples
            .set(self.area_light_samples.get().saturating_add(1));
    }

    pub(in crate::render) fn record_cpu_bake_triangles(
        &self,
        triangles: usize,
        corner_bytes_copied: u64,
    ) {
        self.cpu_bake_subdivided_triangles.set(
            self.cpu_bake_subdivided_triangles
                .get()
                .saturating_add(triangles as u64),
        );
        self.cpu_bake_corner_bytes_copied.set(
            self.cpu_bake_corner_bytes_copied
                .get()
                .saturating_add(corner_bytes_copied),
        );
    }

    pub(in crate::render) fn record_cpu_bake_shaded_vertex(&self, texture_samples: u64) {
        self.cpu_bake_shaded_vertices
            .set(self.cpu_bake_shaded_vertices.get().saturating_add(1));
        self.record_texture_samples(texture_samples);
    }

    pub(in crate::render) fn record_texture_samples(&self, samples: u64) {
        self.texture_samples
            .set(self.texture_samples.get().saturating_add(samples));
    }

    pub(in crate::render) fn record_gpu_resource_creations(
        &self,
        buffers: u64,
        textures: u64,
        pipelines: u64,
        bind_groups: u64,
        shader_modules: u64,
    ) {
        self.gpu_buffer_creations
            .set(self.gpu_buffer_creations.get().saturating_add(buffers));
        self.gpu_texture_creations
            .set(self.gpu_texture_creations.get().saturating_add(textures));
        self.gpu_pipeline_creations
            .set(self.gpu_pipeline_creations.get().saturating_add(pipelines));
        self.gpu_bind_group_creations.set(
            self.gpu_bind_group_creations
                .get()
                .saturating_add(bind_groups),
        );
        self.gpu_shader_module_creations.set(
            self.gpu_shader_module_creations
                .get()
                .saturating_add(shader_modules),
        );
    }

    pub(in crate::render) fn record_gpu_triangle_shader_cache(&self, hit: bool) {
        let counter = if hit {
            &self.gpu_triangle_shader_cache_hits
        } else {
            &self.gpu_triangle_shader_cache_misses
        };
        counter.set(counter.get().saturating_add(1));
    }

    pub(in crate::render) fn record_gpu_prepare_poll(&self, blocking: bool) {
        let counter = if blocking {
            &self.gpu_blocking_polls
        } else {
            &self.gpu_nonblocking_polls
        };
        counter.set(counter.get().saturating_add(1));
    }

    pub(in crate::render) fn record_draw_uniform_indexing(
        &self,
        unique_values: usize,
        lookup_probes: u64,
        bytes_copied: u64,
    ) {
        self.draw_uniform_unique_values.set(
            self.draw_uniform_unique_values
                .get()
                .saturating_add(unique_values as u64),
        );
        self.draw_uniform_lookup_probes.set(
            self.draw_uniform_lookup_probes
                .get()
                .saturating_add(lookup_probes),
        );
        self.draw_uniform_bytes_copied.set(
            self.draw_uniform_bytes_copied
                .get()
                .saturating_add(bytes_copied),
        );
    }

    pub(in crate::render) fn snapshot(&self) -> PrepareWorkMetrics {
        PrepareWorkMetrics {
            prepared_triangle_count: self.prepared_triangle_count.get(),
            prepared_model_vertex_buffer_count: self.prepared_model_vertex_buffer_count.get(),
            prepared_model_vertex_bytes: self.prepared_model_vertex_bytes.get(),
            prepared_unique_draw_transforms: self.prepared_unique_draw_transforms.get(),
            prepared_draw_transform_bytes: self.prepared_draw_transform_bytes.get(),
            prepared_triangle_reference_bytes: self.prepared_triangle_reference_bytes.get(),
            prepared_list_copy_bytes: self.prepared_list_copy_bytes.get(),
            asset_storage_lock_acquisitions: self.asset_storage_lock_acquisitions.get(),
            generated_tangent_calls: self.generated_tangent_calls.get(),
            generated_tangent_triangles: self.generated_tangent_triangles.get(),
            generated_tangent_vertices: self.generated_tangent_vertices.get(),
            generated_tangent_cache_hits: self.generated_tangent_cache_hits.get(),
            generated_tangent_cache_misses: self.generated_tangent_cache_misses.get(),
            tangent_input_transform_bytes: self.tangent_input_transform_bytes.get(),
            tangent_output_bytes: self.tangent_output_bytes.get(),
            deformed_vertex_bytes_materialized: self.deformed_vertex_bytes_materialized.get(),
            shadow_rays: self.shadow_rays.get(),
            shadow_visibility_cache_hits: self.shadow_visibility_cache_hits.get(),
            shadow_visibility_cache_misses: self.shadow_visibility_cache_misses.get(),
            bvh_node_bounds_tests: self.bvh_node_bounds_tests.get(),
            ray_triangle_intersection_tests: self.ray_triangle_intersection_tests.get(),
            area_light_samples: self.area_light_samples.get(),
            cpu_bake_subdivided_triangles: self.cpu_bake_subdivided_triangles.get(),
            cpu_bake_shaded_vertices: self.cpu_bake_shaded_vertices.get(),
            texture_samples: self.texture_samples.get(),
            cpu_bake_corner_bytes_copied: self.cpu_bake_corner_bytes_copied.get(),
            gpu_buffer_creations: self.gpu_buffer_creations.get(),
            gpu_texture_creations: self.gpu_texture_creations.get(),
            gpu_pipeline_creations: self.gpu_pipeline_creations.get(),
            gpu_bind_group_creations: self.gpu_bind_group_creations.get(),
            gpu_shader_module_creations: self.gpu_shader_module_creations.get(),
            gpu_triangle_shader_cache_hits: self.gpu_triangle_shader_cache_hits.get(),
            gpu_triangle_shader_cache_misses: self.gpu_triangle_shader_cache_misses.get(),
            gpu_nonblocking_polls: self.gpu_nonblocking_polls.get(),
            gpu_blocking_polls: self.gpu_blocking_polls.get(),
            draw_uniform_unique_values: self.draw_uniform_unique_values.get(),
            draw_uniform_lookup_probes: self.draw_uniform_lookup_probes.get(),
            draw_uniform_bytes_copied: self.draw_uniform_bytes_copied.get(),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod movement_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod phase4_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod phase5_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod post_quality_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod post_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(gpu) = &mut self.gpu {
            gpu.release_prepared_resources();
            let _ = gpu.poll_device();
        }
    }
}
