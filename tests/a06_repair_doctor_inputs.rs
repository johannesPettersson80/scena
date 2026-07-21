#![cfg(all(feature = "inspection", feature = "scene-host"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::json;

const VALID_ASSET: &str = "tests/assets/gltf/mesh_material_vertex_color_scene.gltf";

#[test]
fn repair_validates_raw_asset_and_recipe_targets_before_planning() {
    let root = fixture_root("repair-targets");
    let report = root.join("diagnosis.json");
    fs::write(
        &report,
        include_str!("assets/stable-contracts/visibility_diagnosis.v1.json"),
    )
    .expect("diagnosis fixture writes");

    let valid = run(&["repair", VALID_ASSET, "--from", path_str(&report)]);
    assert!(valid.status.success(), "stderr={}", stderr(&valid));
    assert_eq!(stdout_json(&valid)["schema"], "scena.visual_repair_plan.v1");

    let missing = root.join("missing.gltf");
    let missing = run(&["repair", path_str(&missing), "--from", path_str(&report)]);
    assert_eq!(missing.status.code(), Some(1));
    let missing = stdout_json(&missing);
    assert_eq!(missing["schema"], "scena.asset_doctor.v1");
    assert_eq!(missing["ok"], false);

    let malformed = root.join("malformed.gltf");
    fs::write(&malformed, b"not glTF").expect("malformed target writes");
    let malformed = run(&["repair", path_str(&malformed), "--from", path_str(&report)]);
    assert_eq!(malformed.status.code(), Some(1));
    let malformed = stdout_json(&malformed);
    assert_eq!(malformed["schema"], "scena.asset_doctor.v1");
    assert_eq!(malformed["ok"], false);

    let recipe = write_recipe(&root.join("valid.recipe.json"), VALID_ASSET);
    let recipe_repair = run(&["repair", path_str(&recipe), "--from", path_str(&report)]);
    assert!(
        recipe_repair.status.success(),
        "stderr={}",
        stderr(&recipe_repair)
    );
    assert_eq!(
        stdout_json(&recipe_repair)["schema"],
        "scena.visual_repair_plan.v1"
    );
}

#[test]
fn repair_rejects_a_second_positional_target() {
    let root = fixture_root("repair-conflict");
    let report = root.join("diagnosis.json");
    fs::write(
        &report,
        include_str!("assets/stable-contracts/visibility_diagnosis.v1.json"),
    )
    .expect("diagnosis fixture writes");
    let output = run(&[
        "repair",
        VALID_ASSET,
        "another-target.gltf",
        "--from",
        path_str(&report),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("argument error is JSON");
    assert_eq!(error["schema"], "scena.cli_error.v1");
    assert_eq!(error["code"], "invalid_arguments");
}

#[test]
fn repair_command_help_explains_target_validation() {
    let output = run(&["repair", "--help", "--json"]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let help = stdout_json(&output);
    assert_eq!(help["schema"], "scena.cli_help.v1");
    assert!(help["notes"].as_array().is_some_and(|notes| {
        notes.iter().any(|note| {
            note.as_str()
                .is_some_and(|note| note.contains("loaded through asset doctor"))
        })
    }));
}

#[test]
fn doctor_routes_valid_missing_malformed_and_policy_rejected_recipe_inputs() {
    let root = fixture_root("doctor-recipes");
    let valid = write_recipe(&root.join("valid.recipe.json"), VALID_ASSET);
    let valid = run(&["doctor", path_str(&valid)]);
    assert!(valid.status.success(), "stderr={}", stderr(&valid));
    assert_eq!(
        stdout_json(&valid)["schema"],
        "scena.recipe_build_result.v1"
    );

    let missing_path = root.join("missing.recipe.json");
    let missing = run(&["doctor", path_str(&missing_path)]);
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(stdout_json(&missing)["schema"], "scena.asset_doctor.v1");

    let malformed_path = root.join("malformed.recipe.json");
    fs::write(&malformed_path, b"{").expect("malformed recipe writes");
    let malformed = run(&["doctor", path_str(&malformed_path)]);
    assert_eq!(malformed.status.code(), Some(1));
    assert_eq!(
        stdout_json(&malformed)["schema"],
        "scena.scene_recipe_validation.v1"
    );

    let policy_path = write_recipe(
        &root.join("policy-rejected.recipe.json"),
        "/tmp/scena-a06-outside-policy.gltf",
    );
    let policy = run(&["doctor", path_str(&policy_path)]);
    assert_eq!(policy.status.code(), Some(1));
    let policy = stdout_json(&policy);
    assert_eq!(policy["schema"], "scena.recipe_build_result.v1");
    assert_eq!(policy["ok"], false);
}

fn write_recipe(path: &Path, asset: &str) -> PathBuf {
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{"id": "primary", "uri": asset}]
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");
    path.to_owned()
}

fn fixture_root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/a06-inputs")
        .join(name);
    fs::create_dir_all(&root).expect("A06 fixture directory creates");
    root
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .expect("scena command runs")
}

fn stdout_json(output: &Output) -> serde_json::Value {
    assert!(output.stderr.is_empty(), "stderr={}", stderr(output));
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is UTF-8")
}
