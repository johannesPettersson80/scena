use crate::app::prelude::*;

pub(crate) fn check_fr08_recipe_spatial_state_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "FR08-RECIPE-SPATIAL-STATE";
    let required: &[(&str, &[&str])] = &[
        (
            "docs/specs/recipe-spatial-state-v1.md",
            &[
                "persistent feature ID",
                "scene meters",
                "Scene::connect_by_key",
                "Authored bounds cannot replace geometry- or asset-owned bounds",
                "single inheritance",
                "V1 rejects transform entries that",
            ],
        ),
        (
            "src/scene/recipe/types/spatial_state.rs",
            &[
                "SceneRecipeSpatialTargetV1",
                "ImportRoot",
                "ImportNode",
                "SceneRecipeAnchorSourceV1",
                "SceneRecipeConnectorSourceV1",
                "SceneRecipeBoundsSourceV1",
                "SceneRecipeNamedStateV1",
                "deny_unknown_fields",
            ],
        ),
        (
            "src/scene/recipe/validation/spatial_state.rs",
            &["unknown_spatial_target", "invalid_authored_bounds"],
        ),
        (
            "src/scene/recipe/validation/spatial_state/states.rs",
            &[
                "state_inheritance_cycle",
                "animated_state_transform_conflict",
                "multiple_active_named_states",
            ],
        ),
        (
            "src/scene_host/recipe/spatial_state.rs",
            &[
                "persistent_recipe_id",
                "scene_meters",
                "set_authored_node_bounds",
            ],
        ),
        (
            "src/scene_host/recipe/spatial_state/connectors.rs",
            &["persistent_recipe_id", "connector_mate_failed"],
        ),
        (
            "src/scene_host/recipe/spatial_state/states.rs",
            &[
                "persistent_recipe_id",
                "store_visual_state",
                "apply_visual_state",
            ],
        ),
        (
            "src/scene/bounds.rs",
            &[
                "set_authored_node_bounds",
                "NodeKind::Empty",
                "authored bounds cannot override geometry- or asset-owned bounds",
            ],
        ),
        (
            "src/scene/recipe/field_model.rs",
            &["anchors", "connectors", "bounds", "named_states"],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "docs/specs/recipe-spatial-state-v1.md",
                "persistent feature id",
                "Connection rows report",
                "Named-state rows report",
            ],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "never persist the numeric handles",
                "import_node",
                "scene meters",
            ],
        ),
        (
            "tests/fr08_recipe_spatial_state.rs",
            &[
                "target/gate-artifacts/fr08-recipe-spatial-state",
                "scena.fr08_recipe_spatial_state_proof.v1",
                "animated_state_transform_conflict",
                "connector_mate_failed",
                "authored_bounds_override",
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
        "tests/fr08_recipe_spatial_state.rs",
        &[
            "fr08_recipe_spatial_sections_validate_and_round_trip_without_unsupported_fallback",
            "fr08_build_maps_every_owner_applies_mate_and_active_state_and_changes_pixels",
            "fr08_import_aliases_preserve_source_metadata_and_exact_identity",
            "fr08_all_spatial_target_kinds_round_trip",
            "fr08_spatial_and_state_failures_are_structured_and_atomic",
        ],
    );
}
