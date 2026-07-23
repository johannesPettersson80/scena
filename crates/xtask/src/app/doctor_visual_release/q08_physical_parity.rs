use crate::app::prelude::*;

const REQUIRED_TESTS: &[(&str, &str)] = &[
    (
        "tests/transmission_parity.rs",
        "physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep",
    ),
    (
        "tests/c13_depth_clipping_parity.rs",
        "close_camera_near_clip_matches_cpu_and_gpu_rendered_output",
    ),
    (
        "tests/dynamic_transform_parity.rs",
        "dynamic_transform_motion_matches_cpu_and_gpu_for_authored_animation_and_imports",
    ),
    (
        "tests/dynamic_transform_parity.rs",
        "z_up_imported_rotation_frame_matches_cpu_and_gpu_after_basis_conversion",
    ),
    (
        "tests/pbr_brdf_parity.rs",
        "core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep",
    ),
    (
        "tests/pf08_texture_bake_parity.rs",
        "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu",
    ),
];

pub(crate) fn check_q08_required_physical_parity(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "tests/support/parity.rs",
        &[
            "ParityExecutionPolicy::SkipDiagnostic",
            "ParityExecutionPolicy::RequiredPhysicalHardware",
            "SCENA_REQUIRE_GPU_PARITY",
            "record_cpu_gpu_parity_pass",
            "assertions_executed",
            "physical-hardware-required",
        ],
    );
    for (source, test_name) in REQUIRED_TESTS {
        require_contains(
            root,
            findings,
            "Q08-REQUIRED-PHYSICAL-PARITY",
            source,
            &[test_name, "record_cpu_gpu_parity_pass"],
        );
    }
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "tests/pbr_brdf_parity.rs",
        &["PathBuf::from(\"target/gate-artifacts/pbr-brdf-parity\")"],
    );
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "tests/pf08_texture_bake_parity.rs",
        &["PathBuf::from(\"target/gate-artifacts/pf08-texture-bake-parity\")"],
    );
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        for (_, test_name) in REQUIRED_TESTS {
            require_contains(
                root,
                findings,
                "Q08-REQUIRED-PHYSICAL-PARITY",
                workflow,
                &[
                    "SCENA_REQUIRE_GPU_PARITY=1 bash scripts/release_lane_command.sh macos-metal cargo test",
                    test_name,
                    "-- --exact",
                ],
            );
        }
    }
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "scripts/build_windows_complete_hardware_bundle.sh",
        &[
            "tests/transmission_parity.rs",
            "tests/c13_depth_clipping_parity.rs",
            "tests/dynamic_transform_parity.rs",
            "tests/pbr_brdf_parity.rs",
            "tests/pf08_texture_bake_parity.rs",
        ],
    );
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "scripts/run_windows_complete_hardware_proof.ps1",
        &[
            "$env:SCENA_REQUIRE_GPU_PARITY = \"1\"",
            "$q08ParityCommands",
            "q08-required-parity",
            "Copy-Item -Path (Join-Path $bundleRoot \"tests\\*.rs\")",
        ],
    );
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "crates/xtask/src/app/release/q08_parity.rs",
        &[
            "validate_q08_parity_results",
            "assertions_executed",
            "release_evidence",
            "physical-hardware-required",
            "source_checksums",
        ],
    );
    require_contains(
        root,
        findings,
        "Q08-REQUIRED-PHYSICAL-PARITY",
        "tests/release/windows_complete_hardware_proof_validation.js",
        &[
            "validateQ08Parity",
            "executed zero assertions",
            "required_physical_cpu_gpu_parity",
        ],
    );
}
