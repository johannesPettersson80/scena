use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn scena_bin() -> std::path::PathBuf {
    std::env::var_os("SCENA_A05_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_BIN_EXE_scena")))
}

#[test]
fn installed_cli_exports_public_agent_guidance_outside_the_repository() {
    let work = std::env::temp_dir().join(format!(
        "scena-a05-guide-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos()
    ));
    fs::create_dir(&work).expect("guide temp directory creates");

    let json = Command::new(scena_bin())
        .args(["guide", "agent", "--json"])
        .current_dir(&work)
        .output()
        .expect("agent JSON guide command runs");
    assert!(
        json.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&json.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("agent guide is JSON");
    assert_eq!(report["schema"], "scena.agent_guide.v1");
    assert_eq!(report["name"], "llm-app-builder");
    assert!(
        report["markdown"]
            .as_str()
            .is_some_and(|value| value.starts_with("# LLM App Builder Guide"))
    );
    for key in ["commands", "schemas", "policies", "templates"] {
        assert!(
            report[key]
                .as_array()
                .is_some_and(|values| !values.is_empty()),
            "missing {key}: {report:#}"
        );
    }

    let markdown = Command::new(scena_bin())
        .args(["guide", "agent", "--markdown"])
        .current_dir(&work)
        .output()
        .expect("agent Markdown guide command runs");
    assert!(markdown.status.success());
    assert!(String::from_utf8_lossy(&markdown.stdout).starts_with("# LLM App Builder Guide"));
    fs::remove_dir_all(work).expect("guide temp directory removes");
}
