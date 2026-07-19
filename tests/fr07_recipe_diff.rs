#![cfg(feature = "scene-host")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

use scena::{
    SCENE_RECIPE_DIFF_SCHEMA_V1, SceneRecipeDiffOptions, SceneRecipeV1, diff_scene_recipes,
};

fn recipe(value: Value) -> SceneRecipeV1 {
    serde_json::from_value(value).expect("FR07 recipe fixture parses")
}

#[test]
fn fr07_typed_recipe_diff_reports_identity_fields_tolerance_and_order() {
    let left = recipe(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [{ "id": "box", "primitive": { "kind": "box", "size": [1.0, 1.0, 1.0] } }],
        "materials": [
            { "id": "body", "kind": "unlit", "base_color": "#112233" },
            { "id": "trim", "kind": "unlit", "base_color": "#445566" }
        ],
        "nodes": [{
            "id": "machine",
            "geometry": "box",
            "material": "body",
            "transform": { "kind": "trs", "translation": [1.0, 0.0, 0.0] }
        }],
        "cameras": [{
            "id": "main", "kind": "perspective", "active": true,
            "fov_degrees": 45.0,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.0], "target": [0.0, 0.0, 0.0] }
        }]
    }));
    let right = recipe(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [{ "id": "box", "primitive": { "kind": "box", "size": [1.0, 1.0, 1.0] } }],
        "materials": [
            { "id": "trim", "kind": "unlit", "base_color": "#445566" },
            { "id": "body", "kind": "unlit", "base_color": "#112233" },
            { "id": "warning", "kind": "unlit", "base_color": "#FFAA00" }
        ],
        "nodes": [{
            "id": "machine",
            "geometry": "box",
            "material": "body",
            "transform": { "kind": "trs", "translation": [1.00001, 0.0, 0.25] }
        }]
    }));

    let report = diff_scene_recipes(&left, &right, SceneRecipeDiffOptions::new(0.001));
    assert_eq!(report.schema, SCENE_RECIPE_DIFF_SCHEMA_V1);
    assert!(!report.equal);
    let value = serde_json::to_value(&report).expect("FR07 report serializes");
    let changes = value["changes"].as_array().expect("changes are an array");
    assert!(changes.iter().any(|change| {
        change["scope"] == "material" && change["id"] == "warning" && change["kind"] == "added"
    }));
    assert!(changes.iter().any(|change| {
        change["scope"] == "material"
            && change["kind"] == "reordered"
            && change["order_before"] == json!(["body", "trim"])
            && change["order_after"] == json!(["trim", "body", "warning"])
    }));
    assert!(
        changes.iter().any(|change| {
            change["scope"] == "node"
                && change["id"] == "machine"
                && change["kind"] == "modified"
                && change["fields"].as_array().is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| field == "transform.translation[2]")
                })
        }),
        "typed node change is present: {value:#}"
    );
    assert!(!changes.iter().any(|change| {
        change["fields"].as_array().is_some_and(|fields| {
            fields
                .iter()
                .any(|field| field == "transform.translation[0]")
        })
    }));
    assert!(changes.iter().any(|change| {
        change["scope"] == "camera" && change["id"] == "main" && change["kind"] == "removed"
    }));
}

