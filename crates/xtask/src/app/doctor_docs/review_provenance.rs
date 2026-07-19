use crate::app::prelude::*;

const RULE: &str = "D04-REVIEW-PROVENANCE";
const REVIEW: &str = "docs/reviews/full-repo-review-v1.7.2.md";

pub(crate) fn check_review_provenance_contracts(root: &Path, findings: &mut Vec<Finding>) {
    let Ok(review) = fs::read_to_string(root.join(REVIEW)) else {
        findings.push(Finding::new(RULE, format!("could not read {REVIEW}")));
        return;
    };
    let normalized = review
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    for required in [
        "main@bea2a36",
        "cargo package version is 1.7.2",
        "14 commits after tag",
        "not a review of the tagged release",
        "schema_entry_rows()",
        "additional versioned schema literals",
        "universal uniqueness claim",
        "review-pass",
        "independent reviewers",
    ] {
        if !normalized.contains(required) {
            findings.push(Finding::new(
                RULE,
                format!("{REVIEW} is missing provenance correction {required}"),
            ));
        }
    }
    for forbidden in [
        "# review: full repo at v1.7.2",
        "method: eight",
        "no competitor has",
        "nothing like it exists anywhere",
        "single most common real-world asset blocker",
        "every serious cad viewer",
        "45 versioned json contracts",
        "verified live:",
    ] {
        if normalized.contains(forbidden) {
            findings.push(Finding::new(
                RULE,
                format!("{REVIEW} retains unsupported claim {forbidden}"),
            ));
        }
    }
}
