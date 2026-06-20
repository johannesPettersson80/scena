#![cfg(feature = "inspection")]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

const TEST_ASSET: &str = "tests/assets/gltf/mesh_material_vertex_color_scene.gltf";
const ANCHORED_ASSET: &str = "tests/assets/gltf/anchored_triangle_scene.gltf";
const ANCHOR_ASSET: &str = "tests/assets/gltf/anchor_debug_scene.gltf";
const CONNECTOR_ASSET: &str = "tests/assets/gltf/connector_basis_scene.gltf";
const LAVAPIPE_ICD: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";

fn configure_command_for_lavapipe(command: &mut Command) {
    if Path::new(LAVAPIPE_ICD).exists() {
        command.env("VK_ICD_FILENAMES", LAVAPIPE_ICD);
    }
}

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

#[cfg(feature = "scene-host")]
#[test]
fn scena_render_gpu_flag_reports_actual_backend_for_recipe_input() {
    let dir = artifact_dir("render-gpu");
    let recipe_path = write_valid_recipe(&dir);
    let png_path = dir.join("gpu-frame.png");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--gpu",
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render --gpu recipe command runs");

    assert!(
        output.status.success(),
        "render --gpu should render or fall back cleanly, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render --gpu emits JSON");
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    let backend = report["capabilities"]["backend"]
        .as_str()
        .expect("backend is reported");
    assert!(
        backend == "headless" || backend == "headless_gpu",
        "render --gpu must report the actual backend used: {report:#}"
    );
    assert_eq!(
        report["capabilities"]["gpu_device"],
        backend == "headless_gpu",
        "gpu_device must reflect the actual backend, not the requested flag: {report:#}"
    );
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
    assert!(!oversized.status.success(), "stderr={}", stderr(&oversized));
    let report: serde_json::Value =
        serde_json::from_slice(&oversized.stdout).expect("oversized validation emits JSON");
    assert_eq!(report["schema"], "scena.scene_recipe_validation.v1");
    assert_eq!(report["ok"], false);
    assert_diagnostic(&report, "extent_out_of_range", "error");
}

