#![cfg(feature = "agent")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn render_commands_emit_introspection_without_the_compatibility_flag() {
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/recipe-invalid/valid_for_commands.recipe.json");
    let output_dir = unique_temp_dir();
    fs::create_dir(&output_dir).expect("output directory creates");

    for (name, prefix) in [
        ("render", vec!["render"]),
        ("recipe", vec!["recipe", "render"]),
    ] {
        let png = output_dir.join(format!("{name}.png"));
        let mut args = prefix;
        args.push(recipe.to_str().expect("recipe path is UTF-8"));
        args.extend(["--out", png.to_str().expect("PNG path is UTF-8")]);
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(&args)
            .output()
            .expect("render command runs");
        assert!(
            output.status.success(),
            "args={args:?} status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("render output is JSON");
        assert_eq!(report["schema"], "scena.render_introspection.v1");
        assert_eq!(report["ok"], true, "args={args:?} report={report:#}");
        assert!(png.exists(), "args={args:?} writes PNG");
    }

    fs::remove_dir_all(output_dir).expect("output directory removes");
}

#[test]
fn introspect_flag_remains_an_accepted_no_op() {
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/recipe-invalid/valid_for_commands.recipe.json");
    let output_dir = unique_temp_dir();
    fs::create_dir(&output_dir).expect("output directory creates");
    let png = output_dir.join("compat.png");
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            recipe.to_str().expect("recipe path is UTF-8"),
            "--introspect",
            "--out",
            png.to_str().expect("PNG path is UTF-8"),
        ])
        .output()
        .expect("compatibility render runs");
    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render output is JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    fs::remove_dir_all(output_dir).expect("output directory removes");
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "scena-a08-introspection-{}-{nonce}",
        std::process::id()
    ))
}
