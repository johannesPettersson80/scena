use crate::app::prelude::*;

pub(crate) fn contains_scope_term(lower_text: &str, term: &str) -> bool {
    if term.contains(' ') {
        return lower_text.contains(term);
    }

    lower_text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| token == term)
}

pub(crate) fn check_unit_test_first_governance(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "TEST-FIRST-AGENTS",
        "AGENTS.md",
        &[
            "## Unit Test First Rule",
            "Run the focused test and confirm it fails for the expected reason",
            "Do not mark a checklist implementation item complete without naming the test-first proof",
        ],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-SKILL-QUALITY",
        ".codex/skills/scena-renderer-quality/SKILL.md",
        &[
            "## Unit Test First Workflow",
            "Run the focused test and verify the failure is the expected failure",
        ],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-SKILL-ARCH",
        ".codex/skills/scena-renderer-architecture/SKILL.md",
        &["Before production implementation"],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-DOCTOR-CONTRACT",
        "docs/specs/doctor-contract.md",
        &["unit-test-first governance"],
    );

    for rel in MILESTONE_CHECKLISTS {
        require_contains(
            root,
            findings,
            "TEST-FIRST-CHECKLIST",
            rel,
            &["Unit-test-first evidence"],
        );
    }
}

pub(crate) const MILESTONE_CHECKLISTS: &[&str] = &[
    "docs/checklists/m0-foundation.md",
    "docs/checklists/m1-geometry-materials.md",
    "docs/checklists/m2-lighting-depth-clipping.md",
    "docs/checklists/m3a-app-features.md",
    "docs/checklists/m3b-gltf-animation.md",
    "docs/checklists/m4-performance-platform.md",
    "docs/checklists/m5-v1-release.md",
    "docs/checklists/acceptance-index.md",
];

pub(crate) fn check_backend_vocabulary(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/platform.rs",
        &["browser_webgpu_canvas", "browser_webgl2_canvas"],
    );
    require_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/diagnostics/capabilities.rs",
        &["WebGpu", "WebGl2"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/diagnostics.rs",
        &["BrowserSurface"],
    );
}

pub(crate) fn check_agent_validation(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "AGENTS-VALIDATION",
        "AGENTS.md",
        &["cargo run -p xtask -- doctor --full", "Use `scena-doctor`"],
    );
    require_contains(
        root,
        findings,
        "SKILL-DOCTOR",
        ".codex/skills/scena-doctor/SKILL.md",
        &["cargo run -p xtask -- doctor --full"],
    );
}
