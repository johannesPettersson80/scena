#![cfg(feature = "inspection")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

const TEST_ASSET: &str = "tests/assets/gltf/mesh_material_vertex_color_scene.gltf";
const ANCHORED_ASSET: &str = "tests/assets/gltf/anchored_triangle_scene.gltf";
const ANCHOR_ASSET: &str = "tests/assets/gltf/anchor_debug_scene.gltf";
const CONNECTOR_ASSET: &str = "tests/assets/gltf/connector_basis_scene.gltf";

#[test]
fn scena_render_cli_accepts_scene_recipe_input() {
    let dir = artifact_dir("render");
    let recipe_path = write_valid_recipe(&dir);
    let png_path = dir.join("recipe-frame.png");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render recipe command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "recipe render keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render recipe emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["artifacts"]["capture_png_path"], path_str(&png_path));
    assert!(fs::metadata(&png_path).expect("PNG artifact exists").len() > 0);
}

#[test]
fn scena_inspect_and_diagnose_cli_accept_scene_recipe_input() {
    let dir = artifact_dir("inspect-diagnose");
    let recipe_path = write_valid_recipe(&dir);

    let inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["inspect", path_str(&recipe_path)])
        .output()
        .expect("scena inspect recipe command runs");
    assert!(inspect.status.success(), "stderr={}", stderr(&inspect));
    let inspection: serde_json::Value =
        serde_json::from_slice(&inspect.stdout).expect("inspect recipe emits JSON");
    assert_eq!(inspection["schema"], "scena.scene_inspection.v1");
    assert!(
        inspection["counts"]["visible_drawable"]
            .as_u64()
            .expect("visible_drawable count is numeric")
            > 0,
        "recipe inspection should include the imported mesh: {inspection:#}"
    );

    let diagnose = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "diagnose",
            path_str(&recipe_path),
            "--visibility",
            "--handle",
            "999999",
        ])
        .output()
        .expect("scena diagnose recipe command runs");
    assert!(!diagnose.status.success());
    assert!(
        diagnose.stderr.is_empty(),
        "recipe diagnosis failures stay machine-readable on stdout, stderr={}",
        stderr(&diagnose)
    );
    let diagnosis: serde_json::Value =
        serde_json::from_slice(&diagnose.stdout).expect("diagnose recipe emits JSON");
    assert_eq!(diagnosis["schema"], "scena.visibility_diagnosis.v1");
    assert_eq!(diagnosis["ok"], false);
    assert!(
        diagnosis["reasons"]
            .as_array()
            .expect("diagnosis reasons array")
            .iter()
            .any(|reason| reason["code"] == "stale_handle"),
        "recipe diagnosis should explain stale handle: {diagnosis:#}"
    );
}

