use crate::app::prelude::*;

#[test]
fn a04_doctor_rejects_restoring_error_exit_for_command_help() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a04-cli-ergonomics");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena.rs",
        "src/bin/scena/help.rs",
        "src/bin/scena/examples_agent.rs",
        "src/bin/scena/examples_agent/catalog.rs",
        "src/bin/scena/diff.rs",
        "src/bin/scena/process_output_shared.rs",
        "src/schema_catalog.rs",
        "src/schema_catalog/agent_smoke.rs",
        "src/schema_catalog/fixtures.rs",
        "tests/a04_cli_ergonomics.rs",
        "tests/assets/stable-contracts/agent_template_catalog.v1.json",
        "tests/fr04_cli_schema_matrix.rs",
        "tests/stable_contracts.rs",
        "README.md",
        "docs/examples.md",
        "docs/schema-contracts.md",
        "docs/troubleshooting.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A04 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A04 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a04_cli_ergonomics(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let dispatch = fixture_root.join("src/bin/scena.rs");
    let source = fs::read_to_string(&dispatch).expect("A04 dispatch source reads");
    let mutated = source.replacen(
        "scena_help::command_help_json(&args)",
        "scena_help::removed_command_help_json(&args)",
        1,
    );
    assert_ne!(source, mutated, "A04 mutation must remove help dispatch");
    fs::write(dispatch, mutated).expect("A04 dispatch mutation writes");
    findings.clear();
    check_a04_cli_ergonomics(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A04-CLI-ERGONOMICS" && finding.message.contains("command_help_json")
        }),
        "restoring command-help errors must fail doctor: {findings:?}",
    );
}
