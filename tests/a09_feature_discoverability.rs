use std::process::Command;

#[test]
fn manifest_keeps_defaults_empty_and_declares_one_step_agent_composition() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo manifest reads");
    assert!(manifest.contains("default = []"));
    assert!(manifest.contains("scene-host = [\"inspection\"]"));
    assert!(
        manifest.contains("agent = [\"scene-host\"]"),
        "agent must be a composition alias over scene-host, which already enables inspection"
    );
    assert!(!manifest.contains("agent = [\"scene-host\", \"inspection\"]"));
}

#[cfg(feature = "agent")]
#[test]
fn agent_feature_enables_the_complete_self_verification_surface() {
    const { assert!(cfg!(feature = "scene-host")) };
    const { assert!(cfg!(feature = "inspection")) };
    let _ = std::any::type_name::<scena::SceneHostCore<scena::DefaultAssetFetcher>>();
    let _ = std::any::type_name::<scena::SceneInspectionReportV1>();

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .arg("--version")
        .output()
        .expect("agent-feature CLI version runs");
    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("version output is JSON");
    assert_eq!(report["features"]["agent"], true);
    assert_eq!(report["features"]["scene_host"], true);
    assert_eq!(report["features"]["inspection"], true);
}

#[cfg(not(any(feature = "inspection", feature = "scene-host", feature = "agent")))]
#[test]
fn unavailable_agent_commands_name_one_installable_feature_remedy() {
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["recipe", "build", "missing.recipe.json"])
        .output()
        .expect("default-feature CLI runs");
    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("feature error is JSON");
    let message = report["message"].as_str().expect("feature message is text");
    assert!(message.contains("cargo install scena --features agent"));
    assert!(!message.contains("scene-host,inspection"));
}
