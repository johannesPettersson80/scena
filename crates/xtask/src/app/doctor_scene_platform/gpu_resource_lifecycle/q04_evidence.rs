use crate::app::prelude::*;

pub(super) fn check_q04_required_gpu_lifecycle_evidence(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "RENDER-C09";
    let required: &[(&str, &[&str])] = &[
        (
            "tests/c09_gpu_resource_lifecycle.rs",
            &[
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE",
                "scena.q04.optional_gpu_resource_lifecycle_smoke.v1",
                "optional-developer-smoke",
                "scena.q04.required_gpu_resource_lifecycle.v1",
                "physical-hardware-required",
                "required_lifecycle_evaluator_rejects_known_leak_and_missing_adapter",
                "leaked.poll_pending_after = 1",
                "missing_adapter.adapter = None",
                "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
                "validate_required_lifecycle_evidence(&evidence)",
                "required_lifecycle_source_checksums()",
                "missing valid source checksums",
                "required-result.json",
            ],
        ),
        (
            "crates/xtask/src/app/release/review_artifacts.rs",
            &[
                "c09-gpu-resource-lifecycle/required-result.json",
                "required_gpu_resource_lifecycle_proof_passes",
                "physical-hardware-required",
                "poll_pending_after",
                "software rasterizer",
            ],
        ),
        (
            "crates/xtask/src/app/release/lane_artifacts.rs",
            &[
                "c09-gpu-resource-lifecycle/required-result.json",
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1 cargo test --test c09_gpu_resource_lifecycle",
                "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            ],
        ),
        (
            ".github/workflows/hardware-gpu.yml",
            &[
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE: \"1\"",
                "Required native GPU resource lifecycle",
                "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            ],
        ),
        (
            ".github/workflows/ci.yml",
            &[
                "Required physical GPU resource lifecycle",
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE: \"1\"",
                "bash scripts/release_lane_command.sh macos-metal",
                "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            ],
        ),
        (
            ".github/workflows/release.yml",
            &[
                "Required physical GPU resource lifecycle",
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE: \"1\"",
                "bash scripts/release_lane_command.sh macos-metal",
                "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            ],
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            &[
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE",
                "required physical GPU resource lifecycle",
                "scena-q04-gpu-resource-lifecycle.exe",
                "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            ],
        ),
        (
            "scripts/build_windows_complete_hardware_bundle.sh",
            &[
                "c09_gpu_resource_lifecycle",
                "scena-q04-gpu-resource-lifecycle.exe",
                "x86_64-pc-windows-gnu",
            ],
        ),
        (
            "docs/browser.md",
            &[
                "synthetic lifecycle evidence, not physical-GPU device-loss injection",
                "native required resource-retirement proof",
            ],
        ),
        (
            "docs/lifecycle.md",
            &[
                "GPU resource-retirement evidence",
                "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1 cargo test",
                "scena.q04.required_gpu_resource_lifecycle.v1",
                "nonzero pending count fail the required lane",
            ],
        ),
        (
            "docs/specs/release-gates.md",
            &[
                "Required GPU resource lifecycle",
                "c09-gpu-resource-lifecycle/required-result.json",
                "proof_class:\"optional-developer-smoke\"",
            ],
        ),
        (
            "README.md",
            &["Adapter-optional GPU lifecycle tests report a typed skip"],
        ),
        (
            "CHANGELOG.md",
            &["Split C09 GPU lifecycle diagnostics from release evidence"],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["published C09 GPU resource-lifecycle tests silently returned"],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c09_gpu_resource_lifecycle.rs",
        &[
            "required_lifecycle_evaluator_rejects_known_leak_and_missing_adapter",
            "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            "msaa8_is_fully_prepared_or_rejected_before_render_optional_gpu_smoke",
            "output_resource_changes_require_prepare_and_stats_are_complete_before_render_optional_gpu_smoke",
            "cpu_poll_reports_explicitly_unsupported_instead_of_success",
            "resize_and_context_recovery_rebuild_the_same_resource_shape_optional_gpu_smoke",
            "output_revision_and_native_readback_modes_are_explicit_and_render_allocates_no_gpu_resources_optional_gpu_smoke",
            "double_buffered_async_readback_batch_preserves_input_order_optional_gpu_smoke",
        ],
    );

    if fs::read_to_string(root.join("tests/c09_gpu_resource_lifecycle.rs"))
        .is_ok_and(|source| source.contains("let Ok(mut renderer) = Renderer::headless_gpu"))
    {
        findings.push(Finding::new(
            RULE,
            "C09 lifecycle tests must emit explicit optional-skip evidence or fail under the required hardware policy; a silent let-Ok early return is forbidden",
        ));
    }
}
