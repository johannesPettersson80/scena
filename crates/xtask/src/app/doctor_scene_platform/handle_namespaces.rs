use crate::app::prelude::*;

pub(crate) fn check_c07_handle_namespace_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "SCENE-C07";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene_host/handles.rs",
            &[
                "const SLOT_BITS: u32 = 28",
                "const GENERATION_BITS: u32 = 21",
                "const KIND_SHIFT: u32 = SLOT_BITS + GENERATION_BITS",
                "pub(super) enum HandleKind",
                "Node = 1",
                "Import = 2",
                "InstanceRoot = 3",
                "Animation = 4",
                "kind: HandleKind",
                "SceneHostErrorCode::WrongHandleNamespace",
                "slot.retired = true",
                "handle > ((1_u64 << 53) - 1)",
                "every_live_handle_kind_is_rejected_by_every_other_table",
                "every_namespace_reuses_slots_with_a_new_generation_and_rejects_old_handles",
                "exhausted_high_generation_slots_retire_instead_of_repeating_a_handle",
            ],
        ),
        (
            "src/scene_host/core.rs",
            &[
                "HandleTable::new(HandleKind::Node)",
                "HandleTable::new(HandleKind::Import)",
                "HandleTable::new(HandleKind::InstanceRoot)",
                "HandleTable::new(HandleKind::Animation)",
                "self.invalidate_stale_animation_handles()",
            ],
        ),
        (
            "src/scene_host/core_handles.rs",
            &["handle_kind(handle) == Some(HandleKind::InstanceRoot)"],
        ),
        ("src/scene_host/error.rs", &["WrongHandleNamespace"]),
        (
            "docs/errors.md",
            &[
                "SceneHostErrorCode::WrongHandleNamespace",
                "explicit node, import, instance-root, or animation",
                "exact JavaScript integer range",
                "generation space is",
                "exhausted is retired",
            ],
        ),
        (
            "tests/browser/scene_host_browser_proof.js",
            &["addProductGridFloorUnderNode(leftFrameHandle)"],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c07_handle_namespaces.rs",
        &[
            "import_handle_cannot_mutate_the_first_node_slot",
            "every_public_handle_kind_is_distinct_and_wrong_resolvers_are_non_mutating",
            "every_public_namespace_reuses_slots_without_reviving_stale_handles",
        ],
    );

    for (relative, forbidden) in [
        ("src/scene_host/handles.rs", "with_generation_base"),
        ("src/scene_host/core.rs", "HandleTable::new()"),
        ("src/scene_host/core.rs", "with_generation_base"),
        ("src/scene_host/core.rs", "HANDLE_GENERATION_BASE"),
        ("src/scene_host/core_handles.rs", "handle / (1_u64 << 32)"),
        (
            "src/scene_host/core_handles.rs",
            "INSTANCE_HANDLE_GENERATION_BASE",
        ),
        (
            "src/scene_host/instances.rs",
            "INSTANCE_HANDLE_GENERATION_BASE",
        ),
        (
            "tests/browser/scene_host_browser_proof.js",
            "addProductGridFloorUnderNode(handleBigInt(leftImportReport.import))",
        ),
    ] {
        if let Ok(source) = fs::read_to_string(root.join(relative))
            && source.contains(forbidden)
        {
            findings.push(Finding::new(
                RULE,
                format!("{relative} contains forbidden untagged handle convention `{forbidden}`"),
            ));
        }
    }
}
