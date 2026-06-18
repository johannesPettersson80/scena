use std::process::Command;

#[test]
fn scena_help_points_to_llm_app_builder_guide() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--help")
        .output()
        .expect("scena --help runs");

    assert!(
        output.status.success(),
        "scena --help should succeed, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let help: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("scena --help emits JSON");
    let guides = help
        .get("guides")
        .and_then(serde_json::Value::as_array)
        .expect("help JSON exposes guide pointers");

    assert!(
        guides.iter().any(|guide| {
            guide.get("name").and_then(serde_json::Value::as_str) == Some("llm-app-builder")
                && guide.get("path").and_then(serde_json::Value::as_str)
                    == Some("docs/guides/llm-app-builder.md")
        }),
        "help JSON should point to the LLM app-builder guide, stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
