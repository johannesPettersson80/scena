use crate::app::prelude::*;

const RULE: &str = "Q01-REQUIRED-WEBGPU-PIXEL-PARITY";

pub(crate) fn check_q01_required_webgpu_pixel_parity(root: &Path, findings: &mut Vec<Finding>) {
    require_tokens(
        root,
        findings,
        ".github/workflows/hardware-gpu.yml",
        &[
            "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]",
            "SCENA_REQUIRE_PARITY: \"1\"",
            "SCENA_BROWSER_BACKENDS: webgpu,webgl2",
            "Required WebGPU pixel parity",
            "SCENA_BROWSER_BACKENDS: webgpu",
            "run: npm run browser:q01-parity",
        ],
    );
    require_tokens(
        root,
        findings,
        "src/browser_probe.rs",
        &[
            "Backend::WebGpu => workflow == \"triangle\"",
            "cpu_browser_gpu_report",
            "renderer_readback",
        ],
    );
    require_tokens(
        root,
        findings,
        "src/browser_probe/parity.rs",
        &[
            "scena.m6.cpu_webgpu_parity.v1",
            "renderer-owned-cpu-frame",
            "renderer-owned-gpu-copy",
            "format!(\"{backend:?}\")",
        ],
    );
    require_tokens(
        root,
        findings,
        "tests/browser/required_gpu_parity.js",
        &[
            "rgb_chebyshev_tolerance: 4",
            "within_tolerance_fraction_min: 0.995",
            "p99_5_channel_delta_max: 4",
            "foreground_iou_min: 0.995",
            "two-pixel-gradient-edge-exclusion",
            "source: \"cpu-reference-gradient\"",
            "foreground_domain: \"edge-excluded\"",
            "wrong-colors",
            "geometry-shift",
            "missing-object",
            "vertical-flip",
            "linear-as-srgb",
            "stale-reference",
            "PIXEL_PARITY_MISSING",
            "PIXEL_PARITY_MISMATCH",
            "WebGPU parity candidate is not the renderer headline readback",
        ],
    );
    require_tokens(
        root,
        findings,
        "tests/browser/required_gpu_parity_test.js",
        &[
            "parityFixture",
            "PIXEL_PARITY_MISSING",
            "PIXEL_PARITY_MISMATCH",
            "mutations.every",
            "required GPU parity evaluator: pass",
        ],
    );
    require_tokens(
        root,
        findings,
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "writeRequiredWebgpuParityArtifact",
            "--q01-parity-only",
            "m6-required-webgpu-pixel-parity",
            "cpu-reference.png",
            "gpu-live.png",
            "diff-heatmap.png",
            "scena.q01.required_webgpu_pixel_parity.v1",
            "worst_region",
            "collectBrowserGpuEvidence",
            "browser_gpu: browserGpu",
            "sourcePaths",
        ],
    );
    require_tokens(
        root,
        findings,
        "tests/m6_browser_webgpu_readback.rs",
        &[
            "m6_cpu_webgpu_parity_uses_the_headline_renderer_readback",
            "scena.m6.cpu_webgpu_parity.v1",
            "required parity must evaluate the renderer headline readback",
        ],
    );
    require_tokens(
        root,
        findings,
        "crates/xtask/src/app/release/required_gpu_parity.rs",
        &[
            "required_pixel_parity_evaluation_passes",
            "renderer_parity_source_matches",
            "scena.m6.cpu_webgpu_parity.v1",
            "vertical-flip",
            "diff_heatmap_rgba8_base64",
        ],
    );
    require_tokens(
        root,
        findings,
        "docs/browser.md",
        &[
            "Required WebGPU pixel parity",
            "SCENA_REQUIRE_PARITY=1 SCENA_BROWSER_BACKENDS=webgpu npm run browser:q01-parity",
            "two-pixel gradient-edge mask",
            "m6-required-webgpu-pixel-parity",
        ],
    );
    require_tokens(
        root,
        findings,
        "package.json",
        &["browser:q01-parity", "--q01-parity-only"],
    );
    require_tokens(
        root,
        findings,
        "scripts/run_windows_complete_hardware_proof.ps1",
        &[
            "required live WebGPU pixel parity",
            "npm.cmd run browser:q01-parity",
            "$env:SCENA_BROWSER_BACKENDS = \"webgpu\"",
            "$env:SCENA_RELEASE_COMMIT = $SourceCommit.ToLowerInvariant()",
        ],
    );
    require_tokens(
        root,
        findings,
        "scripts/build_windows_complete_hardware_bundle.sh",
        &[
            "target/m6-browser-pkg",
            "--features browser-probe",
            "SCENA_RELEASE_COMMIT",
            "bundle-files.sha256",
        ],
    );
    require_tokens(
        root,
        findings,
        "docs/specs/release-gates.md",
        &[
            "Required WebGPU hardware parity",
            "scena.q01.required_webgpu_pixel_parity.v1",
            "smoke-only artifact",
            "software-conformance",
        ],
    );
    require_tokens(
        root,
        findings,
        "src/schema_catalog.rs",
        &[
            "scena.q01.required_webgpu_pixel_parity.v1",
            "required_webgpu_pixel_parity.v1.json",
        ],
    );
    require_tokens(
        root,
        findings,
        "src/schema_catalog/fixtures.rs",
        &[
            "scena.q01.required_webgpu_pixel_parity.v1",
            "required_webgpu_pixel_parity.v1.json",
        ],
    );
    require_tokens(
        root,
        findings,
        "tests/assets/stable-contracts/required_webgpu_pixel_parity.v1.json",
        &[
            "scena.q01.required_webgpu_pixel_parity.v1",
            "required-live-webgpu-pixel-parity",
            "vertical-flip",
            "source_checksums",
        ],
    );
    require_tokens(
        root,
        findings,
        "crates/xtask/src/app/doctor_docs/stable_fixtures.rs",
        &[
            "scena.q01.required_webgpu_pixel_parity.v1",
            "required_webgpu_pixel_parity.v1.json",
        ],
    );
    require_tokens(
        root,
        findings,
        "docs/schema-contracts.md",
        &[
            "scena.q01.required_webgpu_pixel_parity.v1",
            "six known-bad mutation outcomes",
        ],
    );
    require_tokens(
        root,
        findings,
        "README.md",
        &[
            "required WebGPU hardware parity",
            "six known-bad image mutations",
        ],
    );
    require_tokens(
        root,
        findings,
        "CHANGELOG.md",
        &["Replace the required WebGPU nonblack smoke check"],
    );
    require_tokens(
        root,
        findings,
        "docs/release-notes/v1.8.0.md",
        &[
            "required browser WebGPU lane",
            "materially wrong",
            "image could pass",
        ],
    );
}

