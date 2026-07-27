#![cfg(all(feature = "inspection", feature = "scene-host"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[test]
fn fr04_command_contracts_match_observed_top_level_output_families() {
    let help = help_report();
    assert_contract(
        &help,
        "schema json <scena.*.vN>",
        &["scena.json_schema_export.v1"],
        &["scena.cli_error.v1"],
    );
    assert_contract(
        &help,
        "validate <file>",
        &["scena.contract_validation.v1"],
        &["scena.contract_validation.v1", "scena.cli_error.v1"],
    );
    assert_contract(
        &help,
        "capabilities [--live] [--json]",
        &["scena.capability_report.v1"],
        &["scena.capability_report.v1", "scena.cli_error.v1"],
    );
    assert_contract(
        &help,
        "recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--max-imports <n>]",
        &[
            "scena.render_introspection.v1",
            "scena.recipe_render_result.v1",
        ],
        &[
            "scena.recipe_render_result.v1",
            "scena.scene_recipe_validation.v1",
            "scena.cli_error.v1",
        ],
    );
    assert_contract(
        &help,
        "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
        &["scena.cad_inspection_result.v1"],
        &["scena.recipe_render_result.v1", "scena.cli_error.v1"],
    );
    assert_contract(
        &help,
        "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
        &["scena.capture_sequence_result.v1"],
        &[
            "scena.recipe_render_result.v1",
            "scena.scene_recipe_validation.v1",
            "scena.cli_error.v1",
        ],
    );
    assert_contract(
        &help,
        "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
        &["scena.semantic_aov_result.v1"],
        &[
            "scena.recipe_render_result.v1",
            "scena.scene_recipe_validation.v1",
            "scena.cli_error.v1",
        ],
    );
    assert_contract(
        &help,
        "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
        &["scena.scene_recipe_diff_result.v1"],
        &[
            "scena.scene_recipe_validation.v1",
            "scena.scene_recipe_build.v1",
            "scena.cli_error.v1",
        ],
    );
    assert_contract(
        &help,
        "photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>]",
        &["scena.photo_plan.v1"],
        &["scena.cli_error.v1"],
    );
    assert_contract(
        &help,
        "photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--max-imports <n>]",
        &["scena.photo_render_result.v1", "scena.photo_report.v1"],
        &["scena.photo_render_result.v1", "scena.cli_error.v1"],
    );
    for (command, success) in [
        (
            "render <asset-or-recipe> --out <png> [--introspect] [--gpu]",
            "scena.render_introspection.v1",
        ),
        ("inspect <asset-or-recipe>", "scena.scene_inspection.v1"),
        (
            "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
            "scena.visibility_diagnosis.v1",
        ),
    ] {
        assert_contract(
            &help,
            command,
            &[success],
            &[
                "scena.asset_doctor.v1",
                "scena.recipe_build_result.v1",
                "scena.scene_recipe_validation.v1",
                "scena.cli_error.v1",
            ],
        );
    }
    for (command, success) in [
        (
            "repair <asset-or-recipe> --from <report.json>",
            vec!["scena.visual_repair_plan.v1", "scena.agent_loop_result.v1"],
        ),
        (
            "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
            vec!["scena.appearance_introspection.v1"],
        ),
        (
            "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
            vec!["scena.animation_introspection.v1"],
        ),
        (
            "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
            vec!["scena.interaction_verification.v1"],
        ),
    ] {
        assert_contract(
            &help,
            command,
            &success,
            &[
                "scena.recipe_build_result.v1",
                "scena.scene_recipe_validation.v1",
                "scena.cli_error.v1",
            ],
        );
    }

    assert_contract(
        &help,
        "doctor <asset-or-recipe>",
        &["scena.asset_doctor.v1", "scena.recipe_build_result.v1"],
        &[
            "scena.asset_doctor.v1",
            "scena.recipe_build_result.v1",
            "scena.scene_recipe_validation.v1",
            "scena.cli_error.v1",
        ],
    );
}

