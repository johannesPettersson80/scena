#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn wasm_host_recipe_validation_rejects_non_ascii_hex_without_runtime_abort() {
    let report = scena::validate_scene_recipe_json(
        r#"{"schema":"scena.scene_recipe.v1","colors":{"bad":"€abc"}}"#,
    );
    assert!(!report.ok);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_color" && diagnostic.path == "$.colors.bad"
    }));
    let json = serde_json::to_string(&report).expect("WASM validation report serializes");
    assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
}
