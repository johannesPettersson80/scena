use crate::app::prelude::*;
pub(crate) fn check_m4_platform_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_c06_finite_atomic_contracts(root, findings);
    check_c07_handle_namespace_contracts(root, findings);
    check_c08_presentation_timeline_contracts(root, findings);
    check_c09_gpu_resource_lifecycle_contracts(root, findings);
    check_c10_overlay_ownership_contracts(root, findings);
    check_c12_deformed_picking_contracts(root, findings);
    check_c13_strict_gpu_construction_contracts(root, findings);
    check_scene_host_input_validation_contracts(root, findings);
    check_phase1_appearance_dirty_contracts(root, findings);
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/scene/dirty.rs",
        &[
            "pub struct SceneDirtyState",
            "transform_revision",
            "pub fn dirty_state",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/diagnostics/capabilities/capability_types.rs",
        &[
            "pub enum HardwareTier",
            "pub enum OutputColorSpace",
            "PbrNeutralDisplayP3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/diagnostics/capabilities.rs",
        &[
            "pub hardware_tier: HardwareTier",
            "pub gpu_frustum_culling: CapabilityStatus",
            "pub per_instance_culling: CapabilityStatus",
            "pub texture_compression_basisu: CapabilityStatus",
            "pub hardware_instancing: CapabilityStatus",
            "pub fragment_high_precision: CapabilityStatus",
            "pub uniform_buffers: CapabilityStatus",
            "pub uniform_buffer_max_bytes: u32",
            "pub compute_shaders: CapabilityStatus",
            "pub storage_buffers: CapabilityStatus",
            "with_display_p3_output",
            "Rgba8UnormSrgb+DisplayP3Canvas",
            "uniform_buffer_max_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/diagnostics/capability_status.rs",
        &[
            "HardwareTier::Medium",
            "Backend::WebGl2 => 128",
            "wide_gamut_output_status",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/settings.rs",
        &[
            "pub enum Profile",
            "pub enum Quality",
            "pub enum RenderMode",
            "pub struct RendererOptions",
            "with_output_color_space",
            "output_color_space",
            "OnChange",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render.rs",
        &[
            "render_generation",
            "skipped_frames",
            "gpu_culling_dispatches",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/prepare_lifecycle.rs",
        &["culling::cull_prepared_primitives"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/build.rs",
        &[
            "headless_with_options",
            "from_surface_with_options",
            "options.output_color_space()",
            "RenderMode::OnChange",
            "resolve_quality",
            "resolve_render_mode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/gpu/browser_color_space.rs",
        &[
            "scenaPrepareBrowserCanvasOutputColorSpace",
            "GPUCanvasConfiguration.colorSpace",
            "drawingBufferColorSpace",
            "RendererOptions::with_output_color_space",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/surface.rs",
        &[
            "handle_surface_event",
            "recover_surface",
            "recover_context",
            "RetainPolicy::Never",
            "loss_error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/platform.rs",
        &[
            "ScaleFactorChanged",
            "Occluded",
            "Lost",
            "ContextLost",
            "ContextRestored",
            "DeviceLost",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/culling.rs",
        &["cull_cpu_frustum", "outside_camera_clip_box", "culled"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/controls.rs",
        &[
            "pub struct OrbitControls",
            "pub struct PointerEvent",
            "pub enum PointerButton",
            "pub enum OrbitControlAction",
            "handle_pointer",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "Cargo.toml",
        &[
            "controls = []",
            "controls-winit = [\"controls\"]",
            "controls-web = [\"controls\"]",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/m4_performance_platform.rs",
        &[
            "capability_matrix_reports_hardware_tier_and_backend_feature_states",
            "texture_compression_basisu",
            "screen_space_ambient_occlusion",
            "physical_glass_transmission",
            "subtle output bloom is an explicit postprocess",
            "AmbientOcclusionDisabled",
            "PhysicalGlassTransmissionDegraded",
            "hardware_instancing",
            // Phase 1F: Capabilities::texture_arrays + max_texture_array_layers
            // gate the per-role 2D-array texture batching planned for step 2.
            // The capability matrix test pins the WebGPU/WebGL2 minimum (256
            // layers) and the headless-CPU absence (FeatureDisabled / 0).
            "texture_arrays",
            "max_texture_array_layers",
            "fragment_high_precision",
            "uniform_buffer_max_bytes",
            "transform_dirty_state_propagates_through_world_transform_queries",
            "renderer_options_apply_profile_quality_and_render_mode_precedence",
            "display_p3_output_requires_explicit_canvas_configuration_proof",
            "on_change_render_static_idle_records_skipped_frame_stats",
            "render_on_change_static_idle_skip_has_zero_allocations",
            "cpu_frustum_culling_drops_offscreen_renderables_before_draw",
            "per_instance_cpu_culling_keeps_visible_instances_and_counts_culled_ones",
            "gpu_capable_renderer_records_compute_culling_dispatch_when_available",
            "surface_loss_requires_recovery_and_prepare_before_render",
            "dpr_change_marks_surface_state_dirty_until_prepare",
            "context_recovery_rejects_assets_without_retained_cpu_data",
            "public_threading_contract_is_statically_enforced",
            "orbit_controls_are_platform_neutral_pointer_actions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m4_platform_smoke.html",
        &[
            "scena.capabilities.v1",
            "linux-webgpu-chromium",
            "linux-webgl2-chromium",
            "gpu_frustum_culling",
            "per_instance_culling",
            "texture_compression_basisu",
            "screen_space_ambient_occlusion",
            "order_independent_transparency",
            "physical_glass_transmission",
            "wide_gamut_output",
            "drawingBufferColorSpace",
            "bloom",
            "hardware_instancing",
            "fragment_high_precision",
            "uniform_buffers",
            "event_sequence",
            "recover_context",
            "webglcontextlost",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m4_platform_smoke.js",
        &[
            "m4-platform-browser-smoke",
            "webgl2",
            "webgpu",
            "capabilities",
            "color_space",
            "loss",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &["scenaM6DisplayP3OutputProbe", "canvas_output_color_space"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &["assertDisplayP3OutputProof", "scenaM6DisplayP3OutputProbe"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "docs/checklists/m4-performance-platform.md",
        &[
            "m4_performance_platform",
            "m4-platform-browser-smoke.json",
            "m4-wasm-size.json",
            "brotli_q11_bytes",
            "ARCH-M4-PLATFORM",
        ],
    );
}

fn check_scene_host_input_validation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SCENE-HOST-INPUTS",
        "src/scene_host/inputs.rs",
        &[
            "SceneHostErrorCode::InvalidInput",
            "QUATERNION_NORM_TOLERANCE",
            "validate_finite_components(\"translation\"",
            "validate_finite_components(\"rotation\"",
            "validate_finite_components(\"scale\"",
            "normalize_scene_host_quaternion",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-HOST-INPUTS",
        "src/scene_host/transforms.rs",
        &[
            "validate_transform(transform)?",
            "set_transforms_components",
            "transform_from_component_array",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-HOST-INPUTS",
        "src/scene_host/subtree.rs",
        &["set_visible", "set_subtree_tint"],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-HOST-INPUTS",
        "src/scene_host/wasm_transforms.rs",
        &[
            "setTransformsTyped",
            "js_sys::BigUint64Array",
            "js_sys::Float32Array",
            "components.length()",
            "set_transforms_components",
        ],
    );
}

fn check_phase1_appearance_dirty_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/scene/dirty.rs",
        &["appearance_revision", "visibility_revision"],
    );
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/scene/materials.rs",
        &[
            "tint_requires_structure_revision",
            "self.appearance_revision = self.appearance_revision.saturating_add(1)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/scene/visibility.rs",
        &["self.visibility_revision = self.visibility_revision.saturating_add(1)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/render/prepare/types.rs",
        &[
            "PreparedPrimitive",
            "source_node",
            "original_vertex_offset",
            "tint",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/render/gpu/draw_uniform.rs",
        &[
            "DRAW_UNIFORM_ENTRY_SIZE: u64 = 160",
            "value.tint",
            "value.semantic_id",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/render/gpu/output_shader.wgsl",
        &["tint: vec4<f32>", "draw.tint"],
    );
    require_contains(
        root,
        findings,
        "ARCH-APPEARANCE-DIRTY",
        "src/render/prepare_lifecycle.rs",
        &["appearance_revision", "reencode_retained_draws"],
    );

    let materials = root.join("src/scene/materials.rs");
    if let Ok(text) = fs::read_to_string(&materials)
        && let Some(body) = braced_body_after(&text, "pub fn set_node_tint")
        && body.contains("self.structure_revision = self.structure_revision.saturating_add(1)")
        && !body.contains("tint_requires_structure_revision")
    {
        findings.push(Finding::new(
            "ARCH-APPEARANCE-DIRTY",
            "src/scene/materials.rs set_node_tint must not unconditionally bump structure_revision",
        ));
    }
}
