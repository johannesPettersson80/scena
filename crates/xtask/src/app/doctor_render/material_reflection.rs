//! Anti-deletion gate for the reflection/transmission contracts.
//!
//! These are source-substring pins: they prove the code paths still exist and
//! still say what they claimed, which is what stops a silent removal. They do
//! **not** prove the renderer produces good images - the photorealism work
//! passed every rule here while its output still read as clay. The rendered
//! evidence gate lives in `photographic_output.rs`.

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
            // CPU glass: no glow without light behind it, and no veil from
            // shading the hidden side of a double-sided surface.
            "m8_glass_in_front_of_nothing_transmits_nothing_in_the_cpu_preview",
            "m8_double_sided_glass_shades_only_its_visible_side_in_the_cpu_preview",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/physical_transmission.rs",
        &[
            "KHR_materials_transmission / KHR_materials_volume",
            "pub(in crate::render) fn physical_transmission_color",
            "pub(in crate::render) fn volume_transmittance",
            "fn refract_vec3",
            "volume_transmittance_matches_beer_lambert_probe",
            "scene_color_transmission_uses_refraction_volume_and_reflection_terms",
            // The physical model: Fresnel split from the IOR, the scene read
            // at the projected exit point of the refracted ray, a blur disc
            // sized by roughness and IOR, absorption along the path.
            "schlick_fresnel(f0_from_ior(material.ior), n_dot_v)",
            "project(input.position + ray)",
            "const SCENE_BLUR_TAPS: u32 = 16;",
            "fresnel_from_the_ior_splits_reflection_and_transmission",
            "thin_walled_glass_samples_the_scene_straight_through",
            "thick_glass_reads_the_scene_at_the_projected_exit_point_of_the_refracted_ray",
            "thickness_is_in_model_units_and_follows_the_model_scale",
            "roughness_blurs_the_scene_with_a_disc_sized_by_roughness_and_ior",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/cpu_transmission.rs",
        &[
            "draw_physical_transmission_cpu",
            "sample_post_scene_color",
            "Color::from_linear_rgba",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/cpu.rs",
        &["primitive_needs_physical_transmission"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "tests/transmission_parity.rs",
        &[
            "physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep",
            "scena.physical_glass_transmission_parity_sweep.v1",
            "physical-glass-transmission-parity.json",
            "clear-thin-neutral",
            "clear-thick-blue",
            "frosted-mid-green",
            "frosted-thick-warm",
            "render_scene_cpu_gpu_pair_with_renderer",
            "GLASS_REGION",
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
    // Transmission and material reflections are one source shared by both
    // triangle shader variants, so the two cannot drift apart.
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/surface_effects.wgsl",
        &[
            "fn physical_transmission_color",
            "fresnel_schlick(n_dot_v, vec3<f32>(f0_from_ior(ior))).x",
            "refract(-view_dir, normal_dir, 1.0 / ior) * thickness * model_scale",
            "camera.clip_from_world * vec4<f32>(world_position + ray, 1.0)",
            "volume_transmittance(length(ray), material.attenuation_color.rgb, material.transmission_factors.w)",
            "textureSampleLevel(transmission_color_texture, transmission_color_sampler, uv + offset, 0.0)",
            "const SCENE_BLUR_TAPS: i32 = 16;",
            "fn screen_space_material_reflection",
            "camera.color_management.z",
            "textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv",
        ],
    );
    for source in [
        "src/render/gpu/surface_effects.wgsl",
        "src/render/physical_transmission.rs",
    ] {
        forbid_contains(
            root,
            findings,
            "ARCH-RENDER-QUALITY",
            source,
            &[
                "thickness_scale",
                "refraction_mix",
                "reflection_weight",
                "rim_fresnel",
                "tint_strength",
            ],
        );
    }
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/gpu/output.rs",
        &["include_str!(\"surface_effects.wgsl\")"],
    );
    for shader in [
        "src/render/gpu/output_shader.wgsl",
        "src/render/gpu/output_shader_texture_2d.wgsl",
    ] {
        require_contains(
            root,
            findings,
            "ARCH-RENDER-QUALITY",
            shader,
            &[
                "output_rgb = screen_space_material_reflection(",
                "let transmitted = physical_transmission_color(",
            ],
        );
        forbid_contains(
            root,
            findings,
            "ARCH-RENDER-QUALITY",
            shader,
            &[
                "fn physical_transmission_color",
                "fn screen_space_material_reflection",
            ],
        );
    }
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/prepare/lighting.rs",
        &["(normal, surface.strength() * 0.175)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/prepare/lighting/tests.rs",
        &["photographic_micro_surface_folds_unresolved_detail_into_uniform_roughness"],
    );
    for shader in [
        "src/render/gpu/output_shader.wgsl",
        "src/render/gpu/output_shader_texture_2d.wgsl",
    ] {
        require_contains(
            root,
            findings,
            "ARCH-RENDER-QUALITY",
            shader,
            &["let unresolved_micro_roughness = material.texture_strengths.z * 0.175;"],
        );
        forbid_contains(
            root,
            findings,
            "ARCH-RENDER-QUALITY",
            shader,
            &["let micro_phase =", "micro_x", "micro_y"],
        );
    }
}
