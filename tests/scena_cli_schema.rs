use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn scena_version_cli_reports_package_version_and_commit_field() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--version")
        .output()
        .expect("scena --version runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "--version should keep stdout JSON clean and stderr empty, got {}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--version emits JSON");
    assert_eq!(report["schema"], "scena.cli_version.v1");
    assert_eq!(report["package_name"], "scena");
    assert_eq!(report["package_version"], env!("CARGO_PKG_VERSION"));
    assert!(
        report["git_commit"].is_null() || report["git_commit"].as_str().is_some(),
        "git_commit must be a string when pinned at compile time, otherwise null: {report:#}"
    );
}

#[test]
fn scena_schema_cli_lists_and_gets_stable_contracts() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "list"])
        .output()
        .expect("scena schema list runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "schema list should keep stdout JSON clean and stderr empty, got {}",
        stderr(&output)
    );
    let catalog: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema list emits JSON");
    assert_eq!(catalog["schema"], "scena.schema_catalog.v1");
    assert_schema(&catalog, "scena.render_introspection.v1");
    assert_schema(&catalog, "scena.render_quality.v1");
    assert_schema(&catalog, "scena.visibility_diagnosis.v1");
    assert_schema(&catalog, "scena.scene_recipe.v1");
    assert_schema(&catalog, "scena.scene_recipe_validation.v1");
    assert_schema(&catalog, "scena.scene_recipe_build.v1");
    assert_schema(&catalog, "scena.recipe_render_result.v1");
    assert_schema(&catalog, "scena.semantic_aov_result.v1");
    assert_schema(&catalog, "scena.scene_recipe_diff_result.v1");
    assert_schema(&catalog, "scena.placement_result.v1");
    assert_schema(&catalog, "scena.asset_catalog.v1");
    assert_schema(&catalog, "scena.asset_readiness_report.v1");
    assert_schema(&catalog, "scena.asset_doctor.v1");
    assert_schema(&catalog, "scena.visual_repair_plan.v1");
    assert_schema(&catalog, "scena.agent_loop_result.v1");
    assert_schema(&catalog, "scena.agent_smoke_template.v1");
    assert_schema(&catalog, "scena.browser_proof_run.v1");
    assert_schema(&catalog, "scena.animation_introspection.v1");
    assert_schema(&catalog, "scena.interaction_expectation.v1");
    assert_schema(&catalog, "scena.interaction_verification.v1");
    assert_schema(&catalog, "scena.connector_browser.v1");
    assert_schema(&catalog, "scena.product_options.v1");
    assert_schema(&catalog, "scena.presentation_timeline.v1");
    assert_schema(&catalog, "scena.scene_host_grounding.v1");
    assert_schema(&catalog, "scena.scene_host_measurement_overlay.v1");
    assert_listed_fixtures_exist(&catalog);

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "get", "scena.render_introspection.v1"])
        .output()
        .expect("scena schema get runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let entry: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema get emits JSON");
    assert_eq!(entry["schema"], "scena.schema_entry.v1");
    assert_eq!(entry["entry"]["schema"], "scena.render_introspection.v1");
    assert_eq!(entry["example"]["schema"], "scena.render_introspection.v1");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "get", "scena.scene_recipe.v1"])
        .output()
        .expect("scena schema get recipe runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let entry: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema get recipe emits JSON");
    assert_eq!(entry["schema"], "scena.schema_entry.v1");
    assert_eq!(entry["entry"]["schema"], "scena.scene_recipe.v1");
    assert_eq!(entry["example"]["schema"], "scena.scene_recipe.v1");
    assert_eq!(entry["invalid_example"]["schema"], "scena.scene_recipe.v1");
    assert_eq!(entry["invalid_example"]["importe"][0]["id"], "part");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "get", "scena.render_introspect.v1"])
        .output()
        .expect("unknown schema command runs");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("scena.render_introspection.v1"),
        "unknown schema should suggest a near miss, stderr={}",
        stderr(&output)
    );
}

