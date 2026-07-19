use crate::app::prelude::*;

const RULE: &str = "O01-REMOTE-BUILDER-BOOTSTRAP";

pub(crate) fn check_remote_builder_bootstrap_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, required) in [
        (
            "AGENTS.md",
            &[
                "scripts/scena_remote_builder_preflight.sh",
                "scripts/collect_ci_failure_evidence.sh",
                "scripts/run_windows_complete_hardware_proof.ps1",
                "validation_mode=isolated",
                "AGENTS.md",
                ".codex/skills",
                "sha256sum",
                "CARGO_TARGET_DIR",
                "Investigation Circuit Breakers",
                "product defect",
                "After two failed remediation attempts",
                "After 30 minutes",
                "second user-assisted run requires",
                "SCENA_M9_TIMING_POLICY=report-only-hosted",
            ] as &[&str],
        ),
        (
            ".codex/skills/scena-remote-builder/SKILL.md",
            &[
                "scripts/scena_remote_builder_preflight.sh",
                "shared_checkout_status=missing",
                "validation_mode=isolated",
                "AGENTS.md",
                ".codex/skills",
                "sha256sum",
                "CARGO_TARGET_DIR",
            ],
        ),
        (
            "scripts/scena_remote_builder_preflight.sh",
            &[
                "shared_checkout_status=missing",
                "validation_mode=isolated",
                "validation_path=",
                "cargo_target_dir=",
                "df -hT",
            ],
        ),
    ] {
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(RULE, format!("could not read {relative}")));
            continue;
        };
        for token in required {
            if !text.contains(token) {
                let contract = if token.contains("Investigation Circuit Breakers") {
                    "investigation circuit breaker"
                } else {
                    token
                };
                findings.push(Finding::new(
                    RULE,
                    format!("{relative} is missing remote-builder contract {contract}"),
                ));
            }
        }
        if relative == "scripts/scena_remote_builder_preflight.sh" && text.contains("rm -rf") {
            findings.push(Finding::new(
                RULE,
                format!("{relative} must not delete caches or checkouts"),
            ));
        }
    }

    for (relative, required) in [
        (
            ".codex/skills/scena-renderer-quality/SKILL.md",
            &[
                "Investigation Circuit Breaker",
                "SCENA_M9_TIMING_POLICY=report-only-hosted",
                "scripts/run_windows_complete_hardware_proof.ps1",
                "second user-assisted run requires explicit approval",
            ] as &[&str],
        ),
        (
            ".codex/skills/scena-doctor/SKILL.md",
            &["investigation circuit breakers", "scena-<task-slug>"],
        ),
        (
            ".codex/skills/scena-release-hygiene/SKILL.md",
            &[
                "scripts/collect_ci_failure_evidence.sh",
                "investigation circuit breaker",
                "SCENA_M9_TIMING_POLICY=report-only-hosted",
            ],
        ),
        (
            ".codex/skills/scena-git-github/SKILL.md",
            &[
                "scripts/collect_ci_failure_evidence.sh",
                "investigation circuit breaker",
                "scena-<task-slug>",
            ],
        ),
        (
            ".codex/skills/scena-remote-builder/SKILL.md",
            &["investigation circuit breaker", "stop after two remedies"],
        ),
        (
            "scripts/collect_ci_failure_evidence.sh",
            &[
                "scena.ci_failure_evidence.v1",
                "failed-jobs.tsv",
                "classification_status",
                "root-cause-checkpoint.md",
                "gh run download",
            ],
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            &[
                "bundle-files.sha256",
                "Compress-Archive",
                "UploadUrl",
                "Invoke-WebRequest -UseBasicParsing -Method Put",
            ],
        ),
    ] {
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(RULE, format!("could not read {relative}")));
            continue;
        };
        for token in required {
            if !text.contains(token) {
                let contract = if token.contains("Investigation Circuit Breaker")
                    || token.contains("investigation circuit breaker")
                {
                    "investigation circuit breaker"
                } else if token.contains("second user-assisted run") {
                    "second user-assisted run"
                } else {
                    token
                };
                findings.push(Finding::new(
                    RULE,
                    format!("{relative} is missing operational safeguard {contract}"),
                ));
            }
        }
    }

    let obsolete_shared_checkout = ["$HOME", "/projects/scena"].concat();
    for relative in [
        ".codex/skills/scena-app-builder/SKILL.md",
        ".codex/skills/scena-renderer-quality/SKILL.md",
        ".codex/skills/scena-doctor/SKILL.md",
        ".codex/skills/scena-gltf-assets/SKILL.md",
        ".codex/skills/scena-release-hygiene/SKILL.md",
        ".codex/skills/scena-git-github/SKILL.md",
        ".codex/skills/scena-renderer-architecture/SKILL.md",
        ".codex/skills/scena-remote-builder/SKILL.md",
        ".codex/skills/scena-rfc-governance/SKILL.md",
    ] {
        if fs::read_to_string(root.join(relative))
            .is_ok_and(|text| text.contains(&obsolete_shared_checkout))
        {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{relative} contains obsolete shared checkout path {obsolete_shared_checkout}; use the task-scoped isolated validation path"
                ),
            ));
        }
    }
}
