use crate::app::prelude::*;

#[test]
fn q03_doctor_rejects_missing_ci_attestation_wiring() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q03-ci-provenance");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q03 doctor fixture");
    }
    for relative in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "scripts/ci_provenance.js",
        "crates/xtask/src/app/release/ci_provenance.rs",
        "crates/xtask/src/app/release/bundle_schema.rs",
        "tests/release/ci_provenance_test.js",
        "docs/specs/release-gates.md",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q03 doctor fixture parent");
        fs::copy(root.join(relative), &destination).expect("copy Q03 doctor contract source");
    }
    let mut findings = Vec::new();
    check_ci_attestation_contracts(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q03 CI provenance contract must satisfy doctor: {findings:?}"
    );

    let workflow = fixture.join(".github/workflows/ci.yml");
    let source = fs::read_to_string(&workflow).expect("read CI workflow fixture");
    let mutated = source.replace(
        "target/release-evidence-integrity-report.log",
        "target/release-artifacts/release-evidence-integrity-report.log",
    );
    assert_ne!(
        source, mutated,
        "Q03 mutation must place the report inside the attested artifact tree"
    );
    fs::write(&workflow, mutated).expect("write Q03 mutable artifact-tree mutation");
    findings.clear();
    check_ci_attestation_contracts(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "RELEASE-CI-PROVENANCE"),
        "doctor must reject files written under the artifact root after its digest is signed: \
         {findings:?}"
    );

    fs::write(&workflow, &source).expect("restore Q03 workflow fixture");
    let mutated = source.replace(
        "actions/attest@f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6 # v4.2.0",
        "actions/attest@missing-immutable-pin # mutation",
    );
    assert_ne!(
        source, mutated,
        "Q03 mutation must remove attestation action"
    );
    fs::write(&workflow, mutated).expect("write Q03 attestation mutation");
    findings.clear();
    check_ci_attestation_contracts(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "RELEASE-CI-PROVENANCE"),
        "doctor must reject missing CI attestation wiring: {findings:?}"
    );

    fs::write(&workflow, &source).expect("restore Q03 workflow fixture");
    let script = fixture.join("scripts/ci_provenance.js");
    let script_source = fs::read_to_string(&script).expect("read Q03 provenance script");
    let mutated = script_source
        .replace(
            "compareUtf8(left.name, right.name)",
            "left.name.localeCompare(right.name)",
        )
        .replace(
            "compareUtf8(left.path, right.path)",
            "left.path.localeCompare(right.path)",
        );
    assert_ne!(
        script_source, mutated,
        "Q03 mutation must restore locale-sensitive path ordering"
    );
    fs::write(script, mutated).expect("write Q03 locale-order mutation");
    findings.clear();
    check_ci_attestation_contracts(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "RELEASE-CI-PROVENANCE"),
        "doctor must reject provenance ordering that disagrees with the Rust verifier: \
         {findings:?}"
    );
}