#[test]
fn scena_schema_cli_stdout_matches_golden_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "list"])
        .output()
        .expect("scena schema list runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema list emits JSON");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("assets/cli-golden/schema_list_stdout.json"))
            .expect("golden schema list fixture parses");
    assert_eq!(actual, expected);

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "get", "scena.scene_recipe.v1"])
        .output()
        .expect("scena schema get recipe runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let mut actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema get emits JSON");
    actual
        .as_object_mut()
        .expect("schema entry is an object")
        .remove("field_model");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/cli-golden/schema_get_scene_recipe_stdout.json"
    ))
    .expect("golden schema get fixture parses");
    assert_eq!(actual, expected);
}

#[test]
fn fr01_schema_get_emits_authoritative_recipe_field_model() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "get", "scena.scene_recipe.v1"])
        .output()
        .expect("scena schema get recipe runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema get emits JSON");
    let model = &report["field_model"];
    assert_eq!(model["schema"], "scena.field_model.v1");
    assert_eq!(model["contract"], "scena.scene_recipe.v1");
    let fields = model["fields"].as_array().expect("field model has fields");

    let schema = field(fields, "$.schema");
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["required"], true);
    assert_eq!(schema["enum"], json_array(&["scena.scene_recipe.v1"]));

    let imports = field(fields, "$.imports");
    assert_eq!(imports["type"], "array");
    assert_eq!(imports["required"], false);
    assert_eq!(imports["default"], serde_json::json!([]));

    let primitive_kind = field(fields, "$.geometries[].primitive.kind");
    assert!(
        primitive_kind["enum"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "box"))
    );
    let capture_width = field(fields, "$.capture.width");
    assert_eq!(capture_width["type"], "integer");
    assert_eq!(capture_width["minimum"].as_f64(), Some(1.0));
    assert!(
        capture_width["examples"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
    );
    assert_eq!(capture_width["deprecated"], false);
}

#[test]
fn fr01_field_model_fixtures_round_trip_and_fail_for_declared_constraints() {
    let valid = include_str!("assets/schema-field-model/scene_recipe_roundtrip.v1.json");
    let report = scena::validate_scene_recipe_json(valid);
    assert!(report.ok, "round-trip fixture must validate: {report:#?}");
    let typed: scena::SceneRecipeV1 = serde_json::from_str(valid).expect("fixture decodes");
    let encoded = serde_json::to_string(&typed).expect("typed recipe encodes");
    let report = scena::validate_scene_recipe_json(&encoded);
    assert!(report.ok, "encoded recipe must revalidate: {report:#?}");

    let invalid = include_str!("assets/schema-field-model/scene_recipe_invalid.v1.json");
    let report = scena::validate_scene_recipe_json(invalid);
    assert!(!report.ok, "invalid field-model fixture must fail closed");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unknown_field" && diagnostic.path == "$.importe")
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_capture" && diagnostic.path == "$.capture.width"
    }));
}

