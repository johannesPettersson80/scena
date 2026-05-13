use crate::app::prelude::*;
pub(crate) fn check_m4_platform_contracts(root: &Path, findings: &mut Vec<Finding>) {
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
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum HardwareTier",
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
            "uniform_buffer_max_bytes",
            "HardwareTier::Medium",
            "Backend::WebGl2 => 128",
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
            "culling::cull_cpu_frustum",
            "gpu_culling_dispatches",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/build.rs",
        &[
            "headless_with_options",
            "from_surface_with_options",
            "RenderMode::OnChange",
            "resolve_quality",
            "resolve_render_mode",
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
            "BloomDisabled",
            "AmbientOcclusionDisabled",
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
            "loss",
        ],
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