#[test]
fn fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas() {
    let root = fixture_dir("polymorphic");
    let invalid = root.join("invalid.recipe.json");
    fs::write(
        &invalid,
        r#"{"schema":"scena.scene_recipe.v1","importe":[]}"#,
    )
    .expect("invalid recipe writes");
    let invalid_png = root.join("invalid.png");
    for args in [
        vec![
            "place",
            path_str(&invalid),
            "--import",
            "part",
            "--verb",
            "center",
        ],
        vec![
            "render",
            path_str(&invalid),
            "--introspect",
            "--out",
            path_str(&invalid_png),
        ],
        vec!["inspect", path_str(&invalid)],
        vec!["diagnose", path_str(&invalid), "--visibility"],
        vec!["doctor", path_str(&invalid)],
    ] {
        assert_failure_stdout_schema(run(&args), "scena.scene_recipe_validation.v1");
    }

    let appearance_expectation = root.join("appearance.json");
    fs::write(
        &appearance_expectation,
        r#"{"schema":"scena.appearance_expectation.v1","targets":[]}"#,
    )
    .expect("appearance expectation writes");
    assert_failure_stdout_schema(
        run(&[
            "verify",
            "appearance",
            path_str(&invalid),
            "--expect",
            path_str(&appearance_expectation),
        ]),
        "scena.scene_recipe_validation.v1",
    );
    assert_failure_stdout_schema(
        run(&[
            "verify",
            "animation",
            path_str(&invalid),
            "--clip",
            "missing",
            "--times",
            "0",
        ]),
        "scena.scene_recipe_validation.v1",
    );
    let interaction_expectation = root.join("interaction.json");
    fs::write(
        &interaction_expectation,
        r#"{"schema":"scena.interaction_expectation.v1","viewport":{"width_css_px":64.0,"height_css_px":64.0,"device_pixel_ratio":1.0},"steps":[{"action":"pick","x_css_px":1.0,"y_css_px":1.0,"expect_hit":false}]}"#,
    )
    .expect("interaction expectation writes");
    assert_failure_stdout_schema(
        run(&[
            "verify",
            "interaction",
            path_str(&invalid),
            "--expect",
            path_str(&interaction_expectation),
        ]),
        "scena.scene_recipe_validation.v1",
    );
    assert_failure_stdout_schema(
        run(&[
            "repair",
            path_str(&invalid),
            "--from",
            path_str(&root.join("unused-report.json")),
        ]),
        "scena.scene_recipe_validation.v1",
    );

    let recipe_png = root.join("recipe.png");
    assert_failure_stdout_schema(
        run(&["recipe", "build", path_str(&invalid)]),
        "scena.recipe_build_result.v1",
    );
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "render",
            path_str(&invalid),
            "--introspect",
            "--out",
            path_str(&recipe_png),
        ]),
        "scena.recipe_render_result.v1",
    );
    let capture_dir = root.join("capture");
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "capture",
            path_str(&invalid),
            "--out-dir",
            path_str(&capture_dir),
        ]),
        "scena.recipe_render_result.v1",
    );
    let aov_dir = root.join("aov");
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "aov",
            path_str(&invalid),
            "--out-dir",
            path_str(&aov_dir),
        ]),
        "scena.recipe_render_result.v1",
    );
    let cad_dir = root.join("cad");
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "inspect-cad",
            path_str(&invalid),
            "--out-dir",
            path_str(&cad_dir),
        ]),
        "scena.recipe_render_result.v1",
    );

    let oversized = root.join("oversized.recipe.json");
    let mut text = r#"{"schema":"scena.scene_recipe.v1","imports":[]}"#.to_owned();
    text.push_str(&" ".repeat(8 * 1024 * 1024));
    fs::write(&oversized, text).expect("oversized recipe writes");
    let oversized_png = root.join("oversized.png");
    assert_failure_stdout_schema(
        run(&["recipe", "build", path_str(&oversized)]),
        "scena.scene_recipe_validation.v1",
    );
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "capture",
            path_str(&oversized),
            "--out-dir",
            path_str(&capture_dir),
        ]),
        "scena.scene_recipe_validation.v1",
    );
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "aov",
            path_str(&oversized),
            "--out-dir",
            path_str(&aov_dir),
        ]),
        "scena.scene_recipe_validation.v1",
    );
    assert_failure_stdout_schema(
        run(&[
            "recipe",
            "render",
            path_str(&oversized),
            "--introspect",
            "--out",
            path_str(&oversized_png),
        ]),
        "scena.scene_recipe_validation.v1",
    );
}

