use crate::app::prelude::*;

pub(crate) fn check_q04_cpu_webgl2_parity_contracts(root: &Path, findings: &mut Vec<Finding>) {
    let workflow_commands = [
        "wasm-pack test --headless --chrome --test m1_browser_rendered_output",
        "wasm-pack test --headless --chrome --test m3a_browser_rendered_output",
        "wasm-pack test --headless --chrome --test m3b_browser_rendered_output",
        "wasm-pack test --headless --chrome --test m6_browser_renderer_parity --features browser-probe",
    ];
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            "Q04-CPU-WEBGL2-PARITY",
            workflow,
            &workflow_commands,
        );
    }
    require_contains(
        root,
        findings,
        "Q04-CPU-WEBGL2-PARITY",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &[
            "const parityOk",
            "scena.m6.cpu_webgl2_parity.v1",
            "renderer-owned-cpu-frame",
            "renderer-owned-gpu-copy",
            "known_bad_mutation.rejected === true",
            "parityOk &&",
        ],
    );
    require_contains(
        root,
        findings,
        "Q04-CPU-WEBGL2-PARITY",
        "docs/checklists/m6-browser-renderer-parity.md",
        &[
            "scena.m6.cpu_webgl2_parity.v1",
            "m1_browser_rendered_output",
            "m3a_browser_rendered_output",
            "m3b_browser_rendered_output",
            "m6_browser_renderer_parity --features browser-probe",
            "renderer-owned-cpu-frame",
            "renderer-owned-gpu-copy",
            "foreground-region RMSE",
        ],
    );
    forbid_contains(
        root,
        findings,
        "Q04-CPU-WEBGL2-PARITY",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &["backend === \"webgpu\" && rendererReadback"],
    );
    require_contains(
        root,
        findings,
        "Q04-CPU-WEBGL2-PARITY",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "function assertCpuWebGl2Parity(result)",
            "postProcessing === \"off\" && backend.toLowerCase() === \"webgl2\"",
            "cpu.rgba8_base64",
            "gpu.rgba8_base64",
            "metrics.rmse <= 0.08",
            "metrics.ssim >= 0.93",
            "metrics.p95_channel_delta <= 24",
            "mutation.rejected !== true",
            "assertCpuWebGl2Parity(result)",
        ],
    );
    require_contains(
        root,
        findings,
        "Q04-CPU-WEBGL2-PARITY",
        "src/browser_probe/parity.rs",
        &[
            "scena.m6.cpu_webgl2_parity.v1",
            "renderer-owned-cpu-frame",
            "renderer-owned-gpu-copy",
            "p95_channel_delta",
            "foreground_region_rmse",
            "known_bad_mutation",
            "gpu-center-channel-perturbation",
        ],
    );
    require_contains(
        root,
        findings,
        "Q04-CPU-WEBGL2-PARITY",
        "crates/xtask/src/app/release/stage_browser_parity.rs",
        &[
            "validate_cpu_webgl2_parity",
            "renderer-owned-cpu-frame",
            "renderer-owned-gpu-copy",
            "rgba8_base64",
            "validate_metrics",
            "validate_known_bad_mutation",
        ],
    );
    check_parity_named_test_frame_inputs(root, findings);
}

fn check_parity_named_test_frame_inputs(root: &Path, findings: &mut Vec<Finding>) {
    let tests = root.join("tests");
    let Ok(entries) = fs::read_dir(&tests) else {
        findings.push(Finding::new(
            "Q04-CPU-WEBGL2-PARITY",
            "tests directory is missing while checking parity-named frame inputs",
        ));
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) != Some("rs")
            || !path
                .file_stem()
                .and_then(OsStr::to_str)
                .is_some_and(|stem| stem.contains("parity"))
        {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "Q04-CPU-WEBGL2-PARITY",
                format!("failed to read parity test {}", path.display()),
            ));
            continue;
        };
        let has_parity_test = source.lines().any(|line| {
            let line = line.trim_start();
            (line.starts_with("fn ") || line.starts_with("async fn ")) && line.contains("parity")
        });
        if has_parity_test
            && !(source.contains("renderer-owned-cpu-frame")
                && source.contains("renderer-owned-gpu-copy"))
        {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            findings.push(Finding::new(
                "Q04-CPU-WEBGL2-PARITY",
                format!(
                    "parity-named test {} must retain both renderer-owned CPU and GPU frame inputs",
                    relative.display()
                ),
            ));
        }
    }
}