#[test]
fn scena_recipe_cli_applies_import_transform_before_inspection() {
    let dir = artifact_dir("transform");
    let recipe_path = dir.join("translated.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                {
                    "id": "part",
                    "uri": TEST_ASSET,
                    "transform": {
                        "translation": [7.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            ]
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");

    let inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["inspect", path_str(&recipe_path)])
        .output()
        .expect("scena inspect translated recipe command runs");
    assert!(inspect.status.success(), "stderr={}", stderr(&inspect));
    let inspection: serde_json::Value =
        serde_json::from_slice(&inspect.stdout).expect("inspect recipe emits JSON");

    assert!(
        inspection["nodes"]
            .as_array()
            .expect("nodes array")
            .iter()
            .any(|node| {
                let translation = &node["world_transform"]["translation"];
                translation[0]
                    .as_f64()
                    .is_some_and(|x| (x - 7.0).abs() < 1.0e-5)
            }),
        "recipe import transform should be applied before inspection: {inspection:#}"
    );
}

#[test]
fn scena_validate_recipe_cli_checks_asset_presence_and_expected_extents() {
    let dir = artifact_dir("validate-assets");
    let missing_path = dir.join("missing-asset.recipe.json");
    fs::write(
        &missing_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "missing", "uri": "missing-file.gltf" }
            ]
        }))
        .expect("missing asset recipe serializes"),
    )
    .expect("missing asset recipe writes");

    let missing = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&missing_path)])
        .output()
        .expect("scena validate-recipe missing asset command runs");
    assert!(!missing.status.success());
    assert!(
        missing.stderr.is_empty(),
        "asset validation diagnostics stay machine-readable on stdout, stderr={}",
        stderr(&missing)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&missing.stdout).expect("missing asset validation emits JSON");
    assert_eq!(report["schema"], "scena.scene_recipe_validation.v1");
    assert_eq!(report["ok"], false);
    assert_diagnostic(&report, "asset_load_failed", "error");

    let oversized_path = dir.join("oversized.recipe.json");
    fs::write(
        &oversized_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                {
                    "id": "part",
                    "uri": TEST_ASSET,
                    "expected_extent": {
                        "min": 0.01,
                        "max": 0.25,
                        "unit": "m"
                    }
                }
            ]
        }))
        .expect("oversized recipe serializes"),
    )
    .expect("oversized recipe writes");

    let oversized = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&oversized_path)])
        .output()
        .expect("scena validate-recipe oversized asset command runs");
    assert!(oversized.status.success(), "stderr={}", stderr(&oversized));
    let report: serde_json::Value =
        serde_json::from_slice(&oversized.stdout).expect("oversized validation emits JSON");
    assert_eq!(report["schema"], "scena.scene_recipe_validation.v1");
    assert_eq!(report["ok"], true);
    assert_diagnostic(&report, "extent_out_of_range", "warning");
}

#[test]
fn scena_place_cli_emits_bounds_based_transform_previews_for_recipe_import() {
    let dir = artifact_dir("place");
    let recipe_path = write_valid_recipe(&dir);

    let centered = run_place(&recipe_path, &["--verb", "center", "--target", "1,2,3"]);
    assert!(centered.status.success(), "stderr={}", stderr(&centered));
    let centered: serde_json::Value =
        serde_json::from_slice(&centered.stdout).expect("center placement emits JSON");
    assert_eq!(centered["schema"], "scena.placement_result.v1");
    assert_eq!(centered["ok"], true);
    assert_vec3(&centered["transform"]["translation"], [1.0, 2.0, 3.0]);

    let grounded = run_place(&recipe_path, &["--verb", "ground", "--ground-y", "0"]);
    assert!(grounded.status.success(), "stderr={}", stderr(&grounded));
    let grounded: serde_json::Value =
        serde_json::from_slice(&grounded.stdout).expect("ground placement emits JSON");
    assert_eq!(grounded["verb"], "ground");
    assert_vec3(&grounded["transform"]["translation"], [0.0, 0.5, 0.0]);

    let fit = run_place(
        &recipe_path,
        &["--verb", "fit_to_size", "--max-size", "0.5"],
    );
    assert!(fit.status.success(), "stderr={}", stderr(&fit));
    let fit: serde_json::Value =
        serde_json::from_slice(&fit.stdout).expect("fit placement emits JSON");
    assert_eq!(fit["verb"], "fit_to_size");
    assert_vec3(&fit["transform"]["scale"], [0.5, 0.5, 0.5]);
}

#[test]
fn scena_place_cli_stdout_matches_golden_fixture() {
    let dir = artifact_dir("place-golden");
    let recipe_path = write_valid_recipe(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "place",
            path_str(&recipe_path),
            "--import",
            "part",
            "--verb",
            "center",
            "--target",
            "1,2,3",
            "--round-floats",
            "3",
        ])
        .output()
        .expect("scena place golden command runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "place golden command keeps stderr empty, stderr={}",
        stderr(&output)
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("place emits JSON");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("assets/cli-golden/place_center_stdout.json"))
            .expect("golden place fixture parses");
    assert_eq!(actual, expected);
}

