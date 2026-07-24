use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_camera_shader_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    check_shader_manifest_ownership(root, findings);
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities.rs",
        &[
            "CapabilityStatus::Degraded",
            "DiagnosticCode::ForwardPbrDegraded",
            "DiagnosticCode::DirectionalShadowsDegraded",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
            "DiagnosticCode::GpuCullingDisabled",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capability_status.rs",
        &[
            "const fn forward_pbr_status",
            "const fn directional_shadow_status",
            "const fn punctual_shadow_status",
            "CapabilityStatus::Degraded",
            "const fn bloom_status",
            "const fn ambient_occlusion_status",
            "fn gpu_frustum_culling_status",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/camera.rs",
        &[
            "pub(super) struct CameraProjection",
            "view_from_world_matrix",
            "world_from_view_matrix",
            "clip_from_view_matrix",
            "view_from_clip_matrix",
            "clip_from_world_matrix",
            "world_to_view",
            "ndc_x",
            "ndc_y",
            "depth: f32",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/cpu_geometry.rs",
        &[
            "CameraProjection",
            "camera.project_clipped",
            "clip_depth_plane",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/cpu.rs",
        &[
            "depth_frame: &'frame mut [f32]",
            "mix_depth",
            "depth > cpu_frame.depth_frame[pixel_index]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "cpu_depth_buffer_keeps_nearer_triangle_visible_when_submitted_first",
            "headless_gpu_depth_buffer_keeps_nearer_triangle_visible_when_available",
        ],
    );

    // Naga validates the production-derived shader manifest. Doctor pins the
    // semantic mutation tests, not implementation substrings from the WGSL.
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &[
            "GPU_TRIANGLE_SHADER",
            "GPU_TRIANGLE_SHADER_TEXTURE_2D",
            "include_str!(\"../pbr_brdf.wgsl\")",
            "triangle_shader_uses_camera_projection_uniform",
            "triangle_shader_declares_material_texture_bindings",
            "triangle_shader_samples_all_material_texture_roles",
            "triangle_shader_discards_alpha_masked_fragments",
            "triangle_shader_consumes_gpu_punctual_light_uniforms",
            "triangle_shader_consumes_gpu_environment_light_uniforms",
            "triangle_shader_builds_tangent_space_normal_from_normal_map",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/material_uniform.rs",
        &[
            "material_uniform_layout_encode_and_bind_size_are_consistent",
            "material_uniform_contract_rejects_an_omitted_shader_lane",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/pbr_brdf.wgsl",
        &[
            "scena.pbr_brdf.wgsl",
            "KhronosGroup/glTF-Sample-Renderer",
            "fn brdf_specular_ggx",
            "fn visibility_ggx_correlated",
            "fn split_sum_brdf_approx",
        ],
    );
}

fn check_shader_manifest_ownership(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ARCH-SHADER-MANIFEST";
    const OWNER: &str = "src/render/gpu/shader_manifest.rs";
    require_contains(
        root,
        findings,
        RULE,
        OWNER,
        &[
            "define_shader_variants!",
            "production_shader_variants",
            "production_shader_modules_are_created_only_by_manifest_owner",
            "every_production_shader_variant_parses_validates_and_exports_required_entries",
            "production_manifest_inventories_feature_axes_and_rejects_an_omitted_variant",
            "offline_shader_gate_rejects_syntax_binding_location_entry_and_capability_mutations",
        ],
    );
    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/workflows/hardware-gpu.yml",
    ] {
        require_contains(
            root,
            findings,
            RULE,
            workflow,
            &["cargo test --lib render::gpu::shader_manifest::tests"],
        );
    }
    for relative in cached_rust_files_below(root, Path::new("src/render/gpu")) {
        if relative == Path::new(OWNER) {
            continue;
        }
        let Ok(source) = read_source_to_string(root, &relative) else {
            continue;
        };
        if source.contains(".create_shader_module(") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{} creates a WGSL module outside {OWNER}; add the production assembly to the generated manifest and use its typed create_shader_module owner",
                    relative.display()
                ),
            ));
        }
    }
}
