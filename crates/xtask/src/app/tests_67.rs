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
        "docs/guides/easy-scene-setup.md",
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
        "agent = [\"scene-host\", \"material-library\"]",
        "agent = [\"scene-host\", \"inspection\", \"material-library\"]",
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
                && finding
                    .message
                    .contains("agent = [\"scene-host\", \"material-library\"]")
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

#[test]
fn x01_doctor_rejects_subject_photo_contract_drift() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/x01-subject-photo-contracts");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        ".github/workflows/ci.yml",
        "README.md",
        "docs/getting-started.md",
        "docs/errors.md",
        "docs/troubleshooting.md",
        "docs/guides/easy-scene-setup.md",
        "docs/guides/llm-app-builder.md",
        "docs/schema-contracts.md",
        "docs/checklists/subject-driven-photo-rendering.md",
        "src/bin/scena/help.rs",
        "src/bin/scena/photo.rs",
        "src/bin/scena/recipe.rs",
        "src/geometry/photographic.rs",
        "src/schema_catalog.rs",
        "src/schema_catalog/fixtures.rs",
        "src/scene_host/photographic_surface.rs",
        "tests/photo_render_cli.rs",
        "tests/scena_cli_recipe.rs",
        "tests/scena_cli_schema.rs",
        "tests/assets/photo/camera_behavior_cad_terminal_block.fixture.json",
        "tests/assets/stable-contracts/exposure_report.v1.json",
        "tests/assets/stable-contracts/focus_report.v1.json",
        "tests/assets/stable-contracts/photo_candidate_plan.v1.json",
        "tests/assets/stable-contracts/photo_plan.v1.json",
        "tests/assets/stable-contracts/photo_render_result.v1.json",
        "tests/assets/stable-contracts/photo_report.v1.json",
        "tests/assets/stable-contracts/photo_shaded_candidate_selection.v1.json",
        "tests/assets/stable-contracts/subject_observation.v1.json",
        "evidence/demo-hero/hero.recipe.json",
        "demo-next/index.html",
        "demo-next/assets/hero.recipe.json",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("X01 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("X01 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_x01_subject_photo_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let schema_docs = fixture_root.join("docs/schema-contracts.md");
    let source = fs::read_to_string(&schema_docs).expect("schema docs fixture reads");
    let mutated = source.replace("scena.photo_report.v1", "scena.photo_report_removed.v1");
    assert_ne!(
        source, mutated,
        "X01 docs mutation must remove photo_report"
    );
    fs::write(&schema_docs, mutated).expect("schema docs mutation writes");
    findings.clear();
    check_x01_subject_photo_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "X01-SUBJECT-PHOTO-CONTRACTS"
                && finding.message.contains("docs/schema-contracts.md")
                && finding.message.contains("scena.photo_report.v1")
        }),
        "missing photo_report docs must fail doctor: {findings:?}",
    );

    fs::write(&schema_docs, source).expect("schema docs restores");
    let mutation_manifest =
        fixture_root.join("tests/assets/photo/camera_behavior_cad_terminal_block.fixture.json");
    let source = fs::read_to_string(&mutation_manifest).expect("camera behavior manifest reads");
    let mutated = source.replacen("average_metered_silhouette", "removed_silhouette", 1);
    assert_ne!(
        source, mutated,
        "X01 fixture mutation must remove known-bad mutation"
    );
    fs::write(&mutation_manifest, mutated).expect("camera behavior manifest mutation writes");
    findings.clear();
    check_x01_subject_photo_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "X01-SUBJECT-PHOTO-CONTRACTS"
                && finding.message.contains("average_metered_silhouette")
        }),
        "missing camera-behavior known-bad mutation must fail doctor: {findings:?}",
    );

    fs::write(&mutation_manifest, source).expect("camera behavior manifest restores");
    let demo_recipe = fixture_root.join("evidence/demo-hero/hero.recipe.json");
    let source = fs::read_to_string(&demo_recipe).expect("demo recipe reads");
    let mut recipe: Value = serde_json::from_str(&source).expect("demo recipe parses");
    recipe["render"]["exposure_ev"] = json!(2.0);
    fs::write(
        &demo_recipe,
        serde_json::to_string_pretty(&recipe).expect("recipe serializes"),
    )
    .expect("demo recipe mutation writes");
    findings.clear();
    check_x01_subject_photo_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "X01-SUBJECT-PHOTO-CONTRACTS"
                && finding.message.contains("manual")
                && finding.message.contains("exposure_ev")
        }),
        "manual exposure override in photo.intent demo recipe must fail doctor: {findings:?}",
    );

    fs::write(&demo_recipe, source).expect("demo recipe restores");
    let workflow = fixture_root.join(".github/workflows/ci.yml");
    let source = fs::read_to_string(&workflow).expect("CI workflow reads");
    let mutated = source.replacen(
        "cargo test --workspace --all-features --tests",
        "cargo test",
        1,
    );
    assert_ne!(source, mutated, "X01 CI mutation must remove feature lane");
    fs::write(&workflow, mutated).expect("CI workflow mutation writes");
    findings.clear();
    check_x01_subject_photo_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "X01-SUBJECT-PHOTO-CONTRACTS"
                && finding.message.contains("all-features")
        }),
        "missing feature-gated camera-photo CI lane must fail doctor: {findings:?}",
    );

    fs::write(&workflow, source).expect("CI workflow restores");
    let troubleshooting = fixture_root.join("docs/troubleshooting.md");
    let source = fs::read_to_string(&troubleshooting).expect("troubleshooting reads");
    let mutated = source.replace("stale_subject_observation", "removed_stale_subject");
    assert_ne!(
        source, mutated,
        "X01 troubleshooting mutation must remove stale-observation docs"
    );
    fs::write(&troubleshooting, mutated).expect("troubleshooting mutation writes");
    findings.clear();
    check_x01_subject_photo_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "X01-SUBJECT-PHOTO-CONTRACTS"
                && finding.message.contains("stale_subject_observation")
        }),
        "missing degraded/fallback docs must fail doctor: {findings:?}",
    );
}

