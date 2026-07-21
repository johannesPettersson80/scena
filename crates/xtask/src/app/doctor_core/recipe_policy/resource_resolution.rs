use crate::app::prelude::*;

pub(crate) fn check_a01_recipe_resource_resolution(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A01-RECIPE-RESOURCE-RESOLUTION";

    require_contains(
        root,
        findings,
        RULE,
        "src/scene/recipe/build/resource_plan.rs",
        &[
            "pub(crate) struct RecipeResourcePlan",
            "resolve_recipe_resources",
            "RecipeResourceRole::Import(index)",
            "RecipeResourceRole::Environment",
            "RecipeResourceRole::Font(index)",
            "RecipeResourceRole::Texture",
            "RecipeResourceRole::BuiltinEnvironment",
            "best_effort_normalized_uri",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/recipe_validation.rs",
        &[
            "RecipeValidationModeV1::FullResolution",
            "policy.resolve_recipe_resources(recipe_path, recipe)",
            "validate_environment_source_with_options",
            "LabelFontFace::from_truetype_bytes",
            ".load_texture(",
            "SceneRecipeResourceStatusV1::LoadFailed",
            "SceneRecipeDiagnosticResourceV1",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene_host/recipe.rs",
        &[
            "policy.resolve_recipe_resources(recipe_path, &recipe)",
            ".resolved_uri(&resource_path)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/args.rs",
        &[
            "\"--syntax-only\"",
            "\"--full\"",
            "pub(crate) syntax_only: bool",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a01_recipe_resolution.rs",
        &[
            "full_validation_resolves_every_authored_resource_family",
            "syntax_only_is_explicit_and_does_not_claim_execution_equivalence",
            "full_validation_checks_nested_gltf_dependencies_and_accepts_builtins",
            "full_validation_success_uses_the_same_plan_as_recipe_build",
        ],
    );
    for (path, needles) in [
        (
            "tests/assets/stable-contracts/scene_recipe_validation.v1.json",
            &[
                "\"validation_mode\": \"syntax_only\"",
                "\"execution_equivalent\": false",
            ][..],
        ),
        (
            "README.md",
            &["`validate-recipe` defaults to full resolution"][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "resolves the same resource plan",
                "`--syntax-only` is the explicit no-I/O alternative",
            ][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &["scena validate-recipe \"$RECIPE\" --full"][..],
        ),
        (
            "CHANGELOG.md",
            &["same complete resource-resolution plan", "as recipe build"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
