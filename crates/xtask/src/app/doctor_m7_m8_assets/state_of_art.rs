use crate::app::prelude::*;
pub(crate) fn check_state_of_art_checklist_links(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-PUBLIC-INDEX",
        "docs/README.md",
        &["API overview", "Rendering", "Assets", "Troubleshooting"],
    );
}
