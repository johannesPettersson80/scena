use crate::app::prelude::*;

pub(crate) fn check_c20_browser_execution_ergonomics(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C20-BROWSER-EXECUTION-ERGONOMICS";

    require_contains(
        root,
        findings,
        RULE,
        "src/viewer_element/element.js",
        &[
            "this.setPointerCapture(event.pointerId)",
            "lostpointercapture",
            "disconnectedCallback()",
            "_detachControlListeners()",
            "_releasePointerState()",
            "this.releasePointerCapture(pointerId)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "pointer_capture_outside_release",
            "pointer_capture_reentry_clean",
            "SCENA_BROWSER_FORCE_REBUILD",
            "msaa-capability",
            "assertMsaaCapabilityProof",
        ],
    );

    require_contains(
        root,
        findings,
        RULE,
        "src/diagnostics/capabilities.rs",
        &[
            "pub render_sample_counts: [u32; 3]",
            "pub depth_sample_counts: [u32; 3]",
            "pub explicit_msaa: CapabilityStatus",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/diagnostics/capabilities/sample_counts.rs",
        &[
            "Backend::HeadlessGpu | Backend::NativeSurface => [1, 4, 8]",
            "Backend::WebGpu | Backend::WebGl2",
            "CapabilityStatus::ErrorIfRequired",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/build.rs",
        &[
            "resolve_automatic_anti_aliasing",
            "DiagnosticCode::MultisampleFallback",
            ".with_applied_fallback(\"anti_aliasing\")",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/prepare_resources_wasm.rs",
        &[
            "PrepareError::UnsupportedSampleCount",
            "requested: output_plan.sample_count()",
            "maximum: 1",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c20_wasm_capability_contracts.rs",
        &["browser_backends_report_the_renderer_owned_sample_count_matrix"],
    );

    forbid_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/input.rs",
        &["gpu_requested_from_env", "SCENA_USE_GPU"],
    );
    for path in [
        "src/bin/scena/args/inspection.rs",
        "src/bin/scena/recipe.rs",
        "src/bin/scena/recipe/capture_sequence.rs",
        "src/bin/scena/recipe/cad_inspection.rs",
    ] {
        require_contains(root, findings, RULE, path, &["let mut gpu = false;"]);
    }
    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/output.rs",
        &[
            "pub(crate) struct CliBackendSelectionV1",
            "source: if gpu_flag { \"cli_flag\" } else { \"default\" }",
            "json_outcome_with_backend_selection",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/help.rs",
        &[
            "\"backend_selection\"",
            "SCENA_USE_GPU is test/proof metadata and is ignored by CLI execution",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/scena_cli_recipe.rs",
        &[
            "scena_render_cli_ignores_scena_use_gpu_and_reports_default_selection",
            "scena_render_cli_gpu_flag_reports_explicit_selection_and_fallback_truth",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/fr05_capture_sequence.rs",
        &["report[\"backend_selection\"][\"source\"]", "\"default\""],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/scena_cli_help.rs",
        &["scena_help_points_to_llm_app_builder_guide"],
    );

    for (path, needles) in [
        ("README.md", &["captured pointer lifecycle"][..]),
        (
            "docs/browser.md",
            &[
                "Releasing outside the element",
                "Capabilities.render_sample_counts",
            ][..],
        ),
        (
            "docs/capabilities.md",
            &["`render_sample_counts` and `depth_sample_counts`"][..],
        ),
        (
            "docs/specs/public-api.md",
            &["Capabilities::render_sample_counts"][..],
        ),
        ("docs/api.md", &["Capabilities::explicit_msaa"][..]),
        (
            "docs/headless-rendering.md",
            &["the only execution selector. `SCENA_USE_GPU` is test/proof metadata"][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &["`SCENA_USE_GPU` never changes CLI execution"][..],
        ),
        (
            "docs/troubleshooting.md",
            &["The CLI used an unexpected backend"][..],
        ),
        (
            "docs/schema-contracts.md",
            &["envelope also includes `backend_selection` with `source`"][..],
        ),
        (
            "CLAUDE.md",
            &["The `scena` CLI deliberately ignores it"][..],
        ),
        (
            "CHANGELOG.md",
            &["Make `--gpu` the sole CLI GPU execution selector"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["custom element could retain orbit state"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
