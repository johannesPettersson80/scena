#![cfg(feature = "agent")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn validate_dispatches_public_input_contracts_by_embedded_schema() {
    for (relative, contract) in [
        (
            "tests/assets/recipe-invalid/valid_for_commands.recipe.json",
            "scena.scene_recipe.v1",
        ),
        (
            "tests/assets/stable-contracts/appearance_expectation.v1.json",
            "scena.appearance_expectation.v1",
        ),
        (
            "tests/assets/stable-contracts/interaction_expectation.v1.json",
            "scena.interaction_expectation.v1",
        ),
        (
            "tests/assets/stable-contracts/recipe_patch.v1.json",
            "scena.recipe_patch.v1",
        ),
        (
            "tests/assets/stable-contracts/capability_report.v1.json",
            "scena.capability_report.v1",
        ),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
        let output = run(&["validate", path.to_str().expect("fixture path is UTF-8")]);
        assert!(
            output.status.success(),
            "relative={relative} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report = stdout_json(&output);
        assert_eq!(report["schema"], "scena.contract_validation.v1");
        assert_eq!(report["contract"], contract);
        assert_eq!(report["ok"], true);
        assert_eq!(report["validation_level"], "typed");
    }
}

#[test]
fn validate_fails_closed_for_malformed_unknown_and_mismatched_contracts() {
    let root = unique_temp_dir();
    fs::create_dir(&root).expect("validation temp directory creates");
    let malformed = root.join("malformed.json");
    fs::write(&malformed, b"{not-json").expect("malformed fixture writes");
    let unknown = root.join("unknown.json");
    fs::write(&unknown, br#"{"schema":"scena.scene_recip.v1"}"#).expect("unknown fixture writes");
    let mismatched = root.join("mismatch.json");
    fs::write(
        &mismatched,
        br#"{"schema":"scena.capability_report.v1","capabilities":"wrong"}"#,
    )
    .expect("mismatch fixture writes");

    for (path, code) in [
        (&malformed, "malformed_json"),
        (&unknown, "unknown_schema"),
        (&mismatched, "contract_mismatch"),
    ] {
        let output = run(&["validate", path.to_str().expect("fixture path is UTF-8")]);
        assert_eq!(output.status.code(), Some(65), "path={path:?}");
        let report = stdout_json(&output);
        assert_eq!(report["schema"], "scena.contract_validation.v1");
        assert_eq!(report["ok"], false);
        assert!(
            report["diagnostics"]
                .as_array()
                .is_some_and(|rows| rows.iter().any(|row| row["code"] == code))
        );
        if code == "unknown_schema" {
            assert!(
                report["diagnostics"][0]["candidates"]
                    .as_array()
                    .is_some_and(|values| values
                        .iter()
                        .any(|value| value == "scena.scene_recipe.v1"))
            );
        }
    }
    fs::remove_dir_all(root).expect("validation temp directory removes");
}

#[test]
fn schema_json_exports_recipe_schema_and_declares_runtime_limits() {
    let output = run(&["schema", "json", "scena.scene_recipe.v1"]);
    assert!(output.status.success());
    let report = stdout_json(&output);
    assert_eq!(report["schema"], "scena.json_schema_export.v1");
    assert_eq!(report["contract"], "scena.scene_recipe.v1");
    assert_eq!(
        report["json_schema"]["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert!(report["json_schema"]["properties"]["schema"].is_object());
    assert!(report["limitations"].as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str().is_some_and(|text| text.contains("runtime")))
    }));
}

#[test]
fn validate_reuses_recipe_patch_owner_invariants() {
    let mut patch: serde_json::Value =
        serde_json::from_str(include_str!("assets/stable-contracts/recipe_patch.v1.json"))
            .expect("patch fixture parses");
    patch["source_sha256"] = serde_json::json!("not-a-digest");
    let root = unique_temp_dir();
    fs::create_dir(&root).expect("validation temp directory creates");
    let path = root.join("invalid-patch.json");
    fs::write(
        &path,
        serde_json::to_vec(&patch).expect("patch mutation serializes"),
    )
    .expect("patch mutation writes");

    let output = run(&["validate", path.to_str().expect("fixture path is UTF-8")]);
    assert_eq!(output.status.code(), Some(65));
    let report = stdout_json(&output);
    assert_eq!(report["schema"], "scena.contract_validation.v1");
    assert_eq!(report["diagnostics"][0]["code"], "contract_mismatch");
    assert!(
        report["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("source_sha256"))
    );
    fs::remove_dir_all(root).expect("validation temp directory removes");
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
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("scena-a09-validate-{}-{nonce}", std::process::id()))
}
