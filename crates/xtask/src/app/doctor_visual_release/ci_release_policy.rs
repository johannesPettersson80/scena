use crate::app::prelude::*;

pub(super) const HOSTED_M9_TIMING_POLICY_ENV: &str = "SCENA_M9_TIMING_POLICY=report-only-hosted";

pub(super) fn check_hosted_m9_timing_policy(root: &Path, findings: &mut Vec<Finding>) {
    for relative in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(
                "RELEASE-CI-M9",
                format!("could not read {relative}"),
            ));
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.contains("SCENA_RUN_M9_PLATFORM_BENCHMARK=1")
                && !line.contains(HOSTED_M9_TIMING_POLICY_ENV)
            {
                findings.push(Finding::new(
                    "RELEASE-CI-M9",
                    format!(
                        "{relative}:{} must set {HOSTED_M9_TIMING_POLICY_ENV} on the same GitHub-hosted M9 benchmark command",
                        index + 1
                    ),
                ));
            }
        }
        if text.contains("SCENA_RUN_DEDICATED_4K_BENCHMARK: \"1\"")
            && !text.contains("SCENA_M9_TIMING_POLICY: report-only-hosted")
        {
            findings.push(Finding::new(
                "RELEASE-CI-M9",
                format!(
                    "{relative} dedicated hosted benchmark must set SCENA_M9_TIMING_POLICY=report-only-hosted"
                ),
            ));
        }
    }
}

pub(super) fn check_agent_template_cli_isolation(root: &Path, findings: &mut Vec<Finding>) {
    let relative = "tests/scena_cli_agent_templates.rs";
    let Ok(text) = fs::read_to_string(root.join(relative)) else {
        findings.push(Finding::new(
            "RELEASE-CI-M9",
            format!("could not read {relative}"),
        ));
        return;
    };
    let guard_count = text
        .matches("let _cli_guard = template_cli_guard();")
        .count();
    if !text.contains("static TEMPLATE_CLI_LOCK") || guard_count != 4 {
        findings.push(Finding::new(
            "RELEASE-CI-M9",
            format!(
                "{relative} must use TEMPLATE_CLI_LOCK in all four heavyweight subprocess tests; found {guard_count} guards"
            ),
        ));
    }
}

pub(super) fn check_m9_platform_benchmark_isolation(root: &Path, findings: &mut Vec<Finding>) {
    let relative = "tests/m9_platform_release.rs";
    let Ok(source) = fs::read_to_string(root.join(relative)) else {
        return;
    };
    let Some(start) = source.find("fn m9_platform_rendered_output_suite_writes_release_artifacts")
    else {
        return;
    };
    let rendered_output_test = &source[start..];
    let end = rendered_output_test
        .find("\n#[test]")
        .unwrap_or(rendered_output_test.len());
    if rendered_output_test[..end].contains("write_benchmark_artifact") {
        findings.push(Finding::new(
            "RELEASE-CI-M9",
            "tests/m9_platform_release.rs must not measure performance inside the broad parallel rendered-output test; run the environment-gated exact benchmark with --test-threads=1",
        ));
    }
}
