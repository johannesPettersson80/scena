use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    assert_schema(&catalog, "scena.visibility_diagnosis.v1");
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

fn assert_schema(catalog: &serde_json::Value, schema: &str) {
    let entries = catalog["entries"]
        .as_array()
        .expect("schema catalog has entries");
    assert!(
        entries.iter().any(|entry| entry["schema"] == schema),
        "catalog missing {schema}: {catalog:#}"
    );
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
