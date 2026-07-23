use std::process::Command;

#[test]
fn cli_errors_expose_stable_typed_exit_taxonomy() {
    let cases: &[(&[&str], i32, &str, &str)] = &[
        (&["definitely-not-a-command"], 2, "invalid_command", "usage"),
        (
            &["capabilities", "--definitely-invalid"],
            2,
            "invalid_arguments",
            "usage",
        ),
        (
            &["schema", "get", "scena.not_a_real_contract.v1"],
            65,
            "unknown_schema",
            "input",
        ),
    ];

    for (args, exit, code, exit_class) in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(*args)
            .output()
            .unwrap_or_else(|error| panic!("scena {args:?} runs: {error}"));
        assert_eq!(output.status.code(), Some(*exit), "args={args:?}");
        assert!(output.stdout.is_empty(), "error stdout must be empty");
        let report: serde_json::Value = serde_json::from_slice(&output.stderr)
            .unwrap_or_else(|error| panic!("args={args:?} stderr is JSON: {error}"));
        assert_eq!(report["schema"], "scena.cli_error.v1", "args={args:?}");
        assert_eq!(report["ok"], false, "args={args:?}");
        assert_eq!(report["code"], *code, "args={args:?}");
        assert_eq!(report["exit_class"], *exit_class, "args={args:?}");
        assert_eq!(report["exit_code"], *exit, "args={args:?}");
        assert!(
            report["message"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "args={args:?}"
        );
        assert!(
            report["context"]["command"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "args={args:?}"
        );
        assert!(
            report["help"].as_str().is_some_and(|text| !text.is_empty()),
            "args={args:?}"
        );
        assert!(report["candidates"].is_array(), "args={args:?}");
    }
}

#[test]
fn runtime_and_feature_failures_are_not_invalid_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["render", "missing.gltf", "--out", "missing.png"])
        .output()
        .expect("scena render runs");
    assert_ne!(output.status.code(), Some(2));
    if output.stderr.is_empty() {
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .expect("enabled render emits its typed domain failure on stdout");
        assert_eq!(report["schema"], "scena.asset_doctor.v1");
        assert_eq!(report["ok"], false);
        assert_eq!(report["status"], "failed");
        assert!(
            report["findings"]
                .as_array()
                .is_some_and(|rows| { rows.iter().any(|row| row["code"] == "asset_not_found") })
        );
    } else {
        let report: serde_json::Value = serde_json::from_slice(&output.stderr)
            .expect("unavailable render emits typed CLI JSON");
        assert_eq!(report["schema"], "scena.cli_error.v1");
        assert_ne!(report["code"], "invalid_arguments");
        assert!(matches!(
            report["exit_class"].as_str(),
            Some("unsupported") | Some("input") | Some("runtime")
        ));
    }
}

#[test]
fn every_declared_command_has_error_schema_and_exit_class_inventory() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena help runs");
    assert!(output.status.success());
    let help: serde_json::Value = serde_json::from_slice(&output.stdout).expect("help is JSON");
    let taxonomy = help["error_taxonomy"]
        .as_array()
        .expect("help exposes the exit taxonomy");
    let known = taxonomy
        .iter()
        .map(|row| row["class"].as_str().expect("class is text"))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(known.contains("comparison"));
    assert!(known.contains("policy"));
    assert!(known.contains("interrupted"));

    for contract in help["command_contracts"]
        .as_array()
        .expect("command contracts are an array")
    {
        assert!(
            contract["emits"]["error"]
                .as_array()
                .is_some_and(|schemas| schemas.iter().any(|schema| schema == "scena.cli_error.v1")),
            "{} must declare the typed CLI error schema",
            contract["command"]
        );
        let classes = contract["failure_exit_classes"]
            .as_array()
            .expect("command declares failure classes");
        assert!(!classes.is_empty(), "command={}", contract["command"]);
        assert!(
            classes
                .iter()
                .all(|class| class.as_str().is_some_and(|class| known.contains(class))),
            "command={} classes={classes:?}",
            contract["command"]
        );
    }
}
