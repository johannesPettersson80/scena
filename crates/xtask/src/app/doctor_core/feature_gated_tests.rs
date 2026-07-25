use crate::app::prelude::*;

/// `TESTS-FEATURE-GATED-CONTRACT-SUITES`: a whole-file feature gate on a
/// contract or agent CLI test suite makes default `cargo test` report
/// "running 0 tests". Keep the required feature-enabled cargo command in the
/// roadmap so reviews and CI don't mistake the default run for coverage.
pub(crate) fn check_feature_gated_contract_tests_documented(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    let roadmap_path = root.join("docs/checklists/application-builder-roadmap.md");
    let Ok(roadmap) = fs::read_to_string(&roadmap_path) else {
        findings.push(Finding::new(
            "TESTS-FEATURE-GATED-CONTRACT-SUITES",
            "docs/checklists/application-builder-roadmap.md must document feature-gated contract test commands",
        ));
        return;
    };
    let Ok(read_dir) = fs::read_dir(root.join("tests")) else {
        return;
    };

    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        entries.push(entry.path());
    }
    entries.sort();

    for path in entries {
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        if !is_feature_gated_contract_suite(&rel, &text) {
            continue;
        }
        let Some(feature) = file_level_feature_gate(&text) else {
            continue;
        };
        let Some(stem) = path.file_stem().and_then(OsStr::to_str) else {
            continue;
        };
        let command = format!("cargo test --features {feature} --test {stem}");
        if !roadmap.contains(&command) {
            findings.push(Finding::new(
                "TESTS-FEATURE-GATED-CONTRACT-SUITES",
                format!(
                    "{rel} is whole-file gated by feature '{feature}' and can run 0 tests under default cargo test; document `{command}` in docs/checklists/application-builder-roadmap.md"
                ),
            ));
        }
    }
}

fn is_feature_gated_contract_suite(rel: &str, text: &str) -> bool {
    file_level_feature_gate(text).is_some()
        && (rel.contains("contract")
            || rel.ends_with("tests/scena_cli_agent.rs")
            || rel.ends_with("tests/scena_cli_recipe.rs"))
}

fn file_level_feature_gate(text: &str) -> Option<String> {
    for line in text.lines().map(str::trim).take(8) {
        let Some(rest) = line.strip_prefix("#![cfg(feature = \"") else {
            continue;
        };
        let Some(end) = rest.find('"') else {
            continue;
        };
        let feature = &rest[..end];
        if !feature.is_empty() {
            return Some(feature.to_string());
        }
    }
    None
}

/// `TESTS-FEATURE-GATED-WORKFLOW-BIJECTION`: every crate-level feature-gated
/// integration binary must be executed by a named workflow command.
///
/// Because `default = []`, such a binary contributes **zero** tests to a
/// default `cargo test`. Documenting the command in a checklist — which
/// `TESTS-FEATURE-GATED-CONTRACT-SUITES` checks — proves only that prose
/// exists, not that CI runs it. A v1.9.0 audit found 29 gated binaries no
/// workflow named, including `capture_contracts` and the agent-mode A03-A05
/// smokes.
pub(crate) fn check_feature_gated_tests_run_in_a_workflow(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    let workflows = read_workflow_text(root);
    // A blanket lane that enables every feature and builds every test target
    // covers all gated binaries at once; it is the preferred form.
    let has_blanket_lane = workflows.contains("--all-features --tests")
        || workflows.contains("--all-features --all-targets");

    let Ok(read_dir) = fs::read_dir(root.join("tests")) else {
        return;
    };
    let mut entries = read_dir
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.extension().and_then(OsStr::to_str) != Some("rs") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if crate_level_cargo_feature_gate(&text).is_none() {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(OsStr::to_str) else {
            continue;
        };
        if has_blanket_lane || workflows.contains(&format!("--test {stem}")) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        findings.push(Finding::new(
            "TESTS-FEATURE-GATED-WORKFLOW-BIJECTION",
            format!(
                "{rel} is crate-level gated behind a cargo feature and runs 0 tests under default `cargo test`, but no workflow in .github/workflows/ executes it; add it to a feature-contract lane"
            ),
        ));
    }
}

fn read_workflow_text(root: &Path) -> String {
    let mut combined = String::new();
    let Ok(read_dir) = fs::read_dir(root.join(".github/workflows")) else {
        return combined;
    };
    let mut paths = read_dir
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if let Ok(text) = fs::read_to_string(&path) {
            combined.push_str(&text);
            combined.push('\n');
        }
    }
    combined
}

/// A crate-level `#![cfg(...)]` that requires at least one cargo feature.
///
/// Gates such as `not(target_arch = "wasm32")` are always true on a Linux
/// runner, so those binaries do run by default and are not orphans.
fn crate_level_cargo_feature_gate(text: &str) -> Option<String> {
    for line in text.lines().map(str::trim) {
        if !line.starts_with("#![cfg(") {
            continue;
        }
        if let Some(rest) = line.split("feature = \"").nth(1)
            && let Some(end) = rest.find('"')
            && !rest[..end].is_empty()
        {
            return Some(rest[..end].to_owned());
        }
        return None;
    }
    None
}
