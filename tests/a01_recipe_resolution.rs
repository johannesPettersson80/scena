use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

#[test]
fn full_validation_resolves_every_authored_resource_family() {
    let root = fixture_dir("all-resources");
    let recipe = root.join("scene.recipe.json");
    let valid_import = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/gltf/mesh_material_vertex_color_scene.gltf");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "valid", "uri": valid_import }],
            "fonts": [{ "id": "missing_font", "uri": "missing/font.ttf" }],
            "materials": [{
                "id": "textured",
                "kind": "unlit",
                "base_color": "white",
                "base_color_texture": { "uri": "missing/albedo.png" }
            }],
            "scene": {
                "environment": { "kind": "uri", "uri": "missing/studio.hdr" }
            }
        }))
        .expect("A01 recipe serializes"),
    )
    .expect("A01 recipe writes");

    let output = validate(&recipe, &[]);
    assert!(
        !output.status.success(),
        "full validation must fail missing resources"
    );
    let report = json_report(&output);
    assert_eq!(report["validation_mode"], "full_resolution");
    assert!(report["policy"]["allowed_roots"].as_array().is_some());
    for (path, kind) in [
        ("$.imports[0].uri", "import"),
        ("$.fonts[0].uri", "font"),
        ("$.materials[0].base_color_texture.uri", "texture"),
        ("$.scene.environment.uri", "environment"),
    ] {
        let resource = resource_at(&report, path);
        assert_eq!(resource["kind"], kind, "{resource:#}");
        assert_eq!(resource["required"], true, "{resource:#}");
        assert!(
            resource["normalized_uri"].as_str().is_some(),
            "{resource:#}"
        );
    }
    for path in [
        "$.fonts[0].uri",
        "$.materials[0].base_color_texture.uri",
        "$.scene.environment.uri",
    ] {
        let diagnostic = diagnostic_at(&report, path);
        assert_eq!(diagnostic["resource"]["required"], true, "{diagnostic:#}");
        assert!(
            diagnostic["resource"]["normalized_uri"].as_str().is_some(),
            "{diagnostic:#}"
        );
        assert!(diagnostic["resource"]["allowed_roots"].as_array().is_some());
        assert!(
            diagnostic["help"]
                .as_str()
                .is_some_and(|help| !help.is_empty())
        );
    }
}

#[test]
fn syntax_only_is_explicit_and_does_not_claim_execution_equivalence() {
    let root = fixture_dir("syntax-only");
    let recipe = root.join("scene.recipe.json");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "missing", "uri": "missing.glb" }],
            "scene": { "environment": { "kind": "uri", "uri": "missing.hdr" } }
        }))
        .expect("A01 recipe serializes"),
    )
    .expect("A01 recipe writes");

    let output = validate(&recipe, &["--syntax-only"]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let report = json_report(&output);
    assert_eq!(report["validation_mode"], "syntax_only");
    assert_eq!(report["execution_equivalent"], false);
    assert!(report["resources"].as_array().is_some_and(|resources| {
        !resources.is_empty()
            && resources
                .iter()
                .all(|resource| resource["status"] == "not_checked")
    }));
}

#[test]
fn full_validation_checks_nested_gltf_dependencies_and_accepts_builtins() {
    let root = fixture_dir("nested-and-builtin");
    let nested = root.join("nested.gltf");
    fs::write(
        &nested,
        serde_json::to_vec_pretty(&json!({
            "asset": { "version": "2.0" },
            "scene": 0,
            "scenes": [{ "nodes": [] }],
            "nodes": [],
            "buffers": [{ "uri": "missing.bin", "byteLength": 12 }]
        }))
        .expect("nested glTF serializes"),
    )
    .expect("nested glTF writes");
    let recipe = root.join("scene.recipe.json");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "nested", "uri": "nested.gltf" }],
            "scene": { "environment": { "preset": "studio" } }
        }))
        .expect("A01 recipe serializes"),
    )
    .expect("A01 recipe writes");

    let output = validate(&recipe, &[]);
    assert!(
        !output.status.success(),
        "missing nested dependency must fail"
    );
    let report = json_report(&output);
    let import = resource_at(&report, "$.imports[0].uri");
    assert_eq!(import["status"], "load_failed");
    let builtin = resource_at(&report, "$.scene.environment.preset");
    assert_eq!(builtin["kind"], "builtin_environment");
    assert_eq!(builtin["status"], "builtin");
    assert_eq!(builtin["required"], true);
}

#[cfg(feature = "scene-host")]
#[test]
fn full_validation_success_uses_the_same_plan_as_recipe_build() {
    let root = fixture_dir("validation-build-equivalence");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let recipe = root.join("scene.recipe.json");
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "triangle",
                "uri": repository.join("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            }],
            "fonts": [{
                "id": "ui",
                "uri": repository.join("src/scene/labels/fonts/LiberationSans-Regular.ttf")
            }],
            "materials": [{
                "id": "textured",
                "kind": "unlit",
                "base_color": "white",
                "base_color_texture": {
                    "uri": repository.join("tests/assets/gltf/khronos/WaterBottle/WaterBottle_baseColor.png")
                }
            }],
            "scene": {
                "environment": {
                    "kind": "uri",
                    "uri": repository.join("tests/assets/environment/polyhaven/studio_small_08_1k.hdr")
                }
            }
        }))
        .expect("A01 recipe serializes"),
    )
    .expect("A01 recipe writes");

    let validation = validate(&recipe, &[]);
    assert!(
        validation.status.success(),
        "stderr={}",
        stderr(&validation)
    );
    let validation = json_report(&validation);
    assert_eq!(validation["validation_mode"], "full_resolution");
    assert!(validation["resources"].as_array().is_some_and(|resources| {
        resources
            .iter()
            .all(|resource| matches!(resource["status"].as_str(), Some("loaded" | "builtin")))
    }));

    let build = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["recipe", "build"])
        .arg(&recipe)
        .output()
        .expect("recipe build runs");
    assert!(build.status.success(), "stderr={}", stderr(&build));
    let build = json_report(&build);
    assert_eq!(build["ok"], true, "{build:#}");
}

fn validate(recipe: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    command.arg("validate-recipe").arg(recipe);
    command.args(extra);
    command.output().expect("validate-recipe runs")
}

fn resource_at<'a>(report: &'a Value, path: &str) -> &'a Value {
    report["resources"]
        .as_array()
        .expect("resources array")
        .iter()
        .find(|resource| resource["path"] == path)
        .unwrap_or_else(|| panic!("missing resource {path}: {report:#}"))
}

fn diagnostic_at<'a>(report: &'a Value, path: &str) -> &'a Value {
    report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .find(|diagnostic| diagnostic["path"] == path)
        .unwrap_or_else(|| panic!("missing diagnostic {path}: {report:#}"))
}

fn json_report(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must be JSON: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn fixture_dir(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/gate-artifacts/a01-recipe-resolution-{name}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("A01 fixture directory creates");
    root
}
