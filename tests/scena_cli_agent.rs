#![cfg(feature = "inspection")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

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
fn scena_render_cli_accepts_round_floats_for_stable_json() {
    let dir = artifact_dir("render-round-floats");
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
            "--round-floats",
            "2",
        ])
        .output()
        .expect("scena render command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render command emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true);
    assert_json_numbers_rounded(&report, 2);
}

#[test]
fn scena_cli_missing_assets_emit_json_not_command_errors() {
    let dir = artifact_dir("missing-assets");
    let missing = dir.join("does-not-exist.gltf");
    let render_png = dir.join("missing.png");
    let commands = [
        vec![
            "render".to_owned(),
            path_str(&missing).to_owned(),
            "--introspect".to_owned(),
            "--out".to_owned(),
            path_str(&render_png).to_owned(),
        ],
        vec!["inspect".to_owned(), path_str(&missing).to_owned()],
        vec![
            "diagnose".to_owned(),
            path_str(&missing).to_owned(),
            "--visibility".to_owned(),
        ],
        vec!["doctor".to_owned(), path_str(&missing).to_owned()],
    ];

    for args in commands {
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(args.iter().map(String::as_str))
            .output()
            .expect("scena command runs");
        assert!(
            !output.status.success(),
            "missing asset command should fail closed"
        );
        assert!(
            output.stderr.is_empty(),
            "missing asset diagnostics must stay on stdout JSON, stderr={}",
            stderr(&output)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("missing asset emits JSON");
        assert_eq!(report["schema"], "scena.asset_doctor.v1");
        assert_eq!(report["ok"], false);
        assert_eq!(report["asset"], path_str(&missing));
        assert!(
            report["findings"]
                .as_array()
                .expect("asset doctor findings is an array")
                .iter()
                .any(
                    |finding| finding["code"] == "asset_io" || finding["code"] == "asset_not_found"
                ),
            "missing asset should have a machine-readable finding: {report:#}"
        );
    }
}

#[test]
fn scena_validate_recipe_stdout_matches_golden_fixture() {
    let dir = artifact_dir("validate-golden");
    let recipe_path = dir.join("invalid.recipe.json");
    fs::write(
        &recipe_path,
        r#"{
  "schema": "scena.scene_recipe.v1",
  "importe": [{
    "id": "part",
    "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
  }]
}"#,
    )
    .expect("invalid recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&recipe_path)])
        .output()
        .expect("scena validate-recipe command runs");

    assert!(!output.status.success(), "invalid recipe must fail closed");
    assert!(
        output.stderr.is_empty(),
        "validation diagnostics stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("validate-recipe emits JSON");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/cli-golden/validate_recipe_invalid_stdout.json"
    ))
    .expect("golden validate-recipe fixture parses");
    assert_eq!(actual, expected);
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
fn scena_doctor_cli_emits_json_and_nonzero_for_broken_asset() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "doctor",
            "tests/assets/gltf/unsupported_required_extension.gltf",
        ])
        .output()
        .expect("scena doctor command runs");

    assert!(!output.status.success(), "broken asset must fail closed");
    assert!(
        output.stderr.is_empty(),
        "doctor report failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor command emits JSON");
    assert_eq!(report["schema"], "scena.asset_doctor.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["findings"]
            .as_array()
            .expect("doctor findings is an array")
            .iter()
            .any(
                |finding| finding["code"] == "unsupported_required_extension"
                    && finding["suggested_fix"]
                        .as_str()
                        .is_some_and(|fix| fix.contains("fallback"))
            ),
        "doctor should explain the unsupported extension with a fix: {report:#}"
    );
}

#[test]
fn scena_doctor_cli_stdout_matches_golden_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "doctor",
            "tests/assets/gltf/unsupported_required_extension.gltf",
        ])
        .output()
        .expect("scena doctor command runs");

    assert!(!output.status.success(), "broken asset must fail closed");
    assert!(
        output.stderr.is_empty(),
        "doctor report failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor command emits JSON");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/cli-golden/doctor_broken_asset_stdout.json"
    ))
    .expect("golden doctor fixture parses");
    assert_eq!(actual, expected);
}

