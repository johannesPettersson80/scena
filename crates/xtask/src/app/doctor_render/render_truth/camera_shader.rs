use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_camera_shader_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities.rs",
        &[
            "const fn forward_pbr_status",
            "const fn directional_shadow_status",
            "const fn punctual_shadow_status",
            "CapabilityStatus::Degraded",
            "DiagnosticCode::ForwardPbrDegraded",
            "DiagnosticCode::DirectionalShadowsDegraded",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "DiagnosticCode::BloomDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
            "DiagnosticCode::GpuCullingDisabled",
            "const fn postprocess_status",
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
        "src/render/cpu.rs",
        &[
            "CameraProjection",
            "camera.project(vertex.position)",
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
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &[
            "clip_from_world: mat4x4<f32>",
            "camera_position_exposure: vec4<f32>",
            "viewport_near_far: vec4<f32>",
            "color_management: vec4<f32>",
            "world_from_model: mat4x4<f32>",
            "normal_from_model: mat4x4<f32>",
            "view_from_world: mat4x4<f32>",
            "clip_from_view: mat4x4<f32>",
            "struct LightingUniform",
            "directional_light_direction_intensity",
            "point_light_position_intensity",
            "spot_light_direction_cones",
            "pbr_light_contribution",
            "pbr_environment_lighting",
            "fresnel_schlick",
            "distribution_ggx",
            "geometry_smith",
            "environment_diffuse_intensity",
            "environment_specular_intensity",
            "OUTPUT_UNIFORM_BYTE_LEN: u64 = 464",
            "camera.clip_from_view * camera.view_from_world * world_position",
            "draw.normal_from_model * vec4<f32>(in.normal, 0.0)",
            "draw.world_from_model * vec4<f32>(in.position, 1.0)",
            "@group(2) @binding(0)",
            "var<uniform> draw: DrawUniform",
            "light_from_world: mat4x4<f32>",
            "var shadow_map: texture_depth_2d",
            "var shadow_sampler: sampler_comparison",
            "fn directional_shadow_factor",
            "textureSampleCompareLevel(shadow_map, shadow_sampler",
            "camera.light_from_world * vec4<f32>(world_position",
            // Phase 1C step 2: GGX prefilter + BRDF LUT split-sum specular.
            "var environment_cubemap: texture_cube<f32>",
            "var environment_sampler: sampler",
            "var brdf_lut: texture_2d<f32>",
            "let prefiltered = textureSampleLevel(environment_cubemap, environment_sampler, reflection",
            "let lut_sample = textureLoad(brdf_lut",
            "f0 * lut_sample.x + vec3<f32>(lut_sample.y)",
            "@location(2) normal: vec3<f32>",
            "@location(3) tex_coord0: vec2<f32>",
            "@location(4) tangent: vec4<f32>",
            "in.tangent.w",
            "let normal_texture_sample = textureSample(normal_texture",
            "normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal",
            "var base_color_sampler: sampler",
            "var base_color_texture: texture_2d_array<f32>",
            "var<uniform> material: MaterialUniform",
            "var normal_sampler: sampler",
            "var normal_texture: texture_2d_array<f32>",
            "var metallic_roughness_sampler: sampler",
            "var metallic_roughness_texture: texture_2d_array<f32>",
            "var occlusion_sampler: sampler",
            "var occlusion_texture: texture_2d_array<f32>",
            "var emissive_sampler: sampler",
            "var emissive_texture: texture_2d_array<f32>",
            "base_color_uv_offset_scale",
            "base_color_uv_rotation",
            "base_color_factor",
            "emissive_strength",
            "metallic_roughness_alpha",
            "base.a < material.metallic_roughness_alpha.z",
            "discard;",
            "textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer)",
            "textureSample(normal_texture",
            "textureSample(metallic_roughness_texture",
            "textureSample(occlusion_texture",
            "textureSample(emissive_texture",
            "triangle_shader_uses_camera_projection_uniform",
            "triangle_shader_declares_material_texture_bindings",
            "triangle_shader_samples_all_material_texture_roles",
            "triangle_shader_discards_alpha_masked_fragments",
            "triangle_shader_consumes_gpu_punctual_light_uniforms",
            "triangle_shader_consumes_gpu_environment_light_uniforms",
            "triangle_shader_builds_tangent_space_normal_from_normal_map",
        ],
    );
    if let Ok(shader_source) = fs::read_to_string(root.join("src/render/gpu/output.rs")) {
        let shader_const = shader_source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(&shader_source);
        if shader_const.contains("let normal_sample = textureSample(normal_texture") {
            findings.push(Finding::new(
                "ARCH-RENDER-TRUTH",
                "src/render/gpu/output.rs redeclares normal_sample in WGSL; Chrome WebGPU rejects \
                 this and the browser canvas can go black while Rust-side render stats still pass",
            ));
        }
    }
}
