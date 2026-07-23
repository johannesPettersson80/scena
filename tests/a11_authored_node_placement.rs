#![cfg(feature = "agent")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn authored_node_bounds_verbs_preview_apply_and_round_trip() {
    let root = unique_temp_dir();
    fs::create_dir(&root).expect("placement temp directory creates");
    let recipe = root.join("authored.recipe.json");
    fs::write(
        &recipe,
        br##"{
          "schema":"scena.scene_recipe.v1",
          "geometries":[{"id":"box","primitive":{"kind":"box","size":[2.0,2.0,2.0]}}],
          "materials":[{"id":"mat","kind":"unlit","base_color":"#ffffff"}],
          "nodes":[{"id":"cube","geometry":"box","material":"mat","transform":{"kind":"trs","translation":[0.0,2.0,0.0]}}]
        }"##,
    )
    .expect("authored recipe writes");

    let preview = run(&[
        "place",
        path(&recipe),
        "--node",
        "cube",
        "--verb",
        "ground",
        "--ground-y",
        "0",
    ]);
    assert!(preview.status.success(), "stderr={}", stderr(&preview));
    let preview = stdout_json(&preview);
    assert_eq!(preview["schema"], "scena.placement_result.v1");
    assert_eq!(preview["target"]["kind"], "node");
    assert_eq!(preview["target"]["id"], "cube");
    assert_eq!(
        preview["transform"]["translation"],
        serde_json::json!([0.0, 1.0, 0.0])
    );

    let applied = run(&[
        "place",
        path(&recipe),
        "--node",
        "cube",
        "--verb",
        "ground",
        "--ground-y",
        "0",
        "--apply",
    ]);
    assert!(applied.status.success(), "stderr={}", stderr(&applied));
    let applied = stdout_json(&applied);
    assert_eq!(applied["schema"], "scena.recipe_patch.v1");
    assert_eq!(applied["target"]["kind"], "node");
    assert_eq!(applied["target"]["id"], "cube");
    assert_eq!(
        applied["updated_recipe"]["nodes"][0]["transform"],
        preview["transform"]
    );
    assert_eq!(
        applied["semantic_changes"][0]["path"],
        "$.nodes[0].transform"
    );

    let updated = root.join("updated.recipe.json");
    fs::write(
        &updated,
        serde_json::to_vec_pretty(&applied["updated_recipe"]).expect("updated recipe serializes"),
    )
    .expect("updated recipe writes");
    let validation = run(&["validate-recipe", path(&updated), "--syntax-only"]);
    assert!(
        validation.status.success(),
        "stderr={}",
        stderr(&validation)
    );
    fs::remove_dir_all(root).expect("placement temp directory removes");
}

#[test]
fn authored_node_target_errors_are_namespace_aware_and_import_features_stay_import_only() {
    let root = unique_temp_dir();
    fs::create_dir(&root).expect("placement temp directory creates");
    let recipe = root.join("authored.recipe.json");
    fs::write(
        &recipe,
        br##"{
          "schema":"scena.scene_recipe.v1",
          "geometries":[{"id":"box","primitive":{"kind":"box","size":[1.0,1.0,1.0]}}],
          "materials":[{"id":"mat","kind":"unlit","base_color":"#ffffff"}],
          "nodes":[{"id":"cube","geometry":"box","material":"mat"}]
        }"##,
    )
    .expect("authored recipe writes");

    let wrong_namespace = run(&[
        "place",
        path(&recipe),
        "--import",
        "cube",
        "--verb",
        "center",
    ]);
    assert_eq!(wrong_namespace.status.code(), Some(1));
    let wrong_namespace = stdout_json(&wrong_namespace);
    assert_eq!(
        wrong_namespace["diagnostics"][0]["code"],
        "wrong_target_namespace"
    );
    assert!(
        wrong_namespace["diagnostics"][0]["candidates"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "--node cube"))
    );

    let near_name = run(&["place", path(&recipe), "--node", "cbe", "--verb", "center"]);
    assert_eq!(near_name.status.code(), Some(1));
    let near_name = stdout_json(&near_name);
    assert_eq!(near_name["diagnostics"][0]["code"], "unknown_node");
    assert!(
        near_name["diagnostics"][0]["candidates"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "cube"))
    );

    let import_only = run(&[
        "place",
        path(&recipe),
        "--node",
        "cube",
        "--verb",
        "align_to_anchor",
    ]);
    assert_eq!(import_only.status.code(), Some(1));
    let import_only = stdout_json(&import_only);
    assert_eq!(import_only["diagnostics"][0]["code"], "import_only_verb");
    fs::remove_dir_all(root).expect("placement temp directory removes");
}

#[test]
fn authored_starter_manifest_teaches_node_place_and_apply() {
    let root = unique_temp_dir();
    let output = run(&[
        "examples",
        "agent",
        "get",
        "primitive-scene",
        "--out",
        path(&root),
    ]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let manifest = stdout_json(&output);
    let commands = manifest["commands"].as_array().expect("manifest commands");
    for name in ["place_node", "apply_node_placement"] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}: {manifest:#}"));
        assert!(
            command["argv"]
                .as_array()
                .is_some_and(|args| args.iter().any(|arg| arg == "--node"))
        );
    }
    fs::remove_dir_all(root).expect("template directory removes");
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .expect("scena command runs")
}

fn stdout_json(output: &Output) -> serde_json::Value {
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

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("scena-a11-place-{}-{nonce}", std::process::id()))
}
