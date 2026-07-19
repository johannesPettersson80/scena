use crate::app::prelude::*;

#[test]
fn fr07_doctor_rejects_silent_or_overclaimed_visual_attribution() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/fr07-recipe-diff");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene/recipe/diff.rs",
        "src/bin/scena/diff.rs",
        "src/bin/scena/diff/attribution.rs",
        "src/bin/scena/help.rs",
        "src/schema_catalog.rs",
        "src/schema_catalog/fixtures.rs",
        "docs/schema-contracts.md",
        "docs/guides/llm-app-builder.md",
        "tests/fr07_recipe_diff.rs",
        "tests/assets/stable-contracts/scene_recipe_diff_result.v1.json",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("FR07 fixture path has parent"))
            .expect("FR07 fixture directory creates");
        fs::copy(root.join(relative), destination).expect("FR07 fixture source copies");
    }

    let mut findings = Vec::new();
    check_fr07_recipe_diff_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let attribution = fixture_root.join("src/bin/scena/diff/attribution.rs");
    let source = fs::read_to_string(&attribution).expect("FR07 attribution fixture reads");
    let mutated = source.replace(
        "\"anti_aliased_edges\": \"ambiguous\"",
        "\"anti_aliased_edges\": \"attributed\"",
    );
    assert_ne!(source, mutated, "FR07 ambiguity mutation must alter source");
    fs::write(&attribution, mutated).expect("FR07 ambiguity mutation writes");
    findings.clear();
    check_fr07_recipe_diff_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR07-RECIPE-DIFF" && finding.message.contains("anti_aliased_edges")
        }),
        "claiming edge attribution is exact must fail: {findings:?}"
    );
}
