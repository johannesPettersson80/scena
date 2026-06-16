use crate::app::prelude::*;

pub(super) fn check_picking_outline_hover(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "src/picking.rs",
        &[
            "pub struct InteractionContext",
            "pub fn set_hover(",
            "pub fn set_primary_selection(",
        ],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "src/scene/picking.rs",
        &[
            "pub fn pick_and_select_with_assets",
            "pub fn pick_and_hover_with_assets",
            "pub fn set_hover_target",
            "pub fn set_primary_selection_target",
        ],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "tests/examples_visual_proof.rs",
        &[
            "examples_visual_picking_selection_hover_renders_pick_state_to_ppm",
            "picking_selection_hover",
            "pick_and_select_with_assets",
        ],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "docs/guides/easy-scene-setup.md",
        &[
            "selection or hover state updates",
            "viewer.on_hover",
            "viewer.hover_at",
        ],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "Picking + hover + selection",
            "Status: **[shipped]**",
            "PICKING-OUTLINE-HOVER",
            "examples_visual_picking_selection_hover_renders_pick_state_to_ppm",
        ],
    );
}
