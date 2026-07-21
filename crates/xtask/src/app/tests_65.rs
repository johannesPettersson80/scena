use crate::app::prelude::*;

#[test]
fn a07_doctor_rejects_dropping_structured_lookup_candidates() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a07-name-candidates");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/diagnostics/name_candidates.rs",
        "src/diagnostics/animation_error.rs",
        "src/diagnostics.rs",
        "src/diagnostics/display.rs",
        "src/diagnostics/display/lookup.rs",
        "src/diagnostics/display_animation.rs",
        "src/scene/import/lookups.rs",
        "src/scene/import/variants.rs",
        "src/scene/mixers.rs",
        "src/scene/recipe/types/build_manifest.rs",
        "src/scene/recipe/validation/authoring/targets/common.rs",
        "src/scene/recipe/validation/authoring/targets/import_refs.rs",
        "src/scene/recipe/validation/setup/scene.rs",
        "src/schema_catalog/fixtures.rs",
        "src/bin/scena.rs",
        "src/bin/scena/examples_agent/catalog.rs",
        "src/scene_host/error.rs",
        "src/scene_host/animation.rs",
        "src/scene_host/recipe/diagnostic.rs",
        "tests/a07_name_candidates.rs",
        "README.md",
        "docs/errors.md",
        "docs/api.md",
        "docs/schema-contracts.md",
        "docs/troubleshooting.md",
        "docs/guides/llm-app-builder.md",
        ".codex/skills/scena-app-builder/references/debugging.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A07 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A07 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a07_name_candidates_and_remedies(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let binary = fixture_root.join("src/bin/scena.rs");
    let source = fs::read_to_string(&binary).expect("CLI source reads");
    let mutated = source.replacen(
        "\"candidates\": cli_error_candidates(&args)",
        "\"candidates\": Vec::<String>::new()",
        1,
    );
    assert_ne!(source, mutated, "A07 mutation must remove CLI candidates");
    fs::write(binary, mutated).expect("CLI mutation writes");
    findings.clear();
    check_a07_name_candidates_and_remedies(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A07-NAME-CANDIDATES-REMEDIES"
                && finding.message.contains("cli_error_candidates(&args)")
        }),
        "dropping structured CLI candidates must fail doctor: {findings:?}",
    );
}