pub(crate) fn check_q04_browser_evidence_classification(root: &Path, findings: &mut Vec<Finding>) {
    const Q04_RULE: &str = "Q04-BROWSER-EVIDENCE-CLASSIFICATION";
    let mut require = |relative: &str, required: &[&str]| {
        let text = match fs::read_to_string(root.join(relative)) {
            Ok(text) => text,
            Err(error) => {
                findings.push(Finding::new(
                    Q04_RULE,
                    format!("could not read {relative}: {error}"),
                ));
                return;
            }
        };
        for token in required {
            if !text.contains(token) {
                findings.push(Finding::new(
                    Q04_RULE,
                    format!("{relative} is missing browser evidence classification {token}"),
                ));
            }
        }
    };
    require(
        "tests/browser/required_gpu_parity.js",
        &[
            "classifyBrowserEvidence",
            "renderer-smoke",
            "renderer-conformance-with-diagnostic-webgpu-pixel-diff",
            "renderer-smoke-with-required-webgpu-full-frame-parity",
            "release_evidence: false",
            "release_evidence: true",
            "webgpu:m6-identical-unlit-triangle-v1",
        ],
    );
    require(
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "classifyBrowserEvidence",
            "evidenceClassification",
            "...evidenceClassification",
        ],
    );
    require(
        "crates/xtask/src/app/release/required_gpu_parity.rs",
        &[
            "browser_evidence_classification_matches",
            "renderer-smoke-with-required-webgpu-full-frame-parity",
            "renderer-conformance-with-diagnostic-webgpu-pixel-diff",
            "full-frame-reference-diff",
        ],
    );
    require(
        "crates/xtask/src/app/release/stage_artifacts.rs",
        &[
            "renderer-conformance-aggregate",
            "\"release_evidence\": false",
            "backend-scoped-diagnostics-only",
        ],
    );
    require(
        "tests/browser/browser_evidence_classification_test.js",
        &[
            "renderer-smoke",
            "release_evidence, false",
            "required-webgpu-parity-failed",
            "browser evidence classification: pass",
        ],
    );
    require(
        "docs/schema-contracts.md",
        &[
            "renderer-smoke-with-required-webgpu-full-frame-parity",
            "renderer-conformance-with-diagnostic-webgpu-pixel-diff",
            "Smoke-only aggregates",
            "`release_evidence: false`",
        ],
    );
}

fn require_tokens(root: &Path, findings: &mut Vec<Finding>, relative: &str, required: &[&str]) {
    let text = match fs::read_to_string(root.join(relative)) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read {relative}: {error}"),
            ));
            return;
        }
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(RULE, format!("{relative} is missing {token}")));
        }
    }
}
