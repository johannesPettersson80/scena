#![cfg(all(feature = "inspection", feature = "scene-host"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const EXTERNAL_ASSET_SOURCE: &str = "tests/assets/gltf/cad_terminal_block.gltf";
const REPAIR_REPORT: &str = "tests/assets/stable-contracts/visibility_diagnosis.v1.json";

#[test]
fn policy_recipe_discovers_repeatable_canonical_operator_roots() {
    let root = external_dir("policy-discovery");
    let first = root.join("first");
    let second = root.join("second");
    fs::create_dir_all(&first).expect("first A02 root creates");
    fs::create_dir_all(&second).expect("second A02 root creates");

    let output = scena(&[
        "policy",
        "recipe",
        "--allow-root",
        path_str(&first),
        "--allow-root",
        path_str(&second),
    ]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = stdout_json(&output);
    assert_operator_root(&report, &first);
    assert_operator_root(&report, &second);

    let missing = root.join("missing");
    let rejected = scena(&["policy", "recipe", "--allow-root", path_str(&missing)]);
    assert_eq!(rejected.status.code(), Some(65), "{rejected:?}");
    let error: Value = serde_json::from_slice(&rejected.stderr).expect("CLI error is JSON");
    assert_eq!(error["code"], "input_not_found");
    assert_eq!(error["exit_class"], "input");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("existing directory")),
        "{error:#}"
    );
}

#[test]
fn allow_root_is_identical_across_recipe_aware_cli_commands() {
    let external = external_dir("command-matrix");
    fs::create_dir_all(&external).expect("external A02 library creates");
    let asset = external.join("terminal.gltf");
    fs::copy(EXTERNAL_ASSET_SOURCE, &asset).expect("external A02 asset copies");

    let local = fixture_dir("command-matrix");
    let recipe = local.join("scene.recipe.json");
    write_recipe(&recipe, &asset);

    let denied = scena(&["validate-recipe", path_str(&recipe), "--full"]);
    assert_eq!(denied.status.code(), Some(1), "stderr={}", stderr(&denied));
    let denied = stdout_json(&denied);
    assert!(has_policy_violation(&denied), "{denied:#}");

    let png_recipe = local.join("recipe.png");
    let png_legacy = local.join("legacy.png");
    let commands = [
        vec!["validate-recipe", path_str(&recipe), "--full"],
        vec!["recipe", "build", path_str(&recipe)],
        vec![
            "recipe",
            "render",
            path_str(&recipe),
            "--introspect",
            "--out",
            path_str(&png_recipe),
        ],
        vec![
            "render",
            path_str(&recipe),
            "--introspect",
            "--out",
            path_str(&png_legacy),
        ],
        vec!["inspect", path_str(&recipe)],
        vec!["diagnose", path_str(&recipe), "--visibility"],
        vec!["doctor", path_str(&recipe)],
        vec!["repair", path_str(&recipe), "--from", REPAIR_REPORT],
    ];

    for mut args in commands {
        args.extend(["--allow-root", path_str(&external)]);
        let output = scena(&args);
        assert!(
            output.status.success(),
            "{args:?} must accept the same operator root; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = stdout_json(&output);
        assert_operator_root(&report, &external);
        assert!(!has_policy_violation(&report), "{args:?}: {report:#}");
    }
}

#[cfg(unix)]
#[test]
fn allow_root_rejects_parent_traversal_and_symlink_escape_after_canonicalization() {
    use std::os::unix::fs::symlink;

    let external = external_dir("canonical-escape");
    let library = external.join("library");
    fs::create_dir_all(&library).expect("A02 library creates");
    let secret = external.join("secret.gltf");
    fs::copy(EXTERNAL_ASSET_SOURCE, &secret).expect("A02 secret asset copies");
    let symlink_path = library.join("escape.gltf");
    symlink(&secret, &symlink_path).expect("A02 escape symlink creates");

    let local = fixture_dir("canonical-escape");
    for (name, asset) in [
        ("symlink", symlink_path),
        ("parent", library.join("..").join("secret.gltf")),
    ] {
        let recipe = local.join(format!("{name}.recipe.json"));
        write_recipe(&recipe, &asset);
        let output = scena(&[
            "validate-recipe",
            path_str(&recipe),
            "--full",
            "--allow-root",
            path_str(&library),
        ]);
        assert_eq!(output.status.code(), Some(1), "{name}: {}", stderr(&output));
        let report = stdout_json(&output);
        assert!(has_policy_violation(&report), "{name}: {report:#}");
        assert_operator_root(&report, &library);
    }
}

fn write_recipe(path: &Path, asset: &Path) {
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "external", "uri": asset }]
        }))
        .expect("A02 recipe serializes"),
    )
    .expect("A02 recipe writes");
}

fn assert_operator_root(report: &Value, expected: &Path) {
    let expected = expected
        .canonicalize()
        .expect("expected root canonicalizes");
    let policy = report.get("policy").unwrap_or(report);
    let roots = policy["allowed_roots"]
        .as_array()
        .unwrap_or_else(|| panic!("policy roots missing: {report:#}"));
    assert!(
        roots.iter().any(|root| {
            root["path"].as_str() == expected.to_str()
                && root["source"].as_str() == Some("operator_override")
        }),
        "missing canonical operator root '{}': {report:#}",
        expected.display()
    );
}

fn has_policy_violation(report: &Value) -> bool {
    report
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            report
                .pointer("/build/diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
        .any(|diagnostic| diagnostic["code"] == "policy_violation")
}

fn scena(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("scena {args:?} runs: {error}"))
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must be JSON: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("A02 path is valid UTF-8")
}

fn fixture_dir(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/gate-artifacts/a02-recipe-policy-{name}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("A02 fixture directory creates");
    root
}

fn external_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("scena-a02-external-{name}-{}", std::process::id()))
}
