use crate::app::prelude::*;

pub(crate) fn check_c21_cad_inspection_lighting(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C21-CAD-INSPECTION-LIGHTING";

    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/recipe/cad_inspection/view.rs",
        &[
            "\"kind\": \"studio_rig\"",
            "\"preset\": \"studio_rig\"",
            "\"exposure_ev\": 0.25",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/scena_cli_recipe.rs",
        &["scena_recipe_inspect_cad_generates_reviewable_feature_views"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "docs/guides/llm-app-builder.md",
        &["They use the oriented studio rig"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "CHANGELOG.md",
        &["Build generated CAD inspection views with the oriented studio-light rig"],
    );
}
