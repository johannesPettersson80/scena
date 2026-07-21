#![cfg(feature = "scene-host")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

static TEMPLATE_CLI_LOCK: Mutex<()> = Mutex::new(());

const TEMPLATE_NAMES: &[&str] = &[
    "product-configurator",
    "live-state-viewer",
    "web-viewer",
    "data-visualization",
    "animated-viewer",
    "interaction-proof",
    "cad-inspection",
    "documentation-renderer",
];

const STARTER_SNIPPET_NAMES: &[&str] = &[
    "primitive-scene",
    "cad-plate",
    "dashboard-bars",
    "machine-state-viewer",
    "product-configurator-starter",
];

#[test]
fn scena_examples_agent_templates_generate_and_run_cli_smoke_commands() {
    let _cli_guard = template_cli_guard();
    let root = artifact_dir("agent-templates");

    for name in TEMPLATE_NAMES {
        let output_dir = root.join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(["examples", "agent", name, "--out", path_str(&output_dir)])
            .output()
            .expect("scena examples agent command runs");

        assert!(
            output.status.success(),
            "template {name} failed: {}",
            output_diagnostic(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "template manifest stays machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("template emits JSON");
        assert_eq!(manifest["schema"], "scena.agent_smoke_template.v1");
        assert_eq!(manifest["name"], *name);
        assert_eq!(manifest["status"], "ready");
        assert!(
            manifest["files"]
                .as_array()
                .expect("files array")
                .iter()
                .all(|file| Path::new(file["path"].as_str().expect("file path")).exists()),
            "template files should exist: {manifest:#}"
        );
        assert_template_recipe_has_beauty_defaults(&output_dir.join("recipe.json"), name);
        if *name == "data-visualization" {
            assert!(
                manifest["required_features"]
                    .as_array()
                    .expect("required_features array")
                    .iter()
                    .any(|feature| feature == "scene-host"),
                "authored data visualization verification requires scene-host: {manifest:#}"
            );
            assert_data_visualization_template_targets_authored_blue_mark(&output_dir);
        }

        for command in manifest["commands"].as_array().expect("commands array") {
            let argv = command["argv"]
                .as_array()
                .expect("argv array")
                .iter()
                .map(|value| value.as_str().expect("argv string").to_owned())
                .collect::<Vec<_>>();
            assert_eq!(argv.first().map(String::as_str), Some("scena"));
            if command["name"] == "render_introspect" {
                assert_eq!(
                    argv.get(1).map(String::as_str),
                    Some("recipe"),
                    "agent templates should use canonical recipe render command: {manifest:#}"
                );
                assert_eq!(
                    argv.get(2).map(String::as_str),
                    Some("render"),
                    "agent templates should use canonical recipe render command: {manifest:#}"
                );
            }
            let command_output = Command::new(env!("CARGO_BIN_EXE_scena"))
                .args(&argv[1..])
                .output()
                .unwrap_or_else(|error| panic!("template {name} command {argv:?} runs: {error}"));
            assert!(
                command_output.status.success(),
                "template {name} command {argv:?} failed: {}",
                output_diagnostic(&command_output)
            );
            assert!(
                command_output.stderr.is_empty(),
                "template {name} command {argv:?} keeps JSON on stdout, stderr={}",
                stderr(&command_output)
            );
            let report: serde_json::Value =
                serde_json::from_slice(&command_output.stdout).expect("command emits JSON");
            assert_eq!(report["schema"], command["expected_schema"]);
            assert_eq!(report["ok"], command["expected_ok"]);
            for artifact in command["artifacts"].as_array().expect("artifacts array") {
                assert!(
                    Path::new(artifact.as_str().expect("artifact path")).exists(),
                    "template {name} command {argv:?} missing artifact {artifact:?}"
                );
            }
        }
    }
}

#[test]
fn scena_examples_agent_cli_stdout_matches_golden_fixture() {
    let _cli_guard = template_cli_guard();
    let root = PathBuf::from("target")
        .join("gate-artifacts")
        .join("scena-cli-agent-template-golden");
    let output_dir = root.join("product-configurator");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("golden template root creates");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "examples",
            "agent",
            "product-configurator",
            "--out",
            path_str(&output_dir),
        ])
        .output()
        .expect("scena examples agent golden command runs");

    assert!(
        output.status.success(),
        "golden template command failed: {}",
        output_diagnostic(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "examples agent golden command keeps stderr empty, stderr={}",
        stderr(&output)
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("examples agent emits JSON");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "assets/cli-golden/examples_agent_product_configurator_stdout.json"
    ))
    .expect("golden examples agent fixture parses");
    assert_eq!(actual, expected);
}

