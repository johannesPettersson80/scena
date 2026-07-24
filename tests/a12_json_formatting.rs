use std::process::{Command, Output};

#[test]
fn compact_and_pretty_are_global_deterministic_and_semantically_identical() {
    let default = run(&["--help"]);
    let pretty = run(&["--pretty", "--help"]);
    let compact = run(&["--compact", "--help"]);
    assert!(default.status.success(), "stderr={}", stderr(&default));
    assert!(pretty.status.success(), "stderr={}", stderr(&pretty));
    assert!(compact.status.success(), "stderr={}", stderr(&compact));
    assert_eq!(
        default.stdout, pretty.stdout,
        "pretty is the compatibility default"
    );
    assert!(String::from_utf8_lossy(&pretty.stdout).lines().count() > 1);
    assert_eq!(String::from_utf8_lossy(&compact.stdout).lines().count(), 1);
    assert_eq!(stdout_json(&pretty), stdout_json(&compact));

    let compact_after_command = run(&["schema", "list", "--compact"]);
    assert!(
        compact_after_command.status.success(),
        "stderr={}",
        stderr(&compact_after_command)
    );
    assert_eq!(
        String::from_utf8_lossy(&compact_after_command.stdout)
            .lines()
            .count(),
        1
    );
}

#[test]
fn formatting_applies_to_domain_failures_and_cli_errors_without_changing_envelopes() {
    let invalid = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/assets/recipe-invalid/invalid_transform.recipe.json"
    );
    let domain_pretty = run(&["validate-recipe", invalid, "--pretty"]);
    let domain_compact = run(&["--compact", "validate-recipe", invalid]);
    assert!(!domain_pretty.status.success());
    assert!(!domain_compact.status.success());
    assert_eq!(stdout_json(&domain_pretty), stdout_json(&domain_compact));
    assert!(
        String::from_utf8_lossy(&domain_pretty.stdout)
            .lines()
            .count()
            > 1
    );
    assert_eq!(
        String::from_utf8_lossy(&domain_compact.stdout)
            .lines()
            .count(),
        1
    );

    let cli_pretty = run(&["--pretty", "unknown-command"]);
    let cli_compact = run(&["--compact", "unknown-command"]);
    assert_eq!(stderr_json(&cli_pretty), stderr_json(&cli_compact));
    assert!(String::from_utf8_lossy(&cli_pretty.stderr).lines().count() > 1);
    assert_eq!(
        String::from_utf8_lossy(&cli_compact.stderr).lines().count(),
        1
    );
}

#[test]
fn conflicting_json_styles_fail_with_typed_usage_error() {
    let output = run(&["--compact", "--pretty", "--help"]);
    assert_eq!(output.status.code(), Some(2));
    let report = stderr_json(&output);
    assert_eq!(report["schema"], "scena.cli_error.v1");
    assert_eq!(report["code"], "invalid_arguments");
    assert!(
        report["message"]
            .as_str()
            .is_some_and(|message| message.contains("mutually exclusive"))
    );
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

fn stderr_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|error| panic!("stderr is JSON: {error}; stderr={}", stderr(output)))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
