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

#[test]
pub(crate) fn doctor_rejects_schema_docs_reference_missing_from_catalog() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/schema-docs-catalog");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("docs")).expect("docs fixture dir");
    fs::create_dir_all(fixture_root.join("tests/assets/stable-contracts"))
        .expect("stable contract fixture dir");
    fs::write(fixture_root.join("README.md"), "").expect("readme fixture");
    fs::write(
        fixture_root.join("AGENTS.md"),
        "# AGENTS\n\nNo schema refs here.\n",
    )
    .expect("agents fixture");
    fs::write(
        fixture_root.join("docs/schema-contracts.md"),
        "This doc references `scena.missing_contract.v1` and proof artifact `scena.m6.example_proof.v1`.\n",
    )
    .expect("schema docs fixture");
    fs::write(
        fixture_root.join("tests/assets/stable-contracts/schema_catalog.v1.json"),
        r#"{"schema":"scena.schema_catalog.v1","entries":[]}"#,
    )
    .expect("schema catalog fixture");
    let mut findings = Vec::new();

    crate::app::doctor_docs::schema_references::check_schema_doc_references_listed_in_catalog(
        &fixture_root,
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "STABLE-CONTRACT-EVIDENCE"
                && finding.message.contains("scena.missing_contract.v1")
        }),
        "doctor must reject documented schemas missing from the schema catalog: {findings:?}",
    );
}