#[test]
fn scena_validate_recipe_cli_rejects_out_of_root_imports_before_build() {
    let dir = artifact_dir("validate-path-policy");
    let outside_asset = std::env::temp_dir().join("scena-outside-root-policy.gltf");
    fs::write(&outside_asset, "{}").expect("outside-root fixture writes");
    let recipe_path = dir.join("outside-root.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "outside", "uri": path_str(&outside_asset) }
            ]
        }))
        .expect("outside-root recipe serializes"),
    )
    .expect("outside-root recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&recipe_path)])
        .output()
        .expect("scena validate-recipe outside-root command runs");

    assert!(!output.status.success(), "validate should fail closed");
    assert!(
        output.stderr.is_empty(),
        "validation diagnostics stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("outside-root validation emits JSON");
    assert_eq!(report["schema"], "scena.scene_recipe_validation.v1");
    assert_eq!(report["ok"], false, "{report:#}");
    let diagnostic = report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .find(|diagnostic| diagnostic["code"] == "policy_violation")
        .unwrap_or_else(|| panic!("expected policy_violation before asset load: {report:#}"));
    assert_eq!(diagnostic["path"], "$.imports[0].uri", "{report:#}");
    let message = diagnostic["message"].as_str().expect("message string");
    let help = diagnostic["help"].as_str().expect("help string");
    assert!(
        message.contains("outside the allowed recipe roots")
            && message.contains("allowed roots:")
            && help.contains("RecipeBuildPolicy"),
        "policy diagnostic should name the allowed roots and policy knob: {report:#}"
    );
}

#[test]
fn invalid_primitive_diagnostic_lists_supported_kinds() {
    let dir = artifact_dir("invalid-primitive-supported-kinds");
    let recipe_path = dir.join("invalid-primitive.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "white": "#FFFFFF" },
            "geometries": [
                { "id": "bad_geo", "primitive": { "kind": "capsule", "radius": 0.1, "height": 0.2 } }
            ],
            "materials": [
                { "id": "mat", "kind": "unlit", "base_color": "white" }
            ],
            "nodes": [
                { "id": "bad", "geometry": "bad_geo", "material": "mat" }
            ],
            "capture": { "width": 64, "height": 64 }
        }))
        .expect("invalid primitive recipe serializes"),
    )
    .expect("invalid primitive recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&recipe_path)])
        .output()
        .expect("scena validate-recipe invalid primitive command runs");

    assert!(!output.status.success(), "invalid primitive should fail");
    assert!(output.stderr.is_empty(), "stderr={}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid primitive report emits JSON");
    let diagnostic = report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .find(|diagnostic| diagnostic["path"] == "$.geometries[0].primitive.kind")
        .unwrap_or_else(|| panic!("expected primitive kind diagnostic: {report:#}"));
    assert_eq!(diagnostic["code"], "unsupported_feature", "{report:#}");
    let text = format!(
        "{} {}",
        diagnostic["message"].as_str().unwrap_or_default(),
        diagnostic["help"].as_str().unwrap_or_default()
    );
    for kind in [
        "box", "plane", "sphere", "cylinder", "cone", "disc", "torus", "wedge", "line", "polyline",
        "arrow", "grid", "axes",
    ] {
        assert!(
            text.contains(kind),
            "diagnostic should list supported primitive kind {kind}: {report:#}"
        );
    }
    assert!(
        !text.contains("until the primitive-coverage slice lands"),
        "diagnostic should not mention a completed slice as pending: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_bbox_fit_expectation_uses_subject_bounds_not_ground_plane() {
    let dir = artifact_dir("recipe-bbox-fit-subject");
    let recipe_path = dir.join("grounded.recipe.json");
    let png_path = dir.join("grounded.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "body": "#3A7BD5",
                "floor": "#1D2733",
                "grid": "#697386"
            },
            "geometries": [
                { "id": "box_geo", "primitive": { "kind": "box", "size": [0.2, 0.12, 0.16] } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "body", "metallic": 0.0, "roughness": 0.55 }
            ],
            "nodes": [
                { "id": "subject", "geometry": "box_geo", "material": "body_mat", "transform": { "kind": "ground" } }
            ],
            "lights": [
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "fill", "kind": "directional", "preset": "fill" },
                { "id": "rim", "kind": "directional", "preset": "rim" }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.34, 0.24, 0.32], "target": "subject" }
            }],
            "scene": {
                "background": { "kind": "dark_studio" },
                "grid": {
                    "padding": 2.0,
                    "line_spacing": 0.1,
                    "color": "floor",
                    "line_color": "grid"
                }
            },
            "capture": { "width": 320, "height": 220 },
            "expect": {
                "expect_bbox_fit": { "min": 0.2, "max": 0.8 }
            }
        }))
        .expect("grounded recipe serializes"),
    )
    .expect("grounded recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena grounded recipe render runs");

    assert!(
        output.status.success(),
        "grounded subject should satisfy bbox fit independent of floor/grid, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], true, "{report:#}");
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
fn scena_recipe_invalid_fixtures_cover_landed_failure_families() {
    let missing = run_validate_recipe_fixture("missing_asset.recipe.json");
    assert!(!missing.status.success());
    let report = json_report(&missing);
    assert_eq!(report["schema"], "scena.scene_recipe_validation.v1");
    assert_eq!(report["ok"], false);
    assert_diagnostic(&report, "asset_load_failed", "error");

    let invalid_transform = run_validate_recipe_fixture("invalid_transform.recipe.json");
    assert!(!invalid_transform.status.success());
    let report = json_report(&invalid_transform);
    assert_eq!(report["ok"], false);
    assert_diagnostic(&report, "invalid_transform", "error");

    let oversized = run_validate_recipe_fixture("oversized_asset.recipe.json");
    assert!(!oversized.status.success(), "stderr={}", stderr(&oversized));
    let report = json_report(&oversized);
    assert_eq!(report["ok"], false);
    assert_diagnostic(&report, "extent_out_of_range", "error");

    let valid_recipe = recipe_invalid_fixture_path("valid_for_commands.recipe.json");
    let unknown_verb = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "place",
            path_str(&valid_recipe),
            "--import",
            "part",
            "--verb",
            "spin",
        ])
        .output()
        .expect("scena place unknown verb command runs");
    assert!(!unknown_verb.status.success());
    let report = json_report(&unknown_verb);
    assert_eq!(report["schema"], "scena.placement_result.v1");
    assert_eq!(report["ok"], false);
    assert_diagnostic(&report, "unknown_verb", "error");

    let stale_handle = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "diagnose",
            path_str(&valid_recipe),
            "--visibility",
            "--handle",
            "999999",
        ])
        .output()
        .expect("scena diagnose stale handle recipe command runs");
    assert!(!stale_handle.status.success());
    let report = json_report(&stale_handle);
    assert_eq!(report["schema"], "scena.visibility_diagnosis.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["reasons"]
            .as_array()
            .expect("diagnosis reasons array")
            .iter()
            .any(|reason| reason["code"] == "stale_handle"),
        "stale handle fixture should produce stale_handle: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_render_cli_applies_scene_setup_for_import_only_recipes() {
    let dir = artifact_dir("render-import-scene-setup");
    let recipe_path = dir.join("white-background.recipe.json");
    let png_path = dir.join("white-background.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "part", "uri": TEST_ASSET }
            ],
            "scene": {
                "background": { "kind": "white" }
            },
            "capture": {
                "width": 96,
                "height": 72
            }
        }))
        .expect("scene setup recipe serializes"),
    )
    .expect("scene setup recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render import-only scene setup recipe runs");

    assert!(
        output.status.success(),
        "import-only scene setup recipe should render, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert!(
        report["luminance"]["mean"]
            .as_f64()
            .is_some_and(|mean| mean > 150.0),
        "white recipe background should be applied through SceneHost routing: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_color_pick_and_fit_expectations() {
    let dir = artifact_dir("recipe-render-verify-pass");
    let recipe_path = dir.join("verified.recipe.json");
    let png_path = dir.join("verified.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&authored_verification_recipe(json!({
            "expect_color": [{
                "id": "plate-is-red",
                "target": { "kind": "node", "id": "plate" },
                "swatch_srgb8": [220, 32, 32],
                "tolerance": 0.20
            }],
            "expect_bbox_fit": {
                "min": 0.20,
                "max": 0.95
            },
            "expect_pick": [{
                "id": "center-picks-plate",
                "x_css_px": 64.0,
                "y_css_px": 64.0,
                "target": { "kind": "node", "id": "plate" }
            }]
        })))
        .expect("verified recipe serializes"),
    )
    .expect("verified recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render command runs");

    assert!(
        output.status.success(),
        "recipe render verification should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "recipe render verification keeps stderr empty, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["build"]["ok"], true, "{report:#}");
    assert_eq!(report["introspection"]["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], true, "{report:#}");
    assert_eq!(
        report["verification"]["appearance"]["ok"], true,
        "{report:#}"
    );
    assert_eq!(
        report["verification"]["interaction"]["ok"], true,
        "{report:#}"
    );
    assert!(png_path.exists(), "recipe render writes the requested PNG");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_introspect_succeeds_without_verify() {
    let dir = artifact_dir("recipe-render-introspect-only");
    let recipe_path = dir.join("introspect-only.recipe.json");
    let png_path = dir.join("introspect-only.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&authored_verification_recipe(json!({
            "expect_bbox_fit": {
                "min": 0.20,
                "max": 0.95
            }
        })))
        .expect("introspection recipe serializes"),
    )
    .expect("introspection recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render introspection-only command runs");

    assert!(
        output.status.success(),
        "recipe render --introspect should not require --verify, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["artifacts"]["capture_png_path"], path_str(&png_path));
    assert!(png_path.exists(), "recipe render writes the requested PNG");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_flag_reports_actual_backend() {
    let dir = artifact_dir("recipe-render-gpu");
    let recipe_path = dir.join("gpu.recipe.json");
    let cpu_png = dir.join("cpu.png");
    let gpu_png = dir.join("gpu.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&authored_verification_recipe(json!({
            "expect_bbox_fit": {
                "min": 0.20,
                "max": 0.95
            }
        })))
        .expect("GPU recipe serializes"),
    )
    .expect("GPU recipe writes");

    let cpu = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&cpu_png),
        ])
        .output()
        .expect("CPU recipe render command runs");
    assert!(cpu.status.success(), "stderr={}", stderr(&cpu));
    let cpu_report = json_report(&cpu);
    assert_eq!(
        cpu_report["introspection"]["capabilities"]["backend"], "headless",
        "{cpu_report:#}"
    );
    assert_eq!(
        cpu_report["introspection"]["capabilities"]["gpu_device"], false,
        "{cpu_report:#}"
    );

    let gpu = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--gpu",
            "--introspect",
            "--verify",
            "--out",
            path_str(&gpu_png),
        ])
        .output()
        .expect("GPU recipe render command runs");
    assert!(
        gpu.status.success(),
        "GPU opt-in should render or fall back cleanly, stdout={}, stderr={}",
        String::from_utf8_lossy(&gpu.stdout),
        stderr(&gpu)
    );
    let gpu_report = json_report(&gpu);
    assert_eq!(gpu_report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(gpu_report["ok"], true, "{gpu_report:#}");
    let backend = gpu_report["introspection"]["capabilities"]["backend"]
        .as_str()
        .expect("backend is reported");
    assert!(
        backend == "headless" || backend == "headless_gpu",
        "GPU opt-in must report the actual backend used: {gpu_report:#}"
    );
    assert_eq!(
        gpu_report["capture"]["capabilities"]["backend"], backend,
        "capture descriptor and introspection must agree on backend: {gpu_report:#}"
    );
    assert_eq!(
        gpu_report["introspection"]["capabilities"]["gpu_device"],
        backend == "headless_gpu",
        "gpu_device must reflect the actual backend, not the requested flag: {gpu_report:#}"
    );
    assert!(
        gpu_png.exists(),
        "GPU/fallback render writes the requested PNG"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_reports_reference_quality_failure() {
    let dir = artifact_dir("recipe-render-reference-quality");
    let recipe_path = dir.join("reference.recipe.json");
    let png_path = dir.join("verified.png");
    let reference_path = dir.join("wrong-reference.png");
    write_rgba_png(&reference_path, 128, 128, &[0, 0, 0, 255]);
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&authored_verification_recipe(json!({
            "expect_bbox_fit": {
                "min": 0.20,
                "max": 0.95
            },
            "expect_reference": [{
                "id": "wrong-reference-ssim",
                "image": "wrong-reference.png",
                "metric": "ssim",
                "min_ssim": 0.99
            }]
        })))
        .expect("reference quality recipe serializes"),
    )
    .expect("reference quality recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render command runs");

    assert!(
        !output.status.success(),
        "wrong reference image must fail recipe verification"
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["verification"]["quality"]["ok"], false);
    assert!(
        report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks")
            .iter()
            .any(|check| check["code"] == "reference_ssim_too_low"
                && check["id"] == "wrong-reference-ssim"
                && check["fix_hint"]
                    .as_str()
                    .is_some_and(|hint| hint.contains("reference"))),
        "expected exact reference_ssim_too_low quality check: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons")
            .iter()
            .any(|reason| reason["code"] == "reference_ssim_too_low"
                && reason["source"] == "quality"),
        "quality reference failure must also appear in compact reasons: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
fn write_label_quality_recipe(
    dir: &Path,
    name: &str,
    background: &str,
    min_intermediate_edge_fraction: f64,
) -> (PathBuf, PathBuf) {
    write_label_quality_recipe_with_antialiasing(
        dir,
        name,
        background,
        min_intermediate_edge_fraction,
        None,
    )
}

#[cfg(feature = "scene-host")]
fn write_label_quality_recipe_with_antialiasing(
    dir: &Path,
    name: &str,
    background: &str,
    min_intermediate_edge_fraction: f64,
    anti_aliasing: Option<&str>,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let font_path = dir.join("dejavu.ttf");
    fs::copy(system_test_font_path(), &font_path).expect("test font copied into recipe sandbox");
    let mut recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "fonts": [
            { "id": "dejavu", "uri": path_str(&font_path) }
        ],
        "labels": [{
            "id": "font_label",
            "text": "BATTERY",
            "font": "dejavu",
            "size_px": 42.0,
            "color": "#FFFFFF",
            "background": background,
            "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] }
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.5], "target": "font_label" }
        }],
        "capture": { "width": 220, "height": 120 },
        "expect": {
            "expect_quality": {
                "profile": "documentation",
                "text": {
                    "min_ink_coverage": 0.06,
                    "max_ink_isolation": 0.02,
                    "min_intermediate_edge_fraction": min_intermediate_edge_fraction,
                    "max_background_luminance_range": 0.03,
                    "max_background_mean_delta": 0.03
                }
            }
        }
    });
    if let Some(anti_aliasing) = anti_aliasing {
        recipe["render"] = json!({ "anti_aliasing": anti_aliasing });
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&recipe).expect("label quality recipe serializes"),
    )
    .expect("label quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_quality_per_label_region_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-label-quality-pass");
    let mut rendered = Vec::new();
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_label_quality_recipe(&dir, &format!("label-quality-{backend}"), "#808080", 0.01);
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let output = command
            .args(args)
            .output()
            .expect("scena label quality recipe render command runs");

        assert!(
            output.status.success(),
            "live {backend} atlas label recipe quality should pass, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        assert_eq!(report["verification"]["quality"]["ok"], true, "{report:#}");
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU quality proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
        rendered.push((backend, png_path));
    }

    let (_, cpu_png) = &rendered[0];
    let (_, gpu_png) = &rendered[1];
    let cpu = decode_png_rgba8(cpu_png);
    let gpu = decode_png_rgba8(gpu_png);
    assert_eq!((cpu.width, cpu.height), (gpu.width, gpu.height));
    let region = expected_label_background_region(cpu.width, cpu.height);
    let diff = frame_delta_in_region(&cpu.rgba8, &gpu.rgba8, cpu.width, region);
    let cpu_luma = mean_luminance_in_region(&cpu.rgba8, cpu.width, region);
    let gpu_luma = mean_luminance_in_region(&gpu.rgba8, gpu.width, region);
    fs::write(
        dir.join("label-quality-full-region-parity.json"),
        format!(
            "{{\n  \"schema\": \"scena.label_quality_parity.v1\",\n  \"max_channel_delta\": {},\n  \"mean_channel_delta\": {:.3},\n  \"cpu_luma\": {:.3},\n  \"gpu_luma\": {:.3},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
            diff.max_channel_delta,
            diff.mean_channel_delta,
            cpu_luma,
            gpu_luma,
            region.x,
            region.y,
            region.width,
            region.height
        ),
    )
    .expect("label quality parity artifact writes");
    assert!(
        diff.max_channel_delta <= 220 && diff.mean_channel_delta <= 8.0,
        "CPU and GPU recipe renders must match over the full label region including the background pill, diff={diff:?}, cpu_luma={cpu_luma:.3}, gpu_luma={gpu_luma:.3}, region={region:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_label_quality_regions_include_background_pill_padding() {
    let dir = artifact_dir("label-quality-region-padding");
    let (recipe_path, _png_path) =
        write_label_quality_recipe(&dir, "label-quality-region-padding", "#1D2733", 0.01);
    let recipe_text = fs::read_to_string(&recipe_path).expect("recipe reads");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        path_str(&recipe_path),
        &recipe_text,
        scena::RecipeBuildPolicy::default(),
    ))
    .expect("label quality recipe builds");
    let regions = build.host.label_quality_regions(220, 120);
    assert_eq!(regions.len(), 1, "one label region expected: {regions:#?}");
    let region = QualityPixelRegion {
        x: regions[0].x,
        y: regions[0].y,
        width: regions[0].width,
        height: regions[0].height,
    };
    assert_label_quality_region_covers_background_pill(region);
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_dark_label_background_matches_cpu_and_gpu_over_full_region() {
    let dir = artifact_dir("recipe-render-label-background-parity");
    let mut rendered = Vec::new();
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) = write_label_quality_recipe(
            &dir,
            &format!("label-background-{backend}"),
            "#1D2733",
            0.01,
        );
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--out", path_str(&png_path)]);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let output = command
            .args(args)
            .output()
            .expect("scena dark label parity render command runs");

        assert!(
            output.status.success(),
            "dark label parity render should succeed on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["capabilities"]["backend"], "headless_gpu",
                "GPU parity proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
        rendered.push((backend, png_path));
    }

    let (_, cpu_png) = &rendered[0];
    let (_, gpu_png) = &rendered[1];
    let cpu = decode_png_rgba8(cpu_png);
    let gpu = decode_png_rgba8(gpu_png);
    assert_eq!((cpu.width, cpu.height), (gpu.width, gpu.height));
    let region = expected_label_background_region(cpu.width, cpu.height);
    let diff = frame_delta_in_region(&cpu.rgba8, &gpu.rgba8, cpu.width, region);
    let cpu_luma = mean_luminance_in_region(&cpu.rgba8, cpu.width, region);
    let gpu_luma = mean_luminance_in_region(&gpu.rgba8, gpu.width, region);
    fs::write(
        dir.join("label-background-full-region-parity.json"),
        format!(
            "{{\n  \"schema\": \"scena.label_background_parity.v1\",\n  \"max_channel_delta\": {},\n  \"mean_channel_delta\": {:.3},\n  \"cpu_luma\": {:.3},\n  \"gpu_luma\": {:.3},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
            diff.max_channel_delta,
            diff.mean_channel_delta,
            cpu_luma,
            gpu_luma,
            region.x,
            region.y,
            region.width,
            region.height
        ),
    )
    .expect("dark label background parity artifact writes");
    assert!(
        diff.max_channel_delta <= 220 && diff.mean_channel_delta <= 8.0,
        "CPU and GPU recipe renders must match over the full dark label region including the background pill, diff={diff:?}, cpu_luma={cpu_luma:.3}, gpu_luma={gpu_luma:.3}, region={region:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_overlay_label_region_is_not_changed_by_fxaa_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-overlay-final-pass");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (none_recipe, none_png) = write_label_quality_recipe_with_antialiasing(
            &dir,
            &format!("overlay-{backend}-none"),
            "#1D2733",
            0.01,
            Some("none"),
        );
        let (fxaa_recipe, fxaa_png) = write_label_quality_recipe_with_antialiasing(
            &dir,
            &format!("overlay-{backend}-fxaa"),
            "#1D2733",
            0.01,
            Some("fxaa"),
        );

        for (mode, recipe_path, png_path) in [
            ("none", &none_recipe, &none_png),
            ("fxaa", &fxaa_recipe, &fxaa_png),
        ] {
            let mut args = vec!["recipe", "render", path_str(recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--out", path_str(png_path)]);
            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let output = command
                .args(args)
                .output()
                .expect("scena overlay final-pass render command runs");
            assert!(
                output.status.success(),
                "{backend} {mode} overlay render should succeed, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["capabilities"]["backend"], "headless_gpu",
                    "GPU overlay proof must use the GPU backend, not a fallback: {report:#}"
                );
            }
        }

        let none = decode_png_rgba8(&none_png);
        let fxaa = decode_png_rgba8(&fxaa_png);
        assert_eq!((none.width, none.height), (fxaa.width, fxaa.height));
        let region = expected_label_background_region(none.width, none.height);
        let diff = frame_delta_in_region(&none.rgba8, &fxaa.rgba8, none.width, region);
        fs::write(
            dir.join(format!("overlay-{backend}-aa-delta.json")),
            format!(
                "{{\n  \"schema\": \"scena.overlay_final_pass_delta.v1\",\n  \"backend\": \"{}\",\n  \"max_channel_delta\": {},\n  \"mean_channel_delta\": {:.3},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
                backend,
                diff.max_channel_delta,
                diff.mean_channel_delta,
                region.x,
                region.y,
                region.width,
                region.height
            ),
        )
        .expect("overlay final-pass delta artifact writes");
        assert!(
            diff.max_channel_delta <= 1 && diff.mean_channel_delta <= 0.05,
            "final overlay labels must not be altered by FXAA on {backend}; diff={diff:?}, region={region:?}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_quality_per_label_region() {
    let dir = artifact_dir("recipe-render-label-quality");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_label_quality_recipe(&dir, &format!("label-quality-{backend}"), "#000000", 0.50);
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let output = command
            .args(args)
            .output()
            .expect("scena label quality recipe render command runs");

        assert!(
            !output.status.success(),
            "impossible label quality threshold should fail on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU quality failure proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().any(|check| {
                check["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("expect_quality.text.label["))
                    && check["code"] == "label_missing_antialiasing"
                    && check["region"]["kind"] == "label"
                    && check["region"]["handle"].as_u64().is_some()
            }),
            "quality verifier must evaluate projected label regions on {backend}, not just the subject bbox: {report:#}"
        );
        assert!(png_path.exists(), "label quality render writes the PNG");
    }
}

