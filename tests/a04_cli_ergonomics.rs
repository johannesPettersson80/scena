#[cfg(feature = "scene-host")]
use std::fs;
#[cfg(feature = "scene-host")]
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

#[cfg(feature = "scene-host")]
const CANONICAL_TEMPLATES: &[&str] = &[
    "animated-viewer",
    "cad-inspection",
    "cad-plate",
    "dashboard-bars",
    "data-visualization",
    "documentation-renderer",
    "interaction-proof",
    "live-state-viewer",
    "machine-state-viewer",
    "primitive-scene",
    "product-configurator",
    "product-configurator-starter",
    "web-viewer",
];

#[test]
fn global_and_every_command_help_are_successful_stdout_json() {
    let global = run(&["--help"]);
    assert_clean_help(&global, "global");

    for path in [
        &["version"][..],
        &["schema"][..],
        &["schema", "list"][..],
        &["schema", "get"][..],
        &["vocab"][..],
        &["vocab", "list"][..],
        &["vocab", "get"][..],
        &["policy"][..],
        &["policy", "recipe"][..],
        &["capabilities"][..],
        &["validate-recipe"][..],
        &["place"][..],
        &["diff"][..],
        &["recipe"][..],
        &["recipe", "build"][..],
        &["recipe", "render"][..],
        &["recipe", "inspect-cad"][..],
        &["recipe", "capture"][..],
        &["recipe", "aov"][..],
        &["examples"][..],
        &["examples", "agent"][..],
        &["examples", "agent", "list"][..],
        &["examples", "agent", "get"][..],
        &["render"][..],
        &["inspect"][..],
        &["diagnose"][..],
        &["doctor"][..],
        &["browser-proof"][..],
        &["repair"][..],
        &["verify"][..],
        &["verify", "appearance"][..],
        &["verify", "animation"][..],
        &["verify", "interaction"][..],
    ] {
        let mut args = path.to_vec();
        args.push("--help");
        let output = run(&args);
        assert_clean_help(&output, &path.join(" "));
        let report = stdout_json(&output);
        assert_eq!(report["scope"], "command");
        assert!(
            report["usage"]
                .as_str()
                .is_some_and(|usage| usage.starts_with("scena "))
        );
    }

    let json_alias = run(&["diff", "--help", "--json"]);
    assert_clean_help(&json_alias, "diff --help --json");
}

#[test]
#[cfg(feature = "scene-host")]
fn template_catalog_has_one_canonical_name_and_aliases_emit_migration_metadata() {
    let output = run(&["examples", "agent", "list"]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(output.stderr.is_empty(), "stderr={}", stderr(&output));
    let catalog = stdout_json(&output);
    assert_eq!(catalog["schema"], "scena.agent_template_catalog.v1");
    let names: Vec<_> = catalog["templates"]
        .as_array()
        .expect("templates is an array")
        .iter()
        .map(|entry| entry["name"].as_str().expect("canonical name"))
        .collect();
    assert_eq!(names, CANONICAL_TEMPLATES);

    for (alias, canonical) in [
        ("primitive_scene", "primitive-scene"),
        ("cad_plate", "cad-plate"),
        ("dashboard_bars", "dashboard-bars"),
        ("machine_state_viewer", "machine-state-viewer"),
        ("product_configurator", "product-configurator-starter"),
    ] {
        let out = artifact_dir().join(alias);
        let output = run(&["examples", "agent", "get", alias, "--out", path(&out)]);
        assert!(
            output.status.success(),
            "alias={alias}; stderr={}",
            stderr(&output)
        );
        let manifest = stdout_json(&output);
        assert_eq!(manifest["name"], canonical, "alias={alias}");
        assert!(manifest["notes"].as_array().is_some_and(|notes| {
            notes.iter().any(|note| {
                note.as_str().is_some_and(|note| {
                    note.contains("deprecated template alias") && note.contains(canonical)
                })
            })
        }));
    }

    let unknown = run(&["examples", "agent", "not-a-template"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
    let error: Value = serde_json::from_slice(&unknown.stderr).expect("error is JSON");
    assert_eq!(error["schema"], "scena.cli_error.v1");
    assert!(error["message"].as_str().is_some_and(|message| {
        message.contains("scena examples agent list") && !message.contains("available templates:")
    }));
}

#[test]
#[cfg(feature = "scene-host")]
fn diff_reports_inequality_as_data_unless_exit_code_mode_is_requested() {
    let root = artifact_dir().join("diff-exit-policy");
    fs::create_dir_all(&root).expect("diff fixture directory creates");
    let before = root.join("before.recipe.json");
    let after = root.join("after.recipe.json");
    let source =
        fs::read_to_string("tests/assets/schema-field-model/scene_recipe_roundtrip.v1.json")
            .expect("recipe fixture reads");
    fs::write(&before, &source).expect("before recipe writes");
    fs::write(&after, source.replace("\"width\": 320", "\"width\": 321"))
        .expect("after recipe writes");

    let default = run(&["diff", path(&before), path(&after)]);
    assert!(default.status.success(), "stderr={}", stderr(&default));
    let default_report = stdout_json(&default);
    assert_eq!(default_report["equal"], false);
    assert_eq!(default_report["exit_policy"], "report_only");

    let ci = run(&["diff", path(&before), path(&after), "--exit-code"]);
    assert_eq!(ci.status.code(), Some(1), "stderr={}", stderr(&ci));
    assert!(ci.stderr.is_empty(), "stderr={}", stderr(&ci));
    let ci_report = stdout_json(&ci);
    assert_eq!(ci_report["equal"], false);
    assert_eq!(ci_report["exit_policy"], "difference_is_failure");

    let equal = run(&["diff", path(&before), path(&before), "--exit-code"]);
    assert!(equal.status.success(), "stderr={}", stderr(&equal));
    assert_eq!(stdout_json(&equal)["equal"], true);
}

#[test]
fn unknown_commands_keep_stdout_clean_and_emit_one_stderr_envelope() {
    let output = run(&["not-a-command", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr is one JSON value");
    assert_eq!(error["schema"], "scena.cli_error.v1");
    assert_eq!(error["code"], "invalid_command");
}

#[cfg(unix)]
#[test]
fn broken_stdout_pipe_exits_successfully_without_stderr_noise() {
    let script = "set -o pipefail; \"$SCENA_BIN\" schema list | dd bs=1 count=0 2>/dev/null";
    let output = Command::new("bash")
        .args(["-c", script])
        .env("SCENA_BIN", env!("CARGO_BIN_EXE_scena"))
        .output()
        .expect("broken-pipe shell proof runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(output.stderr.is_empty(), "stderr={}", stderr(&output));
}

fn assert_clean_help(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context}: stderr={}",
        stderr(output)
    );
    assert!(
        output.stderr.is_empty(),
        "{context}: stderr={}",
        stderr(output)
    );
    assert_eq!(stdout_json(output)["schema"], "scena.cli_help.v1");
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .expect("scena command runs")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must be JSON: {error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(output)
        )
    })
}

#[cfg(feature = "scene-host")]
fn artifact_dir() -> PathBuf {
    PathBuf::from("target/gate-artifacts/a04-cli-ergonomics")
}

#[cfg(feature = "scene-host")]
fn path(path: &Path) -> &str {
    path.to_str().expect("test path is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
