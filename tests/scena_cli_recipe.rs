#![cfg(feature = "inspection")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

const TEST_ASSET: &str = "tests/assets/gltf/mesh_material_vertex_color_scene.gltf";

#[test]
fn scena_render_cli_accepts_scene_recipe_input() {
    let dir = artifact_dir("render");
    let recipe_path = write_valid_recipe(&dir);
    let png_path = dir.join("recipe-frame.png");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render recipe command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "recipe render keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render recipe emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["artifacts"]["capture_png_path"], path_str(&png_path));
    assert!(fs::metadata(&png_path).expect("PNG artifact exists").len() > 0);
}

#[test]
fn scena_inspect_and_diagnose_cli_accept_scene_recipe_input() {
    let dir = artifact_dir("inspect-diagnose");
    let recipe_path = write_valid_recipe(&dir);

    let inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["inspect", path_str(&recipe_path)])
        .output()
        .expect("scena inspect recipe command runs");
    assert!(inspect.status.success(), "stderr={}", stderr(&inspect));
    let inspection: serde_json::Value =
        serde_json::from_slice(&inspect.stdout).expect("inspect recipe emits JSON");
    assert_eq!(inspection["schema"], "scena.scene_inspection.v1");
    assert!(
        inspection["counts"]["visible_drawable"]
            .as_u64()
            .expect("visible_drawable count is numeric")
            > 0,
        "recipe inspection should include the imported mesh: {inspection:#}"
    );

    let diagnose = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "diagnose",
            path_str(&recipe_path),
            "--visibility",
            "--handle",
            "999999",
        ])
        .output()
        .expect("scena diagnose recipe command runs");
    assert!(!diagnose.status.success());
    assert!(
        diagnose.stderr.is_empty(),
        "recipe diagnosis failures stay machine-readable on stdout, stderr={}",
        stderr(&diagnose)
    );
    let diagnosis: serde_json::Value =
        serde_json::from_slice(&diagnose.stdout).expect("diagnose recipe emits JSON");
    assert_eq!(diagnosis["schema"], "scena.visibility_diagnosis.v1");
    assert_eq!(diagnosis["ok"], false);
    assert!(
        diagnosis["reasons"]
            .as_array()
            .expect("diagnosis reasons array")
            .iter()
            .any(|reason| reason["code"] == "stale_handle"),
        "recipe diagnosis should explain stale handle: {diagnosis:#}"
    );
}

fn write_valid_recipe(dir: &Path) -> PathBuf {
    let recipe_path = dir.join("scene.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "part", "uri": TEST_ASSET }
            ],
            "capture": {
                "width": 96,
                "height": 72
            }
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");
    recipe_path
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target")
        .join("gate-artifacts")
        .join(format!("scena-cli-recipe-{name}-{}", std::process::id()));
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
