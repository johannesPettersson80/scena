use crate::app::prelude::*;

pub(super) fn check_anisotropy_material_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/material_extensions.rs",
        &[
            "KHR_materials_anisotropy",
            "anisotropyStrength",
            "anisotropyRotation",
            "anisotropyTexture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/materials.rs",
        &["with_anisotropy_strength_factor", "with_anisotropy_texture"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/material/extensions.rs",
        &[
            "with_anisotropy_strength_factor",
            "with_anisotropy_rotation_radians",
            "with_anisotropy_texture",
            "anisotropy_strength_factor",
            "anisotropy_rotation_radians",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/materials.rs",
        &["anisotropy_texture_sample"],
    );
    // The CPU renderer reshapes the base specular lobe the way the shader
    // does, instead of adding an anisotropic lobe on top of the isotropic one.
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/anisotropy.rs",
        &[
            "fn surface_anisotropy_tangent(",
            "fn distribution_ggx_anisotropic(",
            "fn visibility_ggx_anisotropic(",
            "fn anisotropic_reflection_normal(",
            "equal_alphas_reduce_to_the_isotropic_ggx_lobe",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/surface.rs",
        &[
            "pub(in crate::render::prepare) fn with_anisotropy(",
            "surface_anisotropic_specular(s, incoming, half_vector, n_dot_l, n_dot_h)",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/surface/tests.rs",
        &["anisotropy_reshapes_the_base_lobe_instead_of_adding_one"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_anisotropy_material_factors_are_parsed_from_gltf",
            "m8_anisotropy_texture_slot_is_parsed_from_gltf",
            "m8_anisotropy_png_texture_affects_cpu_preview_pixels",
            "anisotropy_texture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/resources.rs",
        &[
            "anisotropy",
            "backend_material_slots_preserve_all_texture_roles_and_material_only_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/material_batch.rs",
        &[
            "MaterialTextureRole::Anisotropy",
            "anisotropy_sampler_mismatch_blocks_batching_with_diagnostic_role",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_uniform.rs",
        &[
            "material_uniform_layout_encode_and_bind_size_are_consistent",
            "anisotropy_factors",
            "material.anisotropy_strength_factor()",
            "material.anisotropy_rotation_radians()",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_upload.rs",
        &["from_anisotropy_texture", "FALLBACK_ANISOTROPY_RGBA8"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_bindings.rs",
        &["ANISOTROPY_BINDINGS", "MATERIAL_TEXTURE_BINDING_INDICES"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/materials.rs",
        &[
            "MaterialTextureUpload::from_anisotropy_texture",
            "scena.material.anisotropy",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_batched.rs",
        &["scena.material.batched_anisotropy"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output_shader.wgsl",
        &[
            "anisotropy_factors: vec4<f32>",
            "var anisotropy_texture: texture_2d_array<f32>",
            "anisotropy_sample.rg * 2.0",
            "material.anisotropy_factors.x * anisotropy_sample.b",
            "surface.anisotropy_t = surface_anisotropy_tangent(",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &[
            "anisotropy_factors: vec4<f32>",
            "var anisotropy_texture: texture_2d<f32>",
            "surface.anisotropy_t = surface_anisotropy_tangent(",
        ],
    );
    // The lobe itself lives in the surface model shared by both variants.
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/surface_lighting.wgsl",
        &[
            "fn surface_anisotropic_specular(",
            "specular_brdf = surface_anisotropic_specular(s, incoming, half_vector, n_dot_l, n_dot_h);",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output.rs",
        &["triangle_shader_applies_anisotropy_lobe_in_native_and_webgl2_variants"],
    );
}