#[test]
fn scena_agent_cli_stdout_matches_golden_fixtures() {
    let dir = fixed_artifact_dir("scena-cli-agent-golden");

    let render_png = dir.join("render-frame.png");
    let render = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            TEST_ASSET,
            "--introspect",
            "--out",
            path_str(&render_png),
            "--width",
            "64",
            "--height",
            "48",
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena render golden command runs");
    assert_success_stdout_matches(&render, "render_introspection_stdout.json");

    let inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "inspect",
            TEST_ASSET,
            "--width",
            "64",
            "--height",
            "48",
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena inspect golden command runs");
    assert_success_stdout_matches(&inspect, "inspect_asset_stdout.json");

    let diagnose = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "diagnose",
            TEST_ASSET,
            "--visibility",
            "--handle",
            "999999",
            "--width",
            "64",
            "--height",
            "48",
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena diagnose golden command runs");
    assert_failure_stdout_matches(&diagnose, "diagnose_stale_handle_stdout.json");

    let diagnosis_path = dir.join("hidden-node-diagnosis.json");
    fs::write(
        &diagnosis_path,
        serde_json::to_string_pretty(&hidden_node_diagnosis_json())
            .expect("diagnosis fixture serializes"),
    )
    .expect("diagnosis fixture writes");
    let repair = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "repair",
            TEST_ASSET,
            "--from",
            path_str(&diagnosis_path),
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena repair golden command runs");
    assert_success_stdout_matches(&repair, "repair_plan_stdout.json");

    let expectation_path = dir.join("appearance-expectation.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.appearance_expectation.v1",
            "targets": [{
                "id": "expected-noon",
                "variant": "noon",
                "color_family": "green",
                "swatch_srgb8": [0, 255, 0],
                "require_source_material": true,
                "alpha_mode": "opaque"
            }]
        }))
        .expect("appearance expectation serializes"),
    )
    .expect("appearance expectation writes");
    let appearance = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "appearance",
            "tests/assets/gltf/material_variants_scene.gltf",
            "--expect",
            path_str(&expectation_path),
            "--width",
            "64",
            "--height",
            "48",
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena verify appearance golden command runs");
    assert_success_stdout_matches(&appearance, "verify_appearance_stdout.json");

    let animation = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            "tests/assets/gltf/animated_triangle_scene.glb",
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0.5,1.0",
            "--expect-change",
            "--width",
            "64",
            "--height",
            "48",
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena verify animation golden command runs");
    assert_success_stdout_matches(&animation, "verify_animation_stdout.json");
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
fn scena_verify_appearance_cli_checks_variant_color_and_fails_closed() {
    let dir = artifact_dir("verify-appearance");
    let expectation_path = dir.join("appearance-expectation.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.appearance_expectation.v1",
            "targets": [{
                "id": "expected-noon",
                "variant": "noon",
                "color_family": "green",
                "swatch_srgb8": [0, 255, 0],
                "require_source_material": true,
                "alpha_mode": "opaque"
            }]
        }))
        .expect("appearance expectation serializes"),
    )
    .expect("appearance expectation writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "appearance",
            "tests/assets/gltf/material_variants_scene.gltf",
            "--expect",
            path_str(&expectation_path),
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify appearance command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "appearance report stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("appearance command emits JSON");
    assert_eq!(report["schema"], "scena.appearance_introspection.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["active_variant"], "noon");
    assert_eq!(report["targets"][0]["sampled_color_family"], "green");

    let wrong_path = dir.join("wrong-appearance-expectation.json");
    fs::write(
        &wrong_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.appearance_expectation.v1",
            "targets": [{
                "id": "wrong-noon",
                "variant": "noon",
                "color_family": "blue",
                "swatch_srgb8": [0, 0, 255],
                "require_source_material": true
            }]
        }))
        .expect("wrong appearance expectation serializes"),
    )
    .expect("wrong appearance expectation writes");

    let wrong_output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "appearance",
            "tests/assets/gltf/material_variants_scene.gltf",
            "--expect",
            path_str(&wrong_path),
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify appearance command runs");

    assert!(
        !wrong_output.status.success(),
        "wrong appearance should fail closed"
    );
    assert!(
        wrong_output.stderr.is_empty(),
        "appearance failures stay machine-readable on stdout, stderr={}",
        stderr(&wrong_output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&wrong_output.stdout).expect("appearance failure emits JSON");
    assert_eq!(report["schema"], "scena.appearance_introspection.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["reasons"]
            .as_array()
            .expect("appearance reasons array")
            .iter()
            .any(|reason| reason["code"] == "color_family_mismatch"),
        "appearance failure should explain color mismatch: {report:#}"
    );
}

