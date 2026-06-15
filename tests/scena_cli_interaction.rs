#![cfg(feature = "scene-host")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[test]
fn scena_verify_interaction_cli_runs_synthetic_select_and_fails_wrong_handle() {
    let dir = artifact_dir("verify-interaction");
    let expectation_path = dir.join("interaction-expectation.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [
                {
                    "action": "hover",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expected_events": ["hover"]
                },
                {
                    "action": "select",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expect_selection": true,
                    "expected_events": ["selection_changed"]
                }
            ]
        }))
        .expect("interaction expectation serializes"),
    )
    .expect("interaction expectation writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            path_str(&expectation_path),
        ])
        .output()
        .expect("scena verify interaction command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "interaction report stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("interaction command emits JSON");
    assert_eq!(report["schema"], "scena.interaction_verification.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["summary"]["step_count"], 2);
    assert_eq!(report["steps"][0]["observed"]["hit"], true);
    assert!(report["steps"][0]["observed"]["hover_handle"].is_number());
    assert_eq!(report["steps"][0]["observed"]["events"][0], "hover");
    assert_eq!(report["steps"][1]["observed"]["hit"], true);
    assert!(report["steps"][1]["observed"]["hover_handle"].is_number());
    assert!(report["steps"][1]["observed"]["selection_handle"].is_number());
    assert_eq!(
        report["steps"][1]["observed"]["events"][0],
        "selection_changed"
    );

    let wrong_path = dir.join("interaction-wrong-handle.json");
    fs::write(
        &wrong_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [{
                "action": "pick",
                "x_css_px": 64.0,
                "y_css_px": 64.0,
                "expect_hit": true,
                "expected_handle": 999999,
                "expected_events": ["pick"]
            }]
        }))
        .expect("wrong interaction expectation serializes"),
    )
    .expect("wrong interaction expectation writes");

    let wrong = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            path_str(&wrong_path),
        ])
        .output()
        .expect("scena verify interaction wrong-handle command runs");

    assert!(!wrong.status.success(), "wrong handle must fail closed");
    assert!(
        wrong.stderr.is_empty(),
        "interaction failures stay machine-readable on stdout, stderr={}",
        stderr(&wrong)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&wrong.stdout).expect("interaction failure emits JSON");
    assert_eq!(report["schema"], "scena.interaction_verification.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["reasons"]
            .as_array()
            .expect("interaction reasons")
            .iter()
            .any(|reason| reason["code"] == "handle_mismatch"),
        "wrong handle failure should be machine-readable: {report:#}"
    );
}

#[test]
fn scena_verify_interaction_cli_stdout_matches_golden_fixture() {
    let dir = artifact_dir("verify-interaction-golden");
    let expectation_path = dir.join("interaction-expectation.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [
                {
                    "action": "hover",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expected_events": ["hover"]
                },
                {
                    "action": "select",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expect_selection": true,
                    "expected_events": ["selection_changed"]
                }
            ]
        }))
        .expect("interaction expectation serializes"),
    )
    .expect("interaction expectation writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            path_str(&expectation_path),
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena verify interaction golden command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "interaction golden command keeps stderr empty, stderr={}",
        stderr(&output)
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("interaction emits JSON");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/cli-golden/verify_interaction_stdout.json"
    ))
    .expect("golden interaction fixture parses");
    assert_eq!(actual, expected);
}

#[test]
fn scena_verify_interaction_failing_fixture_emits_expected_reason() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            "tests/assets/agent-failure-fixtures/interaction_wrong_handle.expectation.json",
        ])
        .output()
        .expect("scena verify interaction failure fixture command runs");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "interaction failure fixture must keep stderr empty, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("interaction failure emits JSON");
    assert_eq!(report["schema"], "scena.interaction_verification.v1");
    assert_eq!(report["ok"], false);
    assert_reason(&report, "handle_mismatch");
}

