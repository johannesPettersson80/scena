#![cfg(feature = "inspection")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

const TEST_ASSET: &str = "tests/assets/gltf/mesh_material_vertex_color_scene.gltf";

#[test]
fn scena_render_cli_writes_png_descriptor_and_introspection_json() {
    let dir = artifact_dir("render");
    let png_path = dir.join("frame.png");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            TEST_ASSET,
            "--introspect",
            "--out",
            path_str(&png_path),
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena render command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "render command keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render command emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["artifacts"]["capture_png_path"], path_str(&png_path));
    let descriptor_path = dir.join("frame.capture.json");
    assert_eq!(
        report["artifacts"]["capture_descriptor_path"],
        path_str(&descriptor_path)
    );
    assert!(fs::metadata(&png_path).expect("PNG artifact exists").len() > 0);

    let descriptor: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&descriptor_path).expect("capture descriptor artifact reads"),
    )
    .expect("capture descriptor artifact is JSON");
    assert_eq!(descriptor["schema"], "scena.capture.v1");
    assert_eq!(descriptor["width"], 96);
    assert_eq!(descriptor["height"], 72);
}

#[test]
fn scena_render_cli_exits_nonzero_and_emits_json_for_empty_frame() {
    let dir = artifact_dir("render-empty");
    let asset_path = dir.join("empty-scene.gltf");
    let png_path = dir.join("empty-frame.png");
    fs::write(
        &asset_path,
        r#"{
  "asset": { "version": "2.0" },
  "extensionsUsed": ["KHR_materials_unlit"],
  "scene": 0,
  "scenes": [{ "nodes": [0] }],
  "nodes": [{ "name": "TransparentTriangle", "mesh": 0 }],
  "materials": [{
    "name": "TransparentUnlit",
    "pbrMetallicRoughness": {
      "baseColorFactor": [0.0, 0.0, 0.0, 0.0],
      "metallicFactor": 0.0,
      "roughnessFactor": 1.0
    },
    "alphaMode": "BLEND",
    "extensions": { "KHR_materials_unlit": {} }
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "material": 0
    }]
  }],
  "buffers": [{
    "byteLength": 42,
    "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA"
  }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    {
      "bufferView": 0,
      "componentType": 5126,
      "count": 3,
      "type": "VEC3",
      "min": [-0.5, -0.5, 0.0],
      "max": [0.5, 0.5, 0.0]
    },
    {
      "bufferView": 1,
      "componentType": 5123,
      "count": 3,
      "type": "SCALAR"
    }
  ]
}"#,
    )
    .expect("empty glTF fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&asset_path),
            "--introspect",
            "--out",
            path_str(&png_path),
            "--width",
            "32",
            "--height",
            "24",
        ])
        .output()
        .expect("scena render command runs");

    assert!(!output.status.success(), "empty render must fail closed");
    assert!(
        output.stderr.is_empty(),
        "render ok=false reports stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render failure still emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["reasons"]
            .as_array()
            .expect("render reasons is an array")
            .iter()
            .any(|reason| reason["code"] == "empty_frame"
                || reason["code"] == "no_visible_drawables"),
        "empty render should explain why ok=false: {report:#}"
    );
    assert!(fs::metadata(&png_path).expect("PNG artifact exists").len() > 0);
    assert!(
        fs::metadata(dir.join("empty-frame.capture.json"))
            .expect("capture descriptor artifact exists")
            .len()
            > 0
    );
}

#[test]
fn scena_diagnose_cli_emits_json_and_nonzero_for_invisible_target() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["diagnose", TEST_ASSET, "--visibility", "--handle", "999999"])
        .output()
        .expect("scena diagnose command runs");

    assert!(
        !output.status.success(),
        "diagnose should fail for stale handle"
    );
    assert!(
        output.stderr.is_empty(),
        "diagnose report failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("diagnose command emits JSON");
    assert_eq!(report["schema"], "scena.visibility_diagnosis.v1");
    assert_eq!(report["ok"], false);
    assert_eq!(report["target"]["handle"], 999999);
    assert!(
        report["reasons"]
            .as_array()
            .expect("diagnosis reasons is an array")
            .iter()
            .any(|reason| reason["code"] == "stale_handle"),
        "diagnosis should explain the stale handle: {report:#}"
    );
}

