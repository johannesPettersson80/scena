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

    for (relative, contents) in [
        (
            "AGENTS.md",
            "scripts/scena_remote_builder_preflight.sh\nscripts/collect_ci_failure_evidence.sh\nscripts/run_windows_complete_hardware_proof.ps1\nvalidation_mode=isolated\nAGENTS.md\n.codex/skills\nsha256sum\nCARGO_TARGET_DIR\nInvestigation Circuit Breakers\nproduct defect\nAfter two failed remediation attempts\nAfter 30 minutes\nsecond user-assisted run requires\nSCENA_M9_TIMING_POLICY=report-only-hosted\n",
        ),
        (
            ".codex/skills/scena-remote-builder/SKILL.md",
            "scripts/scena_remote_builder_preflight.sh\nshared_checkout_status=missing\nvalidation_mode=isolated\nAGENTS.md\n.codex/skills\nsha256sum\nCARGO_TARGET_DIR\ninvestigation circuit breaker\nstop after two remedies\n",
        ),
        (
            ".codex/skills/scena-renderer-quality/SKILL.md",
            "Investigation Circuit Breaker\nSCENA_M9_TIMING_POLICY=report-only-hosted\nscripts/run_windows_complete_hardware_proof.ps1\nsecond user-assisted run requires explicit approval\n",
        ),
        (
            ".codex/skills/scena-doctor/SKILL.md",
            "investigation circuit breakers\nscena-<task-slug>\n",
        ),
        (
            ".codex/skills/scena-release-hygiene/SKILL.md",
            "scripts/collect_ci_failure_evidence.sh\ninvestigation circuit breaker\nSCENA_M9_TIMING_POLICY=report-only-hosted\n",
        ),
        (
            ".codex/skills/scena-git-github/SKILL.md",
            "scripts/collect_ci_failure_evidence.sh\ninvestigation circuit breaker\nscena-<task-slug>\n",
        ),
        (
            "scripts/scena_remote_builder_preflight.sh",
            "shared_checkout_status=missing\nvalidation_mode=isolated\nvalidation_path=\ncargo_target_dir=\ndf -hT\n",
        ),
        (
            "scripts/collect_ci_failure_evidence.sh",
            "scena.ci_failure_evidence.v1\nfailed-jobs.tsv\nclassification_status\nroot-cause-checkpoint.md\ngh run download\n",
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            "bundle-files.sha256\nCompress-Archive\nUploadUrl\nInvoke-WebRequest -UseBasicParsing -Method Put\n",
        ),
    ] {
        let path = fixture_root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture file has parent"))
            .expect("strong builder fixture directory");
        fs::write(path, contents).expect("strong builder fixture writes");
    }
    findings.clear();
    check_remote_builder_bootstrap_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}