#[test]
fn fr07_diff_cli_keeps_structural_diff_renderer_free() {
    let root = fixture_dir("structural");
    let before = root.join("before.recipe.json");
    let after = root.join("after.recipe.json");
    write_recipe_pair(&before, &after);

    let output = run_diff(&before, &after, &["--numeric-tolerance", "0.0001"]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert_eq!(report["schema"], "scena.scene_recipe_diff_result.v1");
    assert_eq!(report["equal"], false);
    assert_eq!(report["structural"]["schema"], SCENE_RECIPE_DIFF_SCHEMA_V1);
    assert_eq!(report["structural"]["equal"], false);
    assert_eq!(report["visual"], Value::Null);
    assert_eq!(report["execution"]["renderer_constructions"], 0);
    assert_eq!(report["execution"]["prepare_calls"], 0);
    assert_eq!(report["execution"]["render_calls"], 0);
    assert_eq!(report["execution"]["capture_constructions"], 0);
}

#[test]
fn fr07_rendered_diff_reuses_aggregate_diff_and_attributes_only_supported_pixels() {
    let root = fixture_dir("rendered");
    let before = root.join("before.recipe.json");
    let after = root.join("after.recipe.json");
    let out_dir = root.join("diff-output");
    write_recipe_pair(&before, &after);

    let output = run_diff(
        &before,
        &after,
        &[
            "--render",
            "--out-dir",
            path(&out_dir),
            "--max-abs-diff",
            "0",
        ],
    );
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert_eq!(report["schema"], "scena.scene_recipe_diff_result.v1");
    assert_eq!(
        report["visual"]["aggregate"]["schema"],
        "scena.capture_baseline.v1"
    );
    assert!(
        report["visual"]["aggregate"]["diff"]["mismatched_pixels"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "rendered recipes must differ: {report:#}"
    );
    let summary = &report["visual"]["attribution"]["summary"];
    let changed = summary["changed_pixels"].as_u64().expect("changed pixels");
    let attributed = summary["attributed_pixels"]
        .as_u64()
        .expect("attributed pixels");
    let ambiguous = summary["ambiguous_pixels"]
        .as_u64()
        .expect("ambiguous pixels");
    let unattributed = summary["unattributed_pixels"]
        .as_u64()
        .expect("unattributed pixels");
    assert_eq!(changed, attributed + ambiguous + unattributed);
    assert!(
        attributed > 0,
        "node-owned color changes are attributed: {report:#}"
    );
    assert!(
        report["visual"]["attribution"]["regions"]
            .as_array()
            .is_some_and(|regions| regions.iter().any(|region| {
                region["classification"] == "attributed"
                    && (region["before"]["persistent_identity"]["node_id"] == "box"
                        || region["after"]["persistent_identity"]["node_id"] == "box")
            })),
        "attribution must use persistent recipe identity: {report:#}"
    );
    assert_eq!(
        report["visual"]["attribution"]["semantics"]["anti_aliased_edges"],
        "ambiguous"
    );
    assert_eq!(
        report["visual"]["attribution"]["semantics"]["transparent_and_excluded_surfaces"],
        "unattributed_or_ambiguous"
    );
    for artifact in [
        "before.png",
        "after.png",
        "diff.png",
        "recipe-diff-result.json",
    ] {
        assert!(out_dir.join(artifact).is_file(), "missing {artifact}");
    }
}

#[test]
fn fr07_diff_cli_emits_declared_validation_and_build_failure_schemas() {
    let root = fixture_dir("failures");
    let invalid = root.join("invalid.recipe.json");
    let valid = root.join("valid.recipe.json");
    fs::write(&invalid, r#"{"schema":"scena.scene_recipe.v1","nodez":[]}"#)
        .expect("invalid recipe writes");
    fs::write(
        &valid,
        r##"{
          "schema":"scena.scene_recipe.v1",
          "geometries":[{"id":"g","primitive":{"kind":"box","size":[1,1,1]}}],
          "materials":[{"id":"m","kind":"unlit","base_color":"#FFFFFF"}],
          "nodes":[{"id":"n","geometry":"g","material":"m"}]
        }"##,
    )
    .expect("valid recipe writes");
    let output = run_diff(&invalid, &valid, &[]);
    assert_eq!(output.status.code(), Some(1), "stderr={}", stderr(&output));
    assert_eq!(
        stdout_json(&output)["schema"],
        "scena.scene_recipe_validation.v1"
    );

    let missing = root.join("missing.recipe.json");
    fs::write(
        &missing,
        r#"{"schema":"scena.scene_recipe.v1","imports":[{"id":"part","uri":"missing.glb"}]}"#,
    )
    .expect("missing-import recipe writes");
    let output = run_diff(
        &missing,
        &valid,
        &["--render", "--out-dir", path(&root.join("build-output"))],
    );
    assert_eq!(output.status.code(), Some(1), "stderr={}", stderr(&output));
    assert_eq!(
        stdout_json(&output)["schema"],
        "scena.scene_recipe_build.v1"
    );
}

#[test]
fn fr07_rendered_diff_never_assigns_excluded_transparency_to_the_opaque_node_behind_it() {
    let root = fixture_dir("transparent");
    let before = root.join("before.recipe.json");
    let after = root.join("after.recipe.json");
    let out_dir = root.join("diff-output");
    let recipe = |transparent_color: &str| {
        json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "box_geo",
                "primitive": { "kind": "box", "size": [0.36, 0.36, 0.04] }
            }],
            "materials": [
                {
                    "id": "opaque", "kind": "unlit", "base_color": "#606060",
                    "double_sided": true
                },
                {
                    "id": "transparent", "kind": "unlit",
                    "base_color": transparent_color,
                    "alpha_mode": { "kind": "blend" }, "double_sided": true
                }
            ],
            "nodes": [
                { "id": "back", "geometry": "box_geo", "material": "opaque" },
                {
                    "id": "glass", "geometry": "box_geo", "material": "transparent",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.08] }
                }
            ],
            "cameras": [{
                "id": "main", "kind": "perspective", "active": true,
                "transform": {
                    "kind": "look_at", "eye": [0.0, 0.0, 1.4],
                    "target": [0.0, 0.0, 0.0]
                }
            }],
            "capture": { "width": 96, "height": 72 }
        })
    };
    fs::write(
        &before,
        serde_json::to_vec_pretty(&recipe("#FF3030")).expect("before serializes"),
    )
    .expect("before writes");
    fs::write(
        &after,
        serde_json::to_vec_pretty(&recipe("#3060FF")).expect("after serializes"),
    )
    .expect("after writes");

    let output = run_diff(&before, &after, &["--render", "--out-dir", path(&out_dir)]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    let attribution = &report["visual"]["attribution"];
    assert!(
        attribution["before_exclusions"]["transparent_triangle_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "fixture must exercise excluded transparent geometry: {report:#}"
    );
    assert!(
        attribution["summary"]["changed_pixels"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "transparent color mutation must visibly change pixels: {report:#}"
    );
    assert_eq!(
        attribution["summary"]["attributed_pixels"], 0,
        "without a transparent-surface ID mask, changed pixels must not be assigned to the opaque node behind the transparent surface: {report:#}"
    );
}

fn fixture_dir(name: &str) -> PathBuf {
    let root = PathBuf::from("target/gate-artifacts/fr07-recipe-diff").join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("FR07 fixture directory creates");
    root
}

fn write_recipe_pair(before: &Path, after: &Path) {
    let common = |color: &str, x: f64| {
        json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "box_geo",
                "primitive": { "kind": "box", "size": [0.36, 0.36, 0.12] }
            }],
            "materials": [{
                "id": "box_mat", "kind": "unlit", "base_color": color,
                "double_sided": true
            }],
            "nodes": [{
                "id": "box", "geometry": "box_geo", "material": "box_mat",
                "transform": { "kind": "trs", "translation": [x, 0.0, 0.0] }
            }],
            "cameras": [{
                "id": "main", "kind": "perspective", "active": true,
                "transform": {
                    "kind": "look_at", "eye": [0.0, 0.0, 1.4],
                    "target": [0.0, 0.0, 0.0]
                }
            }],
            "capture": { "width": 96, "height": 72 }
        })
    };
    fs::write(
        before,
        serde_json::to_vec_pretty(&common("#C43D3D", -0.16)).expect("before serializes"),
    )
    .expect("before recipe writes");
    fs::write(
        after,
        serde_json::to_vec_pretty(&common("#3D70C4", 0.16)).expect("after serializes"),
    )
    .expect("after recipe writes");
}

fn run_diff(before: &Path, after: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    command
        .args(["diff", path(before), path(after)])
        .args(extra);
    command.output().expect("scena diff command runs")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(output)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test path is UTF-8")
}
