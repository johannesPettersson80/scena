use std::process::Command;

#[test]
fn scena_browser_proof_dry_run_reports_scene_host_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["browser-proof", "scene-host", "--dry-run"])
        .output()
        .expect("browser-proof dry-run command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "browser-proof dry-run keeps stdout machine-readable, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("dry-run emits JSON");
    assert_eq!(report["schema"], "scena.browser_proof_run.v1");
    assert_eq!(report["lane"], "scene-host");
    assert_eq!(report["script"], "browser:scene-host-proof");
    assert_eq!(
        report["command"],
        serde_json::json!(["npm", "run", "browser:scene-host-proof"])
    );
    assert_eq!(report["env"]["SCENA_BROWSER_BACKENDS"], "webgl2");
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["status"], "ready");
    assert_eq!(report["exit_code"], 0);
    assert_eq!(
        report["artifact_json_path"],
        "target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json"
    );
}

#[test]
fn scena_browser_proof_dry_run_reports_m6_command_and_custom_backend() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["browser-proof", "m6", "--backend", "webgpu", "--dry-run"])
        .output()
        .expect("browser-proof m6 dry-run command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("dry-run emits JSON");
    assert_eq!(report["schema"], "scena.browser_proof_run.v1");
    assert_eq!(report["lane"], "m6");
    assert_eq!(report["script"], "browser:m6");
    assert_eq!(report["env"]["SCENA_BROWSER_BACKENDS"], "webgpu");
    assert_eq!(
        report["command"],
        serde_json::json!([
            "bash",
            "-lc",
            "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe && npm run browser:m6"
        ])
    );
    assert_eq!(
        report["artifact_json_path"],
        "target/gate-artifacts/m6-rust-wasm-renderer-probe.json"
    );
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
