use crate::app::prelude::*;

#[test]
fn a09_doctor_rejects_a_redundant_or_default_agent_feature() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a09-feature-discoverability");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "Cargo.toml",
        "src/bin/scena.rs",
        "src/bin/scena/help.rs",
        "src/bin/scena/validate.rs",
        "src/bin/scena/schema.rs",
        "src/contract_validation.rs",
        "src/bin/scena/guide.rs",
        "src/schema_catalog/agent_guide.rs",
        "tests/a03_llm_guide_smoke.rs",
        "tests/a04_packaged_cli_contract.rs",
        "tests/a05_public_agent_guide.rs",
        "tests/a09_feature_discoverability.rs",
        "tests/a07_vocabulary_parity.rs",
        "tests/a08_default_introspection.rs",
        "tests/a09_generic_validation.rs",
        "tests/a10_cli_contract_table.rs",
        "tests/assets/cli-golden/process_contract_table.sha256",
        "tests/assets/stable-contracts/agent_guide.v1.json",
        "tests/assets/stable-contracts/contract_validation.v1.json",
        "tests/assets/stable-contracts/json_schema_export.v1.json",
        "tests/assets/stable-contracts/vocab.v1.json",
        "src/vocabulary.rs",
        "src/bin/scena/args/inspection.rs",
        "src/bin/scena/recipe.rs",
        "docs/specs/feature-ownership.json",
        "README.md",
        "docs/getting-started.md",
        "docs/feature-flags.md",
        "docs/api.md",
        "docs/errors.md",
        "docs/examples.md",
        "docs/guides/llm-app-builder.md",
        "docs/specs/cli-install-contract.md",
        "docs/schema-contracts.md",
        ".codex/skills/scena-app-builder/SKILL.md",
        ".codex/skills/scena-app-builder/references/recipe-loop.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A09 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A09 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let manifest = fixture_root.join("Cargo.toml");
    let source = fs::read_to_string(&manifest).expect("manifest reads");
    let mutated = source.replacen(
        "agent = [\"scene-host\"]",
        "agent = [\"scene-host\", \"inspection\"]",
        1,
    );
    assert_ne!(
        source, mutated,
        "A09 mutation must add the redundant feature"
    );
    fs::write(&manifest, mutated).expect("A09 mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("agent = [\"scene-host\"]")
        }),
        "redundant agent composition must fail doctor: {findings:?}",
    );

    fs::write(&manifest, source).expect("manifest restores");
    let source = fs::read_to_string(&manifest).expect("manifest reads");
    let mutated = source.replacen(
        "default-contract = \"core-discovery-validation\"",
        "default-contract = \"undocumented\"",
        1,
    );
    assert_ne!(source, mutated, "A04 mutation must alter install metadata");
    fs::write(&manifest, mutated).expect("install metadata mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding
                    .message
                    .contains("default-contract = \"core-discovery-validation\"")
        }),
        "install contract metadata drift must fail doctor: {findings:?}",
    );

    fs::write(&manifest, source).expect("manifest restores");
    let guide = fixture_root.join("docs/guides/llm-app-builder.md");
    let source = fs::read_to_string(&guide).expect("guide reads");
    let mutated = source.replacen(
        "target/scena-agent/primitive-scene/recipe.json",
        "target/scena-agent/primitive_scene/recipe.json",
        1,
    );
    assert_ne!(source, mutated, "A03 mutation must change the guide path");
    fs::write(&guide, mutated).expect("guide mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("primitive-scene")
        }),
        "guide path drift must fail doctor: {findings:?}",
    );

    fs::write(&guide, source).expect("guide restores");
    let public_guide = fixture_root.join("src/schema_catalog/agent_guide.rs");
    let source = fs::read_to_string(&public_guide).expect("public guide source reads");
    let mutated = source.replace(
        "include_str!(\"../../docs/guides/llm-app-builder.md\")",
        "String::new()",
    );
    assert_ne!(source, mutated, "A05 mutation must remove embedded guide");
    fs::write(&public_guide, mutated).expect("public guide mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("include_str!")
        }),
        "removing packaged guide ownership must fail doctor: {findings:?}",
    );

    fs::write(&public_guide, source).expect("public guide restores");
    let vocabulary = fixture_root.join("src/vocabulary.rs");
    let source = fs::read_to_string(&vocabulary).expect("vocabulary source reads");
    let mutated = source.replacen("MaterialDesc::PRESET_NAMES", "&[]", 1);
    assert_ne!(source, mutated, "A07 mutation must omit material presets");
    fs::write(&vocabulary, mutated).expect("vocabulary mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("MaterialDesc::PRESET_NAMES")
        }),
        "omitting an authoritative preset registry must fail doctor: {findings:?}",
    );

    fs::write(&vocabulary, source).expect("vocabulary restores");
    let render_args = fixture_root.join("src/bin/scena/args/inspection.rs");
    let source = fs::read_to_string(&render_args).expect("render args read");
    let mutated = source.replacen(
        "usage: scena render",
        "missing --introspect; usage: scena render",
        1,
    );
    assert_ne!(source, mutated, "A08 mutation must require introspection");
    fs::write(&render_args, mutated).expect("render args mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("introspection by default")
        }),
        "requiring --introspect must fail doctor: {findings:?}",
    );

    fs::write(&render_args, source).expect("render args restores");
    let validator = fixture_root.join("src/contract_validation.rs");
    let source = fs::read_to_string(&validator).expect("contract validator reads");
    let mutated = source.replacen("nearest_name_candidates", "removed_name_candidates", 1);
    assert_ne!(
        source, mutated,
        "A09 mutation must remove schema suggestions"
    );
    fs::write(&validator, mutated).expect("contract validator mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("nearest_name_candidates")
        }),
        "removing generic-validation schema suggestions must fail doctor: {findings:?}",
    );

    fs::write(&validator, source).expect("contract validator restores");
    let help = fixture_root.join("src/bin/scena/help.rs");
    let source = fs::read_to_string(&help).expect("CLI help source reads");
    let mutated = source.replacen(
        "\"failure_exits\": failure_exits",
        "\"removed_failure_exits\": failure_exits",
        1,
    );
    assert_ne!(source, mutated, "A10 mutation must remove numeric exits");
    fs::write(&help, mutated).expect("CLI help mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("\"failure_exits\": failure_exits")
        }),
        "removing the complete process exit table must fail doctor: {findings:?}",
    );
}
