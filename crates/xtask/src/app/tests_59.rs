use crate::app::prelude::*;

#[test]
fn a01_doctor_rejects_removed_environment_resource_planning() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a01-resource-resolution");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene/recipe/build/resource_plan.rs",
        "src/assets/recipe_validation.rs",
        "src/scene_host/recipe.rs",
        "src/bin/scena/args.rs",
        "tests/a01_recipe_resolution.rs",
        "tests/assets/stable-contracts/scene_recipe_validation.v1.json",
        "README.md",
        "docs/schema-contracts.md",
        "docs/guides/llm-app-builder.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A01 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A01 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a01_recipe_resource_resolution(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let plan = fixture_root.join("src/scene/recipe/build/resource_plan.rs");
    let source = fs::read_to_string(&plan).expect("A01 plan source reads");
    let mutated = source.replacen("RecipeResourceRole::Environment,", "", 1);
    assert_ne!(source, mutated, "A01 mutation must remove environment role");
    fs::write(plan, mutated).expect("A01 plan mutation writes");
    findings.clear();
    check_a01_recipe_resource_resolution(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A01-RECIPE-RESOURCE-RESOLUTION"
                && finding.message.contains("RecipeResourceRole::Environment")
        }),
        "removing environment planning must fail doctor: {findings:?}",
    );
}