#[test]
fn scena_examples_agent_cad_and_documentation_templates_are_runnable_with_overlay_notes() {
    let _cli_guard = template_cli_guard();
    let root = artifact_dir("agent-templates-phase2");

    for name in ["cad-inspection", "documentation-renderer"] {
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args([
                "examples",
                "agent",
                name,
                "--out",
                path_str(&root.join(name)),
            ])
            .output()
            .expect("scena examples agent phase2 command runs");

        assert!(
            output.status.success(),
            "phase2 template {name} failed: {}",
            output_diagnostic(&output)
        );
        assert!(output.stderr.is_empty(), "stderr={}", stderr(&output));
        let manifest: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("template emits JSON");
        assert_eq!(manifest["schema"], "scena.agent_smoke_template.v1");
        assert_eq!(manifest["name"], name);
        assert_eq!(manifest["status"], "ready");
        assert!(
            manifest["commands"]
                .as_array()
                .expect("commands array")
                .len()
                >= 3,
            "CAD/docs template must include runnable CLI smoke commands: {manifest:#}"
        );
        assert!(
            manifest["notes"]
                .as_array()
                .expect("notes array")
                .iter()
                .all(|note| !note
                    .as_str()
                    .is_some_and(|text| text.contains("native SceneHost APIs"))),
            "template should no longer defer overlay authoring to native-only APIs: {manifest:#}"
        );
        assert!(
            manifest["files"]
                .as_array()
                .expect("files array")
                .iter()
                .any(|file| file["kind"] == "recipe" && file["schema"] == "scena.scene_recipe.v1"),
            "template should include a recipe file: {manifest:#}"
        );
        let render_command = manifest["commands"]
            .as_array()
            .expect("commands array")
            .iter()
            .find(|command| command["name"] == "render_introspect")
            .expect("render command exists");
        let argv = render_command["argv"]
            .as_array()
            .expect("argv array")
            .iter()
            .map(|value| value.as_str().expect("argv string").to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            argv.get(1).map(String::as_str),
            Some("recipe"),
            "CAD/docs template should use canonical recipe render command: {manifest:#}"
        );
        assert_eq!(
            argv.get(2).map(String::as_str),
            Some("render"),
            "CAD/docs template should use canonical recipe render command: {manifest:#}"
        );
        let render_output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(&argv[1..])
            .output()
            .expect("template render command runs");
        assert!(
            render_output.status.success(),
            "phase2 render command failed: {}",
            output_diagnostic(&render_output)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&render_output.stdout).expect("render emits JSON");
        assert_eq!(report["schema"], "scena.render_introspection.v1");
        assert_eq!(report["ok"], true);

        let inspect_output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(["inspect", path_str(&root.join(name).join("recipe.json"))])
            .output()
            .expect("template recipe inspects");
        assert!(
            inspect_output.status.success(),
            "phase2 inspect command failed: {}",
            output_diagnostic(&inspect_output)
        );
        let inspection: serde_json::Value =
            serde_json::from_slice(&inspect_output.stdout).expect("inspect emits JSON");
        let draw_list = inspection["draw_list"].as_array().expect("draw list array");
        assert!(
            draw_list
                .iter()
                .any(|draw| draw["material"]["kind"] == "line")
                && inspection["nodes"]
                    .as_array()
                    .expect("nodes array")
                    .iter()
                    .any(|node| node["kind"] == "Label"),
            "overlay recipe should produce line geometry and labels: {inspection:#}"
        );
    }
}

