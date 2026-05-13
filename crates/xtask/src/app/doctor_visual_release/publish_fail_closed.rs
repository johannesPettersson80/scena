use crate::app::prelude::*;

pub(crate) fn check_release_publish_dry_run_helper(root: &Path, findings: &mut Vec<Finding>) {
    // RELEASE-PUBLISH-DRY-RUN-RECORD: Phase 8 closure depends on
    // scripts/release_publish_dry_run.sh producing a clean-tree publish
    // dry-run log under target/gate-artifacts/release-lanes/publish-dry-run.log.
    // The helper must fail-closed on its own bash machinery (set -euo
    // pipefail) so a failed git rev-parse / git worktree / tee is not
    // silently ignored before any run_step is reached. Closes
    // scena-doctor-reviewer 4b0e621 finding N1.
    require_contains(
        root,
        findings,
        "RELEASE-PUBLISH-DRY-RUN-RECORD",
        "scripts/release_publish_dry_run.sh",
        &[
            "set -euo pipefail",
            "cargo publish --dry-run",
            "publish-dry-run.log",
            "git worktree add --detach",
            "git worktree remove --force",
        ],
    );
}

pub(crate) fn check_release_readiness_ci_fail_closed(root: &Path, findings: &mut Vec<Finding>) {
    // RELEASE-READINESS-CI-FAIL-CLOSED: any GHA workflow job that runs
    // `cargo run -p xtask -- release-readiness` must fail closed (no
    // `continue-on-error: true`) once ADR-0005 moves out of `Status: Accepted`.
    // While ADR-0005 is still in `Status: Accepted`, the rule recognizes the
    // documented transitional exception and only flags configurations that
    // would mask release-readiness on the publish-tag path.
    let adr_active = match fs::read_to_string(
        root.join("docs/decisions/ADR-0005-local-release-candidate-deferrals.md"),
    ) {
        Ok(text) => text.contains("Status: Accepted"),
        Err(_) => false,
    };
    for workflow_rel in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let path = root.join(workflow_rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for offending in jobs_with_continue_on_error_release_readiness(&text) {
            if adr_active && offending.starts_with("premerge-release-readiness") {
                continue;
            }
            findings.push(Finding::new(
                "RELEASE-READINESS-CI-FAIL-CLOSED",
                format!(
                    "{workflow_rel} job '{offending}' runs release-readiness with \
                     continue-on-error: true; the gate must fail closed once \
                     ADR-0005 is superseded"
                ),
            ));
        }
    }
}

pub(crate) fn jobs_with_continue_on_error_release_readiness(text: &str) -> Vec<String> {
    let mut offending = Vec::new();
    let mut current_job: Option<String> = None;
    let mut continue_on_error = false;
    let mut runs_release_readiness = false;
    for raw in text.lines() {
        let leading_whitespace = raw.len() - raw.trim_start().len();
        let trimmed = raw.trim();
        // Top-level job heading is two-space indented (`  <name>:`) inside the
        // workflow's `jobs:` block. We treat any header at column 2 as a new job.
        if leading_whitespace == 2 && trimmed.ends_with(':') && !trimmed.contains(' ') {
            if let Some(job) = current_job.take()
                && continue_on_error
                && runs_release_readiness
            {
                offending.push(job);
            }
            current_job = Some(trimmed.trim_end_matches(':').to_string());
            continue_on_error = false;
            runs_release_readiness = false;
            continue;
        }
        if current_job.is_some() {
            if trimmed.contains("continue-on-error: true") {
                continue_on_error = true;
            }
            if trimmed.contains("cargo run -p xtask -- release-readiness") {
                runs_release_readiness = true;
            }
        }
    }
    if let Some(job) = current_job
        && continue_on_error
        && runs_release_readiness
    {
        offending.push(job);
    }
    offending
}
