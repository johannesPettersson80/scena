#![cfg(feature = "scene-host")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TEMPLATE_NAMES: &[&str] = &[
    "product-configurator",
    "live-state-viewer",
    "web-viewer",
    "data-visualization",
    "animated-viewer",
    "interaction-proof",
];

#[test]
fn scena_examples_agent_templates_generate_and_run_cli_smoke_commands() {
    let root = artifact_dir("agent-templates");

    for name in TEMPLATE_NAMES {
        let output_dir = root.join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(["examples", "agent", name, "--out", path_str(&output_dir)])
            .output()
            .expect("scena examples agent command runs");

        assert!(
            output.status.success(),
            "template {name} failed, stderr={}",
            stderr(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "template manifest stays machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("template emits JSON");
        assert_eq!(manifest["schema"], "scena.agent_smoke_template.v1");
        assert_eq!(manifest["name"], *name);
        assert_eq!(manifest["status"], "ready");
        assert!(
            manifest["files"]
                .as_array()
                .expect("files array")
                .iter()
                .all(|file| Path::new(file["path"].as_str().expect("file path")).exists()),
            "template files should exist: {manifest:#}"
        );

        for command in manifest["commands"].as_array().expect("commands array") {
            let argv = command["argv"]
                .as_array()
                .expect("argv array")
                .iter()
                .map(|value| value.as_str().expect("argv string").to_owned())
                .collect::<Vec<_>>();
            assert_eq!(argv.first().map(String::as_str), Some("scena"));
            let command_output = Command::new(env!("CARGO_BIN_EXE_scena"))
                .args(&argv[1..])
                .output()
                .unwrap_or_else(|error| panic!("template {name} command {argv:?} runs: {error}"));
            assert!(
                command_output.status.success(),
                "template {name} command {argv:?} failed, stderr={}",
                stderr(&command_output)
            );
            assert!(
                command_output.stderr.is_empty(),
                "template {name} command {argv:?} keeps JSON on stdout, stderr={}",
                stderr(&command_output)
            );
            let report: serde_json::Value =
                serde_json::from_slice(&command_output.stdout).expect("command emits JSON");
            assert_eq!(report["schema"], command["expected_schema"]);
            assert_eq!(report["ok"], command["expected_ok"]);
            for artifact in command["artifacts"].as_array().expect("artifacts array") {
                assert!(
                    Path::new(artifact.as_str().expect("artifact path")).exists(),
                    "template {name} command {argv:?} missing artifact {artifact:?}"
                );
            }
        }
    }
}

#[test]
fn scena_examples_agent_cli_stdout_matches_golden_fixture() {
    let root = PathBuf::from("target")
        .join("gate-artifacts")
        .join("scena-cli-agent-template-golden");
    let output_dir = root.join("product-configurator");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("golden template root creates");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "examples",
            "agent",
            "product-configurator",
            "--out",
            path_str(&output_dir),
        ])
        .output()
        .expect("scena examples agent golden command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "examples agent golden command keeps stderr empty, stderr={}",
        stderr(&output)
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("examples agent emits JSON");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/cli-golden/examples_agent_product_configurator_stdout.json"
    ))
    .expect("golden examples agent fixture parses");
    assert_eq!(actual, expected);
}

#[test]
fn scena_examples_agent_phase2_templates_are_structured_deferred_manifests() {
    let root = artifact_dir("agent-templates-deferred");

    for name in ["cad-inspection", "documentation-renderer"] {
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args([
                "examples",
                "agent",
                name,
                "--out",
                path_str(&root.join(name)),
            ])
            .output()
            .expect("scena examples agent deferred command runs");

        assert!(output.status.success(), "stderr={}", stderr(&output));
        assert!(output.stderr.is_empty(), "stderr={}", stderr(&output));
        let manifest: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("deferred template emits JSON");
        assert_eq!(manifest["schema"], "scena.agent_smoke_template.v1");
        assert_eq!(manifest["name"], name);
        assert_eq!(manifest["status"], "deferred");
        assert!(
            manifest["commands"]
                .as_array()
                .expect("commands array")
                .is_empty(),
            "deferred template must not pretend Phase 2 commands are runnable: {manifest:#}"
        );
        assert!(
            !manifest["notes"]
                .as_array()
                .expect("notes array")
                .is_empty(),
            "deferred template should explain its dependency: {manifest:#}"
        );
    }
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