#[test]
fn scena_repair_cli_plans_visual_patch_from_diagnosis_json() {
    let dir = artifact_dir("repair");
    let diagnosis_path = dir.join("diagnosis.json");
    fs::write(
        &diagnosis_path,
        serde_json::to_string_pretty(&hidden_node_diagnosis_json())
            .expect("diagnosis fixture serializes"),
    )
    .expect("diagnosis fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["repair", TEST_ASSET, "--from", path_str(&diagnosis_path)])
        .output()
        .expect("scena repair command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "repair plan stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("repair command emits JSON");
    assert_eq!(report["schema"], "scena.visual_repair_plan.v1");
    assert_eq!(report["status"], "repairable");
    assert_eq!(report["auto_fixable"], true);
    assert_eq!(report["visual_patch"]["schema"], "scena.visual_patch.v1");
    assert_eq!(report["applied_actions"][0]["action"], "set_visible");
}

#[test]
fn scena_repair_cli_exits_nonzero_for_irreducible_diagnosis() {
    let dir = artifact_dir("repair-irreducible");
    let diagnosis_path = dir.join("diagnosis.json");
    let mut diagnosis = hidden_node_diagnosis_json();
    diagnosis["target"]["handle"] = json!(999);
    diagnosis["reasons"][0]["code"] = json!("stale_handle");
    diagnosis["reasons"][0]["auto_fixable"] = json!(false);
    diagnosis["reasons"][0]["affected_handles"] = json!([999]);
    diagnosis["reasons"][0]["message"] =
        json!("target handle is not present in the inspection report");
    diagnosis["fixes"] = json!([]);
    fs::write(
        &diagnosis_path,
        serde_json::to_string_pretty(&diagnosis).expect("diagnosis fixture serializes"),
    )
    .expect("diagnosis fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "repair",
            TEST_ASSET,
            "--from",
            path_str(&diagnosis_path),
            "--iteration-budget",
            "3",
        ])
        .output()
        .expect("scena repair command runs");

    assert!(
        !output.status.success(),
        "irreducible repair should fail closed"
    );
    assert!(
        output.stderr.is_empty(),
        "irreducible report stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("repair command emits JSON");
    assert_eq!(report["schema"], "scena.agent_loop_result.v1");
    assert_eq!(report["status"], "irreducible");
    assert_eq!(report["ok"], false);
    assert_eq!(report["remaining_reasons"][0]["code"], "stale_handle");
}

#[test]
fn scena_inspect_cli_emits_scene_inspection_json_for_asset() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["inspect", TEST_ASSET, "--width", "96", "--height", "72"])
        .output()
        .expect("scena inspect command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "inspect command keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("inspect command emits JSON");
    assert_eq!(report["schema"], "scena.scene_inspection.v1");
    assert!(
        report["nodes"]
            .as_array()
            .expect("inspection nodes is an array")
            .iter()
            .any(|node| node["kind"] == "Mesh"),
        "inspection should include imported mesh nodes: {report:#}"
    );
    assert!(
        report["counts"]["visible_drawable"]
            .as_u64()
            .expect("visible_drawable count is numeric")
            > 0,
        "inspection should report visible drawable content: {report:#}"
    );
}

fn hidden_node_diagnosis_json() -> serde_json::Value {
    json!({
        "schema": "scena.visibility_diagnosis.v1",
        "ok": false,
        "target": {"kind": "node", "handle": 42},
        "reasons": [{
            "code": "node_hidden",
            "severity": "error",
            "confidence": "high",
            "auto_fixable": true,
            "affected_handles": [42],
            "message": "target node is hidden"
        }],
        "fixes": [{
            "action": "set_visible",
            "target_handle": 42,
            "patch": {"visibility": [{"node": 42, "visible": true}]},
            "risk": "content",
            "help": "set the target node visible, then render and diagnose again"
        }],
        "summary": {
            "visible_nodes": 0,
            "hidden_nodes": 1,
            "visible_drawables": 0,
            "culled_objects": 0,
            "not_prepared": false
        },
        "evidence": []
    })
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target")
        .join("gate-artifacts")
        .join(format!("scena-cli-agent-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("artifact directory creates");
    dir
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is valid UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
