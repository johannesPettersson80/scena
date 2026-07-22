use crate::app::prelude::*;

#[test]
fn fr01_fr04_contract_discovery_doctor_rejects_missing_emits_registry() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/fr01-fr04-discovery");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/vocabulary.rs",
        "src/scene/recipe/build.rs",
        "src/scene/recipe/build/report.rs",
        "src/scene/recipe/build/sandbox.rs",
        "src/scene/recipe/field_model.rs",
        "src/scene/recipe/validation/suggestions.rs",
        "src/assets.rs",
        "src/assets/fetch.rs",
        "src/assets/environment_loading.rs",
        "src/bin/scena/help.rs",
        "src/bin/scena.rs",
        "src/bin/scena/recipe.rs",
        "src/bin/scena/place.rs",
        "src/scene_host/core.rs",
        "src/scene_host/recipe.rs",
        "src/scene_host/recipe/manifest.rs",
        "src/scene_host/recipe/setup.rs",
        "src/scene/recipe/types/build_manifest.rs",
        "src/scene/placement.rs",
        "src/schema_catalog.rs",
        "src/schema_catalog/entries.rs",
        "src/schema_catalog/reports.rs",
        "docs/schema-contracts.md",
        "tests/scena_cli_schema.rs",
        "tests/scena_cli_recipe.rs",
        "tests/fr02_recipe_build_cli.rs",
        "tests/fr04_cli_schema_matrix.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture path has parent"))
            .expect("discovery fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("discovery source fixture copies");
    }

    let mut findings = Vec::new();
    check_fr01_fr04_contract_discovery(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let help = fixture_root.join("src/bin/scena/help.rs");
    let source = fs::read_to_string(&help)
        .expect("help fixture reads")
        .replace("command_contracts", "removed_contract_registry");
    fs::write(&help, source).expect("help mutation writes");
    findings.clear();
    check_fr01_fr04_contract_discovery(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR01-FR04-CONTRACT-DISCOVERY"
                && finding.message.contains("command_contracts")
        }),
        "removing command emits discovery must fail: {findings:?}"
    );

    fs::copy(root.join("src/bin/scena/help.rs"), &help).expect("help fixture restores");
    let field_model = fixture_root.join("src/scene/recipe/field_model.rs");
    let source = fs::read_to_string(&field_model).expect("field-model fixture reads");
    let mutated = source.replace("FIELD_MODEL_SCHEMA_V1", "REMOVED_SCHEMA_V1");
    assert_ne!(
        source, mutated,
        "FR01 field-model mutation must alter source"
    );
    fs::write(&field_model, mutated).expect("field-model mutation writes");
    findings.clear();
    check_fr01_fr04_contract_discovery(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR01-FR04-CONTRACT-DISCOVERY"
                && finding.message.contains("FIELD_MODEL_SCHEMA_V1")
        }),
        "removing the authoritative field model must fail: {findings:?}"
    );
    fs::copy(root.join("src/scene/recipe/field_model.rs"), &field_model)
        .expect("field-model fixture restores");

    let recipe = fixture_root.join("src/scene_host/recipe/manifest.rs");
    let source = fs::read_to_string(&recipe).expect("recipe fixture reads");
    let mutated = source.replace(
        "RecipeBuildMode::ManifestOnly",
        "RecipeBuildMode::Host(RecipeBackendPolicy::Cpu)",
    );
    assert_ne!(source, mutated, "FR02 renderer mutation must alter source");
    fs::write(recipe, mutated).expect("recipe mutation writes");
    findings.clear();
    check_fr01_fr04_contract_discovery(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR01-FR04-CONTRACT-DISCOVERY"
                && finding.message.contains("RecipeBuildMode::ManifestOnly")
        }),
        "restoring renderer-backed recipe build must fail: {findings:?}"
    );
}

#[test]
fn fr05_capture_sequence_doctor_rejects_bypassed_prepare_render_lifecycle() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/fr05-capture-sequence");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena/recipe/capture_sequence.rs",
        "src/bin/scena/recipe/capture_shared.rs",
        "src/bin/scena/recipe/capture_sequence/output.rs",
        "src/bin/scena/recipe/capture_sequence/view.rs",
        "src/bin/scena/recipe/cad_inspection/view.rs",
        "src/bin/scena/recipe/cad_inspection/image.rs",
        "src/bin/scena/help.rs",
        "src/schema_catalog.rs",
        "docs/schema-contracts.md",
        "tests/fr05_capture_sequence.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("FR05 fixture path has parent"))
            .expect("FR05 fixture directory creates");
        fs::copy(root.join(relative), destination).expect("FR05 source fixture copies");
    }

    let mut findings = Vec::new();
    check_fr05_capture_sequence_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let output = fixture_root.join("src/bin/scena/recipe/capture_sequence/output.rs");
    let source = fs::read_to_string(&output).expect("FR05 output fixture reads");
    let mutated = source.replace("host.prepare()", "host.renderer_mut().render()");
    assert_ne!(source, mutated, "FR05 lifecycle mutation must alter source");
    fs::write(output, mutated).expect("FR05 lifecycle mutation writes");
    findings.clear();
    check_fr05_capture_sequence_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR05-CAPTURE-SEQUENCE" && finding.message.contains("host.prepare()")
        }),
        "bypassing explicit prepare must fail: {findings:?}"
    );
}