#[test]
fn scena_examples_agent_get_starter_snippets_are_authored_and_runnable() {
    let _cli_guard = template_cli_guard();
    let root = artifact_dir("agent-starter-snippets");

    for name in STARTER_SNIPPET_NAMES {
        let output_dir = root.join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args([
                "examples",
                "agent",
                "get",
                name,
                "--out",
                path_str(&output_dir),
            ])
            .output()
            .expect("scena examples agent get command runs");

        assert!(
            output.status.success(),
            "starter snippet {name} failed: {}",
            output_diagnostic(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "starter snippet {name} keeps JSON on stdout, stderr={}",
            stderr(&output)
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("starter snippet emits JSON");
        assert_eq!(manifest["schema"], "scena.agent_smoke_template.v1");
        assert_eq!(manifest["name"], *name);
        assert_eq!(manifest["status"], "ready");

        let recipe_path = output_dir.join("recipe.json");
        let recipe: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&recipe_path).expect("starter snippet recipe exists"),
        )
        .expect("starter snippet recipe parses");
        assert_eq!(recipe["schema"], "scena.scene_recipe.v1");
        assert!(
            recipe["imports"]
                .as_array()
                .is_none_or(|imports| imports.is_empty()),
            "starter snippet should be authored from scratch, not import-only: {recipe:#}"
        );
        assert!(
            recipe["geometries"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
                && recipe["materials"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
                && recipe["nodes"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
            "starter snippet should contain authored geometry/material/node content: {recipe:#}"
        );
        assert_template_recipe_has_beauty_defaults(&recipe_path, name);

        for command in manifest["commands"].as_array().expect("commands array") {
            if command["name"] != "validate_recipe" && command["name"] != "render_introspect" {
                continue;
            }
            let argv = command["argv"]
                .as_array()
                .expect("argv array")
                .iter()
                .map(|value| value.as_str().expect("argv string").to_owned())
                .collect::<Vec<_>>();
            if command["name"] == "render_introspect" {
                assert_eq!(
                    argv.get(1).map(String::as_str),
                    Some("recipe"),
                    "starter snippet should use canonical recipe render command: {manifest:#}"
                );
                assert_eq!(
                    argv.get(2).map(String::as_str),
                    Some("render"),
                    "starter snippet should use canonical recipe render command: {manifest:#}"
                );
            }
            let command_output = Command::new(env!("CARGO_BIN_EXE_scena"))
                .args(&argv[1..])
                .output()
                .unwrap_or_else(|error| {
                    panic!("starter snippet {name} command {argv:?} runs: {error}")
                });
            assert!(
                command_output.status.success(),
                "starter snippet {name} command {argv:?} failed: {}",
                output_diagnostic(&command_output)
            );
            assert!(
                command_output.stderr.is_empty(),
                "starter snippet {name} command {argv:?} keeps JSON on stdout, stderr={}",
                stderr(&command_output)
            );
            let report: serde_json::Value =
                serde_json::from_slice(&command_output.stdout).expect("command emits JSON");
            assert_eq!(report["schema"], command["expected_schema"]);
            assert_eq!(report["ok"], command["expected_ok"]);
        }
    }
}

#[test]
fn scena_examples_agent_primitive_flow_runs_from_an_unrelated_working_directory() {
    let _cli_guard = template_cli_guard();
    let root = artifact_dir("agent-template-portable-primitive");
    let cwd = root.join("unrelated-working-directory");
    let output_dir = root.join("generated");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&cwd).expect("unrelated working directory creates");

    let generate = Command::new(env!("CARGO_BIN_EXE_scena"))
        .current_dir(&cwd)
        .args([
            "examples",
            "agent",
            "get",
            "primitive-scene",
            "--out",
            path_str(&output_dir),
        ])
        .output()
        .expect("portable starter generation runs");
    assert!(
        generate.status.success(),
        "portable starter generation failed: {}",
        output_diagnostic(&generate)
    );

    let recipe_path = output_dir.join("recipe.json");
    for args in [
        vec!["validate-recipe", path_str(&recipe_path)],
        vec!["recipe", "build", path_str(&recipe_path)],
        vec![
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&output_dir.join("frame.png")),
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .current_dir(&cwd)
            .args(&args)
            .output()
            .unwrap_or_else(|error| panic!("portable command {args:?} runs: {error}"));
        assert!(
            output.status.success(),
            "portable command {args:?} failed: {}",
            output_diagnostic(&output)
        );
    }
}

#[test]
fn scena_examples_agent_defaults_preserve_an_explicit_environment() {
    let _cli_guard = template_cli_guard();
    let root = artifact_dir("agent-template-explicit-environment");
    let _ = fs::remove_dir_all(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "examples",
            "agent",
            "get",
            "product-configurator-starter",
            "--out",
            path_str(&root),
        ])
        .output()
        .expect("product configurator starter generation runs");
    assert!(
        output.status.success(),
        "product configurator starter generation failed: {}",
        output_diagnostic(&output)
    );

    let recipe: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("recipe.json")).expect("starter recipe exists"),
    )
    .expect("starter recipe parses");
    assert_eq!(
        recipe["scene"]["environment"],
        serde_json::json!({ "kind": "default" }),
        "presentation defaults must not overwrite an explicitly authored environment: {recipe:#}"
    );
}

