use crate::app::prelude::*;

pub(crate) fn check_fr07_recipe_diff_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "FR07-RECIPE-DIFF";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene/recipe/diff.rs",
            &[
                "scena.scene_recipe_diff.v1",
                "SceneRecipeDiffScopeV1",
                "Recipe,",
                "Material,",
                "Node,",
                "Camera,",
                "Added,",
                "Removed,",
                "Modified,",
                "Reordered,",
                "numeric_tolerance",
                "diff_scene_recipes",
            ],
        ),
        (
            "src/bin/scena/diff.rs",
            &[
                "scena.scene_recipe_diff_result.v1",
                "scena::diff_scene_recipes",
                "scena::compare_captures_with_tolerance",
                "capture_semantic_aovs",
                "execution_report(0)",
                "--render requires --out-dir",
                "recipe-diff-result.json",
            ],
        ),
        (
            "src/bin/scena/diff/attribution.rs",
            &[
                "same_persistent_identity",
                "persistent_identity_added",
                "persistent_identity_removed",
                "semantic_identity_edge",
                "background_or_excluded_surface",
                "different_persistent_identity_candidates",
                "excluded_surface_present_without_pixel_mask",
                "has_unresolved_exclusions",
                "\"anti_aliased_edges\": \"ambiguous\"",
                "\"transparent_and_excluded_surfaces\": \"unattributed_or_ambiguous\"",
                "not_claimed",
            ],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "diff <before.recipe.json> <after.recipe.json>",
                "scena.scene_recipe_diff_result.v1",
            ],
        ),
        (
            "src/schema_catalog.rs",
            &[
                "scena.scene_recipe_diff_result.v1",
                "tests/assets/stable-contracts/scene_recipe_diff_result.v1.json",
            ],
        ),
        (
            "src/schema_catalog/fixtures.rs",
            &["scena.scene_recipe_diff_result.v1"],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "### `scena.scene_recipe_diff_result.v1`",
                "anti-aliased identity edges are ambiguous",
                "No competitive uniqueness claim",
            ],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "scena diff",
                "attributed_pixels + ambiguous_pixels + unattributed_pixels",
            ],
        ),
        (
            "tests/assets/stable-contracts/scene_recipe_diff_result.v1.json",
            &[
                "scena.scene_recipe_diff_result.v1",
                "attributed_pixels",
                "ambiguous_pixels",
                "unattributed_pixels",
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
        "tests/fr07_recipe_diff.rs",
        &[
            "fr07_typed_recipe_diff_reports_identity_fields_tolerance_and_order",
            "fr07_diff_cli_keeps_structural_diff_renderer_free",
            "fr07_rendered_diff_reuses_aggregate_diff_and_attributes_only_supported_pixels",
            "fr07_diff_cli_emits_declared_validation_and_build_failure_schemas",
            "fr07_rendered_diff_never_assigns_excluded_transparency_to_the_opaque_node_behind_it",
        ],
    );
}
