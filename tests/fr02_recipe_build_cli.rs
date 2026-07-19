#![cfg(feature = "scene-host")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::json;

#[test]
fn fr02_recipe_build_emits_manifest_policy_and_zero_render_execution() {
    let root = fixture_dir("success");
    let recipe = root.join("scene.recipe.json");
    write_json(
        &recipe,
        &json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "box_geo",
                "primitive": { "kind": "box", "size": [0.2, 0.2, 0.2] }
            }],
            "materials": [{
                "id": "box_mat",
                "kind": "pbr_metallic_roughness",
                "base_color": "#D85C5C",
                "roughness": 0.45,
                "metallic": 0.0
            }],
            "nodes": [{
                "id": "box",
                "geometry": "box_geo",
                "material": "box_mat"
            }]
        }),
    );

    let output = run_build(&recipe, &["--max-imports", "2"]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert_eq!(report["schema"], "scena.recipe_build_result.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["build"]["schema"], "scena.scene_recipe_build.v1");
    assert_eq!(report["build"]["ok"], true);
    assert_eq!(report["build"]["nodes"][0]["id"], "box");
    assert_eq!(report["policy"]["schema"], "scena.recipe_policy.v1");
    assert_eq!(report["policy"]["network"]["allowed"], false);
    assert_eq!(report["policy"]["limits"]["max_imports"]["value"], 2);
    assert_eq!(
        report["policy"]["limits"]["max_imports"]["source"],
        "operator_override"
    );
    assert_eq!(report["execution"]["asset_fetches"], 0);
    assert_zero_render_execution(&report);
}

#[test]
fn fr02_recipe_build_reports_broken_asset_and_policy_denial_without_rendering() {
    let root = fixture_dir("failures");
    let missing = root.join("missing.recipe.json");
    write_json(
        &missing,
        &json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "missing", "uri": "does-not-exist.glb" }]
        }),
    );
    let output = run_build(&missing, &[]);
    assert_eq!(output.status.code(), Some(1), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert_eq!(report["schema"], "scena.recipe_build_result.v1");
    assert_eq!(report["ok"], false);
    assert!(has_diagnostic(&report, "import_load_failed"), "{report:#}");
    assert!(
        report["execution"]["asset_fetches"]
            .as_u64()
            .is_some_and(|n| n > 0)
    );
    assert_zero_render_execution(&report);

    let denied = root.join("denied.recipe.json");
    write_json(
        &denied,
        &json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "remote", "uri": "https://example.com/model.glb" }]
        }),
    );
    let output = run_build(&denied, &[]);
    assert_eq!(output.status.code(), Some(1), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert!(has_diagnostic(&report, "policy_violation"), "{report:#}");
    assert_eq!(report["policy"]["network"]["allowed"], false);
    assert_eq!(report["execution"]["asset_fetches"], 0);
    assert_zero_render_execution(&report);
}

#[test]
fn fr02_recipe_build_validates_required_environment_and_counts_real_fetch_attempts() {
    let root = fixture_dir("environment");
    let recipe = root.join("missing-environment.recipe.json");
    write_json(
        &recipe,
        &json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "box_geo",
                "primitive": { "kind": "box", "size": [0.2, 0.2, 0.2] }
            }],
            "materials": [{
                "id": "box_mat",
                "kind": "unlit",
                "base_color": "#FFFFFF"
            }],
            "nodes": [{
                "id": "box",
                "geometry": "box_geo",
                "material": "box_mat"
            }],
            "scene": {
                "environment": {
                    "kind": "uri",
                    "uri": "missing-environment.hdr"
                }
            }
        }),
    );

    let output = run_build(&recipe, &[]);
    assert_eq!(output.status.code(), Some(1), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert!(
        has_diagnostic(&report, "environment_load_failed"),
        "{report:#}"
    );
    assert!(
        report["execution"]["asset_fetches"]
            .as_u64()
            .is_some_and(|count| count >= 1),
        "the build report must count real environment fetch attempts: {report:#}"
    );
    assert_zero_render_execution(&report);
}

#[test]
fn fr02_recipe_build_result_matches_stable_schema_fixture_shape() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "assets/stable-contracts/recipe_build_result.v1.json"
    ))
    .expect("stable recipe-build result fixture parses");
    assert_eq!(fixture["schema"], "scena.recipe_build_result.v1");
    assert_eq!(fixture["build"]["schema"], "scena.scene_recipe_build.v1");
    assert_eq!(fixture["policy"]["schema"], "scena.recipe_policy.v1");
    assert_zero_render_execution(&fixture);
}

#[test]
fn fr02_renderer_free_build_matches_existing_typed_manifest_golden() {
    let recipe = Path::new("tests/assets/stable-contracts/scene_recipe.v1.json");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/stable-contracts/scene_recipe_build.v1.json"
    ))
    .expect("stable build manifest fixture parses");
    let output = run_build(recipe, &[]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert_eq!(report["build"], expected);
    assert_eq!(
        report["execution"]["asset_fetches"], 3,
        "the stable glTF import fetches its source plus two external resources"
    );
    assert_zero_render_execution(&report);
}

fn assert_zero_render_execution(report: &serde_json::Value) {
    for counter in [
        "renderer_constructions",
        "gpu_context_constructions",
        "prepare_calls",
        "render_calls",
        "capture_constructions",
    ] {
        assert_eq!(report["execution"][counter], 0, "{counter}: {report:#}");
    }
}

fn has_diagnostic(report: &serde_json::Value, code: &str) -> bool {
    report["build"]["diagnostics"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|row| row["code"] == code))
}

fn run_build(recipe: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    command
        .args(["recipe", "build", path_str(recipe)])
        .args(extra);
    command.output().expect("scena recipe build runs")
}

fn fixture_dir(name: &str) -> PathBuf {
    let path = PathBuf::from("target/fr02-recipe-build-cli").join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("fixture directory creates");
    path
}

fn write_json(path: &Path, value: &serde_json::Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).expect("fixture writes");
}

fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not JSON: {error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(output)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("fixture path is UTF-8")
}
