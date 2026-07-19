use crate::app::prelude::*;

#[test]
fn d04_review_provenance_rejects_tag_and_universal_claim_drift() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/d04-review-provenance");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("docs/reviews")).expect("review fixture directory");
    fs::write(
        fixture_root.join("docs/reviews/full-repo-review-v1.7.2.md"),
        "# Review: full repo at v1.7.2\nMethod: eight parallel passes. Nothing like it exists anywhere.\n",
    )
    .expect("weak review fixture writes");

    let mut findings = Vec::new();
    check_review_provenance_contracts(&fixture_root, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "D04-REVIEW-PROVENANCE"),
        "tag conflation and universal claims must fail closed: {findings:?}"
    );

    fs::write(
        fixture_root.join("docs/reviews/full-repo-review-v1.7.2.md"),
        "# Review: source snapshot main@bea2a36\nCargo package version is 1.7.2 and this snapshot is 14 commits after tag `v1.7.2`. This is not a review of the tagged release. `schema_entry_rows()` exposes 45 entries but excludes additional versioned schema literals. No universal uniqueness claim is made without a dated official-documentation matrix. Unsupported review-pass and independent reviewers process claims are withdrawn.\n",
    )
    .expect("strong review fixture writes");
    findings.clear();
    check_review_provenance_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}

#[test]
fn o01_remote_builder_contract_rejects_missing_fallback_and_bootstrap() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/o01-remote-builder");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in [".codex/skills/scena-remote-builder", "scripts"] {
        fs::create_dir_all(fixture_root.join(directory)).expect("builder fixture directory");
    }
    fs::write(
        fixture_root.join("AGENTS.md"),
        "Remote repo path: /home/johannes/projects/scena\n",
    )
    .expect("weak AGENTS fixture writes");
    fs::write(
        fixture_root.join(".codex/skills/scena-remote-builder/SKILL.md"),
        "Run cargo in $HOME/projects/scena.\n",
    )
    .expect("weak remote skill fixture writes");

    let mut findings = Vec::new();
    check_remote_builder_bootstrap_contracts(&fixture_root, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "O01-REMOTE-BUILDER-BOOTSTRAP"),
        "missing-path fallback and manual bootstrap omissions must fail: {findings:?}"
    );

    fs::write(
        fixture_root.join("AGENTS.md"),
        "scripts/scena_remote_builder_preflight.sh\nvalidation_mode=isolated\nAGENTS.md\n.codex/skills\nsha256sum\nCARGO_TARGET_DIR\n",
    )
    .expect("strong AGENTS fixture writes");
    fs::write(
        fixture_root.join(".codex/skills/scena-remote-builder/SKILL.md"),
        "scripts/scena_remote_builder_preflight.sh\nshared_checkout_status=missing\nvalidation_mode=isolated\nAGENTS.md\n.codex/skills\nsha256sum\nCARGO_TARGET_DIR\n",
    )
    .expect("strong remote skill fixture writes");
    fs::write(
        fixture_root.join("scripts/scena_remote_builder_preflight.sh"),
        "shared_checkout_status=missing\nvalidation_mode=isolated\nvalidation_path=\ncargo_target_dir=\ndf -hT\n",
    )
    .expect("strong preflight fixture writes");
    findings.clear();
    check_remote_builder_bootstrap_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}