#[test]
fn x02_doctor_rejects_orphaned_camera_behavior_feature_gated_tests() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/x02-ci-bijection");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join(".github/workflows"))
        .expect("workflow fixture directory creates");
    fs::create_dir_all(fixture_root.join("tests")).expect("tests fixture directory creates");
    fs::write(
        fixture_root.join(".github/workflows/ci.yml"),
        "name: CI\njobs:\n  default:\n    steps:\n      - run: cargo test\n",
    )
    .expect("workflow fixture writes");
    fs::write(
        fixture_root.join("tests/photo_render_cli.rs"),
        r#"#![cfg(all(feature = "inspection", feature = "scene-host"))]

#[test]
fn photo_render_camera_behavior_is_easy_path_for_imported_asset() {}
"#,
    )
    .expect("camera behavior fixture test writes");
    fs::write(
        fixture_root.join("tests/scena_cli_recipe.rs"),
        r#"#![cfg(feature = "inspection")]

#[test]
fn recipe_render_product_quality_uses_exact_subject_observation_pixels() {}
"#,
    )
    .expect("subject observation fixture test writes");

    let mut findings = Vec::new();
    check_feature_gated_tests_run_in_a_workflow(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-FEATURE-GATED-WORKFLOW-BIJECTION"
                && finding.message.contains("tests/photo_render_cli.rs")
        }),
        "orphaned camera-behavior proof test must fail CI bijection doctor: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-FEATURE-GATED-WORKFLOW-BIJECTION"
                && finding.message.contains("tests/scena_cli_recipe.rs")
        }),
        "orphaned subject-observation proof test must fail CI bijection doctor: {findings:?}",
    );

    fs::write(
        fixture_root.join(".github/workflows/ci.yml"),
        "name: CI\njobs:\n  feature_gated:\n    steps:\n      - run: cargo test --workspace --all-features --tests\n",
    )
    .expect("workflow blanket lane writes");
    findings.clear();
    check_feature_gated_tests_run_in_a_workflow(&fixture_root, &mut findings);
    assert_eq!(
        findings,
        Vec::new(),
        "blanket all-features test lane must cover camera-behavior and subject-observation gated tests",
    );
}
