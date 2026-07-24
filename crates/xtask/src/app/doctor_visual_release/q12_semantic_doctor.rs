use crate::app::prelude::*;

pub(crate) fn check_q12_semantic_doctor_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "Q12-SEMANTIC-DOCTOR";
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/core.rs",
        &[
            "CURRENT_RELEASE_VERSION",
            "CURRENT_RELEASE_NOTES",
            "CURRENT_REVIEW_REPORT",
            "CURRENT_REMEDIATION_CHECKLIST",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/doctor_core/runner.rs",
        &[
            "check_current_release_document_version",
            "package_version != Some(CURRENT_RELEASE_VERSION)",
            "CURRENT_RELEASE_NOTES",
            "CURRENT_REVIEW_REPORT",
            "CURRENT_REMEDIATION_CHECKLIST",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/material_uniform.rs",
        &[
            "material_uniform_min_binding_size",
            "material_uniform_layout_encode_and_bind_size_are_consistent",
            "material_uniform_contract_rejects_an_omitted_shader_lane",
            "TypeInner::Struct { members, span }",
        ],
    );
    for relative in [
        "src/render/gpu/materials.rs",
        "src/render/gpu/materials/bind_group.rs",
    ] {
        require_contains(
            root,
            findings,
            RULE,
            relative,
            &["material_uniform_min_binding_size()"],
        );
    }
    for relative in cached_rust_files_below(root, Path::new("crates/xtask/src/app")) {
        if relative
            == Path::new("crates/xtask/src/app/doctor_visual_release/q12_semantic_doctor.rs")
        {
            continue;
        }
        let Ok(source) = read_source_to_string(root, &relative) else {
            continue;
        };
        if source.contains("MATERIAL_UNIFORM_BYTE_LEN: u64 = 224") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{} pins the material uniform implementation literal instead of the semantic layout test",
                    relative.display()
                ),
            ));
        }
    }
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/shader_manifest.rs",
        &[
            "define_shader_variants!",
            "every_production_shader_variant_parses_validates_and_exports_required_entries",
            "production_manifest_inventories_feature_axes_and_rejects_an_omitted_variant",
            "offline_shader_gate_rejects_syntax_binding_location_entry_and_capability_mutations",
            "validate_manifest_coverage(&PRODUCTION_SHADER_IDS[..PRODUCTION_SHADER_IDS.len() - 1])",
        ],
    );
    forbid_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/doctor_render/render_truth/camera_shader.rs",
        &[
            "clip_from_world: mat4x4<f32>",
            "textureSampleCompareLevel(shadow_map, shadow_sampler",
            "normal_sample.x * world_tangent",
            "base.a < material.metallic_roughness_alpha.z",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/doctor_easy_scene/showcase_performance.rs",
        &[
            "SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS",
            "check_wasm_size_budget_with_policy",
            "generated_wasm_absence_is_only_blocking_in_explicit_release_mode",
            "explicit generated-artifact release mode requires",
        ],
    );
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            RULE,
            workflow,
            &[
                "SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS=1 cargo run -p xtask -- doctor --full",
                "cargo test --lib render::gpu::shader_manifest::tests",
            ],
        );
    }
    require_contains(
        root,
        findings,
        RULE,
        "docs/specs/release-gates.md",
        &[
            "SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS=1",
            "production-derived WGSL manifest",
            "Naga",
            "ordinary source doctor",
        ],
    );
}