#[test]
fn scena_place_cli_exits_nonzero_for_unknown_import() {
    let dir = artifact_dir("place-invalid");
    let recipe_path = write_valid_recipe(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "place",
            path_str(&recipe_path),
            "--import",
            "missing",
            "--verb",
            "center",
        ])
        .output()
        .expect("scena place invalid import command runs");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "placement diagnostics stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("place failure emits JSON");
    assert_eq!(report["schema"], "scena.placement_result.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "unknown_import"),
        "unknown import should be structured: {report:#}"
    );
}

#[test]
fn scena_place_cli_supports_authored_feature_verbs() {
    let dir = artifact_dir("place-authored-features");
    let anchor_recipe = write_two_import_recipe(&dir, "anchors.recipe.json", ANCHOR_ASSET);
    let connector_recipe = write_two_import_recipe(&dir, "connectors.recipe.json", CONNECTOR_ASSET);

    let look_at = run_place_for_import(
        &anchor_recipe,
        "source",
        &["--verb", "look_at", "--target", "0,0,-2"],
    );
    assert!(look_at.status.success(), "stderr={}", stderr(&look_at));
    let transform: scena::Transform =
        serde_json::from_value(json_transform(&look_at)).expect("look_at transform deserializes");
    assert_vec3_value(
        transform.rotation * scena::Vec3::new(0.0, 0.0, -1.0),
        [0.0, 0.0, -1.0],
    );

    let aligned = run_place_for_import(
        &connector_recipe,
        "source",
        &[
            "--verb",
            "align_to_anchor",
            "--source-connector",
            "basis-connector",
            "--target-import",
            "target",
            "--target-connector",
            "basis-connector",
        ],
    );
    assert!(aligned.status.success(), "stderr={}", stderr(&aligned));
    let aligned: serde_json::Value =
        serde_json::from_slice(&aligned.stdout).expect("align placement emits JSON");
    assert_eq!(aligned["verb"], "align_to_anchor");
    assert_vec3(&aligned["transform"]["translation"], [2.0, 0.0, 0.0]);

    let placed = run_place_for_import(
        &anchor_recipe,
        "source",
        &[
            "--verb",
            "place_on",
            "--source-anchor",
            "inspection",
            "--target-import",
            "target",
            "--target-anchor",
            "pivot",
        ],
    );
    assert!(placed.status.success(), "stderr={}", stderr(&placed));
    let placed: serde_json::Value =
        serde_json::from_slice(&placed.stdout).expect("place_on emits JSON");
    assert_eq!(placed["verb"], "place_on");
    assert_vec3(&placed["transform"]["translation"], [2.1, -0.1, 0.0]);
}

#[test]
fn scena_place_cli_exits_nonzero_for_unknown_authored_feature() {
    let dir = artifact_dir("place-missing-feature");
    let recipe_path = write_two_import_recipe(&dir, "anchors.recipe.json", ANCHOR_ASSET);

    let output = run_place_for_import(
        &recipe_path,
        "source",
        &[
            "--verb",
            "align_to_anchor",
            "--source-anchor",
            "missing",
            "--target-import",
            "target",
            "--target-anchor",
            "mount",
        ],
    );

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "authored feature diagnostics stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("place failure emits JSON");
    assert_eq!(report["schema"], "scena.placement_result.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "authored_feature_not_found"),
        "unknown authored feature should be structured: {report:#}"
    );
}

