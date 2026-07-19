use crate::app::prelude::*;

#[test]
fn fr08_doctor_rejects_unsupported_fallback_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/fr08-recipe-spatial-state");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "docs/specs/recipe-spatial-state-v1.md",
        "src/scene/recipe/types/spatial_state.rs",
        "src/scene/recipe/validation/spatial_state.rs",
        "src/scene/recipe/validation/spatial_state/states.rs",
        "src/scene_host/recipe/spatial_state.rs",
        "src/scene_host/recipe/spatial_state/connectors.rs",
        "src/scene_host/recipe/spatial_state/states.rs",
        "src/scene/bounds.rs",
        "src/scene/recipe/field_model.rs",
        "docs/schema-contracts.md",
        "docs/guides/llm-app-builder.md",
        "tests/fr08_recipe_spatial_state.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("FR08 fixture path has parent"))
            .expect("FR08 fixture directory creates");
        fs::copy(root.join(relative), destination).expect("FR08 fixture source copies");
    }

    let mut findings = Vec::new();
    check_fr08_recipe_spatial_state_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let validation = fixture_root.join("src/scene/recipe/validation/spatial_state.rs");
    let source = fs::read_to_string(&validation).expect("FR08 validation fixture reads");
    let mutated = source.replacen("unknown_spatial_target", "unsupported_feature", 1);
    assert_ne!(source, mutated, "FR08 fallback mutation must alter source");
    fs::write(&validation, mutated).expect("FR08 validation mutation writes");
    findings.clear();
    check_fr08_recipe_spatial_state_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR08-RECIPE-SPATIAL-STATE"
                && finding.message.contains("unknown_spatial_target")
        }),
        "restoring unsupported fallback must fail: {findings:?}"
    );
}
