use crate::app::prelude::*;

pub(crate) fn check_c04_release_readiness_contract(root: &Path, findings: &mut Vec<Finding>) {
    let rule = "C04-FAIL-CLOSED-RELEASE-READINESS";

    require_contains(
        root,
        findings,
        rule,
        "crates/xtask/src/app/core.rs",
        &[
            "release-readiness [--artifact-root <staged-artifact-root>]",
            "artifact_root: Some(args[2].clone())",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "crates/xtask/src/app/release/readiness.rs",
        &[
            "scena.release_readiness.v1",
            "SCENA_RELEASE_ARTIFACT_ROOT",
            "validated_artifact_count > 0",
            "validated_artifact_count == summary.required_artifact_count",
        ],
    );
    let inventory_path = root.join("crates/xtask/src/app/release/review_artifacts.rs");
    match fs::read_to_string(&inventory_path) {
        Ok(source) => {
            let required_block = source
                .split_once("pub(crate) const REQUIRED_RELEASE_ARTIFACT_SUFFIXES")
                .and_then(|(_, tail)| tail.split_once("];"))
                .map(|(block, _)| block);
            if required_block.is_none_or(|block| {
                !block.contains("m9-platform/linux-native-vulkan/rendered-output.json")
            }) {
                findings.push(Finding::new(
                    rule,
                    "crates/xtask/src/app/release/review_artifacts.rs must existence-require m9-platform/linux-native-vulkan/rendered-output.json in REQUIRED_RELEASE_ARTIFACT_SUFFIXES",
                ));
            }
        }
        Err(error) => findings.push(Finding::new(
            rule,
            format!("could not read {}: {error}", inventory_path.display()),
        )),
    }
    for (path, needles) in [
        (
            ".github/workflows/ci.yml",
            &["SCENA_RELEASE_ARTIFACT_ROOT=target/gate-artifacts"][..],
        ),
        (
            ".github/workflows/release.yml",
            &["SCENA_RELEASE_ARTIFACT_ROOT: target/gate-artifacts"][..],
        ),
        (
            "scripts/local_release_readiness.sh",
            &[
                "RELEASE_ARTIFACT_ROOT=",
                "release-readiness --artifact-root",
            ][..],
        ),
        (
            "scripts/release_publish_dry_run.sh",
            &[
                "release_artifact_root=",
                "release-readiness --artifact-root",
            ][..],
        ),
        (
            "docs/specs/release-gates.md",
            &[
                "scena.release_readiness.v1",
                "validated_artifact_count",
                "--artifact-root",
            ][..],
        ),
        (
            "docs/troubleshooting.md",
            &["RELEASE-READY-ARTIFACT-ROOT", "validated_artifact_count"][..],
        ),
        (
            "src/schema_catalog/entries.rs",
            &["schema: \"scena.release_readiness.v1\""][..],
        ),
        (
            "tests/assets/stable-contracts/schema_catalog.v1.json",
            &["\"schema\": \"scena.release_readiness.v1\""][..],
        ),
        (
            "tests/assets/cli-golden/schema_list_stdout.json",
            &["\"schema\": \"scena.release_readiness.v1\""][..],
        ),
        (
            "crates/xtask/src/app/tests_41.rs",
            &[
                "c04_release_readiness_requires_nonempty_explicit_artifact_root",
                "c04_release_readiness_reports_zero_validated_evidence_for_missing_or_incomplete_root",
                "c04_every_specialized_release_artifact_is_required_for_existence",
            ][..],
        ),
        (
            "crates/xtask/src/app/tests_01.rs",
            &["release_readiness_rejects_commit_mismatched_json_artifact"][..],
        ),
        (
            "crates/xtask/src/app/tests_08.rs",
            &["release_readiness_rejects_stale_timestamped_artifact"][..],
        ),
        (
            "crates/xtask/src/app/tests_11.rs",
            &[
                "stage_release_artifacts_generates_canonical_release_evidence",
                "substituting the PNG after result generation must fail",
            ][..],
        ),
        (
            "crates/xtask/src/app/tests_19.rs",
            &[
                "assert_m5_provenance_mutations_rejected",
                "corrupted-after-hash",
            ][..],
        ),
        (
            "crates/xtask/src/app/tests_20.rs",
            &[
                "browser_release_headline_validation_fails_closed_by_contract_dimension",
                "wrong backend must fail",
            ][..],
        ),
    ] {
        require_contains(root, findings, rule, path, needles);
    }
}
