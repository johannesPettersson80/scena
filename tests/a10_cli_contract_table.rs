use std::collections::BTreeSet;
use std::process::Command;

use sha2::{Digest, Sha256};

#[test]
fn every_help_row_is_a_self_contained_process_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena --help runs");
    assert!(output.status.success());
    let help: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("help is structured JSON");
    let contracts = help["command_contracts"]
        .as_array()
        .expect("help has command contracts");
    assert_eq!(
        contracts.len(),
        help["commands"]
            .as_array()
            .expect("help has commands")
            .len()
    );

    for contract in contracts {
        let command = contract["command"].as_str().expect("command is text");
        assert_eq!(contract["streams"]["success"], "stdout", "{command}");
        assert_eq!(contract["streams"]["domain_failure"], "stdout", "{command}");
        assert_eq!(contract["streams"]["cli_error"], "stderr", "{command}");
        let failures = contract["failure_exits"]
            .as_array()
            .unwrap_or_else(|| panic!("{command} has numeric failure exits"));
        let classes = contract["failure_exit_classes"]
            .as_array()
            .expect("legacy failure class inventory remains available");
        assert_eq!(failures.len(), classes.len(), "{command}");
        assert!(
            failures.iter().any(|row| {
                row["class"] == "io"
                    && row["exit_code"] == 74
                    && row["schema"] == "scena.cli_error.v1"
                    && row["stream"] == "stderr"
            }),
            "{command} must expose I/O exit 74 without a source lookup"
        );
        for row in failures {
            assert!(row["class"].as_str().is_some(), "{command}: {row}");
            assert!(row["exit_code"].as_i64().is_some(), "{command}: {row}");
        }
        assert!(
            contract["feature_requirements"].as_array().is_some(),
            "{command} feature requirements must be explicit, including []"
        );
    }
}

#[test]
fn agent_only_commands_name_the_one_step_install_feature() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena --help runs");
    let help: serde_json::Value = serde_json::from_slice(&output.stdout).expect("help is JSON");
    let agent_commands = help["command_contracts"]
        .as_array()
        .expect("command contracts")
        .iter()
        .filter(|row| {
            row["feature_requirements"]
                .as_array()
                .is_some_and(|features| features.iter().any(|feature| feature == "agent"))
        })
        .filter_map(|row| row["command"].as_str())
        .collect::<BTreeSet<_>>();
    for command in [
        "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
        "photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>] [--allow-root <directory>]...",
        "photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--optimize] [--max-imports <n>] [--allow-root <directory>]...",
        "recipe build <recipe.json> [--max-imports <n>] [--allow-root <directory>]...",
        "recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--max-imports <n>] [--allow-root <directory>]...",
        "examples agent list",
        "render <asset-or-recipe> --out <png> [--introspect] [--gpu] [--allow-root <directory>]...",
        "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
    ] {
        assert!(
            agent_commands.contains(command),
            "missing agent gate: {command}"
        );
    }
    for command in [
        "schema list",
        "schema json <scena.*.vN>",
        "validate <file>",
        "validate-recipe <recipe.json> [--full|--syntax-only] [--max-imports <n>] [--allow-root <directory>]...",
    ] {
        assert!(
            !agent_commands.contains(command),
            "core command incorrectly gated: {command}"
        );
    }
}

#[test]
fn errors_doc_points_to_the_machine_authoritative_complete_table() {
    let docs = include_str!("../docs/errors.md");
    for required in [
        "Complete CLI process contract",
        "success and domain-failure JSON use stdout",
        "CLI dispatch and runtime errors use stderr",
        "feature_requirements",
        "failure_exits",
        "I/O 74",
        "scena --help",
    ] {
        assert!(docs.contains(required), "errors docs missing {required}");
    }
}

#[test]
fn complete_process_table_matches_the_reviewable_golden_digest() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena --help runs");
    let help: serde_json::Value = serde_json::from_slice(&output.stdout).expect("help is JSON");
    let bytes = serde_json::to_vec(&help["command_contracts"])
        .expect("command contract table serializes deterministically");
    let actual = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let expected = include_str!("assets/cli-golden/process_contract_table.sha256").trim();
    assert_eq!(
        actual, expected,
        "CLI process table changed; review the schema/stream/exit/feature diff, then update the golden digest intentionally"
    );
}
