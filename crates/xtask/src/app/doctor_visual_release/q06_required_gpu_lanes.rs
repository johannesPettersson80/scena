use crate::app::prelude::*;

const RULE: &str = "Q06-REQUIRED-GPU-LANES";

pub(crate) fn check_q06_required_gpu_lane_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let Some(text) = read_required(root, findings, workflow) else {
            continue;
        };
        if text.contains("SCENA_BROWSER_ALLOW_UNAVAILABLE") {
            findings.push(Finding::new(
                RULE,
                format!("{workflow} must not enable diagnostic allow-unavailable behavior"),
            ));
        }
        let Some(native) = workflow_job_block(&text, "linux-native-vulkan") else {
            findings.push(Finding::new(
                RULE,
                format!("{workflow} is missing required job linux-native-vulkan"),
            ));
            continue;
        };
        if !native.contains("SCENA_REQUIRE_PARITY: \"1\"") {
            findings.push(Finding::new(
                RULE,
                format!("{workflow} job linux-native-vulkan must set SCENA_REQUIRE_PARITY: \"1\""),
            ));
        }
        let Some(webgpu) = workflow_job_block(&text, "linux-browser-webgpu") else {
            findings.push(Finding::new(
                RULE,
                format!("{workflow} is missing required job linux-browser-webgpu"),
            ));
            continue;
        };
        if workflow == ".github/workflows/ci.yml" {
            if webgpu.contains("SCENA_REQUIRE_PARITY: \"1\"") {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{workflow} hosted linux-browser-webgpu must not claim hardware parity"
                    ),
                ));
            }
            for required in [
                "runs-on: ubuntu-24.04",
                "SCENA_GPU_EVIDENCE_CLASS: \"software-conformance\"",
            ] {
                if !webgpu.contains(required) {
                    findings.push(Finding::new(
                        RULE,
                        format!("{workflow} hosted WebGPU conformance job is missing {required}"),
                    ));
                }
            }
        } else {
            for required in [
                "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]",
                "SCENA_REQUIRE_PARITY: \"1\"",
                "SCENA_GPU_EVIDENCE_CLASS: \"hardware-release\"",
            ] {
                if !webgpu.contains(required) {
                    findings.push(Finding::new(
                        RULE,
                        format!("{workflow} hardware WebGPU release job is missing {required}"),
                    ));
                }
            }
        }
        for required in [
            "SCENA_BROWSER_BACKENDS: webgpu",
            "npm run test:required-gpu-parity",
            "npm run browser:m6",
        ] {
            if !webgpu.contains(required) {
                findings.push(Finding::new(
                    RULE,
                    format!("{workflow} WebGPU job is missing {required}"),
                ));
            }
        }
    }

    require_source_tokens(
        root,
        findings,
        ".github/workflows/hardware-gpu.yml",
        &[
            "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]",
            "SCENA_REQUIRE_HARDWARE_GPU: \"1\"",
            "SCENA_REQUIRE_PARITY: \"1\"",
            "SCENA_BROWSER_BACKENDS: webgpu,webgl2",
        ],
    );

    require_source_tokens(
        root,
        findings,
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "SCENA_REQUIRE_PARITY",
            "evaluateRequiredGpuParity",
            "required_parity",
            "renderer-owned-gpu-copy",
        ],
    );
    require_source_tokens(
        root,
        findings,
        "tests/browser/required_gpu_parity.js",
        &[
            "NO_ADAPTER",
            "ZERO_RENDERER_OUTPUT",
            "SOFTWARE_ADAPTER",
            "ADAPTER_HARDWARE_UNPROVEN",
        ],
    );
    require_source_tokens(
        root,
        findings,
        "tests/browser/required_gpu_parity_test.js",
        &[
            "NoAdapter",
            "Google SwiftShader",
            "drawCalls",
            "required GPU parity evaluator: pass",
        ],
    );
    require_source_tokens(
        root,
        findings,
        "src/browser_probe.rs",
        &[
            "gpu_adapter_report",
            "\"adapter\": adapter",
            "renderer_readback",
            "gpu_submissions",
        ],
    );
    require_source_tokens(
        root,
        findings,
        "tests/m9_platform_release.rs",
        &[
            "Renderer::headless_gpu",
            "SCENA_REQUIRE_PARITY",
            "required_parity",
            "host_gpu_available",
            "pbr_light_gpu_proof",
        ],
    );
    require_source_tokens(
        root,
        findings,
        "crates/xtask/src/app/release/lane_artifacts.rs",
        &[
            "linux-native-vulkan",
            "linux-webgpu-chromium",
            "RELEASE-LANE-REQUIRED-PARITY",
            "SCENA_REQUIRE_PARITY",
            "native_gpu_render_proof_passes",
        ],
    );
    require_source_tokens(
        root,
        findings,
        "crates/xtask/src/app/release/required_gpu_parity.rs",
        &[
            "browser_probe_release_proof_passes",
            "required_browser_gpu_parity_passes",
            "adapter_is_hardware",
            "swiftshader",
            "llvmpipe",
        ],
    );
}

fn read_required(root: &Path, findings: &mut Vec<Finding>, relative: &str) -> Option<String> {
    match fs::read_to_string(root.join(relative)) {
        Ok(text) => Some(text),
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read {relative}: {error}"),
            ));
            None
        }
    }
}

fn require_source_tokens(
    root: &Path,
    findings: &mut Vec<Finding>,
    relative: &str,
    required: &[&str],
) {
    let Some(text) = read_required(root, findings, relative) else {
        return;
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(RULE, format!("{relative} is missing {token}")));
        }
    }
}

fn workflow_job_block<'a>(text: &'a str, job: &str) -> Option<&'a str> {
    let needle = format!("  {job}:\n");
    let start = text.find(&needle)?;
    let rest = &text[start..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if offset > 0
            && line.starts_with("  ")
            && !line.starts_with("    ")
            && line.trim_end().ends_with(':')
        {
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }
    Some(rest)
}
