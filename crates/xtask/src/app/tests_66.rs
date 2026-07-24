use crate::app::prelude::*;

#[test]
fn a08_doctor_rejects_restoring_the_untagged_import_transform_type() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a08-transform-grammar");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene/recipe/types/authoring/transform.rs",
        "src/scene/recipe/types/authoring/imports.rs",
        "src/scene/recipe/validation/imports.rs",
        "src/scene/recipe/validation/authoring/targets/common.rs",
        "src/scene/recipe/field_model.rs",
        "src/scene_host/recipe.rs",
        "src/scene_host/recipe/authoring/transform.rs",
        "src/scene/placement.rs",
        "src/scene/placement/serialization.rs",
        "src/bin/scena/place.rs",
        "tests/a08_transform_grammar.rs",
        "tests/stable_contracts.rs",
        "tests/assets/stable-contracts/placement_result.v1.json",
        "tests/assets/stable-contracts/recipe_patch.v1.json",
        "tests/assets/cli-golden/place_center_stdout.json",
        "README.md",
        "docs/api.md",
        "docs/schema-contracts.md",
        "docs/guides/llm-app-builder.md",
        ".codex/skills/scena-app-builder/references/recipe-loop.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A08 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A08 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a08_transform_grammar(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let imports = fixture_root.join("src/scene/recipe/types/authoring/imports.rs");
    let source = fs::read_to_string(&imports).expect("import type source reads");
    let mutated = source.replacen(
        "pub transform: Option<SceneRecipeTransformV1>",
        "pub transform: Option<Transform>",
        1,
    );
    assert_ne!(source, mutated, "A08 mutation must restore the old type");
    fs::write(imports, mutated).expect("A08 mutation writes");
    findings.clear();
    check_a08_transform_grammar(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A08-CANONICAL-TRANSFORM-GRAMMAR"
                && finding
                    .message
                    .contains("pub transform: Option<SceneRecipeTransformV1>")
        }),
        "restoring the untagged import type must fail doctor: {findings:?}",
    );
}