#[test]
fn scena_place_cli_previews_render_as_visible_framed_recipes() {
    let dir = artifact_dir("place-render-proof");
    let base_recipe = write_valid_recipe(&dir);

    let center = run_place(&base_recipe, &["--verb", "center", "--target", "1,2,3"]);
    assert!(center.status.success(), "stderr={}", stderr(&center));
    assert_placed_recipe_renders_visible(&dir, "center", TEST_ASSET, json_transform(&center));

    let ground = run_place(&base_recipe, &["--verb", "ground", "--ground-y", "0"]);
    assert!(ground.status.success(), "stderr={}", stderr(&ground));
    assert_placed_recipe_renders_visible(&dir, "ground", TEST_ASSET, json_transform(&ground));

    let anchored_recipe = write_two_import_recipe(&dir, "anchored.recipe.json", ANCHORED_ASSET);
    let aligned = run_place_for_import(
        &anchored_recipe,
        "source",
        &[
            "--verb",
            "align_to_anchor",
            "--source-anchor",
            "mount",
            "--target-import",
            "target",
            "--target-anchor",
            "mount",
        ],
    );
    assert!(aligned.status.success(), "stderr={}", stderr(&aligned));
    assert_placed_recipe_renders_visible(
        &dir,
        "align-to-anchor",
        ANCHORED_ASSET,
        json_transform(&aligned),
    );
}

fn write_valid_recipe(dir: &Path) -> PathBuf {
    let recipe_path = dir.join("scene.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "part", "uri": TEST_ASSET }
            ],
            "capture": {
                "width": 96,
                "height": 72
            }
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");
    recipe_path
}

fn write_two_import_recipe(dir: &Path, name: &str, asset: &str) -> PathBuf {
    let recipe_path = dir.join(name);
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "source", "uri": asset },
                {
                    "id": "target",
                    "uri": asset,
                    "transform": {
                        "translation": [2.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            ],
            "capture": {
                "width": 96,
                "height": 72
            }
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");
    recipe_path
}

fn assert_placed_recipe_renders_visible(
    dir: &Path,
    name: &str,
    asset: &str,
    transform: serde_json::Value,
) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                {
                    "id": "placed",
                    "uri": asset,
                    "transform": transform
                }
            ],
            "capture": {
                "width": 112,
                "height": 84
            }
        }))
        .expect("placed proof recipe serializes"),
    )
    .expect("placed proof recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render placed recipe command runs");
    assert!(
        output.status.success(),
        "{name} placed recipe should render successfully, stderr={}",
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "rendered proof keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("placed render emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true, "{name} render report: {report:#}");
    assert!(
        report["visible_pixel_fraction"]
            .as_f64()
            .is_some_and(|fraction| fraction > 0.001),
        "{name} placement render should contain visible pixels: {report:#}"
    );
    assert!(
        report["content_bbox_css_px"].is_object(),
        "{name} placement render should have a content bbox: {report:#}"
    );
    assert!(fs::metadata(&png_path).expect("PNG artifact exists").len() > 0);
}

fn run_place(recipe_path: &Path, args: &[&str]) -> std::process::Output {
    run_place_for_import(recipe_path, "part", args)
}

fn run_place_for_import(
    recipe_path: &Path,
    import_id: &str,
    args: &[&str],
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    command
        .arg("place")
        .arg(path_str(recipe_path))
        .arg("--import")
        .arg(import_id);
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("scena place command runs")
}

fn json_transform(output: &std::process::Output) -> serde_json::Value {
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("placement output emits JSON");
    assert_eq!(report["schema"], "scena.placement_result.v1");
    assert_eq!(report["ok"], true);
    report["transform"].clone()
}

fn assert_vec3(value: &serde_json::Value, expected: [f64; 3]) {
    let actual = value.as_array().expect("vec3 serializes as an array");
    for (index, expected) in expected.into_iter().enumerate() {
        let actual = actual[index].as_f64().expect("vec3 component is numeric");
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "component {index}: expected {expected}, got {actual}"
        );
    }
}

fn assert_vec3_value(actual: scena::Vec3, expected: [f32; 3]) {
    assert!(
        actual.abs_diff_eq(scena::Vec3::from_array(expected), 1.0e-5),
        "expected {expected:?}, got {actual:?}"
    );
}

fn assert_diagnostic(report: &serde_json::Value, code: &str, severity: &str) {
    assert!(
        report["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == code && diagnostic["severity"] == severity),
        "missing diagnostic {code}/{severity}: {report:#}"
    );
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target")
        .join("gate-artifacts")
        .join(format!("scena-cli-recipe-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("artifact directory creates");
    dir
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is valid UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