#[cfg(feature = "scene-host")]
fn write_line_quality_recipe(
    dir: &Path,
    name: &str,
    min_intermediate_edge_fraction: f64,
    max_straightness_error: f64,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                { "id": "marker_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
            ],
            "materials": [
                { "id": "marker_mat", "kind": "unlit", "base_color": "#3A7BD5" }
            ],
            "nodes": [
                { "id": "marker", "geometry": "marker_geo", "material": "marker_mat" }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "#808080" }
            },
            "measurements": [{
                "id": "length-line",
                "kind": "distance",
                "start": [-0.7, -0.25, 0.0],
                "end": [0.7, 0.35, 0.0],
                "label": "LENGTH",
                "unit": "m",
                "precision": 2
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.0], "target": "marker" }
            }],
            "capture": { "width": 260, "height": 160 },
            "expect": {
                "expect_quality": {
                    "profile": "documentation",
                    "line": {
                        "min_intermediate_edge_fraction": min_intermediate_edge_fraction,
                        "max_straightness_error": max_straightness_error
                    }
                }
            }
        }))
        .expect("line quality recipe serializes"),
    )
    .expect("line quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_quality_per_line_region_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-line-quality-pass");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_line_quality_recipe(&dir, &format!("line-quality-{backend}"), 0.005, 0.12);
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let output = command
            .args(args)
            .output()
            .expect("scena line quality recipe render command runs");

        assert!(
            output.status.success(),
            "live {backend} antialiased line recipe quality should pass, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        assert_eq!(report["verification"]["quality"]["ok"], true, "{report:#}");
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU line quality proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_quality_per_line_region() {
    let dir = artifact_dir("recipe-render-line-quality");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_line_quality_recipe(&dir, &format!("line-quality-{backend}"), 0.005, 0.0);
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let output = command
            .args(args)
            .output()
            .expect("scena line quality recipe render command runs");

        assert!(
            !output.status.success(),
            "impossible line quality threshold should fail on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU line quality failure proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().any(|check| {
                check["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("expect_quality.line.segment["))
                    && check["code"] == "line_not_straight"
                    && check["region"]["kind"] == "line"
                    && check["region"]["handle"].as_u64().is_some()
            }),
            "quality verifier must evaluate projected line regions on {backend}, not just the subject bbox: {report:#}"
        );
        assert!(png_path.exists(), "line quality render writes the PNG");
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_verify_animation_accepts_authored_recipe_clip() {
    let dir = artifact_dir("authored-animation");
    let recipe_path = dir.join("authored-animation.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "cube_blue": "#3A7BD5"
            },
            "geometries": [
                { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
            ],
            "materials": [
                { "id": "cube_mat", "kind": "unlit", "base_color": "cube_blue" }
            ],
            "nodes": [
                { "id": "cube", "geometry": "cube_geo", "material": "cube_mat" }
            ],
            "animations": [{
                "id": "move_cube",
                "duration": 1.0,
                "channels": [{
                    "target": { "kind": "node", "id": "cube" },
                    "path": "translation",
                    "interpolation": "linear",
                    "times": [0.0, 1.0],
                    "values": [[0.0, 0.0, 0.0], [0.15, 0.0, 0.0]]
                }]
            }],
            "cameras": [
                { "id": "main", "kind": "perspective", "fov_degrees": 32.0, "active": true, "transform": { "kind": "look_at", "eye": [0.25, 0.18, 0.25], "target": [0.05, 0.0, 0.0] } }
            ],
            "capture": { "width": 320, "height": 220 }
        }))
        .expect("authored animation recipe serializes"),
    )
    .expect("authored animation recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            path_str(&recipe_path),
            "--clip",
            "move_cube",
            "--times",
            "0,1",
            "--expect-change",
            "--expect-translations",
            "0,0,0;0.15,0,0",
            "--width",
            "320",
            "--height",
            "220",
        ])
        .output()
        .expect("scena verify animation authored recipe command runs");

    assert!(
        output.status.success(),
        "authored recipe animation should verify, stderr={}, stdout={}",
        stderr(&output),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "authored animation verification keeps JSON on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("animation verification emits JSON");
    assert_eq!(report["schema"], "scena.animation_introspection.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["clip"]["name"], "move_cube");
    assert_eq!(report["summary"]["changed_channel_count"], 1);
    assert_eq!(report["summary"]["invalid_channel_count"], 0);
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_color_pick_and_fit_expectations() {
    let dir = artifact_dir("recipe-render-verify-fail");
    let recipe_path = dir.join("verified-negative.recipe.json");
    let png_path = dir.join("verified-negative.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&authored_verification_recipe(json!({
            "expect_color": [{
                "id": "plate-should-not-be-green",
                "target": { "kind": "node", "id": "plate" },
                "swatch_srgb8": [0, 255, 0],
                "tolerance": 0.05
            }],
            "expect_bbox_fit": {
                "min": 0.95
            },
            "expect_pick": [{
                "id": "center-should-not-pick-side",
                "x_css_px": 64.0,
                "y_css_px": 64.0,
                "target": { "kind": "node", "id": "side" }
            }]
        })))
        .expect("negative verified recipe serializes"),
    )
    .expect("negative verified recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena negative recipe render command runs");

    assert!(!output.status.success(), "negative verification must fail");
    assert!(
        output.stderr.is_empty(),
        "recipe verification failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], false, "{report:#}");
    assert_eq!(report["build"]["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], false, "{report:#}");
    let reasons = report["verification"]["reasons"]
        .as_array()
        .expect("verification reasons array");
    for code in [
        "swatch_mismatch",
        "fit_fraction_below_min",
        "handle_mismatch",
    ] {
        assert!(
            reasons.iter().any(|reason| reason["code"] == code),
            "expected {code} reason in {report:#}"
        );
    }
    assert!(
        png_path.exists(),
        "negative verification still writes the proof PNG"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_visibility_warning_and_fit_max_expectations() {
    let dir = artifact_dir("recipe-render-verify-render-negatives");

    let fit_path = dir.join("fit-visible-negative.recipe.json");
    let fit_png = dir.join("fit-visible-negative.png");
    fs::write(
        &fit_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "red": "#DC2020",
                "blue": "#2D68C4"
            },
            "geometries": [
                { "id": "plate_geo", "primitive": { "kind": "box", "size": [0.6, 0.6, 0.08] } },
                { "id": "side_geo", "primitive": { "kind": "box", "size": [0.12, 0.12, 0.08] } }
            ],
            "materials": [
                { "id": "red_mat", "kind": "unlit", "base_color": "red" },
                { "id": "blue_mat", "kind": "unlit", "base_color": "blue" }
            ],
            "nodes": [
                {
                    "id": "plate",
                    "geometry": "plate_geo",
                    "material": "red_mat",
                    "transform": { "kind": "center" }
                },
                {
                    "id": "side",
                    "geometry": "side_geo",
                    "material": "blue_mat",
                    "visible": false,
                    "transform": {
                        "kind": "trs",
                        "translation": [0.8, 0.0, 0.0]
                    }
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "plate" }
            }],
            "capture": { "width": 128, "height": 128 },
            "expect": {
                "expect_visible": [{
                    "id": "side-must-be-drawn",
                    "target": { "kind": "node", "id": "side" }
                }],
                "expect_bbox_fit": {
                    "max": 0.10
                }
            }
        }))
        .expect("fit negative recipe serializes"),
    )
    .expect("fit negative recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&fit_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&fit_png),
        ])
        .output()
        .expect("scena fit negative recipe render command runs");

    assert!(!output.status.success(), "negative verification must fail");
    assert!(
        output.stderr.is_empty(),
        "recipe verification failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], false, "{report:#}");
    let reasons = report["verification"]["reasons"]
        .as_array()
        .expect("verification reasons array");
    for code in ["target_not_visible", "fit_fraction_above_max"] {
        assert!(
            reasons.iter().any(|reason| reason["code"] == code),
            "expected {code} reason in {report:#}"
        );
    }

    let warning_path = dir.join("warning-negative.recipe.json");
    let warning_png = dir.join("warning-negative.png");
    fs::write(
        &warning_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "red": "#DC2020" },
            "geometries": [
                { "id": "pin_geo", "primitive": { "kind": "box", "size": [0.02, 0.02, 0.02] } }
            ],
            "materials": [
                { "id": "pin_mat", "kind": "unlit", "base_color": "red" }
            ],
            "nodes": [
                { "id": "pin", "geometry": "pin_geo", "material": "pin_mat" }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 4.0], "target": "pin" }
            }],
            "capture": { "width": 128, "height": 128 },
            "expect": {
                "expect_no_warnings": true
            }
        }))
        .expect("warning negative recipe serializes"),
    )
    .expect("warning negative recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&warning_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&warning_png),
        ])
        .output()
        .expect("scena warning negative recipe render command runs");

    assert!(!output.status.success(), "warning expectation must fail");
    assert!(
        output.stderr.is_empty(),
        "recipe warning failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], false, "{report:#}");
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .any(|reason| reason["code"] == "render_warning"),
        "expected render_warning reason in {report:#}"
    );
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

#[cfg(feature = "scene-host")]
fn authored_verification_recipe(expect: serde_json::Value) -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "red": "#DC2020",
            "blue": "#2D68C4"
        },
        "geometries": [
            { "id": "plate_geo", "primitive": { "kind": "box", "size": [0.6, 0.6, 0.08] } },
            { "id": "side_geo", "primitive": { "kind": "box", "size": [0.12, 0.12, 0.08] } }
        ],
        "materials": [
            { "id": "red_mat", "kind": "unlit", "base_color": "red" },
            { "id": "blue_mat", "kind": "unlit", "base_color": "blue" }
        ],
        "nodes": [
            {
                "id": "plate",
                "geometry": "plate_geo",
                "material": "red_mat",
                "transform": { "kind": "center" }
            },
            {
                "id": "side",
                "geometry": "side_geo",
                "material": "blue_mat",
                "transform": {
                    "kind": "trs",
                    "translation": [0.8, 0.0, 0.0]
                }
            }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "fov_degrees": 36.0,
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "plate" }
        }],
        "capture": { "width": 128, "height": 128 },
        "expect": expect
    })
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