#[test]
fn fr04_each_command_has_a_real_structured_argument_error_fixture() {
    for args in [
        vec!["--version", "unexpected"],
        vec!["schema", "list", "unexpected"],
        vec!["schema", "get"],
        vec!["schema", "json"],
        vec!["guide", "agent", "--unexpected"],
        vec!["vocab", "list", "unexpected"],
        vec!["vocab", "get"],
        vec!["capabilities", "unexpected"],
        vec!["policy", "recipe", "unexpected"],
        vec!["validate"],
        vec!["validate-recipe"],
        vec!["place"],
        vec!["photo", "render"],
        vec!["recipe", "build"],
        vec!["recipe", "render"],
        vec!["recipe", "inspect-cad"],
        vec!["recipe", "capture"],
        vec!["recipe", "aov"],
        vec!["examples", "agent"],
        vec!["render"],
        vec!["inspect"],
        vec!["diagnose"],
        vec!["doctor"],
        vec!["browser-proof", "unknown-lane", "--dry-run"],
        vec!["repair"],
        vec!["verify", "appearance"],
        vec!["verify", "animation"],
        vec!["verify", "interaction"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "args={args:?}");
        let report: serde_json::Value =
            serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
                panic!(
                    "args={args:?} stderr is not JSON: {error}; stderr={}",
                    String::from_utf8_lossy(&output.stderr)
                )
            });
        assert_eq!(report["schema"], "scena.cli_error.v1", "args={args:?}");
    }
}

#[test]
fn fr04_vocab_get_has_a_real_success_fixture() {
    let output = run(&["vocab", "get", "render_backends"]);
    assert!(output.status.success());
    assert_stdout_schema(output, "scena.vocab.v1");
}

#[test]
fn fr04_validate_recipe_has_a_real_success_fixture() {
    let output = run(&[
        "validate-recipe",
        "tests/assets/schema-field-model/scene_recipe_roundtrip.v1.json",
    ]);
    assert!(output.status.success());
    assert_stdout_schema(output, "scena.scene_recipe_validation.v1");
}

#[test]
fn fr04_every_declared_output_schema_has_real_cli_fixture_evidence() {
    let help = help_report();
    for contract in help["command_contracts"]
        .as_array()
        .expect("command contracts are an array")
    {
        let command = contract["command"].as_str().expect("command is a string");
        let evidence_command = command.replace(" [--allow-root <directory>]...", "");
        for outcome in ["success", "error"] {
            for schema in contract["emits"][outcome]
                .as_array()
                .expect("emits set is an array")
            {
                let schema = schema.as_str().expect("schema is a string");
                if schema == "scena.cli_error.v1" {
                    continue;
                }
                let evidence = EVIDENCE.iter().find(|row| {
                    row.command == evidence_command
                        && row.outcome == outcome
                        && row.schema == schema
                });
                let evidence = evidence.unwrap_or_else(|| {
                    panic!("missing real CLI fixture evidence for {command} {outcome} {schema}")
                });
                let source = fs::read_to_string(evidence.source)
                    .unwrap_or_else(|error| panic!("{} reads: {error}", evidence.source));
                assert!(
                    source.contains(&format!("fn {}", evidence.test)),
                    "{} does not contain test {}",
                    evidence.source,
                    evidence.test
                );
                assert!(
                    source.contains("CARGO_BIN_EXE_scena") && source.contains(schema),
                    "{} must execute the real CLI and assert {schema}",
                    evidence.source
                );
            }
        }
    }
}

