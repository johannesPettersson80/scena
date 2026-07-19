use crate::app::prelude::*;

#[test]
fn q06_doctor_requires_strict_gpu_lane_contracts() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q06-required-gpu-lanes");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in [
        ".github/workflows",
        "tests/browser",
        "tests",
        "src",
        "crates/xtask/src/app/release",
    ] {
        fs::create_dir_all(fixture_root.join(directory)).expect("Q06 fixture directory");
    }
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            "jobs:\n  linux-native-vulkan:\n    env:\n      SCENA_BROWSER_ALLOW_UNAVAILABLE: \"1\"\n  linux-browser-webgpu:\n    run: npm run browser:m6\n",
        )
        .expect("weak Q06 workflow writes");
    }
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "function isAllowedUnavailable() { return true; }\n",
    )
    .expect("weak browser runner writes");
    fs::write(
        fixture_root.join("tests/browser/required_gpu_parity.js"),
        "function evaluateRequiredGpuParity() { return { status: 'passed' }; }\n",
    )
    .expect("weak evaluator writes");
    fs::write(
        fixture_root.join("tests/browser/required_gpu_parity_test.js"),
        "// no unavailable, output, or software mutations\n",
    )
    .expect("weak evaluator test writes");
    fs::write(
        fixture_root.join("src/browser_probe.rs"),
        "gpu_device draw_calls gpu_submissions renderer_readback\n",
    )
    .expect("weak browser probe writes");
    fs::write(
        fixture_root.join("tests/m9_platform_release.rs"),
        "Renderer::headless_gpu(width, height).or_else(Renderer::headless)\n",
    )
    .expect("weak native proof writes");
    fs::write(
        fixture_root.join("crates/xtask/src/app/release/lane_artifacts.rs"),
        "fn release_lane_content_ok() -> bool { true }\n",
    )
    .expect("weak lane validator writes");
    let mut findings = Vec::new();

    check_q06_required_gpu_lane_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q06-REQUIRED-GPU-LANES"),
        "unsafe allow-unavailable, fallback, and missing required mode must fail: {findings:?}"
    );

    let hosted_conformance_workflow = "jobs:\n  linux-native-vulkan:\n    env:\n      SCENA_REQUIRE_PARITY: \"1\"\n    run: cargo test --test m9_platform_release\n  linux-browser-webgpu:\n    runs-on: ubuntu-24.04\n    env:\n      SCENA_BROWSER_BACKENDS: webgpu\n      SCENA_GPU_EVIDENCE_CLASS: \"software-conformance\"\n    run: npm run test:required-gpu-parity && npm run browser:m6\n";
    fs::write(
        fixture_root.join(".github/workflows/ci.yml"),
        hosted_conformance_workflow,
    )
    .expect("hosted Q06 conformance workflow writes");
    let hosted_release_workflow = "jobs:\n  linux-native-vulkan:\n    env:\n      SCENA_REQUIRE_PARITY: \"1\"\n    run: cargo test --test m9_platform_release\n  linux-browser-webgpu:\n    runs-on: ubuntu-24.04\n    env:\n      SCENA_BROWSER_BACKENDS: webgpu\n      SCENA_GPU_EVIDENCE_CLASS: \"software-conformance\"\n    run: npm run test:required-gpu-parity && npm run browser:m6\n";
    fs::write(
        fixture_root.join(".github/workflows/release.yml"),
        hosted_release_workflow,
    )
    .expect("hosted Q06 release workflow writes");
    fs::write(
        fixture_root.join(".github/workflows/hardware-gpu.yml"),
        "jobs:\n  native-browser-gpu:\n    runs-on: [self-hosted, linux, x64, gpu, scena-gpu]\n    env:\n      SCENA_REQUIRE_HARDWARE_GPU: \"1\"\n      SCENA_REQUIRE_PARITY: \"1\"\n      SCENA_BROWSER_BACKENDS: webgpu,webgl2\n",
    )
    .expect("hardware Q06 evidence workflow writes");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "SCENA_REQUIRE_PARITY evaluateRequiredGpuParity required_parity renderer-owned-gpu-copy\n",
    )
    .expect("strong browser runner writes");
    fs::write(
        fixture_root.join("tests/browser/required_gpu_parity.js"),
        "NO_ADAPTER ZERO_RENDERER_OUTPUT SOFTWARE_ADAPTER ADAPTER_HARDWARE_UNPROVEN\n",
    )
    .expect("strong evaluator writes");
    fs::write(
        fixture_root.join("tests/browser/required_gpu_parity_test.js"),
        "NoAdapter Google SwiftShader drawCalls = 0 required GPU parity evaluator: pass\n",
    )
    .expect("strong evaluator test writes");
    fs::write(
        fixture_root.join("src/browser_probe.rs"),
        "gpu_device draw_calls gpu_submissions renderer_readback gpu_adapter_report \"adapter\": adapter\n",
    )
    .expect("strong browser probe writes");
    fs::write(
        fixture_root.join("tests/m9_platform_release.rs"),
        "Renderer::headless_gpu SCENA_REQUIRE_PARITY required_parity host_gpu_available pbr_light_gpu_proof\n",
    )
    .expect("strong native proof writes");
    fs::write(
        fixture_root.join("crates/xtask/src/app/release/lane_artifacts.rs"),
        "linux-native-vulkan linux-webgpu-chromium RELEASE-LANE-REQUIRED-PARITY SCENA_REQUIRE_PARITY native_gpu_render_proof_passes\n",
    )
    .expect("strong lane validator writes");
    fs::write(
        fixture_root.join("crates/xtask/src/app/release/required_gpu_parity.rs"),
        "browser_probe_release_proof_passes browser_gpu_conformance_passes required_browser_gpu_parity_passes adapter_is_hardware swiftshader llvmpipe\n",
    )
    .expect("strong required parity validator writes");
    findings.clear();
    check_q06_required_gpu_lane_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    fs::write(
        fixture_root.join(".github/workflows/release.yml"),
        hosted_release_workflow.replace(
            "runs-on: ubuntu-24.04",
            "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]",
        ),
    )
    .expect("self-hosted release mutation writes");
    findings.clear();
    check_q06_required_gpu_lane_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q06-REQUIRED-GPU-LANES"
                && finding
                    .message
                    .contains("must not depend on a self-hosted runner")
        }),
        "release workflow must reject a mandatory self-hosted WebGPU runner: {findings:?}"
    );
}
