pub(super) struct DiagnosticEarlyReturn {
    pub(super) path: &'static str,
    pub(super) owner: &'static str,
    pub(super) rationale: &'static str,
}

pub(super) const DIAGNOSTIC_EARLY_RETURNS: &[DiagnosticEarlyReturn] = &[
    DiagnosticEarlyReturn {
        path: "tests/c09_gpu_resource_lifecycle.rs",
        owner: "renderer-quality/C09",
        rationale: "native adapter-sensitive accounting diagnostics paired with a required strict WebGL2 lifecycle lane",
    },
    DiagnosticEarlyReturn {
        path: "tests/dynamic_transform_parity.rs",
        owner: "renderer-quality/Q04",
        rationale: "optional adapter probe until the required CPU-WebGL2 parity lane owns it",
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
        path: "tests/pf08_texture_bake_parity.rs",
        owner: "renderer-quality/PF08",
        rationale: "behavioral CPU-headless-GPU parity proof collected on an adapter-equipped remote lane; final required hardware/browser lanes are separate fail-closed release evidence",
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
