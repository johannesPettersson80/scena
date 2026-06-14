use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_feature_gated_contract_suite_without_explicit_command() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/feature-gated-contract-suite");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("docs/checklists")).expect("checklist fixture dir");
    fs::create_dir_all(fixture_root.join("tests")).expect("tests fixture dir");
    fs::write(
        fixture_root.join("docs/checklists/application-builder-roadmap.md"),
        "# Roadmap\n\nNo feature-enabled contract command here.\n",
    )
    .expect("roadmap fixture");
    fs::write(
        fixture_root.join("tests/example_contracts.rs"),
        "#![cfg(feature = \"inspection\")]\n\n#[test]\nfn example_contract() {}\n",
    )
    .expect("feature-gated test fixture");
    let mut findings = Vec::new();

    check_feature_gated_contract_tests_documented(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-FEATURE-GATED-CONTRACT-SUITES"
                && finding
                    .message
                    .contains("cargo test --features inspection --test example_contracts")
        }),
        "doctor must require explicit feature-enabled commands for gated contract suites: {findings:?}",
    );
}

#[test]
pub(crate) fn feature_gated_contract_suites_are_documented_in_current_roadmap() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_feature_gated_contract_tests_documented(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}
