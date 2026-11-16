use crate::app::prelude::*;

pub(super) fn check_sheen_material_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/material_extensions.rs",
        &[
            "KHR_materials_sheen",
            "sheenColorFactor",
            "sheenRoughnessFactor",
            "sheenColorTexture",
            "sheenRoughnessTexture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/materials.rs",
        &["with_sheen_color_texture", "with_sheen_roughness_texture"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/material/extensions.rs",
        &[
            "with_sheen_color_factor",
            "with_sheen_roughness_factor",
            "with_sheen_color_texture",
            "with_sheen_roughness_texture",
            "sheen_color_factor",
            "sheen_roughness_factor",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/materials.rs",
        &[
            "sheen_color_texture_sample",
            "sheen_roughness_texture_sample",
        ],
    );
    // The CPU renderer evaluates the same sheen layer as the shader: a Charlie
    // lobe on top of a base scaled down by the energy the layer reflects.
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/sheen.rs",
        &[
            "fn distribution_charlie(",
            "fn visibility_neubelt(",
            "fn sheen_environment_brdf(",
            "sheen_energy_compensation_matches_the_shader_constants",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/surface.rs",
        &[
            "pub(in crate::render::prepare) fn with_sheen(",
            "scale_vec3(layer, s.sheen_scaling)",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/surface/tests.rs",
        &["sheen_scales_the_base_by_its_energy_compensation_and_adds_the_charlie_lobe"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/environment/surface_light.rs",
        &["sheen_environment_brdf(s.n_dot_v, s.sheen_roughness)"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_sheen_material_factors_are_parsed_from_gltf",
            "m8_sheen_texture_slots_are_parsed_from_gltf",
            "m8_sheen_png_textures_affect_cpu_preview_pixels",
            "sheen_color_texture",
            "sheen_roughness_texture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/resources.rs",
        &[
            "sheen_color",
            "sheen_roughness",
            "backend_material_slots_preserve_all_texture_roles_and_material_only_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/material_batch.rs",
        &[
            "MaterialTextureRole::SheenColor",
            "MaterialTextureRole::SheenRoughness",
            "sheen_roughness_sampler_mismatch_blocks_batching_with_diagnostic_role",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_uniform.rs",
        &[
            "material_uniform_layout_encode_and_bind_size_are_consistent",
            "sheen_factors",
            "material.sheen_color_factor()",
            "material.sheen_roughness_factor()",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_upload.rs",
        &["from_sheen_color_texture", "from_sheen_roughness_texture"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_bindings.rs",
        &[
            "SHEEN_COLOR_BINDINGS",
            "SHEEN_ROUGHNESS_BINDINGS",
            "MATERIAL_TEXTURE_BINDING_INDICES",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/materials.rs",
        &[
            "MaterialTextureUpload::from_sheen_color_texture",
            "MaterialTextureUpload::from_sheen_roughness_texture",
            "scena.material.sheen_color",
            "scena.material.sheen_roughness",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_batched.rs",
        &[
            "scena.material.batched_sheen_color",
            "scena.material.batched_sheen_roughness",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output_shader.wgsl",
        &[
            "sheen_factors: vec4<f32>",
            "var sheen_color_texture: texture_2d_array<f32>",
            "var sheen_roughness_texture: texture_2d_array<f32>",
            "material.sheen_factors.rgb * sheen_color_sample.rgb",
            "material.sheen_factors.a * sheen_roughness_sample.a",
            "surface.sheen_color = clamp(sheen_color, vec3<f32>(0.0), vec3<f32>(1.0));",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &[
            "sheen_factors: vec4<f32>",
            "var sheen_color_texture: texture_2d<f32>",
            "var sheen_roughness_texture: texture_2d<f32>",
            "material.sheen_factors.rgb * sheen_color_sample.rgb",
            "surface.sheen_color = clamp(sheen_color, vec3<f32>(0.0), vec3<f32>(1.0));",
        ],
    );
    // The lobe itself lives in the surface model shared by both variants.
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/surface_lighting.wgsl",
        &[
            "fn distribution_charlie(",
            "layer = layer * s.sheen_scaling +",
            "fn sheen_environment_brdf(",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output.rs",
        &[
            "triangle_shader_applies_sheen_lobe_in_native_and_webgl2_variants",
            "triangle_shader_samples_all_material_texture_roles",
            "triangle_shader_texture_2d_variant_declares_webgl2_material_bindings",
        ],
    );
}
