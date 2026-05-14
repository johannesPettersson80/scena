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
    // `cargo run -p xtask -- release-readiness` must not use
    // `continue-on-error: true`. Pre-merge CI may turn the command into an
    // explicit blocker report while ADR-0005 is Accepted, but GitHub must not
    // leave a permanently red "allowed failure" job in the normal push UI.
    // The publish-tag path remains hard fail-closed through release.yml.
    for workflow_rel in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let path = root.join(workflow_rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for offending in jobs_with_continue_on_error_release_readiness(&text) {
            findings.push(Finding::new(
                "RELEASE-READINESS-CI-FAIL-CLOSED",
                format!(
                    "{workflow_rel} job '{offending}' runs release-readiness with \
                     continue-on-error: true; use an explicit report step for \
                     ADR-0005 pre-merge blockers and keep release.yml fail-closed"
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
