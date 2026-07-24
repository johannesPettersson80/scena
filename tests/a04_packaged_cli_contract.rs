#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn packaged_cli_matches_the_declared_install_feature_contract() {
    let scena = std::env::var_os("SCENA_A04_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_scena")));
    let expect_agent = std::env::var("SCENA_A04_EXPECT_AGENT")
        .ok()
        .map(|value| value == "1")
        .unwrap_or(cfg!(feature = "agent"));
    let work = unique_temp_dir();
    fs::create_dir(&work).expect("packaged CLI temp directory creates");

    let version = run(&scena, &work, &["--version"]);
    assert!(version.status.success());
    let version = stdout_json(&version);
    assert_eq!(version["features"]["agent"], expect_agent);

    for args in [["--help"].as_slice(), ["schema", "list"].as_slice()] {
        let output = run(&scena, &work, args);
        assert!(output.status.success(), "args={args:?}");
        assert!(stdout_json(&output)["schema"].as_str().is_some());
    }

    let recipe = work.join("minimal.recipe.json");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "capture": {"width": 64, "height": 48}
        }))
        .expect("minimal recipe serializes"),
    )
    .expect("minimal recipe writes");
    let recipe_text = recipe.to_str().expect("recipe path is UTF-8");
    let validate = run(&scena, &work, &["validate-recipe", recipe_text]);
    assert_eq!(
        stdout_json(&validate)["schema"],
        "scena.scene_recipe_validation.v1"
    );

    if expect_agent {
        assert_agent_surface(&scena, &work);
    } else {
        for args in [
            vec!["examples", "agent", "list"],
            vec!["render", recipe_text, "--out", "frame.png"],
            vec!["inspect", recipe_text],
            vec!["diagnose", recipe_text, "--visibility"],
            vec!["repair", recipe_text, "--from", "report.json"],
        ] {
            let output = run(&scena, &work, &args);
            assert_eq!(output.status.code(), Some(69), "args={args:?}");
            let error = stderr_json(&output);
            assert_eq!(error["schema"], "scena.cli_error.v1");
            assert_eq!(error["code"], "feature_unavailable");
            assert_eq!(error["exit_class"], "unsupported");
            let message = error["message"].as_str().expect("remedy is text");
            assert!(message.contains("cargo install scena --features agent"));
            assert!(!message.contains("scene-host,inspection"));
        }
    }
    fs::remove_dir_all(work).expect("packaged CLI temp directory removes");
}

fn assert_agent_surface(scena: &Path, work: &Path) {
    let templates = run(scena, work, &["examples", "agent", "list"]);
    assert!(templates.status.success());
    assert_eq!(
        stdout_json(&templates)["schema"],
        "scena.agent_template_catalog.v1"
    );

    let generated = work.join("generated");
    let generated_text = generated.to_str().expect("generated path is UTF-8");
    let template = run(
        scena,
        work,
        &[
            "examples",
            "agent",
            "get",
            "primitive-scene",
            "--out",
            generated_text,
        ],
    );
    assert!(
        template.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&template.stderr)
    );
    let recipe = generated.join("recipe.json");
    let recipe = recipe.to_str().expect("recipe path is UTF-8");
    let frame = work.join("frame.png");
    let frame = frame.to_str().expect("frame path is UTF-8");
    let render_report = work.join("render.json");

    let commands: &[(&[&str], &str)] = &[
        (
            &["recipe", "render", recipe, "--introspect", "--out", frame],
            "scena.render_introspection.v1",
        ),
        (&["inspect", recipe], "scena.scene_inspection.v1"),
        (
            &["diagnose", recipe, "--visibility"],
            "scena.visibility_diagnosis.v1",
        ),
        (&["doctor", recipe], "scena.recipe_build_result.v1"),
    ];
    for (args, schema) in commands {
        let output = run(scena, work, args);
        assert!(
            output.status.success(),
            "args={args:?} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = stdout_json(&output);
        assert_eq!(report["schema"], *schema, "args={args:?}");
        if *schema == "scena.render_introspection.v1" {
            fs::write(
                &render_report,
                serde_json::to_vec_pretty(&report).expect("render report serializes"),
            )
            .expect("render report writes");
        }
    }
    let repair_report = render_report.to_str().expect("report path is UTF-8");
    let repair = run(scena, work, &["repair", recipe, "--from", repair_report]);
    let report = stdout_json(&repair);
    assert!(matches!(
        report["schema"].as_str(),
        Some("scena.visual_repair_plan.v1") | Some("scena.agent_loop_result.v1")
    ));
}

fn run(program: &Path, work: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(work)
        .output()
        .unwrap_or_else(|error| panic!("{} {args:?} runs: {error}", program.display()))
}

fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is JSON: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn stderr_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "stderr is JSON: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("scena-a04-package-{}-{nonce}", std::process::id()))
}
