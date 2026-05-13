use crate::app::prelude::*;

pub(crate) fn check_architecture_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CONTRACT",
        "docs/specs/architecture-contract.md",
        &[
            "Every production feature must have exactly one owner module",
            "Dependency Direction",
            "Render Lifecycle",
            "Public API Ownership",
            "SOLID/KISS Gate",
            "Architecture Evidence",
            "Exceptions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CONTRACT",
        "docs/checklists/architecture-perfection-checklist.md",
        &[
            "Architecture Perfection Checklist",
            "Dependency Direction",
            "Public API Ownership",
            "Tooling Architecture",
            "`crates/xtask/src/main.rs` is physically split into focused modules",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CONTRACT",
        "docs/RFC-rust-3d-renderer.md",
        &[
            "docs/specs/architecture-contract.md",
            "cargo run -p xtask -- architecture-map",
            "cargo run -p xtask -- doctor --architecture",
        ],
    );
}
