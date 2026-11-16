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
            "select_release_commit",
            "std::env::var(\"GITHUB_SHA\").ok()",
            // The edge-search proof: FXAA must straighten a shallow edge,
            // leave flat regions alone and keep a one-pixel line, and the
            // oracle must reject the cross filter FXAA replaced.
            "q07_gpu_fxaa_follows_edges_and_keeps_flat_regions_and_thin_lines",
            "q07_fxaa_quality_oracle_rejects_noop_blur_and_cross_filter_mutations",
            "fn legacy_cross_filter",
            "shallow_edge_residual_px",
            "flat_region_max_delta",
        ],
    );
    // GPU FXAA is an edge search, shared by the FXAA pass and the combined
    // bloom + FXAA surface pass so the two cannot drift apart.
    require_contains(
        root,
        findings,
        "Q07-ANTIALIASING-EFFECT",
        "src/render/gpu/post/fxaa_resolve.wgsl",
        &[
            "fn fxaa_resolve",
            "FXAA_SEARCH_STEPS",
            "run_turns_away",
            "fn fxaa_display_weight",
        ],
    );
    for owner in [
        "src/render/gpu/post/fxaa.rs",
        "src/render/gpu/post/bloom_fxaa.rs",
    ] {
        require_contains(
            root,
            findings,
            "Q07-ANTIALIASING-EFFECT",
            owner,
            &["include_str!(\"fxaa_resolve.wgsl\")"],
        );
    }
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
