use crate::app::prelude::*;

pub(crate) fn check_q11_reference_stability(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "Q11-REFERENCE-STABILITY";
    require_contains(
        root,
        findings,
        RULE,
        "tests/q01_waterbottle_cpu_reference.rs",
        &[
            "q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison",
            "independent in-process WaterBottle renders must be byte-identical before the committed reference is consulted",
            "independent-render-before-committed-reference",
            "scena.q11.reference_stability.v1",
            "metric_distribution",
            "\"source_checksums\": [",
            "SCENA_Q11_REFERENCE_CANDIDATE_DIR",
            "scena.q11.reference_candidate.v1",
            "\"release_evidence\": false",
            "\"candidate_only\": true",
            "tolerance_change_allowed",
        ],
    );
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            RULE,
            workflow,
            &[
                "headless-cpu cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
                "macos-metal cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
                "windows-dx12 cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
            ],
        );
    }
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/release/review_artifacts.rs",
        &[
            "q11-reference-stability/linux-x86_64.json",
            "q11-reference-stability/macos-aarch64.json",
            "q11-reference-stability/windows-x86_64.json",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/release/waterbottle_results.rs",
        &[
            "validate_q11_reference_stability_result",
            "Q11 reference-stability renders are not byte-identical",
            "Q11 metric distribution exceeds the approved fixed oracle",
            "0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "scripts/stage_q01_waterbottle_reference_candidate.sh",
        &[
            "git status --porcelain",
            "SCENA_Q11_REFERENCE_CANDIDATE_DIR",
            "q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison",
            "separately authored approval JSON",
        ],
    );
    forbid_contains(
        root,
        findings,
        RULE,
        "scripts/stage_q01_waterbottle_reference_candidate.sh",
        &[
            "reference_cpu_256.png",
            "promote_q01_waterbottle_reference.cjs <",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "scripts/promote_q01_waterbottle_reference.cjs",
        &[
            "scena.q11.reference_approval.v1",
            "named human reviewer",
            "external_anchor_reviewed",
            "before_after_diff_reviewed",
            "tolerance_change_approved !== false",
            "reference promotion requires a clean checkout",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/release/windows_complete_hardware_proof_validation.js",
        &[
            "validateQ11ReferenceStability",
            "report.byte_identical === true",
            "q11-reference-stability/windows-x86_64.json",
            "Q11 metric distribution exceeds the fixed oracle",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
        &[
            "[scena_cpu_256.regeneration]",
            "candidate_only = true",
            "clean_checkout_required = true",
            "approval_schema = \"scena.q11.reference_approval.v1\"",
            "tolerance_policy = \"Chebyshev/RMSE/fraction thresholds may not be loosened",
            "q11-reference-stability/macos-aarch64.json",
        ],
    );
}