#[test]
fn scena_verify_interaction_cli_fails_unexpected_hover_and_selection_state() {
    let dir = artifact_dir("verify-interaction-negative-state");
    let expectation_path = dir.join("interaction-negative-state.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [
                {
                    "action": "hover",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": false,
                    "expected_events": ["hover"]
                },
                {
                    "action": "select",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_selection": false,
                    "expected_events": ["selection_changed"]
                }
            ]
        }))
        .expect("negative-state interaction expectation serializes"),
    )
    .expect("negative-state interaction expectation writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            path_str(&expectation_path),
        ])
        .output()
        .expect("scena verify interaction command runs");

    assert!(
        !output.status.success(),
        "unexpected hover/selection state must fail closed"
    );
    assert!(
        output.stderr.is_empty(),
        "interaction failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("interaction failure emits JSON");
    assert_eq!(report["schema"], "scena.interaction_verification.v1");
    assert_eq!(report["ok"], false);
    assert_reason(&report, "hover_unexpected");
    assert_reason(&report, "selection_unexpected");
}

#[test]
fn scena_verify_interaction_cli_covers_miss_leave_and_css_physical_mismatch() {
    let dir = artifact_dir("verify-interaction-fixtures");
    let expectation_path = dir.join("interaction-state-cycle.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [
                {
                    "action": "pick",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expected_events": ["pick"]
                },
                {
                    "action": "pick",
                    "x_css_px": 1.0,
                    "y_css_px": 1.0,
                    "expect_hit": false,
                    "expected_events": ["pick"]
                },
                {
                    "action": "hover",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expected_events": ["hover"]
                },
                {
                    "action": "hover",
                    "x_css_px": 1.0,
                    "y_css_px": 1.0,
                    "expect_hit": false,
                    "expect_hover": false,
                    "expected_events": ["hover"]
                },
                {
                    "action": "select",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expect_selection": true,
                    "expected_events": ["selection_changed"]
                },
                {
                    "action": "select",
                    "x_css_px": 1.0,
                    "y_css_px": 1.0,
                    "expect_hit": false,
                    "expect_hover": false,
                    "expect_selection": false,
                    "expected_events": ["selection_changed"]
                }
            ]
        }))
        .expect("interaction state-cycle expectation serializes"),
    )
    .expect("interaction state-cycle expectation writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            path_str(&expectation_path),
        ])
        .output()
        .expect("scena verify interaction state-cycle command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "interaction report stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("interaction command emits JSON");
    assert_eq!(report["ok"], true);
    assert_eq!(report["summary"]["step_count"], 6);
    assert_eq!(report["summary"]["hit_count"], 3);
    assert_eq!(report["summary"]["miss_count"], 3);
    assert_eq!(
        report["steps"][3]["observed"]["hover_handle"],
        serde_json::Value::Null
    );
    assert_eq!(
        report["steps"][5]["observed"]["selection_handle"],
        serde_json::Value::Null
    );

    let mismatch_path = dir.join("interaction-physical-mismatch.json");
    fs::write(
        &mismatch_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 2.0
            },
            "steps": [{
                "action": "pick",
                "x_css_px": 16.0,
                "y_css_px": 16.0,
                "coordinate_space": "physical",
                "expect_hit": true,
                "expected_events": ["pick"]
            }]
        }))
        .expect("interaction physical-mismatch expectation serializes"),
    )
    .expect("interaction physical-mismatch expectation writes");

    let mismatch = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "interaction",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "--expect",
            path_str(&mismatch_path),
        ])
        .output()
        .expect("scena verify interaction physical-mismatch command runs");

    assert!(
        !mismatch.status.success(),
        "CSS-vs-physical coordinate mistakes must fail closed"
    );
    assert!(
        mismatch.stderr.is_empty(),
        "interaction failures stay machine-readable on stdout, stderr={}",
        stderr(&mismatch)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&mismatch.stdout).expect("interaction failure emits JSON");
    assert_eq!(report["ok"], false);
    assert_eq!(
        report["steps"][0]["coordinates"]["coordinate_space"],
        "physical"
    );
    assert_eq!(report["steps"][0]["coordinates"]["x_css_px"], 8.0);
    assert_eq!(report["steps"][0]["coordinates"]["x_physical_px"], 16.0);
    assert_reason(&report, "hit_mismatch");
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    fs::create_dir_all(&dir).expect("artifact dir exists");
    dir
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_reason(report: &serde_json::Value, code: &str) {
    assert!(
        report["reasons"]
            .as_array()
            .expect("interaction reasons")
            .iter()
            .any(|reason| reason["code"] == code),
        "expected interaction reason '{code}' in report: {report:#}"
    );
}