struct Evidence {
    command: &'static str,
    outcome: &'static str,
    schema: &'static str,
    source: &'static str,
    test: &'static str,
}

macro_rules! evidence {
    ($command:literal, $outcome:literal, $schema:literal, $source:literal, $test:literal) => {
        Evidence {
            command: $command,
            outcome: $outcome,
            schema: $schema,
            source: $source,
            test: $test,
        }
    };
}

const EVIDENCE: &[Evidence] = &[
    evidence!(
        "--version",
        "success",
        "scena.cli_version.v1",
        "tests/scena_cli_schema.rs",
        "scena_version_cli_reports_package_version_and_commit_field"
    ),
    evidence!(
        "schema list",
        "success",
        "scena.schema_catalog.v1",
        "tests/scena_cli_schema.rs",
        "scena_schema_cli_lists_and_gets_stable_contracts"
    ),
    evidence!(
        "schema get <scena.*.vN>",
        "success",
        "scena.schema_entry.v1",
        "tests/scena_cli_schema.rs",
        "scena_schema_cli_lists_and_gets_stable_contracts"
    ),
    evidence!(
        "schema json <scena.*.vN>",
        "success",
        "scena.json_schema_export.v1",
        "tests/a09_generic_validation.rs",
        "schema_json_exports_recipe_schema_and_declares_runtime_limits"
    ),
    evidence!(
        "guide agent [--json|--markdown]",
        "success",
        "scena.agent_guide.v1",
        "tests/a05_public_agent_guide.rs",
        "installed_cli_exports_public_agent_guidance_outside_the_repository"
    ),
    evidence!(
        "vocab list",
        "success",
        "scena.vocab.v1",
        "tests/scena_cli_schema.rs",
        "fr01_vocab_and_fr04_policy_are_machine_discoverable"
    ),
    evidence!(
        "vocab get <name>",
        "success",
        "scena.vocab.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_vocab_get_has_a_real_success_fixture"
    ),
    evidence!(
        "capabilities [--live] [--json]",
        "success",
        "scena.capability_report.v1",
        "tests/a03_capabilities_cli.rs",
        "static_capabilities_are_explicitly_no_device_and_json_alias_matches"
    ),
    evidence!(
        "capabilities [--live] [--json]",
        "error",
        "scena.capability_report.v1",
        "tests/a03_capabilities_cli.rs",
        "live_capabilities_are_measured_or_fail_closed_with_a_structured_reason"
    ),
    evidence!(
        "policy recipe",
        "success",
        "scena.recipe_policy.v1",
        "tests/scena_cli_schema.rs",
        "fr01_vocab_and_fr04_policy_are_machine_discoverable"
    ),
    evidence!(
        "validate <file>",
        "success",
        "scena.contract_validation.v1",
        "tests/a09_generic_validation.rs",
        "validate_dispatches_public_input_contracts_by_embedded_schema"
    ),
    evidence!(
        "validate <file>",
        "error",
        "scena.contract_validation.v1",
        "tests/a09_generic_validation.rs",
        "validate_fails_closed_for_malformed_unknown_and_mismatched_contracts"
    ),
    evidence!(
        "validate-recipe <recipe.json> [--full|--syntax-only] [--max-imports <n>]",
        "success",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_validate_recipe_has_a_real_success_fixture"
    ),
    evidence!(
        "validate-recipe <recipe.json> [--full|--syntax-only] [--max-imports <n>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/scena_cli_recipe.rs",
        "scena_validate_recipe_cli_checks_asset_presence_and_expected_extents"
    ),
    evidence!(
        "place <recipe.json> (--import <id>|--node <id>) --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
        "success",
        "scena.placement_result.v1",
        "tests/scena_cli_recipe.rs",
        "scena_place_cli_emits_bounds_based_transform_previews_for_recipe_import"
    ),
    evidence!(
        "place <recipe.json> (--import <id>|--node <id>) --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
        "success",
        "scena.recipe_patch.v1",
        "tests/scena_cli_recipe.rs",
        "fr03_place_apply_emits_persistent_recipe_and_rejects_stale_source"
    ),
    evidence!(
        "place <recipe.json> (--import <id>|--node <id>) --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
        "error",
        "scena.placement_result.v1",
        "tests/scena_cli_recipe.rs",
        "scena_recipe_invalid_fixtures_cover_landed_failure_families"
    ),
    evidence!(
        "place <recipe.json> (--import <id>|--node <id>) --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
        "error",
        "scena.recipe_patch.v1",
        "tests/scena_cli_recipe.rs",
        "fr03_place_apply_emits_persistent_recipe_and_rejects_stale_source"
    ),
    evidence!(
        "place <recipe.json> (--import <id>|--node <id>) --verb <verb> [--apply] [--expect-source-sha256 <hex>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe build <recipe.json> [--max-imports <n>]",
        "success",
        "scena.recipe_build_result.v1",
        "tests/fr02_recipe_build_cli.rs",
        "fr02_recipe_build_emits_manifest_policy_and_zero_render_execution"
    ),
    evidence!(
        "recipe build <recipe.json> [--max-imports <n>]",
        "error",
        "scena.recipe_build_result.v1",
        "tests/fr02_recipe_build_cli.rs",
        "fr02_recipe_build_reports_broken_asset_and_policy_denial_without_rendering"
    ),
    evidence!(
        "recipe build <recipe.json> [--max-imports <n>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--max-imports <n>]",
        "success",
        "scena.render_introspection.v1",
        "tests/scena_cli_recipe.rs",
        "scena_recipe_render_introspect_succeeds_without_verify"
    ),
    evidence!(
        "recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--max-imports <n>]",
        "success",
        "scena.recipe_render_result.v1",
        "tests/scena_cli_recipe.rs",
        "scena_recipe_render_verify_passes_color_pick_and_fit_expectations"
    ),
    evidence!(
        "recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--max-imports <n>]",
        "error",
        "scena.recipe_render_result.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--max-imports <n>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
        "success",
        "scena.cad_inspection_result.v1",
        "tests/scena_cli_recipe.rs",
        "scena_recipe_inspect_cad_generates_reviewable_feature_views"
    ),
    evidence!(
        "recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu]",
        "error",
        "scena.recipe_render_result.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
        "success",
        "scena.capture_sequence_result.v1",
        "tests/fr05_capture_sequence.rs",
        "fr05_recipe_capture_emits_canonical_turntable_and_clip_frames"
    ),
    evidence!(
        "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
        "error",
        "scena.recipe_render_result.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
        "success",
        "scena.semantic_aov_result.v1",
        "tests/fr06_semantic_aov.rs",
        "fr06_recipe_aov_cli_writes_portable_images_and_persistent_legend"
    ),
    evidence!(
        "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
        "success",
        "scena.scene_recipe_diff_result.v1",
        "tests/fr07_recipe_diff.rs",
        "fr07_diff_cli_keeps_structural_diff_renderer_free"
    ),
    evidence!(
        "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr07_recipe_diff.rs",
        "fr07_diff_cli_emits_declared_validation_and_build_failure_schemas"
    ),
    evidence!(
        "diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]",
        "error",
        "scena.scene_recipe_build.v1",
        "tests/fr07_recipe_diff.rs",
        "fr07_diff_cli_emits_declared_validation_and_build_failure_schemas"
    ),
    evidence!(
        "photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>]",
        "success",
        "scena.photo_plan.v1",
        "tests/photo_render_cli.rs",
        "photo_plan_camera_behavior_emits_render_free_public_plan_for_imported_asset"
    ),
    evidence!(
        "photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--max-imports <n>]",
        "success",
        "scena.photo_render_result.v1",
        "tests/photo_render_cli.rs",
        "photo_render_camera_behavior_is_easy_path_for_imported_asset"
    ),
    evidence!(
        "photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--max-imports <n>]",
        "success",
        "scena.photo_report.v1",
        "tests/photo_render_cli.rs",
        "photo_render_camera_behavior_is_easy_path_for_imported_asset"
    ),
    evidence!(
        "photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--max-imports <n>]",
        "error",
        "scena.photo_render_result.v1",
        "tests/photo_render_cli.rs",
        "photo_render_reports_recipe_build_failure_in_photo_envelope"
    ),
    evidence!(
        "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
        "error",
        "scena.recipe_render_result.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "recipe capture <recipe.json> --out-dir <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip <name> --frames <n>] [--gpu] [--max-imports <n>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "examples agent list",
        "success",
        "scena.agent_template_catalog.v1",
        "tests/a04_cli_ergonomics.rs",
        "template_catalog_has_one_canonical_name_and_aliases_emit_migration_metadata"
    ),
    evidence!(
        "examples agent get <template> [--out <dir>]",
        "success",
        "scena.agent_smoke_template.v1",
        "tests/scena_cli_agent_templates.rs",
        "scena_examples_agent_templates_generate_and_run_cli_smoke_commands"
    ),
    evidence!(
        "render <asset-or-recipe> --out <png> [--introspect] [--gpu]",
        "success",
        "scena.render_introspection.v1",
        "tests/scena_cli_agent.rs",
        "scena_render_cli_writes_png_descriptor_and_introspection_json"
    ),
    evidence!(
        "render <asset-or-recipe> --out <png> [--introspect] [--gpu]",
        "error",
        "scena.asset_doctor.v1",
        "tests/scena_cli_agent.rs",
        "scena_cli_missing_assets_emit_json_not_command_errors"
    ),
    evidence!(
        "render <asset-or-recipe> --out <png> [--introspect] [--gpu]",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "render <asset-or-recipe> --out <png> [--introspect] [--gpu]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "inspect <asset-or-recipe>",
        "success",
        "scena.scene_inspection.v1",
        "tests/scena_cli_agent.rs",
        "scena_inspect_cli_emits_scene_inspection_json_for_asset"
    ),
    evidence!(
        "inspect <asset-or-recipe>",
        "error",
        "scena.asset_doctor.v1",
        "tests/scena_cli_agent.rs",
        "scena_cli_missing_assets_emit_json_not_command_errors"
    ),
    evidence!(
        "inspect <asset-or-recipe>",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "inspect <asset-or-recipe>",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
        "success",
        "scena.visibility_diagnosis.v1",
        "tests/scena_cli_agent.rs",
        "scena_diagnose_cli_emits_json_and_nonzero_for_invisible_target"
    ),
    evidence!(
        "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
        "error",
        "scena.asset_doctor.v1",
        "tests/scena_cli_agent.rs",
        "scena_cli_missing_assets_emit_json_not_command_errors"
    ),
    evidence!(
        "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "doctor <asset-or-recipe>",
        "success",
        "scena.asset_doctor.v1",
        "tests/scena_cli_agent.rs",
        "scena_doctor_cli_stdout_matches_golden_fixture"
    ),
    evidence!(
        "doctor <asset-or-recipe>",
        "success",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "imports_only_recipe_commands_build_every_import"
    ),
    evidence!(
        "doctor <asset-or-recipe>",
        "error",
        "scena.asset_doctor.v1",
        "tests/scena_cli_agent.rs",
        "scena_doctor_cli_emits_json_and_nonzero_for_broken_asset"
    ),
    evidence!(
        "doctor <asset-or-recipe>",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "doctor <asset-or-recipe>",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]",
        "success",
        "scena.browser_proof_run.v1",
        "tests/scena_cli_browser_proof.rs",
        "scena_browser_proof_dry_run_reports_scene_host_command"
    ),
    evidence!(
        "repair <asset-or-recipe> --from <report.json>",
        "success",
        "scena.visual_repair_plan.v1",
        "tests/scena_cli_agent.rs",
        "scena_repair_cli_plans_visual_patch_from_diagnosis_json"
    ),
    evidence!(
        "repair <asset-or-recipe> --from <report.json>",
        "success",
        "scena.agent_loop_result.v1",
        "tests/scena_cli_agent.rs",
        "scena_repair_cli_exits_nonzero_for_irreducible_diagnosis"
    ),
    evidence!(
        "repair <asset-or-recipe> --from <report.json>",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "repair <asset-or-recipe> --from <report.json>",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
        "success",
        "scena.appearance_introspection.v1",
        "tests/scena_cli_agent.rs",
        "scena_verify_appearance_cli_checks_variant_color_and_fails_closed"
    ),
    evidence!(
        "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "verify appearance <asset-or-recipe> --expect <appearance-expectation.json>",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
        "success",
        "scena.animation_introspection.v1",
        "tests/scena_cli_agent.rs",
        "scena_verify_animation_cli_checks_sampled_change_and_fails_closed"
    ),
    evidence!(
        "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "verify animation <asset-or-recipe> --clip <name> --times <seconds> [--expect-change] [--expect-translations 'x,y,z;...']",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
    evidence!(
        "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
        "success",
        "scena.interaction_verification.v1",
        "tests/scena_cli_interaction.rs",
        "scena_verify_interaction_cli_runs_synthetic_select_and_fails_wrong_handle"
    ),
    evidence!(
        "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
        "error",
        "scena.recipe_build_result.v1",
        "tests/scena_cli_recipe.rs",
        "recipe_commands_check_policy_for_every_import"
    ),
    evidence!(
        "verify interaction <asset-or-recipe> --expect <interaction-expectation.json>",
        "error",
        "scena.scene_recipe_validation.v1",
        "tests/fr04_cli_schema_matrix.rs",
        "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas"
    ),
];

