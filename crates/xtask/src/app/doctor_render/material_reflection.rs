use crate::app::prelude::*;

pub(crate) fn check_material_reflection_quality_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/prepare/materials.rs",
        &[
            "fn material_requires_scene_color_transmission(material: &MaterialDesc) -> bool",
            "&& material.transmission_factor() > 0.0",
            "material_requires_scene_color_transmission(material)",
            "opaque_alpha_transmission_uses_scene_color_pass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/material_support.rs",
        &[
            "fn reject_unsupported_volume_texture_slots",
            "Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2",
            "slot.transmission.is_some() || slot.thickness.is_some()",
            "feature: \"gpu_volume_texture_slots\"",
            "transmission_texture and thickness_texture are not bound on the GPU/WebGL2 path yet",
            "gpu_rejects_volume_texture_slots_before_silent_drop",
            "gpu_accepts_scalar_transmission_without_volume_texture_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/prepare_resources.rs",
        &["reject_unsupported_volume_texture_slots(target, material_slots)?"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/prepare_resources_wasm.rs",
        &["reject_unsupported_volume_texture_slots(target, material_slots)?"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_headless_gpu_transmission_volume_ibl_capability_when_available",
            "red_volume_rgb",
            "transmission_volume_ibl_rgb",
            "SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/output.rs",
        &[
            "triangle_shader_applies_material_screen_space_reflections_in_native_and_webgl2_variants",
            "screen_space_material_reflection(",
            "floor-only post reflections do not prove chrome/mirror material reflections",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/output_shader.wgsl",
        &[
            "fn screen_space_material_reflection",
            "camera.color_management.z",
            "textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &[
            "fn screen_space_material_reflection",
            "camera.color_management.z",
            "textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv",
        ],
    );
}
