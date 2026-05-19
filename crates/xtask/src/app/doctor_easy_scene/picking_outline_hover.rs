use crate::app::prelude::*;

pub(super) fn check_picking_outline_hover(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "src/picking.rs",
        &[
            "pub struct InteractionStyle",
            "pub const fn outline(",
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
        "src/render/settings.rs",
        &["pub fn set_hover_style(", "pub fn set_selection_style("],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "tests/examples_visual_proof.rs",
        &[
            "examples_visual_picking_selection_hover_renders_styled_pick_to_ppm",
            "picking_selection_hover",
            "InteractionStyle::outline",
        ],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "docs/guides/easy-scene-setup.md",
        &[
            "InteractionStyle::outline",
            "renderer.set_hover_style",
            "renderer.set_selection_style",
        ],
    );
    require_contains(
        root,
        findings,
        "PICKING-OUTLINE-HOVER",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "Picking + outline + hover",
            "Status: **[shipped]**",
            "PICKING-OUTLINE-HOVER",
            "examples_visual_picking_selection_hover_renders_styled_pick_to_ppm",
        ],
    );
}
