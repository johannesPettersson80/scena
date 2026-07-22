use crate::app::prelude::*;

#[test]
fn d05_d06_doctor_rejects_multiple_active_backlogs_and_persistence_overclaim() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/d05-d06-governance");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "README.md",
        "docs/README.md",
        "docs/RFC-rust-3d-renderer.md",
        "docs/schema-contracts.md",
        "docs/checklists/application-builder-roadmap.md",
        "docs/checklists/full-repo-review-v1.8.0-remediation.md",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        "docs/checklists/renderer-fidelity-dependencies.md",
        "docs/checklists/wasm-scene-host-and-stable-contracts.md",
        "crates/xtask/src/app/tests_72.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("governance fixture parent"))
            .expect("governance fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("governance fixture source copies");
    }

    let mut findings = Vec::new();
    check_document_governance_truth(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let docs_index = fixture_root.join("docs/README.md");
    let source = fs::read_to_string(&docs_index).expect("docs index fixture reads");
    let mutated = source.replace("Active open backlog", "Multiple active backlogs");
    assert_ne!(source, mutated, "D05 mutation must alter the docs index");
    fs::write(&docs_index, mutated).expect("D05 mutation writes");
    findings.clear();
    check_document_governance_truth(&fixture_root, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "D05-D06-DOCUMENT-GOVERNANCE"),
        "multiple-backlog drift must fail doctor: {findings:?}"
    );

    fs::write(&docs_index, source).expect("D05 fixture restores");
    let schema = fixture_root.join("docs/schema-contracts.md");
    let source = fs::read_to_string(&schema).expect("schema fixture reads");
    let mutated = source.replace(
        "not the canonical persisted application document",
        "the canonical persisted application document",
    );
    assert_ne!(source, mutated, "D06 mutation must alter recipe wording");
    fs::write(schema, mutated).expect("D06 mutation writes");
    findings.clear();
    check_document_governance_truth(&fixture_root, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "D05-D06-DOCUMENT-GOVERNANCE"),
        "recipe persistence overclaim must fail doctor: {findings:?}"
    );
}
