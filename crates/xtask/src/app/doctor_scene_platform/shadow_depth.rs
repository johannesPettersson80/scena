use crate::app::prelude::*;

pub(crate) fn check_directional_shadow_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) fn collect_lighting_stats(",
            "backend: Backend",
            "Capabilities::for_backend(backend)",
            "scene.light_nodes()",
            "light.casts_shadows()",
            "PrepareError::MultipleShadowedDirectionalLights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/shadows.rs",
        &[
            "pub(super) fn collect_shadow_occluders",
            "pub(super) fn directional_shadow_factor",
            "ray_intersects_triangle(",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/diagnostics.rs",
        &["MultipleShadowedDirectionalLights"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/diagnostics/display.rs",
        &["only one shadowed directional light"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/lighting.rs",
        &[
            "casts_shadows: bool",
            "input.directional_shadow_factor",
            "primary_shadow_ray_direction",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/geometry.rs",
        &["shadow_visibility: f32"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare.rs",
        &[
            "shadow_visibility_a",
            "shadow_visibility_b",
            "shadow_visibility_c",
            "directional_shadow_factor(position_a",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/gpu/output.rs",
        &[
            // Vertex layout still carries the CPU-baked shadow_visibility
            // attribute for CPU fallback/debug visibility, but the WGSL
            // fragment now sources directional attenuation from a
            // hardware-comparison sample of the shadow map (Phase 1B step 2).
            "@location(5) shadow_visibility: f32",
            "var shadow_map: texture_depth_2d",
            "var shadow_sampler: sampler_comparison",
            "fn directional_shadow_factor",
            "textureSampleCompareLevel(shadow_map, shadow_sampler",
            "* gpu_shadow",
            "triangle_shader_samples_directional_shadow_map_in_fragment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "shadowed_directional_light_is_opt_in_and_single_owner",
            "directional_shadow_receiver_pixels_are_darkened_by_caster",
            "headless_gpu_directional_shadow_visibility_darkens_receiver_when_available",
            "with_shadows(true)",
            "MultipleShadowedDirectionalLights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &["pbr-shadow-visibility", "assertShadowVisibilityProof"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "One opt-in shadowed directional light",
            "with_shadows(true)",
        ],
    );
}

pub(crate) fn check_shadow_map_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/diagnostics.rs",
        &[
            "pub shadow_maps: u64",
            "pub directional_shadow_map_resolution: Option<u32>",
            "pub directional_shadow_pcf_kernel: Option<u8>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/diagnostics/capabilities.rs",
        &[
            "pub directional_shadows: CapabilityStatus",
            "pub point_shadows: CapabilityStatus",
            "pub spot_shadows: CapabilityStatus",
            "const fn directional_shadow_status",
            "const fn punctual_shadow_status",
            "DiagnosticCode::DirectionalShadowsDegraded",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "pub directional_shadow_map_default_size: u32",
            "pub directional_shadow_map_max_size: u32",
            "pub directional_shadow_pcf_kernel: u8",
            "pub bloom: CapabilityStatus",
            "pub screen_space_ambient_occlusion: CapabilityStatus",
            "DiagnosticCode::BloomDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
            "pub reversed_z_depth: CapabilityStatus",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/prepare/stats.rs",
        &[
            "Capabilities::for_backend(backend)",
            "capabilities.directional_shadow_map_default_size",
            "DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3",
            "pub(in crate::render) struct PreparedLightingStats",
            "shadow_maps: 1",
            "directional_shadow_pcf_kernel: Some(DIRECTIONAL_SHADOW_PCF_KERNEL)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu.rs",
        &[
            "shadow_caster: ShadowCasterResources",
            "shadow_sampler: wgpu::Sampler",
            // Phase 1C step 1 split: shadow caster + sampler now allocated
            // through `environment::build_output_resources` which bundles
            // the entire group-0 ensemble into one helper.
            "environment::build_output_resources",
            "environment::OutputResources",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/environment.rs",
        &[
            "ShadowCasterResources",
            "create_shadow_caster_resources(",
            "create_shadow_sampler(device)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/shadow.rs",
        &[
            "pub(super) fn create_shadow_texture",
            "wgpu::TextureFormat::Depth32Float",
            "wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING",
            "scena.m2.directional_shadow_map",
            // Phase 1B step 2: shadow caster pipeline + depth-only render pass.
            "pub(super) const SHADOW_CASTER_SHADER: &str",
            "camera.light_from_world * draw.world_from_model * vec4<f32>(in.position, 1.0)",
            "pub(super) fn create_shadow_caster_resources",
            "pub(super) fn encode_shadow_caster_pass",
            "wgpu::DepthBiasState",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/draw.rs",
        &["encode_shadow_caster_pass(", "&resources.shadow_caster,"],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/stats.rs",
        &[
            "shadow_maps: u64",
            "shadow_map_resolution: Option<u32>",
            "depth_prepass_passes: u64",
            "textures: 1 + material_texture_count + shadow_maps + depth_prepass_passes",
            "render_targets: 1 + shadow_maps + depth_prepass_passes",
            "estimates_single_shadow_map_resource_counters",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "single_shadow_map_records_pcf3x3_prepare_stats",
            "directional_shadow_map_default_size",
            "stats.shadow_maps",
            "directional_shadow_pcf_kernel",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "tests/m4_performance_platform.rs",
        &[
            "headless.point_shadows",
            "headless.spot_shadows",
            "headless.bloom",
            "headless.screen_space_ambient_occlusion",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "DiagnosticCode::BloomDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Single shadow map with PCF 3x3", "ARCH-SHADOW-MAP"],
    );
}

pub(crate) fn check_depth_prepass_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/diagnostics.rs",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) struct PreparedDepthStats",
            "pub(in crate::render) fn collect_depth_prepass_stats(",
            "backend: Backend",
            "DEPTH_PREPASS_MIN_PRIMITIVES: usize = 2",
            "fn depth_prepass_benefits",
            "Primitive::depth_prepass_eligible",
            "passes: 1",
            "draws: primitives.len() as u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/geometry/primitive.rs",
        &["without_depth_prepass", "depth_prepass_eligible"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/prepare/strokes.rs",
        &["without_depth_prepass"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render.rs",
        &[
            "let depth_stats = prepare::collect_depth_prepass_stats(&primitives, self.target.backend)",
            "self.stats.depth_prepass_passes = depth_stats.passes",
            "self.stats.depth_prepass_draws = depth_stats.draws",
            "backend_material_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu.rs",
        &[
            "mod depth;",
            "PreparedDepthStats",
            "depth_prepass: Option<depth::DepthPrepassResources>",
            "depth::create_depth_prepass_resources",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/draw.rs",
        &[
            "depth::encode_depth_prepass",
            "depth_view",
            "scena.headless_gpu.render_pass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/depth.rs",
        &[
            "pub(super) struct DepthPrepassResources",
            "wgpu::TextureFormat::Depth32Float",
            "scena.m2.depth_prepass",
            "clear_depth",
            "pub(super) fn encode_depth_prepass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/stats.rs",
        &[
            "depth_prepass_passes: u64",
            "depth_prepass_bytes",
            "estimates_depth_prepass_resource_counters",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "depth_prepass_is_skipped_for_trivial_single_primitive_scene",
            "depth_prepass_is_prepared_when_multiple_opaque_primitives_benefit",
            "near_far_precision_fixture_keeps_depth_order_for_small_and_large_scenes",
            "exposure_change_rerenders_on_change_and_changes_nonflat_pixels",
            "depth_prepass_passes",
            "depth_prepass_draws",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "tests/m1_geometry_materials.rs",
        &["headless_gpu_renders_technical_material_primitives_when_available"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Depth pre-pass", "ARCH-DEPTH-PREPASS"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "docs/specs/public-api.md",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
            "M2 also prepares a depth pre-pass",
        ],
    );
}
