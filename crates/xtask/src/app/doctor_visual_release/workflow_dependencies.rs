use crate::app::prelude::*;

pub(crate) fn check_workflow_action_pins(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "WORKFLOW-ACTION-PIN",
        ".github/dependabot.yml",
        &[
            "version: 2",
            "package-ecosystem: \"github-actions\"",
            "directory: \"/\"",
            "interval: \"weekly\"",
        ],
    );
    let workflows = root.join(".github/workflows");
    let entries = match fs::read_dir(&workflows) {
        Ok(entries) => entries,
        Err(_) => {
            findings.push(Finding::new(
                "WORKFLOW-ACTION-PIN",
                ".github/workflows must exist for immutable action-reference validation"
                    .to_string(),
            ));
            return;
        }
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(OsStr::to_str),
                Some("yml" | "yaml")
            )
        })
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        let relative = path.strip_prefix(root).unwrap_or(&path).display();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                findings.push(Finding::new(
                    "WORKFLOW-ACTION-PIN",
                    format!("could not read {relative} for action pin validation: {error}"),
                ));
                continue;
            }
        };
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
            let Some(raw) = trimmed.strip_prefix("uses:") else {
                continue;
            };
            validate_action_reference(relative.to_string(), index + 1, raw, findings);
        }
    }
}

fn validate_action_reference(path: String, line: usize, raw: &str, findings: &mut Vec<Finding>) {
    let (reference, comment) = raw
        .split_once('#')
        .map_or((raw, None), |(reference, comment)| {
            (reference, Some(comment.trim()))
        });
    let reference = reference.trim().trim_matches(['\'', '"']);
    if reference.starts_with("./") || reference.starts_with("docker://") {
        return;
    }
    let Some((action, revision)) = reference.rsplit_once('@') else {
        findings.push(Finding::new(
            "WORKFLOW-ACTION-PIN",
            format!("{path}:{line} action reference {reference:?} is missing @revision"),
        ));
        return;
    };
    if action.is_empty()
        || revision.len() != 40
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        findings.push(Finding::new(
            "WORKFLOW-ACTION-PIN",
            format!(
                "{path}:{line} action reference {reference} must use an immutable lowercase 40-hex commit"
            ),
        ));
        return;
    }
    let version = comment
        .and_then(|comment| comment.split_whitespace().next())
        .unwrap_or("");
    let version = version.strip_prefix('v').unwrap_or(version);
    if version.is_empty()
        || !version
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
        || !version.contains('.')
    {
        findings.push(Finding::new(
            "WORKFLOW-ACTION-PIN",
            format!(
                "{path}:{line} immutable action reference {reference} needs a release version comment such as '# v4.3.1'"
            ),
        ));
    }
}
