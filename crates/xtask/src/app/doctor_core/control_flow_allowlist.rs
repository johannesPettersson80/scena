pub(super) struct DiagnosticEarlyReturn {
    pub(super) path: &'static str,
    pub(super) owner: &'static str,
    pub(super) rationale: &'static str,
}

pub(super) const DIAGNOSTIC_EARLY_RETURNS: &[DiagnosticEarlyReturn] = &[
    DiagnosticEarlyReturn {
        path: "tests/c09_gpu_resource_lifecycle.rs",
        owner: "renderer-quality/Q04",
        rationale: "clearly named optional GPU smoke tests write typed skip artifacts; the separate SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE hardware test and release consumer fail closed",
    },
    DiagnosticEarlyReturn {
        path: "tests/q07_antialiasing_effect.rs",
        owner: "renderer-quality/Q07",
        rationale: "the always-on deterministic effect oracle rejects no-op and whole-frame blur mutations; SCENA_REQUIRE_AA_EFFECT_PROOF turns the separate physical native mode into a fail-closed release lane",
    },
    DiagnosticEarlyReturn {
        path: "tests/c13_depth_clipping_parity.rs",
        owner: "renderer-quality/C13",
        rationale: "the default lane writes an explicit non-release parity artifact; the SCENA_REQUIRE_GPU_PARITY lane fails closed and executes the CPU-GPU proof",
    },
    DiagnosticEarlyReturn {
        path: "tests/c14_gltf_semantic_parity.rs",
        owner: "renderer-quality/C14",
        rationale: "the default lane writes an explicit non-release parity artifact; the SCENA_REQUIRE_GPU_PARITY lane fails closed and executes the glTF semantic proof",
    },
    DiagnosticEarlyReturn {
        path: "tests/dynamic_transform_parity.rs",
        owner: "renderer-quality/Q04",
        rationale: "optional adapter probe until the required CPU-WebGL2 parity lane owns it",
    },
    DiagnosticEarlyReturn {
        path: "tests/m1_geometry_materials.rs",
        owner: "renderer-quality/M1",
        rationale: "legacy optional adapter smoke tests and explicit gpu-release-gap artifacts; required native/browser evidence is owned by strict release lanes",
    },
    DiagnosticEarlyReturn {
        path: "tests/m2_lighting_depth_clipping.rs",
        owner: "renderer-quality/Q06",
        rationale: "adapter-sensitive diagnostic tests; required GPU evidence is a separate strict lane",
    },
    DiagnosticEarlyReturn {
        path: "tests/m4_performance_platform.rs",
        owner: "renderer-quality/Q06",
        rationale: "context-recovery diagnostic until required GPU construction is routed through strict mode",
    },
    DiagnosticEarlyReturn {
        path: "tests/m8_assets_materials_ecosystem.rs",
        owner: "renderer-quality/M8",
        rationale: "optional headless GPU material probes record typed gpu-release-gap artifacts before returning; required material proof is consumed separately",
    },
    DiagnosticEarlyReturn {
        path: "tests/m8_compressed_asset_release_proof.rs",
        owner: "renderer-quality/M8",
        rationale: "the non-approved host path writes fail-closed native and browser placeholder artifacts; strict compressed-asset release validation rejects them",
    },
    DiagnosticEarlyReturn {
        path: "tests/m8_real_asset_proof.rs",
        owner: "renderer-quality/M8",
        rationale: "expensive CPU and optional GPU paths write explicit non-release artifacts; release consumers require approved evidence",
    },
    DiagnosticEarlyReturn {
        path: "tests/m9_platform_release.rs",
        owner: "renderer-quality/M9-PF00-PF03",
        rationale: "benchmark collection and reaggregation are explicit opt-in tools; required artifact consumers and dedicated lanes own release acceptance",
    },
    DiagnosticEarlyReturn {
        path: "tests/m7_threejs_ergonomics.rs",
        owner: "renderer-quality/Q06",
        rationale: "optional GPU comparison paired with an unconditional CPU assertion",
    },
    DiagnosticEarlyReturn {
        path: "tests/pbr_brdf_parity.rs",
        owner: "renderer-quality/Q04",
        rationale: "optional adapter probe until the required CPU-WebGL2 parity lane owns it",
    },
    DiagnosticEarlyReturn {
        path: "tests/pf01_output_toggle.rs",
        owner: "renderer-quality/PF01",
        rationale: "optional native adapter smoke returns only outside SCENA_REQUIRE_HARDWARE_GPU; strict mode panics on a missing adapter and browser evidence is separate",
    },
    DiagnosticEarlyReturn {
        path: "tests/pf08_texture_bake_parity.rs",
        owner: "renderer-quality/PF08",
        rationale: "behavioral CPU-headless-GPU parity proof collected on an adapter-equipped remote lane; final required hardware/browser lanes are separate fail-closed release evidence",
    },
    DiagnosticEarlyReturn {
        path: "tests/pf10_cpu_occlusion.rs",
        owner: "renderer-quality/PF10",
        rationale: "the non-benchmark lane writes an explicit required-artifact placeholder; release evidence requires the dedicated benchmark invocation",
    },
    DiagnosticEarlyReturn {
        path: "tests/round_e_material_showcase.rs",
        owner: "doctor",
        rationale: "contains a mutation-test string literal for return detection, not an executable early return",
    },
    DiagnosticEarlyReturn {
        path: "tests/transmission_parity.rs",
        owner: "renderer-quality/Q04",
        rationale: "optional adapter probe until the required CPU-WebGL2 parity lane owns it",
    },
    DiagnosticEarlyReturn {
        path: "tests/trust_platform_repro.rs",
        owner: "external-fixture/Q07",
        rationale: "manual out-of-repo reproduction requiring a separately installed asset",
    },
];
