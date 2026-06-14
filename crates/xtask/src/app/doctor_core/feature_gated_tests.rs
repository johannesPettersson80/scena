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