fn run_validate_recipe_fixture(name: &str) -> std::process::Output {
    let path = recipe_invalid_fixture_path(name);
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&path)])
        .output()
        .unwrap_or_else(|error| panic!("scena validate-recipe {name} runs: {error}"))
}

fn recipe_invalid_fixture_path(name: &str) -> PathBuf {
    PathBuf::from("tests")
        .join("assets")
        .join("recipe-invalid")
        .join(name)
}

fn json_report(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.stderr.is_empty(),
        "command should keep diagnostics on stdout, stderr={}",
        stderr(output)
    );
    serde_json::from_slice(&output.stdout).expect("command emits JSON")
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target")
        .join("gate-artifacts")
        .join(format!("scena-cli-recipe-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("artifact directory creates");
    dir
}

#[cfg(feature = "scene-host")]
#[derive(Debug)]
struct DecodedPng {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

#[cfg(feature = "scene-host")]
fn decode_png_rgba8(path: &Path) -> DecodedPng {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("PNG {path:?} reads: {error}"));
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG header reads");
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("PNG output buffer size is known")
    ];
    let info = reader.next_frame(&mut buffer).expect("PNG frame decodes");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    DecodedPng {
        width: info.width,
        height: info.height,
        rgba8: buffer[..info.buffer_size()].to_vec(),
    }
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QualityPixelRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[cfg(feature = "scene-host")]
fn assert_label_quality_region_covers_background_pill(region: QualityPixelRegion) {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let label = scena::LabelDesc::truetype("BATTERY", font_bytes)
        .expect("TrueType label builds")
        .with_size(42.0);
    let metrics = label.metrics();
    let padding = (label.size() * 0.25).ceil().max(2.0);
    assert!(
        region.width as f32 >= metrics.width_px + padding * 1.75
            && region.height as f32 >= metrics.height_px + padding * 1.75,
        "label quality region must include the rendered background pill padding, region={region:?}, metrics={metrics:?}, padding={padding}"
    );
}

#[cfg(feature = "scene-host")]
fn expected_label_background_region(width: u32, height: u32) -> QualityPixelRegion {
    let font_bytes = fs::read(system_test_font_path()).expect("test TrueType font reads");
    let label = scena::LabelDesc::truetype("BATTERY", font_bytes)
        .expect("TrueType label builds")
        .with_size(42.0);
    let metrics = label.metrics();
    let padding = (label.size() * 0.25).ceil().max(2.0);
    let label_width = metrics.width_px + padding * 2.0;
    let label_height = metrics.height_px + padding * 2.0;
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    let x0 = (center_x - label_width * 0.5).floor().max(0.0);
    let y0 = (center_y - label_height * 0.5).floor().max(0.0);
    let x1 = (center_x + label_width * 0.5).ceil().min(width as f32);
    let y1 = (center_y + label_height * 0.5).ceil().min(height as f32);
    QualityPixelRegion {
        x: x0 as u32,
        y: y0 as u32,
        width: (x1.max(x0) as u32).saturating_sub(x0 as u32).max(1),
        height: (y1.max(y0) as u32).saturating_sub(y0 as u32).max(1),
    }
}

#[cfg(feature = "scene-host")]
#[derive(Debug)]
struct FrameDelta {
    max_channel_delta: u8,
    mean_channel_delta: f32,
}

#[cfg(feature = "scene-host")]
fn frame_delta_in_region(
    left: &[u8],
    right: &[u8],
    frame_width: u32,
    region: QualityPixelRegion,
) -> FrameDelta {
    assert_eq!(left.len(), right.len());
    let mut max_channel_delta = 0_u8;
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            for channel in 0..3 {
                let delta = left[offset + channel].abs_diff(right[offset + channel]);
                max_channel_delta = max_channel_delta.max(delta);
                total = total.saturating_add(u64::from(delta));
                count = count.saturating_add(1);
            }
        }
    }
    FrameDelta {
        max_channel_delta,
        mean_channel_delta: total as f32 / count.max(1) as f32,
    }
}

#[cfg(feature = "scene-host")]
fn mean_luminance_in_region(rgba: &[u8], frame_width: u32, region: QualityPixelRegion) -> f32 {
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            total += 0.2126 * f32::from(rgba[offset])
                + 0.7152 * f32::from(rgba[offset + 1])
                + 0.0722 * f32::from(rgba[offset + 2]);
            count = count.saturating_add(1);
        }
    }
    total / count.max(1) as f32
}

fn system_test_font_path() -> PathBuf {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
        .expect("builder must provide a TrueType test font")
}

fn write_rgba_png(path: &Path, width: u32, height: u32, pixel: &[u8; 4]) {
    let file = fs::File::create(path).expect("reference PNG creates");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("reference PNG header writes");
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width.saturating_mul(height) {
        rgba.extend_from_slice(pixel);
    }
    writer
        .write_image_data(&rgba)
        .expect("reference PNG pixels write");
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is valid UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
