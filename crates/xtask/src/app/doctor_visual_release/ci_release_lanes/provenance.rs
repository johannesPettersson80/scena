use crate::app::prelude::*;

use super::require_contains_in_xtask_app_tree;

pub(crate) fn check_ci_attestation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "RELEASE-CI-PROVENANCE";
    const ATTEST_ACTION: &str = "actions/attest@f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6 # v4.2.0";
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        require_contains(
            root,
            findings,
            RULE,
            workflow,
            &[
                "id-token: write",
                "attestations: write",
                "artifact-metadata: write",
                "node scripts/ci_provenance.js target/release-artifacts target/release-artifacts/ci-provenance.json",
                ATTEST_ACTION,
                "subject-path: target/release-artifacts/ci-provenance.json",
                "GH_TOKEN: ${{ github.token }}",
                "SCENA_REQUIRE_CI_PROVENANCE: \"1\"",
            ],
        );
    }
    for (path, needles) in [
        (
            "scripts/ci_provenance.js",
            &[
                "scena.ci_provenance.v1",
                "GITHUB_ACTIONS",
                "GITHUB_REPOSITORY",
                "GITHUB_WORKFLOW_REF",
                "GITHUB_WORKFLOW_SHA",
                "GITHUB_RUN_ID",
                "GITHUB_RUN_ATTEMPT",
                "GITHUB_JOB",
                "GITHUB_SHA",
                "CI_ATTESTATION_NOT_YET_VERIFIED",
                "canonicalArtifactFiles",
            ][..],
        ),
        (
            "crates/xtask/src/app/release/ci_provenance.rs",
            &[
                "validate_ci_provenance_manifest",
                "canonical_artifact_tree_digest",
                "strict release staging requires a trusted GitHub Actions context",
                "git",
                "cat-file",
                "gh",
                "attestation",
                "verify",
                "--signer-workflow",
                "--source-digest",
                "--source-ref",
            ][..],
        ),
        (
            "crates/xtask/src/app/release/bundle_schema.rs",
            &[
                "require_verified_staging_provenance",
                "RELEASE-CI-PROVENANCE",
                "https://slsa.dev/provenance/v1",
                "verification_receipt_sha256",
            ][..],
        ),
        (
            "tests/release/ci_provenance_test.js",
            &[
                "wrong repository",
                "replayed run",
                "wrong ref",
                "missing job",
                "beforeTamper",
            ][..],
        ),
        (
            "docs/specs/release-gates.md",
            &[
                "## CI-issued release provenance",
                "ci-provenance.json",
                "release_evidence: false",
                "SCENA_REQUIRE_CI_PROVENANCE=1",
                "gh attestation verify",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}

pub(crate) fn check_m10_claim_audit_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains_in_xtask_app_tree(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        &[
            "claim-audit",
            "m10-claim-audit.json",
            "scena.m10.claim_audit.v1",
            "required_final_gates",
            "release-readiness",
            "REQUIRED_RELEASE_ARTIFACT_SUFFIXES",
        ],
    );
    for (path, needles) in [
        (
            "docs/checklists/m10-threejs-replacement-acceptance.md",
            &["m10-claim-audit.json", "claim audit"][..],
        ),
        (
            "docs/api/m10-public-api-diff.md",
            &[
                "M10 Public API Diff From M5 Baseline",
                "Renderer::diagnose_scene",
                "AssetLoadControl",
                "AssetError::UnsupportedTextureFormat",
                "Semver Decision",
            ][..],
        ),
        (
            "docs/release-notes/v1.3.0.md",
            &["scena v1.3.0 Release Notes", "Easy Scene Setup", "Install"][..],
        ),
    ] {
        require_contains(root, findings, "CLAIM-AUDIT-M10", path, needles);
    }
}
