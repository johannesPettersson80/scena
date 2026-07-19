#![cfg(all(unix, not(target_arch = "wasm32")))]

use std::fs;
use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};

#[test]
fn both_binaries_treat_closed_stdout_as_quiet_success() {
    assert_closed_stdout_success(Command::new(env!("CARGO_BIN_EXE_scena")).args(["version"]));
    assert_closed_stdout_success(Command::new(env!("CARGO_BIN_EXE_scena-convert")).args([
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--dry-run",
    ]));
}

#[test]
fn both_binaries_report_non_broken_stdout_errors_as_structured_failures() {
    assert_non_broken_stdout_failure(Command::new(env!("CARGO_BIN_EXE_scena")).args(["version"]));
    assert_non_broken_stdout_failure(Command::new(env!("CARGO_BIN_EXE_scena-convert")).args([
        "--input",
        "model.fbx",
        "--output",
        "model.glb",
        "--dry-run",
    ]));
}

#[test]
fn validate_recipe_rejects_non_ascii_hex_as_json_without_panic_text() {
    let recipe = unicode_color_recipe();
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "validate-recipe",
            recipe.to_str().expect("recipe path is UTF-8"),
        ])
        .output()
        .expect("validate-recipe runs");
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked at"));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("validation stdout is JSON");
    assert_eq!(report["ok"], false);
    assert!(report["diagnostics"].as_array().is_some_and(|diagnostics| {
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "invalid_color" && diagnostic["path"] == "$.colors.bad"
        })
    }));
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_render_rejects_non_ascii_hex_as_json_without_panic_text() {
    let recipe = unicode_color_recipe();
    let png = recipe.with_extension("png");
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            recipe.to_str().expect("recipe path is UTF-8"),
            "--introspect",
            "--out",
            png.to_str().expect("PNG path is UTF-8"),
        ])
        .output()
        .expect("recipe render runs");
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked at"));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render failure stdout is JSON");
    assert_eq!(report["ok"], false);
}

fn unicode_color_recipe() -> std::path::PathBuf {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/c01-cli-input-contracts");
    fs::create_dir_all(&dir).expect("Unicode recipe fixture directory creates");
    let path = dir.join("unicode-color.recipe.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {"bad": "€abc"}
        }))
        .expect("Unicode recipe serializes"),
    )
    .expect("Unicode recipe fixture writes");
    path
}

fn assert_closed_stdout_success(command: &mut Command) {
    let (stdout, peer) = UnixStream::pair().expect("stdout socket pair creates");
    drop(peer);
    let stdout: OwnedFd = stdout.into();
    let output = command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::piped())
        .output()
        .expect("binary runs with closed stdout peer");
    assert!(
        output.status.success(),
        "BrokenPipe must be quiet success, status={}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "BrokenPipe must be quiet");
}

fn assert_non_broken_stdout_failure(command: &mut Command) {
    let full = File::options()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full opens");
    let output = command
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .expect("binary runs with full stdout device");
    assert_eq!(output.status.code(), Some(74));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("stderr is structured JSON");
    assert_eq!(report["schema"], "scena.cli_io_error.v1");
    assert_eq!(report["ok"], false);
    assert_eq!(report["stream"], "stdout");
    assert_ne!(report["error_kind"], "BrokenPipe");
}
