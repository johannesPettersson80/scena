use crate::app::prelude::*;
pub(crate) fn check_state_of_art_checklist_links(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CHECKLIST-STATE-OF-ART",
        "docs/checklists/acceptance-index.md",
        &[
            "state-of-art-threejs-replacement-plan.md",
            "State Of The Art Three.js Replacement Plan",
        ],
    );
    require_contains(
        root,
        findings,
        "CHECKLIST-STATE-OF-ART",
        "docs/checklists/m10-threejs-replacement-acceptance.md",
        &[
            "state-of-art-threejs-replacement-plan.md",
            "State Of The Art Three.js Replacement Plan",
        ],
    );
}