fn help_report() -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena --help runs");
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("help is JSON")
}

fn assert_contract(help: &serde_json::Value, command: &str, success: &[&str], error: &[&str]) {
    let contract = help["command_contracts"]
        .as_array()
        .and_then(|contracts| {
            contracts.iter().find(|row| {
                row["command"].as_str().is_some_and(|declared| {
                    declared == command
                        || declared.strip_suffix(" [--allow-root <directory>]...") == Some(command)
                })
            })
        })
        .unwrap_or_else(|| panic!("missing command contract for {command}: {help:#}"));
    assert_eq!(contract["emits"]["success"], strings(success), "{command}");
    assert_eq!(contract["emits"]["error"], strings(error), "{command}");
}

fn strings(values: &[&str]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|value| serde_json::Value::String((*value).to_owned()))
            .collect(),
    )
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("scena {args:?} runs: {error}"))
}

fn assert_stdout_schema(output: Output, schema: &str) {
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "stdout is not JSON: {error}; stdout={}; stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
    assert_eq!(report["schema"], schema, "report={report:#}");
}

fn assert_failure_stdout_schema(output: Output, schema: &str) {
    assert!(!output.status.success(), "expected {schema} failure output");
    assert_stdout_schema(output, schema);
}

fn fixture_dir(name: &str) -> PathBuf {
    let path = PathBuf::from("target/fr04-cli-schema-matrix").join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("fixture directory creates");
    path
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("fixture path is UTF-8")
}