#[test]
fn scena_verify_animation_cli_checks_sampled_change_and_fails_closed() {
    let animated = "tests/assets/gltf/animated_triangle_scene.glb";
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            animated,
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0.5,1.0",
            "--expect-change",
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify animation command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "animation report stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("animation command emits JSON");
    assert_eq!(report["schema"], "scena.animation_introspection.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["clip"]["name"], "MoveTriangle");
    assert_eq!(report["summary"]["sample_count"], 3);
    assert_eq!(report["summary"]["visible_change"], true);
    assert_eq!(report["samples"][0]["time_seconds"], 0.0);
    assert_eq!(report["samples"][1]["time_seconds"], 0.5);
    assert_eq!(report["samples"][2]["time_seconds"], 1.0);

    let missing = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            animated,
            "--clip",
            "MissingClip",
            "--times",
            "0,0.5",
            "--expect-change",
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify animation missing clip command runs");
    assert!(!missing.status.success(), "missing clip must fail closed");
    assert!(
        missing.stderr.is_empty(),
        "missing clip report stays machine-readable on stdout, stderr={}",
        stderr(&missing)
    );
    let missing_report: serde_json::Value =
        serde_json::from_slice(&missing.stdout).expect("missing clip emits JSON");
    assert_eq!(missing_report["schema"], "scena.animation_introspection.v1");
    assert_eq!(missing_report["ok"], false);
    assert!(
        missing_report["reasons"]
            .as_array()
            .expect("animation reasons array")
            .iter()
            .any(|reason| reason["code"] == "clip_missing"),
        "missing clip should be machine-readable: {missing_report:#}"
    );

    let no_change = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            animated,
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0,0",
            "--expect-change",
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify animation no-change command runs");
    assert!(!no_change.status.success(), "no change must fail closed");
    assert!(
        no_change.stderr.is_empty(),
        "no-change report stays machine-readable on stdout, stderr={}",
        stderr(&no_change)
    );
    let no_change_report: serde_json::Value =
        serde_json::from_slice(&no_change.stdout).expect("no-change failure emits JSON");
    assert_eq!(
        no_change_report["schema"],
        "scena.animation_introspection.v1"
    );
    assert_eq!(no_change_report["ok"], false);
    assert!(
        no_change_report["reasons"]
            .as_array()
            .expect("animation reasons array")
            .iter()
            .any(|reason| reason["code"] == "time_not_advanced"
                || reason["code"] == "no_visible_change"),
        "no-change report should explain temporal failure: {no_change_report:#}"
    );
}

#[test]
fn scena_verify_animation_cli_checks_expected_sampled_translations() {
    let animated = "tests/assets/gltf/animated_triangle_scene.glb";
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            animated,
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0.5,1.0",
            "--expect-change",
            "--expect-translations",
            "0,0,0;0.25,0,0;0.5,0,0",
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify animation expected translations command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "animation expected-value report stays machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("animation command emits JSON");
    assert_eq!(report["schema"], "scena.animation_introspection.v1");
    assert_eq!(report["ok"], true);
    let samples = report["samples"].as_array().expect("samples array");
    assert_eq!(samples.len(), 3);
    for sample in samples {
        let values = sample["observed_values"]
            .as_array()
            .expect("observed values array");
        assert_eq!(values.len(), 1, "{report:#}");
        assert_eq!(values[0]["kind"], "transform");
        assert_eq!(values[0]["within_tolerance"], true);
    }
    assert_eq!(
        samples[2]["observed_values"][0]["transform"]["translation"],
        json!([0.5, 0.0, 0.0])
    );

    let wrong = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            animated,
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0.5,1.0",
            "--expect-change",
            "--expect-translations",
            "0,0,0;0.4,0,0;0.5,0,0",
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify animation wrong expected translations command runs");
    assert!(
        !wrong.status.success(),
        "wrong expected translation should fail closed"
    );
    assert!(
        wrong.stderr.is_empty(),
        "wrong expected-value report stays machine-readable on stdout, stderr={}",
        stderr(&wrong)
    );
    let wrong_report: serde_json::Value =
        serde_json::from_slice(&wrong.stdout).expect("wrong animation command emits JSON");
    assert_eq!(wrong_report["schema"], "scena.animation_introspection.v1");
    assert_eq!(wrong_report["ok"], false);
    assert!(
        wrong_report["reasons"]
            .as_array()
            .expect("animation expected-value reasons array")
            .iter()
            .any(|reason| reason["code"] == "expected_value_mismatch"),
        "wrong expected value should be machine-readable: {wrong_report:#}"
    );
}

#[test]
fn scena_dynamic_verification_failing_fixtures_emit_expected_reasons() {
    let appearance = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "appearance",
            "tests/assets/gltf/material_variants_scene.gltf",
            "--expect",
            "tests/assets/agent-failure-fixtures/appearance_wrong_color.expectation.json",
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify appearance failure fixture command runs");
    assert!(!appearance.status.success());
    assert!(
        appearance.stderr.is_empty(),
        "appearance failure fixture must keep stderr empty, stderr={}",
        stderr(&appearance)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&appearance.stdout).expect("appearance failure emits JSON");
    assert_eq!(report["schema"], "scena.appearance_introspection.v1");
    assert_eq!(report["ok"], false);
    assert_report_reason(&report, "color_family_mismatch");

    let animation_case: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            "tests/assets/agent-failure-fixtures/animation_wrong_translation.case.json",
        )
        .expect("animation failure fixture reads"),
    )
    .expect("animation failure fixture parses");
    let asset = animation_case["asset"].as_str().expect("asset string");
    let clip = animation_case["clip"].as_str().expect("clip string");
    let times = animation_case["times"].as_str().expect("times string");
    let expected_translations = animation_case["expect_translations"]
        .as_str()
        .expect("expect_translations string");
    let expected_reason = animation_case["expected_reason"]
        .as_str()
        .expect("expected_reason string");

    let animation = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            asset,
            "--clip",
            clip,
            "--times",
            times,
            "--expect-change",
            "--expect-translations",
            expected_translations,
            "--width",
            "96",
            "--height",
            "72",
        ])
        .output()
        .expect("scena verify animation failure fixture command runs");
    assert!(!animation.status.success());
    assert!(
        animation.stderr.is_empty(),
        "animation failure fixture must keep stderr empty, stderr={}",
        stderr(&animation)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&animation.stdout).expect("animation failure emits JSON");
    assert_eq!(report["schema"], "scena.animation_introspection.v1");
    assert_eq!(report["ok"], false);
    assert_report_reason(&report, expected_reason);
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

