use crate::app::prelude::*;

pub(super) fn check_dispersion_material_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/material_extensions.rs",
        &["KHR_materials_dispersion", "dispersion"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/materials.rs",
        &["dispersion_extension", "with_dispersion_factor"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/material/extensions.rs",
        &["dispersion_factor", "with_dispersion_factor"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/pbr_contract/dispersion.rs",
        &[
            "dispersion_light_contribution",
            "dispersion_light_contribution_uses_factor_and_ior_spread",
            "dispersion_f0_from_ior",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/lighting.rs",
        &["material.dispersion_factor()", "LayeredMaterialLobes"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/lighting/lobes.rs",
        &["dispersion_light_contribution", "dispersion_factor"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_dispersion_material_factor_is_parsed_from_gltf",
            "m8_dispersion_factor_affects_cpu_preview_pixels",
            "render_center_rgb_for_dispersion_factor",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_uniform.rs",
        &[
            "MATERIAL_UNIFORM_BYTE_LEN: u64 = 224",
            "dispersion_factors",
            "material.dispersion_factor()",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output_shader.wgsl",
        &[
            "dispersion_factors: vec4<f32>",
            "let dispersion_factor = max(material.dispersion_factors.x, 0.0);",
            "dispersion_light_contribution",
            "dispersion_f0_from_ior",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &[
            "dispersion_factors: vec4<f32>",
            "let dispersion_factor = max(material.dispersion_factors.x, 0.0);",
            "dispersion_light_contribution",
            "dispersion_f0_from_ior",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/output.rs",
        &["triangle_shader_applies_dispersion_lobe_in_native_and_webgl2_variants"],
    );
}
