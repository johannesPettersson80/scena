use crate::app::prelude::*;

#[test]
fn q04_doctor_fails_closed_on_missing_browser_parity_contracts() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q04-cpu-webgl2-parity");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in [
        ".github/workflows",
        "tests/browser",
        "tests",
        "src/browser_probe",
        "crates/xtask/src/app/release",
        "docs/checklists",
    ] {
        fs::create_dir_all(fixture_root.join(directory)).expect("Q04 fixture directory");
    }
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            "wasm-pack test --headless --chrome --test m6_browser_renderer_parity\n",
        )
        .expect("weak Q04 workflow writes");
    }
    fs::write(
        fixture_root.join("tests/m6_browser_renderer_parity.rs"),
        "fn browser_parity_test() { let cpu_frame = renderer_owned_cpu_frame(); }\n",
    )
    .expect("weak parity test writes");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"),
        "const useRendererReadback = backend === \"webgpu\" && rendererReadback; result.status = result.draw_calls > 0 ? 'passed' : 'failed';\n",
    )
    .expect("weak browser page writes");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "function accept(result) { return result.status === 'passed'; }\n",
    )
    .expect("weak browser runner writes");
    fs::write(
        fixture_root.join("src/browser_probe/parity.rs"),
        "const SCHEMA: &str = \"scena.m6.cpu_webgl2_parity.v1\";\n",
    )
    .expect("weak parity evaluator writes");
    fs::write(
        fixture_root.join("crates/xtask/src/app/release/stage_browser_parity.rs"),
        "fn validate() -> bool { true }\n",
    )
    .expect("weak stage validator writes");
    let mut findings = Vec::new();

    check_q04_cpu_webgl2_parity_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q04-CPU-WEBGL2-PARITY"),
        "missing workflow, dual-frame, mutation, and staging contracts must fail closed: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q04-CPU-WEBGL2-PARITY"
                && finding
                    .message
                    .contains("backend === \"webgpu\" && rendererReadback")
        }),
        "WebGL2 canvas fallback branch must be mechanically rejected: {findings:?}",
    );

    let workflow_source = [
        "wasm-pack test --headless --chrome --test m1_browser_rendered_output",
        "wasm-pack test --headless --chrome --test m3a_browser_rendered_output",
        "wasm-pack test --headless --chrome --test m3b_browser_rendered_output",
        "wasm-pack test --headless --chrome --test m6_browser_renderer_parity --features browser-probe",
    ]
    .join("\n");
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            &workflow_source,
        )
        .expect("strong Q04 workflow writes");
    }
    fs::write(
        fixture_root.join("tests/m6_browser_renderer_parity.rs"),
        "async fn browser_parity_test() { assert_eq!(cpu.source, \"renderer-owned-cpu-frame\"); assert_eq!(gpu.source, \"renderer-owned-gpu-copy\"); }\n",
    )
    .expect("strong parity test writes");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"),
        "const parityOk = result.parity.schema === 'scena.m6.cpu_webgl2_parity.v1' && result.parity.cpu_frame.source === 'renderer-owned-cpu-frame' && result.parity.gpu_frame.source === 'renderer-owned-gpu-copy' && result.parity.known_bad_mutation.rejected === true; result.status = parityOk && draw ? 'passed' : 'failed';\n",
    )
    .expect("strong browser page writes");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "function assertCpuWebGl2Parity(result) { const {cpu, gpu, metrics, mutation} = result; cpu.rgba8_base64; gpu.rgba8_base64; metrics.rmse <= 0.08; metrics.ssim >= 0.93; metrics.p95_channel_delta <= 24; mutation.rejected !== true; } assertCpuWebGl2Parity(result);\n",
    )
    .expect("strong browser runner writes");
    fs::write(
        fixture_root.join("src/browser_probe/parity.rs"),
        "scena.m6.cpu_webgl2_parity.v1 renderer-owned-cpu-frame renderer-owned-gpu-copy p95_channel_delta foreground_region_rmse known_bad_mutation gpu-center-channel-perturbation\n",
    )
    .expect("strong parity evaluator writes");
    fs::write(
        fixture_root.join("crates/xtask/src/app/release/stage_browser_parity.rs"),
        "fn validate_cpu_webgl2_parity() { renderer-owned-cpu-frame renderer-owned-gpu-copy rgba8_base64 validate_metrics validate_known_bad_mutation }\n",
    )
    .expect("strong stage validator writes");
    fs::write(
        fixture_root.join("docs/checklists/m6-browser-renderer-parity.md"),
        "scena.m6.cpu_webgl2_parity.v1 m1_browser_rendered_output m3a_browser_rendered_output m3b_browser_rendered_output m6_browser_renderer_parity --features browser-probe renderer-owned-cpu-frame renderer-owned-gpu-copy foreground-region RMSE\n",
    )
    .expect("strong parity acceptance document writes");
    findings.clear();
    check_q04_cpu_webgl2_parity_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}
