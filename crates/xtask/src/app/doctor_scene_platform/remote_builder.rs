use crate::app::prelude::*;

const RULE: &str = "O01-REMOTE-BUILDER-BOOTSTRAP";

pub(crate) fn check_remote_builder_bootstrap_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, required) in [
        (
            "AGENTS.md",
            &[
                "scripts/scena_remote_builder_preflight.sh",
                "validation_mode=isolated",
                "AGENTS.md",
                ".codex/skills",
                "sha256sum",
                "CARGO_TARGET_DIR",
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
                findings.push(Finding::new(
                    RULE,
                    format!("{relative} is missing remote-builder contract {token}"),
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
}