#[test]
fn scena_schema_cli_catalog_covers_stable_contract_fixture_schemas() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["schema", "list"])
        .output()
        .expect("scena schema list runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let catalog: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("schema list emits JSON");
    let catalog_schemas = catalog["entries"]
        .as_array()
        .expect("schema catalog has entries")
        .iter()
        .filter_map(|entry| entry["schema"].as_str())
        .collect::<BTreeSet<_>>();

    for entry in fs::read_dir("tests/assets/stable-contracts")
        .expect("stable-contract fixture directory exists")
    {
        let path = entry.expect("fixture dir entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let fixture: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("fixture file reads as UTF-8"))
                .expect("stable-contract fixture parses as JSON");
        let Some(schema) = fixture["schema"].as_str() else {
            continue;
        };
        assert!(
            catalog_schemas.contains(schema),
            "schema catalog missing {schema} for fixture {}",
            path.display()
        );
    }
}

#[test]
fn fr01_vocab_and_fr04_policy_are_machine_discoverable() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["vocab", "list"])
        .output()
        .expect("scena vocab list runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let vocab: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("vocab list emits JSON");
    assert_eq!(vocab["schema"], "scena.vocab.v1");
    for required in [
        "render_backends",
        "recipe_material_kinds",
        "placement_verbs",
        "alpha_modes",
        "texture_color_spaces",
    ] {
        let row = vocab["vocabularies"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["name"] == required))
            .unwrap_or_else(|| panic!("vocab missing {required}: {vocab:#}"));
        assert_eq!(row["version"], 1);
        assert!(row["owner"].as_str().is_some_and(|owner| !owner.is_empty()));
        assert!(
            row["values"]
                .as_array()
                .is_some_and(|values| !values.is_empty())
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["policy", "recipe"])
        .output()
        .expect("scena policy recipe runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let policy: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("policy recipe emits JSON");
    assert_eq!(policy["schema"], "scena.recipe_policy.v1");
    assert_eq!(policy["network"]["allowed"], false);
    assert_eq!(policy["network"]["source"], "compiled_default");
    assert!(policy["allowed_roots"].as_array().is_some_and(|roots| {
        !roots.is_empty()
            && roots.iter().all(|root| {
                root["path"].as_str().is_some_and(|path| !path.is_empty())
                    && root["source"] == "compiled_default"
            })
    }));
    assert_eq!(policy["limits"]["max_imports"]["value"], 64);
    assert_eq!(
        policy["limits"]["max_imports"]["source"],
        "compiled_default"
    );
}

#[test]
fn fr04_machine_help_declares_success_and_error_schemas_per_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena help runs");
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let help: serde_json::Value = serde_json::from_slice(&output.stdout).expect("help emits JSON");
    assert_eq!(help["schema"], "scena.cli_help.v1");
    let contracts = help["command_contracts"]
        .as_array()
        .expect("help has command contracts");
    for command in help["commands"].as_array().expect("help commands") {
        let command = command.as_str().expect("command is a string");
        let contract = contracts
            .iter()
            .find(|contract| contract["command"] == command)
            .unwrap_or_else(|| panic!("missing emits contract for {command}: {help:#}"));
        assert!(
            contract["emits"]["success"]
                .as_array()
                .is_some_and(|schemas| !schemas.is_empty()),
            "{command} must declare success schemas"
        );
        assert!(
            contract["emits"]["error"]
                .as_array()
                .is_some_and(|schemas| !schemas.is_empty()),
            "{command} must declare error schemas"
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("definitely-not-a-command")
        .output()
        .expect("unknown command runs");
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("CLI error is structured JSON");
    assert_eq!(error["schema"], "scena.cli_error.v1");
    assert_eq!(error["ok"], false);
    assert_eq!(error["code"], "invalid_command");
}

fn assert_schema(catalog: &serde_json::Value, schema: &str) {
    let entries = catalog["entries"]
        .as_array()
        .expect("schema catalog has entries");
    assert!(
        entries.iter().any(|entry| entry["schema"] == schema),
        "catalog missing {schema}: {catalog:#}"
    );
}

fn field<'a>(fields: &'a [serde_json::Value], path: &str) -> &'a serde_json::Value {
    fields
        .iter()
        .find(|field| field["path"] == path)
        .unwrap_or_else(|| panic!("field model missing {path}"))
}

fn json_array(values: &[&str]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|value| serde_json::Value::String((*value).to_owned()))
            .collect(),
    )
}

fn assert_listed_fixtures_exist(catalog: &serde_json::Value) {
    let entries = catalog["entries"]
        .as_array()
        .expect("schema catalog has entries");
    for entry in entries {
        let Some(path) = entry["fixture_path"].as_str() else {
            continue;
        };
        assert!(
            Path::new(path).exists(),
            "schema catalog fixture path does not exist: {path}"
        );
    }
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
