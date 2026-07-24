use crate::app::prelude::*;

const EXACT_Q01_COMMAND: &str = "bash scripts/release_lane_command.sh headless-cpu cargo test \
    --test q01_waterbottle_cpu_reference \
    q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact";

pub(crate) fn check_q01_waterbottle_cpu_proof(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "tests/q01_waterbottle_cpu_reference.rs",
        &[
            "const SIZE: u32 = 256;",
            "reference_cpu_256.png",
            "q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders",
            "const RGB_CHEBYSHEV_TOLERANCE: u8 = 4;",
            "const MIN_WITHIN_TOLERANCE_FRACTION: f64 = 0.995;",
            "const MAX_RGB_RMSE: f64 = 2.0;",
            "flattened_chrome_mutation",
            "render_wrong_material_scene",
            "render_wrong_camera_scene",
            "\"mutation_kind\": mutation_kind",
            "scene-mesh-material-before-prepare",
            "active-camera-transform-before-prepare",
            "\"color_space\": \"srgb-output\"",
            "\"row_orientation\": \"top-to-bottom\"",
            "\"alpha_contract\": \"opaque\"",
            "\"rust_test_output_observed\": false",
        ],
    );
    forbid_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "tests/q01_waterbottle_cpu_reference.rs",
        &["SCENA_RUN_", "SCENA_REFERENCE_DIFF"],
    );
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
        &[
            "[scena_cpu_256]",
            "reference_cpu_256.png",
            "RGB Chebyshev <=4 for at least 99.5% of pixels",
            "color_space = \"sRGB output\"",
            "row_orientation = \"top-to-bottom; PNG row zero is the rendered top row\"",
            "alpha_contract = \"opaque RGBA8\"",
            "flattened_chrome",
            "wrong_material",
            "wrong_camera",
        ],
    );
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            "Q01-WATERBOTTLE",
            workflow,
            &[EXACT_Q01_COMMAND],
        );
    }
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "crates/xtask/src/app/release/waterbottle_results.rs",
        &[
            "finalize_waterbottle_cpu_result",
            "q01-waterbottle-cpu/result.json",
            "headless-cpu.commands.jsonl",
            "rust_test_output_observed",
            "validate_waterbottle_cpu_result",
            "validate_waterbottle_mutation_provenance",
        ],
    );
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "crates/xtask/src/app/release/stage_visual_proofs.rs",
        &[
            "write_waterbottle_cpu_visual_proof",
            "cpu-waterbottle-reference",
            "visual-proof/waterbottle-cpu.json",
            "command_record_artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "crates/xtask/src/app/visual_artifacts/typed_visual_proof.rs",
        &[
            "require_waterbottle_cpu_visual_proof",
            "CPU mutation oracle",
            "CPU command-record hash",
            "CPU result artifact hash",
        ],
    );
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "crates/xtask/src/app/release/review_artifacts.rs",
        &[
            "q01-waterbottle-cpu/live.png",
            "q01-waterbottle-cpu/known_bad_flattened_chrome.png",
            "q01-waterbottle-cpu/known_bad_wrong_material.png",
            "q01-waterbottle-cpu/known_bad_wrong_camera.png",
            "q01-waterbottle-cpu/result.json",
            "release-lanes/headless-cpu.commands.jsonl",
            "visual-proof/waterbottle-cpu.json",
        ],
    );
    require_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "CLAUDE.md",
        &[
            "RGB Chebyshev distance 16",
            "horizontal-mirror mutation must fail",
            "release_evidence:false",
        ],
    );
    forbid_contains(
        root,
        findings,
        "Q01-WATERBOTTLE",
        "CLAUDE.md",
        &["DeltaE", "ΔE"],
    );
}
