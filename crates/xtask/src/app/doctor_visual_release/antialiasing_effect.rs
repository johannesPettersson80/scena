use crate::app::prelude::*;

pub(crate) fn check_q07_antialiasing_effect_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "Q07-ANTIALIASING-EFFECT",
        "tests/q07_antialiasing_effect.rs",
        &[
            "high-contrast-asymmetric-diagonal-v1",
            "q07_required_native_antialiasing_modes_have_pixel_effect",
            "intermediate_luma_pixels",
            "squared_edge_energy",
            "saturating_add(baseline.hard_transition_count.saturating_mul(6))",
            "UNSUPPORTED_SAMPLE_COUNT",
            "no_op",
            "blur_everything",
        ],
    );
    require_contains(
        root,
        findings,
        "Q07-ANTIALIASING-EFFECT",
        "tests/browser/pf01_output_toggle_validation.js",
        &[
            "validateFxaaEffect",
            "normalized_squared_edge_energy",
            "FXAA spread coverage beyond the edge-local bound",
        ],
    );
    require_contains(
        root,
        findings,
        "Q07-ANTIALIASING-EFFECT",
        "scripts/build_windows_complete_hardware_bundle.sh",
        &["cp tests/q07_antialiasing_effect.rs \"$bundle_root/tests/\""],
    );
    require_contains(
        root,
        findings,
        "Q07-ANTIALIASING-EFFECT",
        "scripts/run_windows_complete_hardware_proof.ps1",
        &[
            "SCENA_REQUIRE_AA_EFFECT_PROOF",
            "native None/FXAA/MSAA pixel-effect proof",
            "scena-q07-antialiasing-effect.exe",
            "Copy-Item -Path (Join-Path $bundleRoot \"tests\\*.rs\")",
        ],
    );
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            "Q07-ANTIALIASING-EFFECT",
            workflow,
            &[
                "SCENA_REQUIRE_AA_EFFECT_PROOF=1 bash scripts/release_lane_command.sh macos-metal cargo test --test q07_antialiasing_effect q07_required_native_antialiasing_modes_have_pixel_effect -- --exact",
            ],
        );
    }
}