#[test]
fn scena_examples_agent_every_template_runs_end_to_end_outside_a_checkout() {
    let _cli_guard = template_cli_guard();
    let root = artifact_dir("agent-template-portable-catalog");
    let cwd = root.join("unrelated-working-directory");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&cwd).expect("unrelated working directory creates");

    for (starter, name) in TEMPLATE_NAMES
        .iter()
        .map(|name| (false, *name))
        .chain(STARTER_SNIPPET_NAMES.iter().map(|name| (true, *name)))
    {
        let output_dir = root.join(name);
        let mut generate_args = vec!["examples", "agent"];
        if starter {
            generate_args.push("get");
        }
        generate_args.extend([name, "--out", path_str(&output_dir)]);
        let generate = Command::new(env!("CARGO_BIN_EXE_scena"))
            .current_dir(&cwd)
            .args(&generate_args)
            .output()
            .unwrap_or_else(|error| panic!("template {name} generation runs: {error}"));
        assert!(
            generate.status.success(),
            "template {name} generation failed: {}",
            output_diagnostic(&generate)
        );

        let recipe_path = output_dir.join("recipe.json");
        for args in [
            vec!["validate-recipe", path_str(&recipe_path)],
            vec!["recipe", "build", path_str(&recipe_path)],
            vec![
                "recipe",
                "render",
                path_str(&recipe_path),
                "--introspect",
                "--out",
                path_str(&output_dir.join("portable-frame.png")),
            ],
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_scena"))
                .current_dir(&cwd)
                .args(&args)
                .output()
                .unwrap_or_else(|error| panic!("template {name} command {args:?} runs: {error}"));
            assert!(
                output.status.success(),
                "template {name} command {args:?} failed: {}",
                output_diagnostic(&output)
            );
        }
    }
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    fs::create_dir_all(&dir).expect("artifact dir exists");
    dir
}

fn template_cli_guard() -> MutexGuard<'static, ()> {
    TEMPLATE_CLI_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn output_diagnostic(output: &std::process::Output) -> String {
    format!(
        "status={}, stdout={}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_template_recipe_has_beauty_defaults(recipe_path: &Path, name: &str) {
    let recipe: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(recipe_path).expect("template recipe exists"))
            .expect("template recipe parses");
    let Some(lights) = recipe["lights"].as_array() else {
        panic!("template {name} should include a visible light rig: {recipe:#}");
    };
    assert!(
        lights.len() >= 3,
        "template {name} should include a visible light rig: {recipe:#}"
    );
    for preset in ["key", "fill", "rim"] {
        assert!(
            lights
                .iter()
                .any(|light| light["kind"] == "directional" && light["preset"] == preset),
            "template {name} should include directional {preset} light: {recipe:#}"
        );
    }
    assert!(
        recipe["scene"]["environment"] == serde_json::json!({ "preset": "studio" })
            || recipe["scene"]["environment"] == serde_json::json!({ "kind": "default" }),
        "template {name} should preserve an authored environment or use the portable studio preset: {recipe:#}"
    );
    assert!(
        matches!(
            recipe["scene"]["background"]["kind"].as_str(),
            Some("studio" | "dark_studio" | "neutral_gray")
        ),
        "template {name} should choose a presentable background: {recipe:#}"
    );
    assert!(
        recipe["capture"]["width"]
            .as_u64()
            .is_some_and(|width| width >= 512)
            && recipe["capture"]["height"]
                .as_u64()
                .is_some_and(|height| height >= 384),
        "template {name} should render high enough resolution for visual review: {recipe:#}"
    );
    assert_eq!(
        recipe["render"]["anti_aliasing"], "msaa4",
        "template {name} should use sample AA for native-resolution review, not FXAA-only: {recipe:#}"
    );
    assert_eq!(
        recipe["render"]["supersample"], 2,
        "template {name} should opt into the proven hero supersample tier: {recipe:#}"
    );
    assert_eq!(
        recipe["render"]["reconstruction"], "tent",
        "template {name} should use the line-safe reconstruction filter for starter output: {recipe:#}"
    );
    if recipe["scene"]["grid"].is_object() {
        assert!(
            recipe["scene"]["grid"]["line_width_px"]
                .as_f64()
                .is_some_and(|width| width >= 3.6),
            "template {name} with a grid should set a visible grid line width instead of relying on thin defaults: {recipe:#}"
        );
    }
}

fn assert_data_visualization_template_targets_authored_blue_mark(output_dir: &Path) {
    let recipe: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(output_dir.join("recipe.json"))
            .expect("data visualization recipe exists"),
    )
    .expect("data visualization recipe parses");
    assert!(
        recipe["imports"]
            .as_array()
            .is_none_or(|imports| imports.is_empty()),
        "data visualization must author its marks instead of sampling an unrelated textured glTF: {recipe:#}"
    );
    assert!(
        recipe["nodes"]
            .as_array()
            .is_some_and(|nodes| nodes.iter().any(|node| {
                node["tags"]
                    .as_array()
                    .is_some_and(|tags| tags.iter().any(|tag| tag == "data-mark-blue"))
            })),
        "data visualization must identify the rendered blue mark: {recipe:#}"
    );
    let expectation: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(output_dir.join("appearance-expectation.json"))
            .expect("data visualization expectation exists"),
    )
    .expect("data visualization expectation parses");
    assert_eq!(expectation["targets"][0]["tag"], "data-mark-blue");
    assert_ne!(expectation["targets"][0]["require_source_material"], true);
}
