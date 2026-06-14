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