fn fixed_artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target").join("gate-artifacts").join(name);
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

fn assert_json_numbers_rounded(value: &serde_json::Value, digits: u32) {
    match value {
        serde_json::Value::Number(number) => {
            if let Some(float) = number.as_f64() {
                let scale = 10_f64.powi(digits as i32);
                let rounded = (float * scale).round() / scale;
                assert!(
                    (float - rounded).abs() <= 1.0e-9,
                    "number {float} is not rounded to {digits} decimal places"
                );
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                assert_json_numbers_rounded(value, digits);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                assert_json_numbers_rounded(value, digits);
            }
        }
        _ => {}
    }
}

fn assert_report_reason(report: &serde_json::Value, code: &str) {
    assert!(
        report["reasons"]
            .as_array()
            .expect("report reasons array")
            .iter()
            .any(|reason| reason["code"] == code),
        "expected report reason '{code}': {report:#}"
    );
}

fn assert_success_stdout_matches(output: &std::process::Output, fixture: &str) {
    assert!(output.status.success(), "stderr={}", stderr(output));
    assert_stdout_matches(output, fixture);
}

fn assert_failure_stdout_matches(output: &std::process::Output, fixture: &str) {
    assert!(!output.status.success(), "command should fail closed");
    assert_stdout_matches(output, fixture);
}

fn assert_stdout_matches(output: &std::process::Output, fixture: &str) {
    assert!(
        output.stderr.is_empty(),
        "golden stdout command must keep stderr empty, stderr={}",
        stderr(output)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("stdout emits JSON");
    let expected: Value = serde_json::from_str(
        &fs::read_to_string(cli_golden_path(fixture))
            .unwrap_or_else(|error| panic!("failed to read CLI golden fixture {fixture}: {error}")),
    )
    .expect("CLI golden fixture parses");
    assert_eq!(actual, expected, "CLI stdout fixture drifted: {fixture}");
}

fn cli_golden_path(name: &str) -> PathBuf {
    PathBuf::from("tests")
        .join("assets")
        .join("cli-golden")
        .join(name)
}
