use crate::app::prelude::*;

pub(crate) fn check_a09_feature_discoverability(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A09-FEATURE-DISCOVERABILITY";

    for (path, needles) in [
        (
            "Cargo.toml",
            &[
                "default = []",
                "agent = [\"scene-host\", \"material-library\"]",
                "scene-host = [\"inspection\"]",
                "material-library = [\"dep:ureq\", \"dep:zip\"]",
                "[package.metadata.scena.cli-install]",
                "default-contract = \"core-discovery-validation\"",
                "application-builder-feature = \"agent\"",
            ][..],
        ),
        (
            "src/bin/scena.rs",
            &[
                "\"agent\": cfg!(feature = \"agent\")",
                "scena_guide::run_agent_guide_command",
                "cargo install scena --features {feature}",
                "feature_required(\"recipe build\", \"agent\")",
                "feature_required(\"examples agent\", \"agent\")",
                "feature_required(\"verify interaction\", \"agent\")",
                "feature_required(\"inspect\", \"agent\")",
            ][..],
        ),
        (
            "src/bin/scena/guide.rs",
            &[
                "run_agent_guide_command",
                "scena::agent_guide_v1()",
                "--json",
                "--markdown",
            ][..],
        ),
        (
            "src/schema_catalog/agent_guide.rs",
            &[
                "AGENT_GUIDE_SCHEMA_V1",
                "struct AgentGuideV1",
                "include_str!(\"../../docs/guides/llm-app-builder.md\")",
                "pub fn agent_guide_v1",
            ][..],
        ),
        (
            "docs/specs/feature-ownership.json",
            &[
                "\"name\": \"agent\"",
                "\"kind\": \"feature-composition\"",
                "agent = [\\\"scene-host\\\", \\\"material-library\\\"]",
                "\"name\": \"material-library\"",
                "agent_feature_enables_the_complete_self_verification_surface",
            ][..],
        ),
        (
            "README.md",
            &[
                "cargo install scena --features agent",
                "The default feature set remains empty",
                "docs/specs/cli-install-contract.md",
                "`scene-host` | native/browser SceneHost facade; enables `inspection`",
                "scena photo render model.glb --out hero.png --report hero.report.json",
                "photo.intent",
                "no manual camera, exposure, or focus",
            ][..],
        ),
        (
            "docs/getting-started.md",
            &[
                "cargo install scena --features agent",
                "one-step self-verification and",
                "`material-library` compiler",
                "scena photo render model.glb --out hero.png --report hero.report.json",
                "photo.intent",
                "no manual camera, exposure, or focus",
            ][..],
        ),
        (
            "docs/guides/easy-scene-setup.md",
            &[
                "scena photo render model.glb --out hero.png --report hero.report.json",
                "photo.intent",
                "no manual camera, exposure, or focus",
                "Raw Rust camera, lighting, and exposure setup remains the advanced path",
            ][..],
        ),
        (
            "docs/feature-flags.md",
            &[
                "`agent` | complete opt-in self-verification and material-authoring surface",
                "`material-library` | native CC0 material download/import",
                "cargo add scena --features agent",
                "The default feature set is exactly empty",
            ][..],
        ),
        (
            "docs/api.md",
            &[
                "complete agent/self-verification build",
                "default builds remain feature-empty",
            ][..],
        ),
        (
            "docs/examples.md",
            &["cargo install scena --features agent"][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "cargo install scena --features agent",
                "cargo build --release --bin scena --features agent",
                "SCENA_CANONICAL_AGENT_SMOKE_BEGIN",
                "mkdir -p target/scena-agent",
                "target/scena-agent/primitive-scene/recipe.json",
            ][..],
        ),
        (
            "tests/a03_llm_guide_smoke.rs",
            &[
                "canonical_agent_guide_block_runs_from_a_clean_directory",
                "SCENA_A03_BIN",
                "scena.render_introspection.v1",
                "canonical guide frame must contain a visible result",
            ][..],
        ),
        (
            "docs/specs/cli-install-contract.md",
            &[
                "core discovery, schema, vocabulary, policy",
                "Cargo cannot enable a package feature only for one binary",
                "separate `scena-cli` package",
                "code:\"feature_unavailable\"",
                "exit_class:\"unsupported\"",
                "exit 69",
                "Packaged-crate tests install both",
            ][..],
        ),
        (
            "tests/a04_packaged_cli_contract.rs",
            &[
                "packaged_cli_matches_the_declared_install_feature_contract",
                "SCENA_A04_EXPECT_AGENT",
                "feature_unavailable",
                "scena.visual_repair_plan.v1",
            ][..],
        ),
        (
            "tests/a05_public_agent_guide.rs",
            &[
                "installed_cli_exports_public_agent_guidance_outside_the_repository",
                "scena.agent_guide.v1",
                "guide\", \"agent\", \"--markdown",
            ][..],
        ),
        (
            "tests/assets/stable-contracts/agent_guide.v1.json",
            &["scena.agent_guide.v1", "llm-app-builder", "primitive-scene"][..],
        ),
        (
            "src/vocabulary.rs",
            &[
                "MaterialDesc::PRESET_NAMES",
                "PerspectiveCamera::LENS_PRESET_NAMES",
                "FramingOptions::PRESET_NAMES",
                "Color::NAMED_CONSTANTS",
                "EnvironmentPreset::ALL",
                "AutoExposureConfig::PRESET_NAMES",
                "DIRECTIONAL_LIGHT_PRESETS",
                "validate_vocabulary_report_v1",
            ][..],
        ),
        (
            "tests/a07_vocabulary_parity.rs",
            &[
                "every_authoritative_preset_registry_is_machine_discoverable",
                "omitted_preset_mutation_is_rejected",
                "scene_preset_vocabulary_matches_scene_host_registry",
            ][..],
        ),
        (
            "tests/a08_default_introspection.rs",
            &[
                "render_commands_emit_introspection_without_the_compatibility_flag",
                "introspect_flag_remains_an_accepted_no_op",
                "scena.render_introspection.v1",
            ][..],
        ),
        (
            "src/contract_validation.rs",
            &[
                "validate_contract_json_v1",
                "contract_json_schema_export_v1",
                "nearest_name_candidates",
                "validation_level: \"envelope\"",
                "JSON Schema cannot prove filesystem/resource resolution",
            ][..],
        ),
        (
            "src/bin/scena/validate.rs",
            &[
                "run_validate_command",
                "scena::validate_contract_json_v1",
                "exit_code = if report.ok { 0 } else { 65 }",
            ][..],
        ),
        (
            "src/bin/scena/schema.rs",
            &["run_schema_json_command", "contract_json_schema_export_v1"][..],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "render introspection is emitted by default",
                "--introspect remains an accepted compatibility no-op",
                "schema json <scena.*.vN>",
                "validate <file>",
                "scena.contract_validation.v1",
                "scena.json_schema_export.v1",
                "\"failure_exits\": failure_exits",
                "feature_requirements",
                "domain_failure",
                "scena.cli_error.v1",
            ][..],
        ),
        (
            "tests/a09_generic_validation.rs",
            &[
                "validate_dispatches_public_input_contracts_by_embedded_schema",
                "validate_fails_closed_for_malformed_unknown_and_mismatched_contracts",
                "schema_json_exports_recipe_schema_and_declares_runtime_limits",
                "validate_reuses_recipe_patch_owner_invariants",
            ][..],
        ),
        (
            "tests/a10_cli_contract_table.rs",
            &[
                "every_help_row_is_a_self_contained_process_contract",
                "agent_only_commands_name_the_one_step_install_feature",
                "errors_doc_points_to_the_machine_authoritative_complete_table",
                "complete_process_table_matches_the_reviewable_golden_digest",
                "process_contract_table.sha256",
            ][..],
        ),
        (
            "tests/assets/cli-golden/process_contract_table.sha256",
            &["21f9da8b1311dd7b65cdce4bc63c1b2309c4063774743f3e30ee3d4ecfe8919b"][..],
        ),
        (
            "tests/assets/stable-contracts/contract_validation.v1.json",
            &["scena.contract_validation.v1", "typed"][..],
        ),
        (
            "tests/assets/stable-contracts/json_schema_export.v1.json",
            &[
                "scena.json_schema_export.v1",
                "draft/2020-12",
                "limitations",
            ][..],
        ),
        (
            "tests/assets/stable-contracts/vocab.v1.json",
            &["scena.vocab.v1", "\"entries\"", "scene/placement"][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "material/lens/framing/scene/environment/",
                "known-bad omission test",
                "scena validate <file>",
                "scena schema json <scena.*.vN>",
                "envelope validation",
            ][..],
        ),
        (
            "docs/errors.md",
            &[
                "Complete CLI process contract",
                "success and domain-failure JSON use stdout",
                "CLI dispatch and runtime errors use stderr",
                "failure_exits[]",
                "feature_requirements[]",
                "I/O 74",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/SKILL.md",
            &[
                "cargo install scena --features agent",
                "cargo run --bin scena --features agent -- <command>",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/references/recipe-loop.md",
            &["cargo install scena --features agent"][..],
        ),
        ("CHANGELOG.md", &["Add an opt-in `agent` Cargo feature"][..]),
        (
            "docs/release-notes/v1.8.0.md",
            &["add the opt-in `agent` composition"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a10_cli_contract_table.rs",
        &[
            "every_help_row_is_a_self_contained_process_contract",
            "agent_only_commands_name_the_one_step_install_feature",
            "errors_doc_points_to_the_machine_authoritative_complete_table",
            "complete_process_table_matches_the_reviewable_golden_digest",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a09_generic_validation.rs",
        &[
            "validate_dispatches_public_input_contracts_by_embedded_schema",
            "validate_fails_closed_for_malformed_unknown_and_mismatched_contracts",
            "schema_json_exports_recipe_schema_and_declares_runtime_limits",
            "validate_reuses_recipe_patch_owner_invariants",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a09_feature_discoverability.rs",
        &[
            "manifest_keeps_defaults_empty_and_declares_one_step_agent_composition",
            "agent_feature_enables_the_complete_self_verification_surface",
            "unavailable_agent_commands_name_one_installable_feature_remedy",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a08_default_introspection.rs",
        &[
            "render_commands_emit_introspection_without_the_compatibility_flag",
            "introspect_flag_remains_an_accepted_no_op",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a07_vocabulary_parity.rs",
        &[
            "every_authoritative_preset_registry_is_machine_discoverable",
            "omitted_preset_mutation_is_rejected",
            "scene_preset_vocabulary_matches_scene_host_registry",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a05_public_agent_guide.rs",
        &["installed_cli_exports_public_agent_guidance_outside_the_repository"],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a04_packaged_cli_contract.rs",
        &["packaged_cli_matches_the_declared_install_feature_contract"],
    );

    for path in [
        "docs/getting-started.md",
        "docs/guides/llm-app-builder.md",
        "docs/examples.md",
        ".codex/skills/scena-app-builder/SKILL.md",
        ".codex/skills/scena-app-builder/references/recipe-loop.md",
    ] {
        let Ok(source) = fs::read_to_string(root.join(path)) else {
            continue;
        };
        if source.contains("--features scene-host,inspection") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{path} redundantly requests scene-host,inspection; use the one-step agent feature"
                ),
            ));
        }
        if path == "docs/guides/llm-app-builder.md" && source.contains("primitive_scene") {
            findings.push(Finding::new(
                RULE,
                "the canonical guide must use the primitive-scene output path consistently",
            ));
        }
    }

    for path in [
        "src/bin/scena/args/inspection.rs",
        "src/bin/scena/recipe.rs",
    ] {
        let Ok(source) = fs::read_to_string(root.join(path)) else {
            continue;
        };
        if source.contains("missing --introspect") {
            findings.push(Finding::new(
                RULE,
                format!("{path} must emit introspection by default; --introspect is only a compatibility no-op"),
            ));
        }
    }
}
