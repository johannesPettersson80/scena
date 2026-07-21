use crate::app::prelude::*;

pub(crate) fn check_document_governance_truth(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "D05-D06-DOCUMENT-GOVERNANCE";

    for (path, needles) in [
        (
            "docs/README.md",
            vec![
                "Active open backlog",
                "full-repo-review-v1.8.0-remediation.md",
                "Historical evidence",
            ],
        ),
        (
            "docs/RFC-rust-3d-renderer.md",
            vec![
                "One active implementation backlog",
                "full-repo-review-v1.8.0-remediation.md",
                "Historical evidence tracks",
                "`SceneRecipeV1` is a versioned interchange/build input",
                "host owns application persistence",
                "no cross-version lossless round-trip guarantee",
            ],
        ),
        (
            "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
            vec![
                "Status: archived historical evidence",
                "full-repo-review-v1.8.0-remediation.md",
                "Shipped scoped feature rows remain valid evidence",
                "F01-F08",
            ],
        ),
        (
            "docs/checklists/application-builder-roadmap.md",
            vec![
                "Status: completed historical evidence",
                "full-repo-review-v1.8.0-remediation.md",
            ],
        ),
        (
            "docs/checklists/wasm-scene-host-and-stable-contracts.md",
            vec![
                "Status: archived historical evidence",
                "full-repo-review-v1.8.0-remediation.md",
            ],
        ),
        (
            "docs/checklists/renderer-fidelity-dependencies.md",
            vec![
                "Status: archived historical evidence",
                "full-repo-review-v1.8.0-remediation.md",
            ],
        ),
        (
            "README.md",
            vec!["recipe-local stable IDs", "not application-persistence IDs"],
        ),
        (
            "docs/schema-contracts.md",
            vec![
                "versioned interchange/build input",
                "not the canonical persisted application document",
                "same-version canonical output",
                "unknown top-level fields",
                "host owns migrations",
            ],
        ),
    ] {
        require_contains(root, findings, RULE, path, &needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/tests_72.rs",
        &["d05_d06_doctor_rejects_multiple_active_backlogs_and_persistence_overclaim"],
    );
}

pub(crate) fn check_state_of_art_checklist_links(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-PUBLIC-INDEX",
        "docs/README.md",
        &["API overview", "Rendering", "Assets", "Troubleshooting"],
    );
    check_document_governance_truth(root, findings);
}
