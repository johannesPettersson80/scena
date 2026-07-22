#![cfg(all(unix, not(target_arch = "wasm32")))]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[test]
fn machine_mode_emits_one_stable_envelope_for_plan_and_argument_failure() {
    let help = run(&["--json", "--help"]);
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let help = parse_one_json(&help.stdout);
    assert_eq!(help["schema"], "scena.cli_help.v1");
    assert_eq!(help["command"], "scena-convert");

    let plan = run(&[
        "--json",
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--dry-run",
    ]);
    assert!(plan.status.success(), "plan failed: {plan:?}");
    assert!(plan.stderr.is_empty(), "machine stderr must stay empty");
    let plan = parse_one_json(&plan.stdout);
    assert_eq!(plan["schema"], "scena.asset_conversion.v1");
    assert_eq!(plan["ok"], true);
    assert_eq!(plan["status"], "planned");
    assert_eq!(plan["workflow"], "fbx_to_gltf");

    let invalid = run(&[
        "--json",
        "--input",
        "model.obj",
        "--output",
        "model.glb",
        "--dry-run",
    ]);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stderr.is_empty(), "machine stderr must stay empty");
    let invalid = parse_one_json(&invalid.stdout);
    assert_eq!(invalid["schema"], "scena.asset_conversion.v1");
    assert_eq!(invalid["ok"], false);
    assert_eq!(invalid["status"], "invalid_request");
    assert!(
        invalid["message"]
            .as_str()
            .is_some_and(|message| message.contains("FBX"))
    );
}

#[test]
fn machine_mode_reports_an_unavailable_tool_in_the_same_envelope() {
    let output = run(&[
        "--json",
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--tool",
        "scena-a05-definitely-missing-converter",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report = parse_one_json(&output.stdout);
    assert_eq!(report["schema"], "scena.asset_conversion.v1");
    assert_eq!(report["ok"], false);
    assert_eq!(report["status"], "tool_unavailable");
    assert!(report["message"].as_str().is_some_and(|message| {
        message.contains("install FBX2glTF") && message.contains("--tool")
    }));
}

#[test]
fn machine_mode_captures_tool_progress_warnings_and_failures_inside_the_envelope() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/a05-scena-convert");
    fs::create_dir_all(&root).expect("A05 fixture directory creates");
    let success_tool = write_tool(
        &root.join("success-tool.sh"),
        "#!/bin/sh\necho progress-one\necho warning-one >&2\nexit 0\n",
    );
    let success = run(&[
        "--json",
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--tool",
        success_tool.to_str().expect("tool path is UTF-8"),
    ]);
    assert!(success.status.success(), "conversion failed: {success:?}");
    assert!(success.stderr.is_empty(), "tool diagnostics must not leak");
    let success = parse_one_json(&success.stdout);
    assert_eq!(success["status"], "converted");
    assert!(
        success["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|item| item["stream"] == "stdout" && item["message"] == "progress-one")
                    && diagnostics
                        .iter()
                        .any(|item| item["stream"] == "stderr" && item["message"] == "warning-one")
            })
    );

    let failure_tool = write_tool(
        &root.join("failure-tool.sh"),
        "#!/bin/sh\necho conversion-broke >&2\nexit 7\n",
    );
    let failure = run(&[
        "--json",
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--tool",
        failure_tool.to_str().expect("tool path is UTF-8"),
    ]);
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stderr.is_empty(), "machine stderr must stay empty");
    let failure = parse_one_json(&failure.stdout);
    assert_eq!(failure["status"], "conversion_failed");
    assert_eq!(failure["tool_exit_code"], 7);
    assert!(
        failure["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|item| item["message"] == "conversion-broke")
            })
    );
}

#[test]
fn human_mode_is_explicit_and_never_prints_json() {
    let output = run(&[
        "--human",
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--dry-run",
    ]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("human output is UTF-8");
    assert!(stdout.contains("Planned FBX to glTF conversion"));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_scena-convert"))
        .args(args)
        .output()
        .expect("scena-convert runs")
}

fn parse_one_json(bytes: &[u8]) -> serde_json::Value {
    let text = String::from_utf8(bytes.to_vec()).expect("machine output is UTF-8");
    assert_eq!(
        text.lines().count(),
        1,
        "machine output must be one JSON line"
    );
    serde_json::from_str(&text).expect("machine output is one JSON document")
}

fn write_tool(path: &Path, source: &str) -> std::path::PathBuf {
    fs::write(path, source).expect("tool fixture writes");
    let mut permissions = fs::metadata(path)
        .expect("tool metadata reads")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("tool fixture becomes executable");
    path.to_owned()
}
