use crate::app::prelude::*;

pub(crate) fn check_diagnostics_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/diagnostics/diagnostic.rs",
        &[
            "pub struct Diagnostic",
            "pub code: DiagnosticCode",
            "pub severity: DiagnosticSeverity",
            "pub message: String",
            "pub help: Option<String>",
            "pub struct DiagnosticContext",
            "#[serde(skip)]",
            "pub fn node(&self) -> Option<NodeKey>",
            "pub(crate) fn warning_for_node",
            "pub(crate) fn error_for_node",
            "pub enum DiagnosticCode",
            "InvalidCameraProjection",
            "ObjectsBehindCamera",
            "SceneOutsideCameraFrustum",
            "LargeScenePrecisionRisk",
            "DepthPrecisionRisk",
            "WebGl2DepthCompatibility",
            "pub enum DiagnosticSeverity",
            "pub fn warning",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/diagnostics.rs",
        &[
            "pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticContext, DiagnosticSeverity}",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/lib.rs",
        &["Diagnostic", "DiagnosticCode", "DiagnosticSeverity"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render.rs",
        &["diagnostics: Vec<Diagnostic>"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render/prepare_lifecycle.rs",
        &[
            "self.diagnostics.clear()",
            "prepare::collect_precision_diagnostics(scene, self.target.backend)",
            "prepare::collect_camera_visibility_diagnostics",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render/reporting.rs",
        &[
            "prepare::collect_camera_projection_diagnostics(scene)",
            "prepare::collect_asset_camera_visibility_diagnostics",
            "pub fn diagnostics(&self) -> &[Diagnostic]",
            "pub fn diagnose_scene_with_assets",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render/prepare/diagnostics.rs",
        &[
            "pub(in crate::render) fn collect_precision_diagnostics",
            "pub(in crate::render) fn collect_camera_projection_diagnostics",
            "pub(in crate::render) fn collect_camera_visibility_diagnostics",
            "pub(in crate::render) fn collect_asset_camera_visibility_diagnostics",
            "LARGE_SCENE_TRANSLATION_WARNING: f32 = 10_000.0",
            "DEPTH_RANGE_RATIO_WARNING: f32 = 100_000.0",
            "DiagnosticCode::InvalidCameraProjection",
            "DiagnosticCode::LargeScenePrecisionRisk",
            "DiagnosticCode::DepthPrecisionRisk",
            "Diagnostic::warning_for_node",
            "Diagnostic::error_for_node",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "scene.mesh_bounds_nodes()",
            "mesh bounds",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render/prepare/diagnostics.rs",
        &["node {node:?}", "camera node {node:?}"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/scene.rs",
        &[
            "pub(crate) fn node_transforms",
            "pub(crate) fn camera_nodes",
            "pub(crate) fn mesh_bounds_nodes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "prepare_emits_structured_depth_precision_warnings",
            "DiagnosticCode::DepthPrecisionRisk",
            "DiagnosticCode::LargeScenePrecisionRisk",
            "DiagnosticSeverity::Warning",
            "diagnostic.node()",
            "!diagnostic.message().contains(\"NodeKey(\")",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "m7_diagnostics_report_invalid_camera_projection_before_empty_frame",
            "m7_diagnostics_report_camera_visibility_failures_before_empty_frame",
            "m7_diagnostics_report_import_bounds_outside_camera_frustum",
            "m7_diagnostics_with_assets_report_direct_mesh_bounds_outside_camera_frustum",
            "m7_frame_all_uses_imported_mesh_bounds_without_manual_bounds_math",
            "frame_node",
            "DiagnosticCode::InvalidCameraProjection",
            "DiagnosticCode::ObjectsBehindCamera",
            "DiagnosticCode::SceneOutsideCameraFrustum",
            "DiagnosticSeverity::Error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "docs/specs/public-api.md",
        &[
            "pub struct Diagnostic",
            "InvalidCameraProjection",
            "ObjectsBehindCamera",
            "SceneOutsideCameraFrustum",
            "LargeScenePrecisionRisk",
            "DepthPrecisionRisk",
            "far/near ratio greater than",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Large-scene precision diagnostics", "ARCH-DIAGNOSTICS"],
    );
}

pub(crate) fn check_renderer_stats_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/diagnostics/stats.rs",
        &[
            "pub struct RendererStats",
            "pub buffers: u64",
            "pub textures: u64",
            "pub materials: u64",
            "pub render_targets: u64",
            "pub pipelines: u64",
            "pub bind_groups: u64",
            "pub shader_modules: u64",
            "pub environments: u64",
            "pub environment_cubemaps: u64",
            "pub environment_prefilter_passes: u64",
            "pub environment_brdf_luts: u64",
            "pub scene_imports: u64",
            "pub shadow_maps: u64",
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
            "pub ambient_occlusion_passes: u64",
            "pub bloom_passes: u64",
            "pub fxaa_passes: u64",
            "pub live_logical_handles: u64",
            "pub pending_destructions: u64",
            "pub gpu_draw_submissions: u64",
            "pub instances: u64",
            "pub approximate_gpu_memory_bytes: Option<u64>",
            "pub gpu_frame_ms: Option<f32>",
            "pub directional_shadow_map_resolution: Option<u32>",
            "pub directional_shadow_pcf_kernel: Option<u8>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/diagnostics.rs",
        &[
            "pub use stats::RendererStats",
            "pub struct DevicePoll",
            "pub destroyed_resources: u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) struct PreparedEnvironmentStats",
            "pub(in crate::render) struct PreparedDepthStats",
            "pub(in crate::render) fn collect_environment_prepare_stats",
            "pub(in crate::render) fn collect_depth_prepass_stats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare/resources.rs",
        &[
            "pub(in crate::render) struct PreparedLogicalResourceStats",
            "pub(in crate::render) fn collect_logical_resource_stats",
            "material.base_color_texture()",
            "live_logical_handles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/stats.rs",
        &[
            "pub(in crate::render) struct GpuResourceStats",
            "fn estimate_prepared_resource_stats",
            "approximate_gpu_memory_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu.rs",
        &[
            "mod lifecycle;",
            "pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/lifecycle.rs",
        &[
            "pub(in crate::render) fn pending_destructions(&self) -> u64",
            "pub(in crate::render) fn poll_device(&mut self) -> (u64, bool)",
            "pub(in crate::render) fn release_prepared_resources(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render.rs",
        &["pub fn poll_device(&mut self) -> DevicePoll"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare_lifecycle.rs",
        &[
            "self.stats.live_logical_handles = logical_stats.live_logical_handles",
            "self.stats.shadow_maps = lighting_stats.shadow_maps",
            "self.stats.depth_prepass_passes = depth_stats.passes",
            "self.stats.depth_prepass_draws = depth_stats.draws",
            "self.stats.textures = logical_stats.textures",
            "self.stats.environment_cubemaps = environment_prepare_stats.cubemaps",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render.rs",
        &[
            "let mut gpu_draw_submissions = 0",
            "gpu_draw_submissions = gpu_result.draw_submissions",
            "self.stats.gpu_draw_submissions = gpu_draw_submissions",
            "self.stats.instances = self",
            ".map(|set| set.instances().len() as u64)",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render.rs",
        &[
            "gpu_draw_submissions = primitive_count",
            "gpu_draw_submissions = self.stats.triangles",
            "gpu_draw_submissions = self.stats.primitives",
            "self.stats.gpu_draw_submissions = primitive_count",
            "self.stats.gpu_draw_submissions = self.stats.triangles",
            "self.stats.gpu_draw_submissions = self.stats.primitives",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/draw.rs",
        &[
            "let mut draw_submissions = 0",
            "GpuRenderResult {",
            "draw_submissions,",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/pipeline.rs",
        &["*inputs.draw_submissions = inputs.draw_submissions.saturating_add(1)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/phase4_tests.rs",
        &[
            "gpu_stats_report_submission_and_instance_counts_without_renaming_legacy_aliases",
            "stats.gpu_draw_submissions > 0 && stats.gpu_draw_submissions < stats.triangles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "tests/m1_geometry_materials.rs",
        &[
            "m1_cpu_resource_lifetime_counters_return_to_baseline",
            "m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare",
            "m1_headless_gpu_resource_counters_return_to_baseline_after_empty_reprepare",
            "poll.pending_destructions_before",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/stats.rs",
        &[
            "estimates_prepared_headless_gpu_resource_counters",
            "estimates_empty_headless_gpu_resource_counters_at_baseline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "docs/specs/public-api.md",
        &[
            "pub struct RendererStats",
            "pub struct DevicePoll",
            "shadow_maps",
            "depth_prepass_passes",
            "depth_prepass_draws",
            "ambient_occlusion_passes",
            "fxaa_passes",
            "live_logical_handles",
            "pub buffers: u64",
            "pub target_height: u32",
            "logical `TextureHandle` values only",
        ],
    );
}

pub(crate) fn check_headless_gpu_test_guard_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-HEADLESS-GPU-TEST-GUARD",
        "src/render/build.rs",
        &[
            "HEADLESS_GPU_TEST_SUPPORT_SLOT",
            "HEADLESS_GPU_TEST_SUPPORT_DEPTH",
            "HeadlessGpuTestSupportGuard::acquire()",
            "compare_exchange",
            "pollster::block_on(gpu::request_headless_gpu(Backend::HeadlessGpu))",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-HEADLESS-GPU-TEST-GUARD",
        "src/render.rs",
        &["_headless_gpu_test_guard: Option<build::HeadlessGpuTestSupportGuard>"],
    );

    let render_build = root.join("src/render/build.rs");
    let Ok(text) = fs::read_to_string(&render_build) else {
        return;
    };
    let Some(headless_body) = braced_body_after(&text, "pub fn headless_gpu") else {
        findings.push(Finding::new(
            "ARCH-HEADLESS-GPU-TEST-GUARD",
            "src/render/build.rs is missing native Renderer::headless_gpu",
        ));
        return;
    };
    if !headless_body.contains("Self::headless_gpu_with_options") {
        findings.push(Finding::new(
            "ARCH-HEADLESS-GPU-TEST-GUARD",
            "Renderer::headless_gpu must delegate to the guarded options-aware constructor",
        ));
    }
    let Some(options_body) = braced_body_after(&text, "pub fn headless_gpu_with_options") else {
        findings.push(Finding::new(
            "ARCH-HEADLESS-GPU-TEST-GUARD",
            "src/render/build.rs is missing native Renderer::headless_gpu_with_options",
        ));
        return;
    };
    let guard_index = options_body.find("HeadlessGpuTestSupportGuard::acquire()");
    let request_index = options_body.find("gpu::request_headless_gpu");
    match (guard_index, request_index) {
        (Some(guard_index), Some(request_index)) if guard_index < request_index => {}
        _ => findings.push(Finding::new(
            "ARCH-HEADLESS-GPU-TEST-GUARD",
            "Renderer::headless_gpu_with_options must acquire the shared headless GPU test-support guard before device creation",
        )),
    }
}

pub(crate) fn check_render_world_bake_contracts(root: &Path, findings: &mut Vec<Finding>) {
    // Per-draw model/normal uniforms: prepared primitives must carry world_from_model
    // metadata via prepared_primitive(...) instead of being orchestrated through the bare
    // transform_primitive(...) baker that drops the per-renderable transform on the floor.
    // transforms.rs, shadows.rs, diagnostics.rs, and tangents.rs still call transform_primitive
    // and transform_position internally for ray-cast, bounds, and tangent helpers — those
    // call sites operate on local copies that never reach the GPU upload path.
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare.rs",
        &["prepared_primitive(primitive, transform, origin_shift)"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare.rs",
        &["transform_primitive(primitive, transform, origin_shift)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare/transforms.rs",
        &[
            "pub(super) fn prepared_primitive",
            "pub(in crate::render) fn world_from_model_matrix",
            "pub(in crate::render) fn normal_from_model_matrix",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/geometry/primitive.rs",
        &[
            "pub(crate) fn with_world_from_model",
            "pub(crate) fn world_from_model",
            "pub(crate) fn normal_from_model",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/gpu/vertices.rs",
        &[
            "pub(super) draw_uniform_index: u32",
            "pub(super) struct DrawUniformValue",
            "pub(super) world_from_model: [f32; 16]",
            "pub(super) normal_from_model: [f32; 16]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/gpu/vertices.rs",
        &[
            "unbake_position_to_model_space",
            "unbake_normal_to_model_space",
            "primitive.world_from_model()",
            "primitive.normal_from_model()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/gpu/pipeline.rs",
        &[
            "draw_batches: &'a [PrimitiveDrawBatch]",
            "batch.draw_uniform_index",
            "pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset])",
        ],
    );
}
