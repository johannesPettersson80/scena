use crate::app::prelude::*;

pub(crate) fn check_c10_overlay_ownership_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "SCENE-C10";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene.rs",
            &[
                "mod overlay_ownership;",
                "overlay_owners: BTreeMap<NodeKey, overlay_ownership::OverlayOwner>",
            ],
        ),
        (
            "src/scene/overlay_ownership.rs",
            &[
                "pub(super) enum OverlayOwner",
                "Callout(String)",
                "Measurement(String)",
                "expand_overlay_removal_closure",
                "assert_overlay_ownership_invariant",
                "generated overlay nodes and owner registry differ",
            ],
        ),
        (
            "src/scene/removal.rs",
            &[
                "let removed = self.node_removal_closure(node)?;",
                "let mut transaction = SceneTransaction::new(self);",
                "self.expand_overlay_removal_closure(&mut removed);",
                "self.annotations.remove(&id);",
                "self.overlay_owners",
            ],
        ),
        (
            "src/scene/callouts.rs",
            &[
                "OverlayOwner::Callout",
                "register_overlay_node(line_node",
                "register_overlay_node(label_node",
                "unregister_overlay_node(node)",
                "assert_overlay_ownership_invariant",
            ],
        ),
        (
            "src/scene/measurements.rs",
            &[
                "OverlayOwner::Measurement",
                "register_overlay_node(line_node",
                "register_overlay_node(label_node",
                "unregister_overlay_node(node)",
                "assert_overlay_ownership_invariant",
            ],
        ),
        (
            "src/scene_host/core.rs",
            &["let removed = self.scene.node_removal_closure(node_key)?;"],
        ),
        (
            "docs/api.md",
            &[
                "Generated overlay ownership invariant",
                "complete owned overlay closure",
                "StaleNodeHandle",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c10_overlay_ownership.rs",
        &[
            "removing_either_measurement_child_removes_the_complete_overlay",
            "removing_either_callout_child_closes_node_and_world_owned_state",
            "removing_either_callout_handle_invalidates_the_complete_owned_closure",
            "removing_either_measurement_handle_invalidates_the_complete_owned_closure",
        ],
    );
}
