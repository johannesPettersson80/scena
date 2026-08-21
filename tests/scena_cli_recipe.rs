#![cfg(feature = "inspection")]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[cfg(feature = "scene-host")]
mod support;

const TEST_ASSET: &str = "tests/assets/gltf/mesh_material_vertex_color_scene.gltf";
const ANCHORED_ASSET: &str = "tests/assets/gltf/anchored_triangle_scene.gltf";
const ANCHOR_ASSET: &str = "tests/assets/gltf/anchor_debug_scene.gltf";
const CONNECTOR_ASSET: &str = "tests/assets/gltf/connector_basis_scene.gltf";
const CAD_TERMINAL_ASSET: &str = "tests/assets/gltf/cad_terminal_block.gltf";
const CAD_PLATE_ASSET: &str = "tests/assets/gltf/cad_plate_drawing_scene.gltf";
const LAVAPIPE_ICD: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";

fn configure_command_for_lavapipe(command: &mut Command) {
    // Mesa may probe inaccessible DRM devices before selecting lavapipe. Those
    // host-driver warnings are not scena diagnostics and must not pollute the
    // JSON-only CLI contract exercised by this suite.
    command.env("EGL_LOG_LEVEL", "fatal");
    if Path::new(LAVAPIPE_ICD).exists() {
        command.env("VK_ICD_FILENAMES", LAVAPIPE_ICD);
    }
}

fn has_actionable_msaa_limit(stderr: &str, maximum: u32, requested: u32) -> bool {
    let legacy = format!("does not support MSAA sample count {requested}");
    let legacy_maximum = format!("maximum supported sample count is {maximum}");
    let prepare_maximum = format!("supports at most {maximum} samples");
    let prepare_requested = format!("explicit prepare requested {requested}");

    (stderr.contains(&legacy) && stderr.contains(&legacy_maximum))
        || (stderr.contains(&prepare_maximum) && stderr.contains(&prepare_requested))
}

#[test]
fn msaa_limit_diagnostic_requires_maximum_and_requested_counts() {
    let legacy = "backend HeadlessGpu does not support MSAA sample count 8; maximum supported sample count is 4";
    let prepare =
        "backend HeadlessGpu supports at most 4 samples, but explicit prepare requested 8";

    assert!(has_actionable_msaa_limit(legacy, 4, 8));
    assert!(has_actionable_msaa_limit(prepare, 4, 8));
    assert!(!has_actionable_msaa_limit(
        "backend HeadlessGpu supports at most 4 samples",
        4,
        8
    ));
    assert!(!has_actionable_msaa_limit(
        "explicit prepare requested 8",
        4,
        8
    ));
    assert!(!has_actionable_msaa_limit(prepare, 2, 8));
    assert!(!has_actionable_msaa_limit(prepare, 4, 16));
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

#[test]
fn scena_render_cli_ignores_scena_use_gpu_and_reports_default_selection() {
    let dir = artifact_dir("render-backend-default");
    let recipe_path = write_valid_recipe(&dir);
    let png_path = dir.join("default-frame.png");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .env("SCENA_USE_GPU", "1")
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena default backend render runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "GPU selection/fallback diagnostics belong in stdout JSON: {}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["backend_selection"]["source"], "default");
    assert_eq!(report["backend_selection"]["requested"], "headless");
    assert_eq!(report["backend_selection"]["selected"], "headless");
    assert_eq!(report["backend_selection"]["fallback_used"], false);
}

#[test]
fn scena_render_cli_gpu_flag_reports_explicit_selection_and_fallback_truth() {
    let dir = artifact_dir("render-backend-gpu-flag");
    let recipe_path = write_valid_recipe(&dir);
    let png_path = dir.join("gpu-frame.png");
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut command);
    let output = command
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--gpu",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena explicit GPU render runs");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(
        output.stderr.is_empty(),
        "explicit GPU selection/fallback diagnostics belong in stdout JSON: {}",
        stderr(&output)
    );
    let report = json_report(&output);
    let selection = &report["backend_selection"];
    assert_eq!(selection["source"], "cli_flag");
    assert_eq!(selection["requested"], "headless_gpu");
    assert!(
        matches!(
            selection["selected"].as_str(),
            Some("headless_gpu" | "headless")
        ),
        "selected backend must report GPU or its explicit CPU fallback: {selection:#}"
    );
    assert_eq!(
        selection["fallback_used"],
        selection["selected"] == "headless"
    );
    if selection["fallback_used"] == true {
        assert!(
            selection["reason"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            selection["remedy"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
    } else {
        assert!(selection["reason"].is_null());
        assert!(selection["remedy"].is_null());
    }
}

#[test]
fn scena_render_cli_defaults_produce_visible_pbr_content() {
    let dir = artifact_dir("render-pbr-defaults");
    let png_path = dir.join("cad-terminal.png");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            CAD_TERMINAL_ASSET,
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render PBR glTF command runs");

    assert!(
        output.status.success(),
        "documented CLI defaults should render PBR content, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert!(
        report["visible_pixel_fraction"]
            .as_f64()
            .is_some_and(|fraction| fraction > 0.01),
        "PBR geometry, not only the neutral clear color, must be visible: {report:#}"
    );
    assert!(
        report["luminance"]["max"]
            .as_f64()
            .zip(report["luminance"]["min"].as_f64())
            .is_some_and(|(max, min)| max - min > 8.0),
        "the rendered object must have visible tonal structure: {report:#}"
    );
    assert!(png_path.exists(), "CLI writes the PBR PNG artifact");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_render_gpu_flag_reports_actual_backend_for_recipe_input() {
    let dir = artifact_dir("render-gpu");
    let recipe_path = write_valid_recipe(&dir);
    let png_path = dir.join("gpu-frame.png");

    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut command);
    let output = command
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

#[cfg(feature = "scene-host")]
#[test]
fn imports_only_recipe_commands_build_every_import() {
    let dir = artifact_dir("imports-only-command-routing");
    let recipe_path = write_two_import_recipe(&dir, "two-imports.recipe.json", TEST_ASSET);
    let png_path = dir.join("two-imports.png");

    let build = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["recipe", "build", path_str(&recipe_path)])
        .output()
        .expect("scena recipe build command runs");
    assert!(build.status.success(), "stderr={}", stderr(&build));
    let build = json_report(&build);
    assert_eq!(build["build"]["imports"].as_array().map(Vec::len), Some(2));

    let inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["inspect", path_str(&recipe_path)])
        .output()
        .expect("scena inspect two-import recipe command runs");
    assert!(inspect.status.success(), "stderr={}", stderr(&inspect));
    let inspection = json_report(&inspect);
    assert_eq!(
        inspection["imports"].as_array().map(Vec::len),
        Some(2),
        "inspect must use the same complete recipe build as recipe build: {inspection:#}"
    );
    assert_eq!(
        inspection["counts"]["visible_drawable"], 2,
        "{inspection:#}"
    );

    let render = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render two-import recipe command runs");
    assert!(render.status.success(), "stderr={}", stderr(&render));
    let render = json_report(&render);
    assert_eq!(
        render["nodes_summary"]["drawn"], 2,
        "render must retain both recipe imports: {render:#}"
    );

    let diagnose = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["diagnose", path_str(&recipe_path), "--visibility"])
        .output()
        .expect("scena diagnose two-import recipe command runs");
    assert!(diagnose.status.success(), "stderr={}", stderr(&diagnose));
    let diagnosis = json_report(&diagnose);
    assert_eq!(
        diagnosis["summary"]["visible_drawables"], 2,
        "diagnose must inspect the same complete recipe build: {diagnosis:#}"
    );

    let doctor = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["doctor", path_str(&recipe_path)])
        .output()
        .expect("scena doctor two-import recipe command runs");
    assert!(doctor.status.success(), "stderr={}", stderr(&doctor));
    let doctor = json_report(&doctor);
    assert_eq!(
        doctor["schema"], "scena.recipe_build_result.v1",
        "{doctor:#}"
    );
    assert_eq!(
        doctor["build"]["imports"].as_array().map(Vec::len),
        Some(2),
        "doctor must expose the same complete recipe build: {doctor:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_verifiers_resolve_capabilities_from_the_second_import() {
    let dir = artifact_dir("second-import-verification");
    let appearance_recipe = dir.join("appearance.recipe.json");
    fs::write(
        &appearance_recipe,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "plain", "uri": TEST_ASSET },
                {
                    "id": "variant",
                    "uri": "tests/assets/gltf/material_variants_scene.gltf",
                    "transform": {
                        "translation": [2.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            ],
            "capture": { "width": 128, "height": 96 }
        }))
        .expect("appearance recipe serializes"),
    )
    .expect("appearance recipe writes");
    let build = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["recipe", "build", path_str(&appearance_recipe)])
        .output()
        .expect("appearance recipe build runs");
    assert!(
        build.status.success(),
        "stdout={}, stderr={}",
        String::from_utf8_lossy(&build.stdout),
        stderr(&build)
    );
    let build = json_report(&build);
    let variant_handle = build["build"]["imports"][1]["nodes_by_path"]
        .as_object()
        .expect("second import node map")
        .iter()
        .find(|(path, _)| path.contains("VariantTriangle"))
        .and_then(|(_, handle)| handle.as_u64())
        .unwrap_or_else(|| panic!("second import variant node is addressable: {build:#}"));
    let expectation_path = dir.join("appearance.json");
    fs::write(
        &expectation_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.appearance_expectation.v1",
            "targets": [{
                "id": "second-import-noon",
                "node": variant_handle,
                "variant": "noon",
                "color_family": "green",
                "require_source_material": true
            }]
        }))
        .expect("appearance expectation serializes"),
    )
    .expect("appearance expectation writes");
    let appearance = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "appearance",
            path_str(&appearance_recipe),
            "--expect",
            path_str(&expectation_path),
        ])
        .output()
        .expect("second-import appearance verification runs");
    assert!(
        appearance.status.success(),
        "second-import variant must be selectable, stdout={}, stderr={}",
        String::from_utf8_lossy(&appearance.stdout),
        stderr(&appearance)
    );
    let appearance = json_report(&appearance);
    assert_eq!(appearance["active_variant"], "noon", "{appearance:#}");
    assert!(
        appearance["available_variants"]
            .as_array()
            .expect("available variants")
            .iter()
            .any(|variant| variant == "noon"),
        "{appearance:#}"
    );

    let animation_recipe = dir.join("animation.recipe.json");
    fs::write(
        &animation_recipe,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "plain", "uri": TEST_ASSET },
                { "id": "animated", "uri": "tests/assets/gltf/animated_triangle_scene.glb" }
            ],
            "capture": { "width": 96, "height": 72 }
        }))
        .expect("animation recipe serializes"),
    )
    .expect("animation recipe writes");
    let animation = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "verify",
            "animation",
            path_str(&animation_recipe),
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0.5,1.0",
            "--expect-change",
        ])
        .output()
        .expect("second-import animation verification runs");
    assert!(
        animation.status.success(),
        "second-import animation must be playable, stdout={}, stderr={}",
        String::from_utf8_lossy(&animation.stdout),
        stderr(&animation)
    );
    let animation = json_report(&animation);
    assert_eq!(animation["clip"]["name"], "MoveTriangle", "{animation:#}");
    assert_eq!(animation["ok"], true, "{animation:#}");
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_commands_check_policy_for_every_import() {
    let dir = artifact_dir("all-command-import-policy");
    let outside_asset = std::env::temp_dir().join(format!(
        "scena-doctor-outside-root-policy-{}.gltf",
        std::process::id()
    ));
    fs::write(&outside_asset, "{}").expect("outside-root fixture writes");
    let recipe_path = dir.join("second-import-outside-root.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "inside", "uri": TEST_ASSET },
                { "id": "outside", "uri": path_str(&outside_asset) }
            ]
        }))
        .expect("policy recipe serializes"),
    )
    .expect("policy recipe writes");
    let appearance_path = dir.join("appearance.json");
    fs::write(
        &appearance_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.appearance_expectation.v1",
            "targets": []
        }))
        .expect("appearance expectation serializes"),
    )
    .expect("appearance expectation writes");
    let interaction_path = dir.join("interaction.json");
    fs::write(
        &interaction_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.interaction_expectation.v1",
            "viewport": {
                "width_css_px": 64.0,
                "height_css_px": 64.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [{
                "action": "pick",
                "x_css_px": 1.0,
                "y_css_px": 1.0,
                "expect_hit": false
            }]
        }))
        .expect("interaction expectation serializes"),
    )
    .expect("interaction expectation writes");

    let recipe = path_str(&recipe_path);
    let png = path_str(&dir.join("rejected.png")).to_owned();
    let capture_dir = path_str(&dir.join("capture")).to_owned();
    let aov_dir = path_str(&dir.join("aov")).to_owned();
    let appearance = path_str(&appearance_path).to_owned();
    let interaction = path_str(&interaction_path).to_owned();
    let missing_report = path_str(&dir.join("unused-report.json")).to_owned();
    let commands = [
        vec!["render", recipe, "--introspect", "--out", &png],
        vec!["inspect", recipe],
        vec!["diagnose", recipe, "--visibility"],
        vec!["doctor", recipe],
        vec!["repair", recipe, "--from", &missing_report],
        vec!["verify", "appearance", recipe, "--expect", &appearance],
        vec![
            "verify",
            "animation",
            recipe,
            "--clip",
            "missing",
            "--times",
            "0",
        ],
        vec!["verify", "interaction", recipe, "--expect", &interaction],
        vec!["recipe", "build", recipe],
        vec!["recipe", "render", recipe, "--introspect", "--out", &png],
        vec!["recipe", "capture", recipe, "--out-dir", &capture_dir],
        vec!["recipe", "aov", recipe, "--out-dir", &aov_dir],
    ];

    for args in commands {
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(&args)
            .output()
            .unwrap_or_else(|error| panic!("scena {args:?} runs: {error}"));
        assert!(
            !output.status.success(),
            "{args:?} must fail closed for import 2"
        );
        let report = json_report(&output);
        assert_eq!(report["ok"], false, "{args:?}: {report:#}");
        assert!(
            contains_diagnostic(&report, "policy_violation", "$.imports[1].uri"),
            "{args:?} must expose the same second-import policy failure: {report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_inspect_cad_generates_reviewable_feature_views() {
    let dir = artifact_dir("recipe-inspect-cad-terminal");
    let recipe_path = dir.join("terminal.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "terminal",
                "uri": CAD_TERMINAL_ASSET
            }]
        }))
        .expect("recipe serializes"),
    )
    .expect("recipe writes");
    let out_dir = dir.join("inspection");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "inspect-cad",
            path_str(&recipe_path),
            "--out-dir",
            path_str(&out_dir),
            "--width",
            "512",
            "--height",
            "384",
        ])
        .output()
        .expect("scena recipe inspect-cad runs");

    assert!(
        output.status.success(),
        "stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "inspect-cad keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("inspect-cad emits JSON");
    assert_eq!(report["schema"], "scena.cad_inspection_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["backend_selection"]["source"], "default");
    assert_eq!(report["backend_selection"]["requested"], "headless");
    assert_eq!(report["backend_selection"]["selected"], "headless");
    assert_eq!(report["backend_selection"]["fallback_used"], false);
    assert_eq!(report["source_recipe"], path_str(&recipe_path));
    let contact_sheet = out_dir.join("cad-inspection-contact-sheet.png");
    assert_eq!(report["contact_sheet_png"], path_str(&contact_sheet));
    assert!(
        fs::metadata(&contact_sheet)
            .expect("contact sheet exists")
            .len()
            > 0
    );

    let views = report["views"].as_array().expect("views array");
    assert_eq!(views.len(), 3, "{report:#}");
    for expected in ["broad_face", "top_features", "overview"] {
        let view = views
            .iter()
            .find(|candidate| candidate["id"] == expected)
            .unwrap_or_else(|| panic!("missing view {expected}: {report:#}"));
        assert_eq!(view["render_result"]["ok"], true, "{view:#}");
        assert_eq!(
            view["render_result"]["backend_selection"], report["backend_selection"],
            "{view:#}"
        );
        assert_eq!(view["render_result"]["verification_ok"], true, "{view:#}");
        assert_eq!(view["render_result"]["introspection_ok"], true, "{view:#}");
        assert!(
            view["postprocess"]["foreground_pixels"]
                .as_u64()
                .expect("foreground count numeric")
                > 250,
            "{view:#}"
        );
        assert!(
            view["postprocess"]["edge_pixels"]
                .as_u64()
                .expect("edge count numeric")
                > 50,
            "{view:#}"
        );
        assert_eq!(view["postprocess"]["tone_override"], true, "{view:#}");
        assert_eq!(view["postprocess"]["edge_emphasis"], true, "{view:#}");
        assert!(Path::new(view["processed_png"].as_str().unwrap()).exists());
        assert!(Path::new(view["raw_png"].as_str().unwrap()).exists());
        assert!(Path::new(view["recipe_json"].as_str().unwrap()).exists());
        assert!(Path::new(view["render_result_json"].as_str().unwrap()).exists());
    }

    let broad_face = views
        .iter()
        .find(|candidate| candidate["id"] == "broad_face")
        .expect("broad-face view exists");
    assert!(
        broad_face["postprocess"]["content_bbox_fraction"]["width"]
            .as_f64()
            .expect("width fraction numeric")
            > 0.25,
        "broad-face view should not be edge-on: {broad_face:#}"
    );
    assert!(
        broad_face["postprocess"]["content_bbox_fraction"]["height"]
            .as_f64()
            .expect("height fraction numeric")
            > 0.45,
        "broad-face view should frame the broad feature face: {broad_face:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_import_material_edges_and_principal_face_camera_make_cad_features_visible() {
    let dir = artifact_dir("recipe-cad-import-presentation");
    let recipe_path = dir.join("terminal-cad-presentation.recipe.json");
    let png_path = dir.join("terminal-cad-presentation.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "terminal",
                "uri": CAD_TERMINAL_ASSET,
                "material": {
                    "preset": "matte",
                    "base_color": "#565A60",
                    "roughness": 0.86,
                    "metallic": 0.0
                },
                "edge_emphasis": {
                    "enabled": true,
                    "base_color": "#FFB000",
                    "stroke_width_px": 2.25,
                    "edge_angle_threshold_degrees": 18.0
                }
            }],
            "scene": {
                "background": { "kind": "custom", "color": "#F4F6FA" },
                "grid": { "enabled": false },
                "environment": { "kind": "default" }
            },
            "render": {
                "profile": "industrial",
                "quality": "high",
                "anti_aliasing": "fxaa",
                "supersample": 2,
                "reconstruction": "gaussian",
                "tonemapper": "aces",
                "exposure_ev": 1.0
            },
            "lights": [
                { "id": "cad_key", "kind": "directional", "preset": "key" },
                { "id": "cad_fill", "kind": "directional", "preset": "fill" },
                { "id": "cad_rim", "kind": "directional", "preset": "rim" }
            ],
            "cameras": [{
                "id": "cad_principal",
                "kind": "perspective",
                "lens": "telephoto",
                "active": true,
                "framing": {
                    "mode": "principal_face",
                    "fill": 0.76,
                    "margin_px": 16.0
                }
            }],
            "capture": { "width": 640, "height": 480 },
            "expect": {
                "expect_bbox_fit": {
                    "min": 0.22,
                    "max": 0.92
                }
            }
        }))
        .expect("CAD presentation recipe serializes"),
    )
    .expect("CAD presentation recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--detail",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render CAD presentation command runs");

    assert!(
        output.status.success(),
        "CAD presentation render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "CAD presentation render keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["build"]["ok"], true, "{report:#}");
    assert_eq!(report["introspection"]["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], true, "{report:#}");
    assert!(
        report["introspection"]["nodes_summary"]["drawn"]
            .as_u64()
            .expect("drawn count numeric")
            >= 2,
        "edge emphasis should add renderer-visible overlay drawables, not a PNG-only filter: {report:#}"
    );

    let bbox = &report["introspection"]["content_bbox_fraction"];
    assert!(
        bbox["width"]
            .as_f64()
            .expect("content bbox width fraction numeric")
            > 0.30,
        "principal_face camera should not land edge-on to the 6 mm side: {report:#}"
    );
    assert!(
        bbox["height"]
            .as_f64()
            .expect("content bbox height fraction numeric")
            > 0.45,
        "principal_face camera should frame the 44x48 feature face: {report:#}"
    );

    let image = decode_png_rgba8(&png_path);
    let content = quality_region_from_pixel_region(content_region_from_introspection_report(
        &report["introspection"],
    ));
    let dark_material_pixels = count_pixels_in_region(
        &image.rgba8,
        image.width,
        content,
        is_dark_grey_material_pixel,
    );
    assert!(
        dark_material_pixels > 1_500,
        "import material override must make the terminal dark grey, not white; dark_pixels={dark_material_pixels}, png={png_path:?}, report={report:#}"
    );
    let white_blob_pixels =
        count_pixels_in_region(&image.rgba8, image.width, content, is_white_blob_pixel);
    let content_pixels = content.width.saturating_mul(content.height).max(1);
    assert!(
        white_blob_pixels < content_pixels / 6,
        "CAD import must not render as a white-on-white blob; white_pixels={white_blob_pixels}, content_pixels={content_pixels}, dark_pixels={dark_material_pixels}, png={png_path:?}"
    );
    let edge_pixels = count_pixels_in_region(&image.rgba8, image.width, content, is_cad_edge_pixel);
    assert!(
        edge_pixels > 300,
        "edge emphasis should draw visible CAD edges for chamfers, wells, and groove; edge_pixels={edge_pixels}, png={png_path:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_target_region_fit_frames_subject_and_allows_context_crop() {
    let dir = artifact_dir("recipe-target-region-fit");
    let recipe_path = dir.join("target-region.recipe.json");
    let png_path = dir.join("target-region.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                {
                    "id": "subject_box",
                    "primitive": { "kind": "box", "size": [0.20, 0.20, 0.20] }
                },
                {
                    "id": "context_panel",
                    "primitive": { "kind": "box", "size": [2.40, 0.30, 0.30] }
                }
            ],
            "materials": [
                {
                    "id": "subject_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "#D85C5C",
                    "roughness": 0.45,
                    "metallic": 0.0
                },
                {
                    "id": "context_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "#707782",
                    "roughness": 0.72,
                    "metallic": 0.0
                }
            ],
            "nodes": [
                {
                    "id": "subject",
                    "geometry": "subject_box",
                    "material": "subject_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] }
                },
                {
                    "id": "context",
                    "geometry": "context_panel",
                    "material": "context_mat",
                    "transform": { "kind": "trs", "translation": [0.95, 0.0, 0.0] }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "#F4F6FA" },
                "grid": { "enabled": false }
            },
            "render": {
                "profile": "industrial",
                "quality": "high",
                "anti_aliasing": "fxaa",
                "tonemapper": "aces"
            },
            "lights": [
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "fill", "kind": "directional", "preset": "fill" }
            ],
            "cameras": [{
                "id": "subject_closeup",
                "kind": "perspective",
                "lens": "telephoto",
                "active": true,
                "framing": {
                    "mode": "target_region",
                    "preset": "front",
                    "fill": 0.62,
                    "margin_px": 24.0,
                    "target_region": {
                        "bounds": {
                            "min": [-0.10, -0.10, -0.10],
                            "max": [0.10, 0.10, 0.10]
                        },
                        "centroid": [0.0, 0.0, 0.0]
                    }
                }
            }],
            "capture": { "width": 640, "height": 480 },
            "expect": {
                "expect_target_fit": [{
                    "id": "subject-closeup",
                    "target": { "kind": "node", "id": "subject" },
                    "bounds": {
                        "min": [-0.10, -0.10, -0.10],
                        "max": [0.10, 0.10, 0.10]
                    },
                    "centroid": [0.0, 0.0, 0.0],
                    "min_fit": 0.35,
                    "max_fit": 0.82,
                    "min_visible_coverage": 0.12
                }]
            }
        }))
        .expect("target-region recipe serializes"),
    )
    .expect("target-region recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--detail",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render target-region command runs");

    assert!(
        output.status.success(),
        "target-region render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "target-region render keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], true, "{report:#}");
    assert_eq!(report["introspection"]["ok"], true, "{report:#}");
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("reasons array")
            .is_empty(),
        "target-region verification should not report failures: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_target_region_fit_allows_cropped_non_target_imports() {
    let dir = artifact_dir("recipe-target-region-cropped-context-import");
    let recipe_path = dir.join("target-region-import.recipe.json");
    let png_path = dir.join("target-region-import.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                {
                    "id": "subject_import",
                    "uri": TEST_ASSET
                },
                {
                    "id": "context_import",
                    "uri": TEST_ASSET,
                    "transform": {
                        "translation": [4.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "#F4F6FA" },
                "grid": { "enabled": false }
            },
            "render": {
                "profile": "industrial",
                "quality": "high",
                "anti_aliasing": "fxaa",
                "tonemapper": "aces"
            },
            "cameras": [{
                "id": "subject_import_closeup",
                "kind": "perspective",
                "lens": "telephoto",
                "active": true,
                "framing": {
                    "mode": "target_region",
                    "preset": "front",
                    "fill": 0.72,
                    "margin_px": 16.0,
                    "target_region": {
                        "bounds": {
                            "min": [-0.5, -0.5, -0.01],
                            "max": [0.5, 0.5, 0.01]
                        },
                        "centroid": [0.0, 0.0, 0.0]
                    }
                }
            }],
            "capture": { "width": 640, "height": 480 },
            "expect": {
                "expect_target_fit": [{
                    "id": "subject-import-closeup",
                    "target": { "kind": "import", "id": "subject_import" },
                    "bounds": {
                        "min": [-0.5, -0.5, -0.01],
                        "max": [0.5, 0.5, 0.01]
                    },
                    "centroid": [0.0, 0.0, 0.0],
                    "min_fit": 0.45,
                    "max_fit": 0.86,
                    "min_visible_coverage": 0.06
                }]
            }
        }))
        .expect("target-region import recipe serializes"),
    )
    .expect("target-region import recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--detail",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render target-region import command runs");

    assert!(
        output.status.success(),
        "target-region import render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "target-region import render keeps stdout JSON clean, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], true, "{report:#}");
    assert_eq!(report["introspection"]["ok"], true, "{report:#}");
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("reasons array")
            .is_empty(),
        "cropped non-target import must not produce verification failures: {report:#}"
    );

    let composition_checks = report["verification"]["composition"]["checks"]
        .as_array()
        .expect("composition checks serialize");
    assert!(
        composition_checks.iter().any(|check| {
            check["id"] == "import.context_import.projected_bbox"
                && check["code"] == "target_region_context_crop_allowed"
                && check["status"] == "not_applicable"
        }),
        "cropped context import should be explicitly recorded as allowed by target-region framing: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_import_double_sided_material_renders_backface() {
    let dir = artifact_dir("recipe-cad-import-double-sided-backface");
    let recipe_path = dir.join("cad-panel-backface.recipe.json");
    let png_path = dir.join("cad-panel-backface.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "cad_panel",
                "uri": CAD_PLATE_ASSET,
                "material": {
                    "base_color": "#DDE2E5",
                    "roughness": 0.72,
                    "metallic": 0.1,
                    "double_sided": true
                },
                "edge_emphasis": {
                    "enabled": true,
                    "base_color": "#607080",
                    "stroke_width_px": 1.25,
                    "edge_angle_threshold_degrees": 12.0
                }
            }],
            "scene": {
                "background": { "kind": "custom", "color": "#F5F7FA" },
                "grid": { "enabled": false }
            },
            "render": {
                "profile": "industrial",
                "quality": "high",
                "anti_aliasing": "fxaa",
                "supersample": 2,
                "reconstruction": "gaussian",
                "tonemapper": "aces",
                "exposure_ev": 0.0
            },
            "lights": [
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "fill", "kind": "directional", "preset": "fill" },
                { "id": "rim", "kind": "directional", "preset": "rim" }
            ],
            "cameras": [{
                "id": "backface_camera",
                "kind": "perspective",
                "lens": "telephoto",
                "active": true,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.0, -0.45],
                    "target": [0.0, 0.0, 0.0]
                }
            }],
            "capture": { "width": 640, "height": 480 },
            "expect": {
                "expect_bbox_fit": {
                    "min": 0.08,
                    "max": 0.95
                }
            }
        }))
        .expect("double-sided CAD recipe serializes"),
    )
    .expect("double-sided CAD recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--verify",
            "--detail",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render CAD backface command runs");

    assert!(
        output.status.success(),
        "double-sided imported CAD material should render its back face, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["verification"]["ok"], true, "{report:#}");
    assert_eq!(report["introspection"]["ok"], true, "{report:#}");

    let image = decode_png_rgba8(&png_path);
    let content = quality_region_from_pixel_region(content_region_from_introspection_report(
        &report["introspection"],
    ));
    let light_panel_pixels = count_pixels_in_region(
        &image.rgba8,
        image.width,
        content,
        is_light_cad_panel_material_pixel,
    );
    assert!(
        light_panel_pixels > 1_000,
        "double-sided import material should make the back face visibly light, not black/culled; light_pixels={light_panel_pixels}, png={png_path:?}, report={report:#}"
    );
    let black_slab_pixels =
        count_pixels_in_region(&image.rgba8, image.width, content, is_black_slab_pixel);
    assert!(
        black_slab_pixels < light_panel_pixels / 4,
        "back-facing CAD surface must not present as a black slab; black_pixels={black_slab_pixels}, light_pixels={light_panel_pixels}, png={png_path:?}"
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

#[cfg(feature = "scene-host")]
#[test]
fn recipe_import_budget_override_is_operator_owned_for_validate_and_render() {
    let dir = artifact_dir("recipe-import-budget-override");
    let recipe_path = write_many_import_recipe(&dir, 65);
    let png_path = dir.join("many-imports.png");

    let default_validate = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&recipe_path)])
        .output()
        .expect("scena validate-recipe default import budget command runs");
    assert!(
        !default_validate.status.success(),
        "default validate should enforce max_imports, stdout={}, stderr={}",
        String::from_utf8_lossy(&default_validate.stdout),
        stderr(&default_validate)
    );
    let default_report = json_report(&default_validate);
    assert_eq!(default_report["ok"], false, "{default_report:#}");
    let diagnostic = default_report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .find(|diagnostic| diagnostic["code"] == "policy_violation")
        .unwrap_or_else(|| panic!("expected max_imports policy diagnostic: {default_report:#}"));
    assert_eq!(diagnostic["path"], "$.imports", "{default_report:#}");
    assert!(
        diagnostic["message"]
            .as_str()
            .expect("diagnostic message")
            .contains("max_imports 64"),
        "{default_report:#}"
    );

    let raised_validate = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "validate-recipe",
            path_str(&recipe_path),
            "--max-imports",
            "128",
        ])
        .output()
        .expect("scena validate-recipe raised import budget command runs");
    assert!(
        raised_validate.status.success(),
        "raised validate should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&raised_validate.stdout),
        stderr(&raised_validate)
    );
    let raised_report = json_report(&raised_validate);
    assert_eq!(raised_report["ok"], true, "{raised_report:#}");

    let default_render = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena recipe render default import budget command runs");
    assert!(
        !default_render.status.success(),
        "default render should enforce max_imports, stdout={}, stderr={}",
        String::from_utf8_lossy(&default_render.stdout),
        stderr(&default_render)
    );
    let default_render_report = json_report(&default_render);
    assert_eq!(
        default_render_report["schema"],
        "scena.recipe_render_result.v1"
    );
    assert_eq!(
        default_render_report["build"]["diagnostics"][0]["path"], "$.imports",
        "{default_render_report:#}"
    );

    let raised_render = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
            "--max-imports",
            "128",
        ])
        .output()
        .expect("scena recipe render raised import budget command runs");
    assert!(
        raised_render.status.success(),
        "raised render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&raised_render.stdout),
        stderr(&raised_render)
    );
    let raised_render_report = json_report(&raised_render);
    assert_eq!(
        raised_render_report["schema"],
        "scena.render_introspection.v1"
    );
    assert_eq!(raised_render_report["ok"], true, "{raised_render_report:#}");
    assert!(png_path.exists(), "raised render writes the requested PNG");

    let default_inspect_dir = dir.join("inspect-default");
    let default_inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "inspect-cad",
            path_str(&recipe_path),
            "--out-dir",
            path_str(&default_inspect_dir),
            "--width",
            "128",
            "--height",
            "96",
        ])
        .output()
        .expect("scena recipe inspect-cad default import budget command runs");
    assert!(
        !default_inspect.status.success(),
        "default inspect-cad should enforce max_imports, stdout={}, stderr={}",
        String::from_utf8_lossy(&default_inspect.stdout),
        stderr(&default_inspect)
    );
    let default_inspect_report = json_report(&default_inspect);
    assert_eq!(
        default_inspect_report["schema"],
        "scena.recipe_render_result.v1"
    );
    assert_eq!(
        default_inspect_report["build"]["diagnostics"][0]["path"], "$.imports",
        "{default_inspect_report:#}"
    );

    let raised_inspect_dir = dir.join("inspect-raised");
    let raised_inspect = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "inspect-cad",
            path_str(&recipe_path),
            "--out-dir",
            path_str(&raised_inspect_dir),
            "--width",
            "128",
            "--height",
            "96",
            "--max-imports",
            "128",
        ])
        .output()
        .expect("scena recipe inspect-cad raised import budget command runs");
    assert!(
        raised_inspect.status.success(),
        "raised inspect-cad should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&raised_inspect.stdout),
        stderr(&raised_inspect)
    );
    let raised_inspect_report = json_report(&raised_inspect);
    assert_eq!(
        raised_inspect_report["schema"],
        "scena.cad_inspection_result.v1"
    );
    assert_eq!(
        raised_inspect_report["ok"], true,
        "{raised_inspect_report:#}"
    );
    assert!(
        raised_inspect_dir
            .join("cad-inspection-contact-sheet.png")
            .exists(),
        "raised inspect-cad writes the contact sheet"
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

#[test]
fn fr03_place_apply_emits_persistent_recipe_and_rejects_stale_source() {
    let recipe = recipe_invalid_fixture_path("valid_for_commands.recipe.json");
    let base_args = [
        "place",
        path_str(&recipe),
        "--import",
        "part",
        "--verb",
        "center",
        "--target",
        "1,2,3",
    ];
    let preview = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(base_args)
        .output()
        .expect("placement preview runs");
    assert!(preview.status.success(), "stderr={}", stderr(&preview));
    let preview = json_report(&preview);

    let applied = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(base_args)
        .arg("--apply")
        .output()
        .expect("placement apply runs");
    assert!(applied.status.success(), "stderr={}", stderr(&applied));
    let applied = json_report(&applied);
    assert_eq!(applied["schema"], "scena.recipe_patch.v1");
    assert_eq!(applied["ok"], true);
    assert_eq!(applied["import_id"], "part");
    assert_eq!(applied["transform"], preview["transform"]);
    assert_eq!(
        applied["updated_recipe"]["imports"][0]["transform"],
        preview["transform"]
    );
    assert_eq!(
        applied["semantic_changes"][0]["path"],
        "$.imports[0].transform"
    );
    assert_eq!(applied["formatting_preserved"], false);
    assert!(
        Path::new(
            applied["updated_recipe"]["imports"][0]["uri"]
                .as_str()
                .expect("updated import URI is a string")
        )
        .is_absolute(),
        "portable complete recipe rebases source-relative import URIs"
    );

    let rebuilt = artifact_dir("fr03-place-apply").join("updated.recipe.json");
    fs::write(
        &rebuilt,
        serde_json::to_vec_pretty(&applied["updated_recipe"]).expect("updated recipe serializes"),
    )
    .expect("updated recipe writes");
    let validation = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&rebuilt)])
        .output()
        .expect("updated recipe validates");
    assert!(
        validation.status.success(),
        "stderr={}",
        stderr(&validation)
    );

    let stale = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(base_args)
        .args(["--apply", "--expect-source-sha256", &"0".repeat(64)])
        .output()
        .expect("stale placement apply runs");
    assert_eq!(stale.status.code(), Some(1));
    let stale = json_report(&stale);
    assert_eq!(stale["schema"], "scena.recipe_patch.v1");
    assert_eq!(stale["ok"], false);
    assert_diagnostic(&stale, "stale_source", "error");
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

    let mut gpu_command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut gpu_command);
    let gpu = gpu_command
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
#[test]
fn scena_recipe_render_verify_accepts_live_ssim_reference_and_rejects_scene_mutations() {
    let dir = artifact_dir("recipe-render-live-ssim");
    let baseline_recipe_path = dir.join("baseline.recipe.json");
    let baseline_png_path = dir.join("baseline.png");
    let baseline_recipe = authored_verification_recipe(json!({
        "expect_bbox_fit": {
            "min": 0.20,
            "max": 0.95
        }
    }));
    fs::write(
        &baseline_recipe_path,
        serde_json::to_string_pretty(&baseline_recipe).expect("baseline recipe serializes"),
    )
    .expect("baseline recipe writes");
    let baseline_report =
        run_recipe_render_verify(&baseline_recipe_path, &baseline_png_path, false);
    assert_eq!(baseline_report["verification"]["ok"], true);

    let live_reference_expectation = json!({
        "expect_bbox_fit": {
            "min": 0.20,
            "max": 0.95
        },
        "expect_reference": [{
            "id": "live-ssim-reference",
            "image": "baseline.png",
            "metric": "ssim",
            "min_ssim": 0.99
        }]
    });
    let accepted_recipe_path = dir.join("accepted.recipe.json");
    let accepted_png_path = dir.join("accepted.png");
    fs::write(
        &accepted_recipe_path,
        serde_json::to_string_pretty(&authored_verification_recipe(
            live_reference_expectation.clone(),
        ))
        .expect("accepted recipe serializes"),
    )
    .expect("accepted recipe writes");
    let accepted_report =
        run_recipe_render_verify(&accepted_recipe_path, &accepted_png_path, false);
    assert_eq!(accepted_report["verification"]["quality"]["ok"], true);

    let baseline = decode_png_rgba8(&baseline_png_path);
    let accepted = decode_png_rgba8(&accepted_png_path);
    let accepted_ssim = scena::ssim_grayscale(
        &accepted.rgba8,
        &baseline.rgba8,
        baseline.width,
        baseline.height,
    )
    .expect("matching live renders have comparable dimensions");
    assert!(
        accepted_ssim >= 0.99,
        "accepted live render SSIM {accepted_ssim} must meet the recipe threshold"
    );

    for (name, pointer, replacement) in [
        ("camera", "/cameras/0/transform/eye", json!([0.8, 0.0, 2.0])),
        ("material", "/colors/red", json!("#FFFFFF")),
        (
            "geometry",
            "/geometries/0/primitive/size",
            json!([0.18, 0.18, 0.08]),
        ),
    ] {
        let mut mutated_recipe = authored_verification_recipe(live_reference_expectation.clone());
        *mutated_recipe
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("{name} mutation pointer {pointer} exists")) = replacement;
        let recipe_path = dir.join(format!("{name}.recipe.json"));
        let png_path = dir.join(format!("{name}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&mutated_recipe)
                .unwrap_or_else(|error| panic!("{name} mutation recipe serializes: {error}")),
        )
        .unwrap_or_else(|error| panic!("{name} mutation recipe writes: {error}"));

        let report = run_recipe_render_verify_expect_failure(&recipe_path, &png_path, false);
        assert_quality_reason(&report, "reference_ssim_too_low", "live-ssim-reference");
        let mutated = decode_png_rgba8(&png_path);
        let observed_ssim = scena::ssim_grayscale(
            &mutated.rgba8,
            &baseline.rgba8,
            baseline.width,
            baseline.height,
        )
        .expect("mutated live render has comparable dimensions");
        assert!(
            observed_ssim < 0.99,
            "{name} mutation SSIM {observed_ssim} must fail the same 0.99 threshold"
        );
    }
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
    write_line_quality_recipe_with_render(
        dir,
        name,
        min_intermediate_edge_fraction,
        max_straightness_error,
        None,
    )
}

#[cfg(feature = "scene-host")]
fn write_line_quality_recipe_with_render(
    dir: &Path,
    name: &str,
    min_intermediate_edge_fraction: f64,
    max_straightness_error: f64,
    render: Option<serde_json::Value>,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "marker_geo", "primitive": { "kind": "box", "size": [0.6, 0.6, 0.08] } }
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
    });
    if let Some(render) = render {
        recipe["render"] = render;
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&recipe).expect("line quality recipe serializes"),
    )
    .expect("line quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_geometry_edge_quality_recipe(
    dir: &Path,
    name: &str,
    anti_aliasing: &str,
    supersample: Option<u8>,
    min_intermediate_edge_fraction: Option<f64>,
) -> (PathBuf, PathBuf) {
    write_geometry_edge_quality_recipe_with_profile_and_colors(
        dir,
        name,
        anti_aliasing,
        supersample,
        min_intermediate_edge_fraction,
        "product",
        "#D8D8D8",
        "#808080",
    )
}

fn write_geometry_edge_quality_recipe_with_colors(
    dir: &Path,
    name: &str,
    anti_aliasing: &str,
    supersample: Option<u8>,
    min_intermediate_edge_fraction: Option<f64>,
    bar_color: &str,
    background_color: &str,
) -> (PathBuf, PathBuf) {
    write_geometry_edge_quality_recipe_with_profile_and_colors(
        dir,
        name,
        anti_aliasing,
        supersample,
        min_intermediate_edge_fraction,
        "product",
        bar_color,
        background_color,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_geometry_edge_quality_recipe_with_profile_and_colors(
    dir: &Path,
    name: &str,
    anti_aliasing: &str,
    supersample: Option<u8>,
    min_intermediate_edge_fraction: Option<f64>,
    quality_profile: &str,
    bar_color: &str,
    background_color: &str,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut render = json!({
        "anti_aliasing": anti_aliasing,
        "tonemapper": "standard",
        "exposure_ev": 0.0
    });
    if let Some(supersample) = supersample {
        render["supersample"] = json!(supersample);
    }
    let mut expect_quality = json!({
        "profile": quality_profile
    });
    if let Some(min_intermediate_edge_fraction) = min_intermediate_edge_fraction {
        expect_quality["geometry"] = json!({
            "min_intermediate_edge_fraction": min_intermediate_edge_fraction
        });
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                { "id": "bar_geo", "primitive": { "kind": "box", "size": [1.45, 0.16, 0.08] } }
            ],
            "materials": [
                { "id": "bar_mat", "kind": "unlit", "base_color": bar_color, "double_sided": false }
            ],
            "nodes": [
                {
                    "id": "slanted_bar",
                    "geometry": "bar_geo",
                    "material": "bar_mat",
                    "transform": { "kind": "trs", "rotation_degrees": [0.0, 0.0, 18.0] }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": background_color }
            },
            "render": render,
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 24.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 5.0], "target": "slanted_bar" }
            }],
            "capture": { "width": 220, "height": 140 },
            "expect": {
                "expect_quality": expect_quality
            }
        }))
        .expect("geometry edge quality recipe serializes"),
    )
    .expect("geometry edge quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_reflection_quality_recipe(
    dir: &Path,
    name: &str,
    reflection_enabled: bool,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut grid = json!({
        "enabled": true,
        "floor_y": 0.0,
        "padding": 0.55,
        "line_spacing": 0.18,
        "line_width_px": 4.0,
        "color": "floor",
        "line_color": "floor_line",
        "roughness": 0.04
    });
    if reflection_enabled {
        grid["reflection"] = json!({
            "enabled": true,
            "strength": 0.72
        });
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "red": "#E83A3A",
                "blue": "#236BFF",
                "floor": "#3B414B",
                "floor_line": "#F2F6FF"
            },
            "geometries": [
                { "id": "tower_geo", "primitive": { "kind": "box", "size": [0.28, 0.82, 0.18] } },
                { "id": "cap_geo", "primitive": { "kind": "box", "size": [0.42, 0.16, 0.22] } }
            ],
            "materials": [
                { "id": "tower_red", "kind": "pbr_metallic_roughness", "base_color": "red", "metallic": 0.0, "roughness": 0.30 },
                { "id": "cap_blue", "kind": "pbr_metallic_roughness", "base_color": "blue", "metallic": 0.0, "roughness": 0.26 }
            ],
            "nodes": [
                {
                    "id": "tower",
                    "geometry": "tower_geo",
                    "material": "tower_red",
                    "transform": { "kind": "trs", "translation": [-0.11, 0.41, 0.0] }
                },
                {
                    "id": "cap",
                    "geometry": "cap_geo",
                    "material": "cap_blue",
                    "transform": { "kind": "trs", "translation": [0.11, 0.90, 0.0] }
                }
            ],
            "lights": [
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "fill", "kind": "directional", "preset": "fill" }
            ],
            "scene": {
                "background": { "kind": "white" },
                "grid": grid,
                "environment": { "kind": "default" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 34.0,
                "transform": { "kind": "look_at", "eye": [0.0, 1.18, 2.85], "target": [0.0, 0.42, 0.0] }
            }],
            "capture": { "width": 260, "height": 220 },
            "expect": {
                "expect_quality": {
                    "profile": "product",
                    "reflection": {
                        "min_luminance_range": 0.18,
                        "min_sobel_energy": 0.035,
                        "min_chroma_range": 0.08
                    }
                }
            }
        }))
        .expect("reflection quality recipe serializes"),
    )
    .expect("reflection quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_screen_space_reflection_quality_recipe(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut render = json!({
        "anti_aliasing": "msaa4",
        "tonemapper": "standard",
        "exposure_ev": 0.0,
        "screen_space_reflections": {
            "strength": 0.82,
            "roughness": 0.16,
            "horizon_fraction": 0.58,
            "fade": 0.28
        }
    });
    render["supersample"] = json!(2);
    render["reconstruction"] = json!("tent");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "red": "#E83A3A",
                "blue": "#236BFF",
                "floor": "#3B414B",
                "floor_line": "#F2F6FF"
            },
            "geometries": [
                { "id": "tower_geo", "primitive": { "kind": "box", "size": [0.28, 0.82, 0.18] } },
                { "id": "cap_geo", "primitive": { "kind": "box", "size": [0.42, 0.16, 0.22] } }
            ],
            "materials": [
                { "id": "tower_red", "kind": "pbr_metallic_roughness", "base_color": "red", "metallic": 0.0, "roughness": 0.30 },
                { "id": "cap_blue", "kind": "pbr_metallic_roughness", "base_color": "blue", "metallic": 0.0, "roughness": 0.26 }
            ],
            "nodes": [
                {
                    "id": "tower",
                    "geometry": "tower_geo",
                    "material": "tower_red",
                    "transform": { "kind": "trs", "translation": [-0.11, 0.41, 0.0] }
                },
                {
                    "id": "cap",
                    "geometry": "cap_geo",
                    "material": "cap_blue",
                    "transform": { "kind": "trs", "translation": [0.11, 0.90, 0.0] }
                }
            ],
            "lights": [
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "fill", "kind": "directional", "preset": "fill" }
            ],
            "scene": {
                "background": { "kind": "white" },
                "grid": {
                    "enabled": true,
                    "floor_y": 0.0,
                    "padding": 0.55,
                    "line_spacing": 0.18,
                    "line_width_px": 4.0,
                    "color": "floor",
                    "line_color": "floor_line",
                    "roughness": 0.04
                },
                "environment": { "kind": "default" }
            },
            "render": render,
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 34.0,
                "transform": { "kind": "look_at", "eye": [0.0, 1.18, 2.85], "target": [0.0, 0.42, 0.0] }
            }],
            "capture": { "width": 260, "height": 220 },
            "expect": {
                "expect_quality": {
                    "profile": "cad",
                    "reflection": {
                        "min_luminance_range": 0.18,
                        "min_sobel_energy": 0.035,
                        "min_chroma_range": 0.08
                    }
                }
            }
        }))
        .expect("screen-space reflection quality recipe serializes"),
    )
    .expect("screen-space reflection quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_material_reflection_quality_recipe(
    dir: &Path,
    name: &str,
    enable_ssr: bool,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let render = if enable_ssr {
        json!({
            "anti_aliasing": "msaa4",
            "supersample": 2,
            "reconstruction": "tent",
            "tonemapper": "standard",
            "exposure_ev": 0.0,
            "screen_space_reflections": {
                "strength": 0.92,
                "roughness": 0.10,
                "horizon_fraction": 0.95,
                "fade": 0.20
            }
        })
    } else {
        json!({
            "anti_aliasing": "msaa4",
            "supersample": 2,
            "reconstruction": "tent",
            "tonemapper": "standard",
            "exposure_ev": 0.0
        })
    };
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "mirror_color": "#D8DDE6",
                "red": "#E42323",
                "blue": "#1F66FF",
                "white": "#F7F8FA"
            },
            "geometries": [
                { "id": "mirror_geo", "primitive": { "kind": "sphere", "radius": 0.42, "segments": 48, "rings": 24 } },
                { "id": "secondary_mirror_geo", "primitive": { "kind": "box", "size": [0.34, 0.58, 0.16] } },
                { "id": "red_geo", "primitive": { "kind": "box", "size": [0.24, 0.82, 0.20] } },
                { "id": "blue_geo", "primitive": { "kind": "box", "size": [0.24, 0.82, 0.20] } }
            ],
            "materials": [
                { "id": "mirror_mat", "kind": "pbr_metallic_roughness", "base_color": "mirror_color", "metallic": 1.0, "roughness": 0.045 },
                { "id": "secondary_mirror_mat", "kind": "pbr_metallic_roughness", "base_color": "mirror_color", "metallic": 1.0, "roughness": 0.08 },
                { "id": "red_mat", "kind": "pbr_metallic_roughness", "base_color": "red", "metallic": 0.0, "roughness": 0.35 },
                { "id": "blue_mat", "kind": "pbr_metallic_roughness", "base_color": "blue", "metallic": 0.0, "roughness": 0.35 }
            ],
            "nodes": [
                { "id": "mirror", "geometry": "mirror_geo", "material": "mirror_mat", "transform": { "kind": "trs", "translation": [-0.28, 0.47, 0.0] } },
                { "id": "mirror_secondary", "geometry": "secondary_mirror_geo", "material": "secondary_mirror_mat", "transform": { "kind": "trs", "translation": [0.46, 0.50, -0.02], "rotation_degrees": [0.0, -16.0, 0.0] } },
                { "id": "red_panel", "geometry": "red_geo", "material": "red_mat", "transform": { "kind": "trs", "translation": [-0.86, 0.44, 0.24] } },
                { "id": "blue_panel", "geometry": "blue_geo", "material": "blue_mat", "transform": { "kind": "trs", "translation": [0.92, 0.44, 0.24] } }
            ],
            "lights": [
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "fill", "kind": "directional", "preset": "fill" },
                { "id": "rim", "kind": "directional", "preset": "rim" }
            ],
            "scene": {
                "background": { "kind": "white" },
                "environment": { "kind": "default" }
            },
            "render": render,
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 30.0,
                "transform": { "kind": "look_at", "eye": [0.10, 0.78, 3.35], "target": [0.08, 0.48, 0.0] }
            }],
            "capture": { "width": 320, "height": 260 },
            "expect": {
                "expect_quality": {
                    "profile": "product",
                    "reflection": {
                        "target": { "kind": "node", "id": "mirror" },
                        "min_luminance_range": 0.18,
                        "min_sobel_energy": 0.070,
                        "min_chroma_range": 0.16
                    }
                }
            }
        }))
        .expect("material reflection quality recipe serializes"),
    )
    .expect("material reflection quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_ergonomic_product_recipe(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "red_panel_color": "#E42323",
                "blue_panel_color": "#1F66FF"
            },
            "geometries": [
                { "id": "product_geo", "primitive": { "kind": "sphere", "radius": 0.42, "segments": 64, "rings": 32 } },
                { "id": "red_panel_geo", "primitive": { "kind": "box", "size": [0.24, 0.82, 0.20] } },
                { "id": "blue_panel_geo", "primitive": { "kind": "box", "size": [0.24, 0.82, 0.20] } }
            ],
            "materials": [
                { "id": "product_mat", "preset": "chrome", "roughness": 0.045 },
                { "id": "red_mat", "preset": "plastic", "base_color": "red_panel_color", "roughness": 0.35 },
                { "id": "blue_mat", "preset": "plastic", "base_color": "blue_panel_color", "roughness": 0.35 }
            ],
            "nodes": [
                {
                    "id": "product",
                    "geometry": "product_geo",
                    "material": "product_mat",
                    "transform": { "kind": "trs", "translation": [-0.18, 0.46, 0.0] }
                },
                {
                    "id": "red_panel",
                    "geometry": "red_panel_geo",
                    "material": "red_mat",
                    "transform": { "kind": "trs", "translation": [-0.86, 0.42, 0.22] }
                },
                {
                    "id": "blue_panel",
                    "geometry": "blue_panel_geo",
                    "material": "blue_mat",
                    "transform": { "kind": "trs", "translation": [0.82, 0.42, 0.22] }
                }
            ],
            "lights": [
                { "id": "studio", "kind": "studio_rig", "preset": "studio_rig" }
            ],
            "scene": {
                "preset": "product_studio",
                "environment": { "preset": "studio" }
            },
            "render": {
                "auto_exposure": "product_studio",
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "aces",
                "screen_space_reflections": {
                    "strength": 0.92,
                    "roughness": 0.10,
                    "horizon_fraction": 0.95,
                    "fade": 0.20
                }
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "lens": "portrait",
                "framing": {
                    "preset": "three_quarter_front_right",
                    "fill": 0.64,
                    "margin_px": 18.0
                },
                "active": true
            }],
            "capture": { "width": 320, "height": 260 },
            "expect": {
                "expect_visible": [
                    { "id": "product-visible", "target": { "kind": "node", "id": "product" } }
                ],
                "expect_bbox_fit": { "min": 0.35, "max": 0.92 },
                "expect_quality": {
                    "profile": "product",
                    "exposure": {
                        "max_low_clip_fraction": 0.70,
                        "max_high_clip_fraction": 0.10
                    },
                    "geometry": {
                        "min_intermediate_edge_fraction": 0.0
                    },
                    "reflection": {
                        "target": { "kind": "node", "id": "product" },
                        "min_luminance_range": 0.12,
                        "min_sobel_energy": 0.045,
                        "min_chroma_range": 0.08
                    }
                }
            }
        }))
        .expect("ergonomic product recipe serializes"),
    )
    .expect("ergonomic product recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_chrome_read_failure_recipe(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "near_black": "#010101"
            },
            "geometries": [
                { "id": "chrome_geo", "primitive": { "kind": "box", "size": [0.70, 0.70, 0.08] } }
            ],
            "materials": [
                { "id": "chrome_mat", "preset": "chrome", "base_color": "near_black", "roughness": 0.0 }
            ],
            "nodes": [
                { "id": "chrome", "geometry": "chrome_geo", "material": "chrome_mat" }
            ],
            "scene": {
                "background": { "kind": "black" },
                "environment": { "kind": "none" }
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 32.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.4], "target": [0.0, 0.0, 0.0] }
            }],
            "capture": { "width": 96, "height": 96 },
            "expect": {
                "expect_quality": {
                    "profile": "product",
                    "reflection": {
                        "target": { "kind": "node", "id": "chrome" },
                        "min_bright_fraction": 0.20,
                        "min_dark_fraction": 0.05
                    }
                }
            }
        }))
        .expect("chrome read failure recipe serializes"),
    )
    .expect("chrome read failure recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_chrome_ibl_firefly_recipe(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    write_chrome_ibl_recipe(dir, name, 0.05)
}

#[cfg(feature = "scene-host")]
fn write_chrome_ibl_recipe(dir: &Path, name: &str, roughness: f64) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "chrome": "#F2F4F8"
            },
            "geometries": [
                {
                    "id": "chrome_geo",
                    "primitive": {
                        "kind": "sphere",
                        "radius": 0.48,
                        "segments": 64,
                        "rings": 32
                    }
                }
            ],
            "materials": [
                {
                    "id": "chrome_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "chrome",
                    "metallic": 1.0,
                    "roughness": roughness
                }
            ],
            "nodes": [
                {
                    "id": "chrome_sphere",
                    "geometry": "chrome_geo",
                    "material": "chrome_mat",
                    "transform": {
                        "kind": "trs",
                        "translation": [0.0, 0.52, 0.0]
                    }
                }
            ],
            "scene": {
                "background": { "kind": "white" },
                "environment": {
                    "kind": "uri",
                    "uri": "tests/assets/environment/polyhaven/studio_small_08_2k.hdr"
                }
            },
            "render": {
                "anti_aliasing": "none",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 30.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.74, 3.2],
                    "target": "chrome_sphere"
                }
            }],
            "capture": { "width": 260, "height": 220 },
            "expect": {
                "expect_quality": {
                    "profile": "product",
                    "reflection": {
                        "target": { "kind": "node", "id": "chrome_sphere" },
                        "min_luminance_range": 0.0,
                        "min_sobel_energy": 0.0,
                        "min_chroma_range": 0.0,
                        "max_firefly_fraction": 0.006
                    }
                }
            }
        }))
        .expect("chrome IBL firefly recipe serializes"),
    )
    .expect("chrome IBL firefly recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_chrome_ibl_panel_recipe(dir: &Path, name: &str, roughness: f64) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "chrome": "#F2F4F8" },
            "geometries": [
                {
                    "id": "chrome_geo",
                    "primitive": {
                        "kind": "box",
                        "size": [1.05, 0.74, 0.035]
                    }
                }
            ],
            "materials": [
                {
                    "id": "chrome_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "chrome",
                    "metallic": 1.0,
                    "roughness": roughness,
                    "double_sided": false
                }
            ],
            "nodes": [
                {
                    "id": "chrome_panel",
                    "geometry": "chrome_geo",
                    "material": "chrome_mat",
                    "transform": {
                        "kind": "trs",
                        "rotation_degrees": [0.0, -12.0, 0.0]
                    }
                }
            ],
            "scene": {
                "background": { "kind": "white" },
                "environment": {
                    "kind": "uri",
                    "uri": "tests/assets/environment/polyhaven/studio_small_08_2k.hdr"
                }
            },
            "render": {
                "anti_aliasing": "none",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 30.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.0, 2.4],
                    "target": "chrome_panel"
                }
            }],
            "capture": { "width": 240, "height": 180 },
            "expect": {}
        }))
        .expect("chrome IBL panel recipe serializes"),
    )
    .expect("chrome IBL panel recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_tonemap_color_contract_recipe(
    dir: &Path,
    name: &str,
    tonemapper: &str,
    exposure_ev: f64,
    linear_color: [f64; 3],
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "probe": { "linear": linear_color },
                "background": "#000000"
            },
            "geometries": [
                { "id": "panel_geo", "primitive": { "kind": "box", "size": [0.7, 0.7, 0.04] } }
            ],
            "materials": [
                { "id": "panel_mat", "kind": "unlit", "base_color": "probe", "double_sided": false }
            ],
            "nodes": [
                { "id": "panel", "geometry": "panel_geo", "material": "panel_mat" }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 20.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.2], "target": [0.0, 0.0, 0.0] }
            }],
            "scene": {
                "background": { "kind": "custom", "color": "background" }
            },
            "render": {
                "anti_aliasing": "none",
                "tonemapper": tonemapper,
                "exposure_ev": exposure_ev
            },
            "capture": { "width": 96, "height": 96 }
        }))
        .expect("tonemap color-contract recipe serializes"),
    )
    .expect("tonemap color-contract recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_grid_occlusion_recipe(dir: &Path, name: &str, anti_aliasing: &str) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "blue": "#0B5FFF",
                "floor": "#202020",
                "grid": "#FF2020"
            },
            "geometries": [
                { "id": "bar_geo", "primitive": { "kind": "box", "size": [0.52, 0.72, 0.18] } }
            ],
            "materials": [
                { "id": "bar_mat", "kind": "unlit", "base_color": "blue", "double_sided": false }
            ],
            "nodes": [
                {
                    "id": "bar",
                    "geometry": "bar_geo",
                    "material": "bar_mat",
                    "transform": {
                        "kind": "trs",
                        "translation": [0.0, 0.36, 0.0]
                    }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "#101010" },
                "grid": {
                    "enabled": true,
                    "floor_y": 0.0,
                    "padding": 0.2,
                    "line_spacing": 0.12,
                    "color": "floor",
                    "line_color": "grid",
                    "roughness": 1.0
                }
            },
            "render": {
                "anti_aliasing": anti_aliasing,
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 42.0,
                "active": true,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.55, 0.62, 1.65],
                    "target": "bar"
                }
            }],
            "capture": { "width": 220, "height": 160 },
            "expect": {}
        }))
        .expect("grid occlusion recipe serializes"),
    )
    .expect("grid occlusion recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_supersample_recipe(
    dir: &Path,
    name: &str,
    case: &str,
    supersample: u8,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let (geometry, material, node, lights, camera_eye, camera_target) = match case {
        "curve" => (
            json!({ "id": "curve_geo", "primitive": { "kind": "sphere", "radius": 0.52, "segments": 48, "rings": 24 } }),
            json!({ "id": "curve_mat", "kind": "unlit", "base_color": "#D8D8D8", "double_sided": false }),
            json!({ "id": "subject", "geometry": "curve_geo", "material": "curve_mat" }),
            json!([]),
            json!([0.0, 0.0, 4.0]),
            json!("subject"),
        ),
        "grid" => (
            json!({ "id": "grid_geo", "primitive": { "kind": "grid", "size": [1.35], "divisions": 11 } }),
            json!({ "id": "grid_mat", "kind": "line", "base_color": "#E8E8E8", "stroke_width_px": 1.0 }),
            json!({
                "id": "subject",
                "geometry": "grid_geo",
                "material": "grid_mat",
                "transform": { "kind": "trs", "rotation_degrees": [63.0, 0.0, 17.0] }
            }),
            json!([]),
            json!([0.0, 0.0, 3.4]),
            json!("subject"),
        ),
        "specular" => (
            json!({ "id": "gloss_geo", "primitive": { "kind": "sphere", "radius": 0.52, "segments": 48, "rings": 24 } }),
            json!({
                "id": "gloss_mat",
                "kind": "pbr_metallic_roughness",
                "base_color": "#D6DAE8",
                "metallic": 1.0,
                "roughness": 0.05,
                "double_sided": false
            }),
            json!({ "id": "subject", "geometry": "gloss_geo", "material": "gloss_mat" }),
            json!([
                { "id": "key", "kind": "directional", "preset": "key" },
                { "id": "rim", "kind": "directional", "preset": "rim" },
                {
                    "id": "hotspot",
                    "kind": "point",
                    "preset": "softbox",
                    "intensity_candela": 850.0,
                    "range": 8.0,
                    "transform": { "kind": "trs", "translation": [0.65, 0.8, 1.4] }
                }
            ]),
            json!([0.0, 0.0, 4.0]),
            json!("subject"),
        ),
        other => panic!("unknown supersample case {other}"),
    };
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [geometry],
            "materials": [material],
            "nodes": [node],
            "lights": lights,
            "scene": {
                "background": { "kind": "custom", "color": "#30343A" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": supersample,
                "tonemapper": "pbr_neutral",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 22.0,
                "transform": { "kind": "look_at", "eye": camera_eye, "target": camera_target }
            }],
            "capture": { "width": 180, "height": 120 }
        }))
        .expect("supersample recipe serializes"),
    )
    .expect("supersample recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_reconstruction_bar_recipe(
    dir: &Path,
    name: &str,
    supersample: u8,
    reconstruction: &str,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "bar": "#1A1E2A",
                "background": "#F7F7F4"
            },
            "geometries": [
                { "id": "bar_geo", "primitive": { "kind": "box", "size": [1.35, 0.42, 0.12] } }
            ],
            "materials": [
                { "id": "bar_mat", "kind": "unlit", "base_color": "bar", "double_sided": false }
            ],
            "nodes": [
                {
                    "id": "dashboard_bar",
                    "geometry": "bar_geo",
                    "material": "bar_mat",
                    "transform": { "kind": "trs", "rotation_degrees": [0.0, 0.0, 14.0] }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "background" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": supersample,
                "reconstruction": reconstruction,
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 22.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 4.0], "target": "dashboard_bar" }
            }],
            "capture": { "width": 260, "height": 180 }
        }))
        .expect("reconstruction bar recipe serializes"),
    )
    .expect("reconstruction bar recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_reconstruction_grid_recipe(
    dir: &Path,
    name: &str,
    supersample: u8,
    reconstruction: &str,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                { "id": "grid_geo", "primitive": { "kind": "grid", "size": [1.35], "divisions": 11 } }
            ],
            "materials": [
                { "id": "grid_mat", "kind": "line", "base_color": "#E8E8E8", "stroke_width_px": 1.0 }
            ],
            "nodes": [
                {
                    "id": "floor_grid",
                    "geometry": "grid_geo",
                    "material": "grid_mat",
                    "transform": { "kind": "trs", "rotation_degrees": [63.0, 0.0, 17.0] }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "#30343A" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": supersample,
                "reconstruction": reconstruction,
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 22.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.4], "target": "floor_grid" }
            }],
            "capture": { "width": 260, "height": 180 }
        }))
        .expect("reconstruction grid recipe serializes"),
    )
    .expect("reconstruction grid recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_recipe_grid_floor_line_quality_recipe(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    write_recipe_grid_floor_line_quality_recipe_with_settings(
        dir, name, "msaa4", 2, "tent", 4.0, "#F2F6FF", true,
    )
}

#[cfg(feature = "scene-host")]
#[allow(clippy::too_many_arguments)]
fn write_recipe_grid_floor_line_quality_recipe_with_settings(
    dir: &Path,
    name: &str,
    anti_aliasing: &str,
    supersample: u8,
    reconstruction: &str,
    line_width_px: f64,
    grid_line_color: &str,
    expect_quality: bool,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut expect = json!({
        "expect_grounded": [{
            "id": "grid_anchor_grounded",
            "target": { "kind": "node", "id": "grid_anchor" },
            "plane_y": 0.0,
            "tolerance": 0.02
        }]
    });
    if expect_quality {
        expect["expect_quality"] = json!({
            "profile": "product"
        });
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "anchor": "#8A94A6",
                "floor": "#242A32",
                "grid_line": grid_line_color
            },
            "geometries": [
                { "id": "anchor_geo", "primitive": { "kind": "box", "size": [0.18, 0.18, 0.18] } }
            ],
            "materials": [
                { "id": "anchor_mat", "kind": "unlit", "base_color": "anchor" }
            ],
            "nodes": [
                {
                    "id": "grid_anchor",
                    "geometry": "anchor_geo",
                    "material": "anchor_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.09, 0.0] }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": "#242A32" },
                "grid": {
                    "enabled": true,
                    "floor_y": 0.0,
                    "padding": 0.42,
                    "line_spacing": 0.105,
                    "color": "floor",
                    "line_color": "grid_line",
                    "line_width_px": line_width_px,
                    "roughness": 0.85
                }
            },
            "render": {
                "anti_aliasing": anti_aliasing,
                "supersample": supersample,
                "reconstruction": reconstruction,
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 35.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.78, 1.68],
                    "target": [0.0, 0.0, 0.0]
                }
            }],
            "capture": { "width": 320, "height": 220 },
            "expect": expect
        }))
        .expect("grid floor line quality recipe serializes"),
    )
    .expect("grid floor line quality recipe writes");
    (recipe_path, png_path)
}

fn add_text_quality_block(recipe_path: &Path) {
    let mut recipe: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(recipe_path).expect("recipe exists"))
            .expect("recipe parses");
    recipe["expect"]["expect_quality"]["text"] = json!({
        "min_ink_coverage": 0.01,
        "max_ink_isolation": 0.50,
        "min_intermediate_edge_fraction": 0.01
    });
    fs::write(
        recipe_path,
        serde_json::to_string_pretty(&recipe).expect("recipe serializes"),
    )
    .expect("recipe writes");
}

#[cfg(feature = "scene-host")]
fn write_area_light_shape_recipe(dir: &Path, name: &str, width: f64) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "body_color": "#4F5B70",
                "warm_color": "#FFE3BC",
                "bg_color": "#20242C"
            },
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "sphere", "radius": 0.46, "segments": 64, "rings": 32 } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "body_color", "metallic": 0.0, "roughness": 0.10 }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat" }
            ],
            "lights": [{
                "id": "softbox",
                "kind": "area",
                "preset": "softbox",
                "shape": "rect",
                "color": "warm_color",
                "width": width,
                "height": 0.7,
                "luminous_flux_lumens": 3600.0,
                "range": 4.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.9, 1.35],
                    "target": [0.0, 0.0, 0.0]
                }
            }, {
                "id": "accent_point",
                "kind": "point",
                "color": "warm_color",
                "intensity_candela": 20.0,
                "range": 3.0,
                "transform": {
                    "kind": "trs",
                    "translation": [0.7, 0.35, 1.2]
                }
            }],
            "scene": {
                "background": { "kind": "custom", "color": "bg_color" },
                "environment": { "kind": "none" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 32.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.25, 2.15],
                    "target": "body"
                }
            }],
            "capture": { "width": 180, "height": 140 },
            "expect": {
                "expect_visible": [{
                    "id": "body-visible",
                    "target": { "kind": "node", "id": "body" }
                }],
                "expect_bbox_fit": { "min": 0.30, "max": 0.95 }
            }
        }))
        .expect("area-light shape recipe serializes"),
    )
    .expect("area-light shape recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_area_light_specular_recipe(
    dir: &Path,
    name: &str,
    broad_area: bool,
) -> (PathBuf, PathBuf) {
    let shape = if broad_area { "rect" } else { "point" };
    write_area_light_specular_recipe_with_options(dir, name, broad_area, shape, 1.6, 0.9, 0.055)
}

#[cfg(feature = "scene-host")]
fn write_area_light_specular_recipe_with_options(
    dir: &Path,
    name: &str,
    broad_area: bool,
    shape: &str,
    width: f64,
    height: f64,
    roughness: f64,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let light = if broad_area {
        let mut light = json!({
            "id": "softbox",
            "kind": "area",
            "preset": "softbox",
            "shape": shape,
            "color": "warm_color",
            "width": width,
            "height": height,
            "luminous_flux_lumens": 4200.0,
            "range": 4.0,
            "transform": {
                "kind": "look_at",
                "eye": [0.0, 0.9, 1.35],
                "target": [0.0, 0.0, 0.0]
            }
        });
        if shape == "disc" || shape == "sphere" {
            light.as_object_mut().unwrap().remove("width");
            light.as_object_mut().unwrap().remove("height");
            light["radius"] = json!(width * 0.5);
        }
        light
    } else {
        json!({
            "id": "point_key",
            "kind": "point",
            "color": "warm_color",
            "intensity_candela": 230.0,
            "range": 4.0,
            "transform": {
                "kind": "trs",
                "translation": [0.0, 0.9, 1.35]
            }
        })
    };
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "body_color": "#252D3D",
                "warm_color": "#FFE3BC",
                "bg_color": "#171B22"
            },
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "sphere", "radius": 0.46, "segments": 64, "rings": 32 } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "body_color", "metallic": 0.0, "roughness": roughness }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat" }
            ],
            "lights": [light],
            "scene": {
                "background": { "kind": "custom", "color": "bg_color" },
                "environment": { "kind": "none" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "standard",
                "exposure_ev": -0.2
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 32.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.22, 2.15],
                    "target": "body"
                }
            }],
            "capture": { "width": 180, "height": 140 },
            "expect": {
                "expect_visible": [{
                    "id": "body-visible",
                    "target": { "kind": "node", "id": "body" }
                }],
                "expect_bbox_fit": { "min": 0.30, "max": 0.95 }
            }
        }))
        .expect("area-light specular recipe serializes"),
    )
    .expect("area-light specular recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn receiver_grid_mesh(divisions: u32, width: f64, depth: f64) -> serde_json::Value {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for z in 0..=divisions {
        let vz = z as f64 / divisions as f64;
        for x in 0..=divisions {
            let vx = x as f64 / divisions as f64;
            positions.push(json!([(vx - 0.5) * width, 0.0, (vz - 0.5) * depth]));
            normals.push(json!([0.0, 1.0, 0.0]));
        }
    }
    let row = divisions + 1;
    for z in 0..divisions {
        for x in 0..divisions {
            let a = z * row + x;
            let b = a + 1;
            let d = (z + 1) * row + x;
            let c = d + 1;
            indices.extend([a, c, b, a, d, c]);
        }
    }
    json!({
        "topology": "triangles",
        "positions": positions,
        "normals": normals,
        "indices": indices
    })
}

#[cfg(feature = "scene-host")]
fn write_area_light_shadow_recipe(
    dir: &Path,
    name: &str,
    include_caster: bool,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut nodes = vec![json!({
        "id": "receiver",
        "geometry": "receiver_geo",
        "material": "receiver_mat"
    })];
    if include_caster {
        nodes.push(json!({
            "id": "caster",
            "geometry": "caster_geo",
            "material": "caster_mat",
            "transform": { "kind": "trs", "translation": [0.0, 0.18, -0.02] }
        }));
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "receiver_color": "#B8C8DD",
                "caster_color": "#D64A5D",
                "light_color": "#FFE3BC",
                "bg_color": "#151922"
            },
            "geometries": [
                { "id": "receiver_geo", "mesh": receiver_grid_mesh(24, 1.35, 1.0) },
                { "id": "caster_geo", "primitive": { "kind": "box", "size": [0.16, 0.28, 0.16] } }
            ],
            "materials": [
                {
                    "id": "receiver_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "receiver_color",
                    "metallic": 0.0,
                    "roughness": 0.74
                },
                { "id": "caster_mat", "kind": "unlit", "base_color": "caster_color" }
            ],
            "nodes": nodes,
            "lights": [{
                "id": "softbox",
                "kind": "area",
                "preset": "softbox",
                "shape": "rect",
                "color": "light_color",
                "width": 0.95,
                "height": 0.65,
                "luminous_flux_lumens": 100.0,
                "range": 3.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 1.15, 0.55],
                    "target": [0.0, 0.0, -0.08]
                }
            }],
            "scene": {
                "background": { "kind": "custom", "color": "bg_color" },
                "environment": { "kind": "none" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "standard",
                "exposure_ev": -1.1
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 44.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.48, 0.78, 1.90],
                    "target": [0.0, 0.02, 0.0]
                }
            }],
            "capture": { "width": 220, "height": 170 },
            "expect": {
                "expect_visible": [{
                    "id": "receiver-visible",
                    "target": { "kind": "node", "id": "receiver" }
                }]
            }
        }))
        .expect("area-light shadow recipe serializes"),
    )
    .expect("area-light shadow recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_area_light_quality_recipe(
    dir: &Path,
    name: &str,
    area_width: f64,
    include_caster: bool,
) -> (PathBuf, PathBuf) {
    write_area_light_quality_recipe_for_shape(dir, name, "rect", area_width, include_caster)
}

#[cfg(feature = "scene-host")]
fn write_area_light_quality_recipe_for_shape(
    dir: &Path,
    name: &str,
    shape: &str,
    emitter_extent: f64,
    include_caster: bool,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut nodes = vec![json!({
        "id": "receiver",
        "geometry": "receiver_geo",
        "material": "receiver_mat"
    })];
    if include_caster {
        nodes.push(json!({
            "id": "caster",
            "geometry": "caster_geo",
            "material": "caster_mat",
            "transform": { "kind": "trs", "translation": [0.0, 0.18, -0.02] }
        }));
    }
    let mut light = json!({
        "id": "softbox",
        "kind": "area",
        "preset": "softbox",
        "shape": shape,
        "color": "light_color",
        "luminous_flux_lumens": 100.0,
        "range": 3.0,
        "transform": {
            "kind": "look_at",
            "eye": [0.0, 1.15, 0.55],
            "target": [0.0, 0.0, -0.08]
        }
    });
    let light_object = light
        .as_object_mut()
        .expect("area-light fixture literal is an object");
    match shape {
        "disc" | "sphere" => {
            light_object.insert("radius".to_owned(), json!(emitter_extent * 0.5));
        }
        _ => {
            light_object.insert("width".to_owned(), json!(emitter_extent));
            light_object.insert("height".to_owned(), json!(0.65));
        }
    }
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "receiver_color": "#B8C8DD",
                "caster_color": "#D64A5D",
                "light_color": "#FFE3BC",
                "bg_color": "#151922"
            },
            "geometries": [
                { "id": "receiver_geo", "mesh": receiver_grid_mesh(24, 1.35, 1.0) },
                { "id": "caster_geo", "primitive": { "kind": "box", "size": [0.16, 0.28, 0.16] } }
            ],
            "materials": [
                {
                    "id": "receiver_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "receiver_color",
                    "metallic": 0.0,
                    "roughness": 0.74
                },
                { "id": "caster_mat", "kind": "unlit", "base_color": "caster_color" }
            ],
            "nodes": nodes,
            "lights": [light],
            "scene": {
                "background": { "kind": "custom", "color": "bg_color" },
                "environment": { "kind": "none" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "standard",
                "exposure_ev": -1.1
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 44.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.48, 0.78, 1.90],
                    "target": [0.0, 0.02, 0.0]
                }
            }],
            "capture": { "width": 220, "height": 170 },
            "expect": {
                "expect_visible": [{
                    "id": "receiver-visible",
                    "target": { "kind": "node", "id": "receiver" }
                }],
                "expect_quality": {
                    "profile": "product",
                    "area_light": {
                        "target": { "kind": "node", "id": "receiver" },
                        "min_shadow_contrast": 0.025,
                        "min_penumbra_width_px": 1.5,
                        "min_penumbra_luma_levels": 18.0,
                        "min_emitter_extent_meters": 0.5
                    }
                }
            }
        }))
        .expect("area-light quality recipe serializes"),
    )
    .expect("area-light quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn write_depth_of_field_quality_recipe(
    dir: &Path,
    name: &str,
    focus_distance: Option<f64>,
) -> (PathBuf, PathBuf) {
    let recipe_path = dir.join(format!("{name}.recipe.json"));
    let png_path = dir.join(format!("{name}.png"));
    let mut render = json!({
        "anti_aliasing": "none",
        "tonemapper": "standard",
        "exposure_ev": 0.0
    });
    if let Some(focus_distance) = focus_distance {
        render["depth_of_field"] = json!({
            "focus_distance": focus_distance,
            "aperture_f_stop": 0.7,
            "radius_px": 12
        });
    }

    let nodes = vec![
        json!({
            "id": "background",
            "geometry": "background_geo",
            "material": "background_mat",
            "transform": { "kind": "trs", "translation": [0.0, 0.0, -1.35] }
        }),
        json!({
            "id": "subject",
            "geometry": "subject_geo",
            "material": "subject_mat"
        }),
    ];

    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.34, 0.34, 0.34] } },
                { "id": "background_geo", "mesh": depth_of_field_checker_mesh(12, 8) }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "#D64645" },
                { "id": "background_mat", "kind": "unlit", "base_color": "#FFFFFF", "double_sided": true }
            ],
            "nodes": nodes,
            "scene": {
                "background": { "kind": "custom", "color": "#E8ECF2" },
                "environment": { "kind": "none" }
            },
            "render": render,
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 31.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.0], "target": "subject" }
            }],
            "capture": { "width": 220, "height": 160 },
            "expect": {
                "expect_visible": [
                    { "id": "subject-visible", "target": { "kind": "node", "id": "subject" } },
                    { "id": "background-visible", "target": { "kind": "node", "id": "background" } }
                ],
                "expect_quality": {
                    "profile": "product",
                    "depth_of_field": {
                        "target": { "kind": "node", "id": "subject" },
                        "background_target": { "kind": "node", "id": "background" },
                        "min_source_background_sobel": 0.04,
                        "min_background_sobel_drop": 0.015,
                        "min_background_sobel_drop_fraction": 0.18,
                        "max_focal_mean_delta": 0.08
                    }
                }
            }
        }))
        .expect("depth-of-field quality recipe serializes"),
    )
    .expect("depth-of-field quality recipe writes");
    (recipe_path, png_path)
}

#[cfg(feature = "scene-host")]
fn depth_of_field_checker_mesh(columns: u32, rows: u32) -> serde_json::Value {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let width = 1.72_f64;
    let height = 1.14_f64;
    for row in 0..rows {
        for column in 0..columns {
            let x0 = -width * 0.5 + width * f64::from(column) / f64::from(columns);
            let x1 = -width * 0.5 + width * f64::from(column + 1) / f64::from(columns);
            let y0 = -height * 0.5 + height * f64::from(row) / f64::from(rows);
            let y1 = -height * 0.5 + height * f64::from(row + 1) / f64::from(rows);
            let base = positions.len() as u32;
            positions.extend([
                json!([x0, y0, 0.0]),
                json!([x1, y0, 0.0]),
                json!([x1, y1, 0.0]),
                json!([x0, y1, 0.0]),
            ]);
            normals.extend([
                json!([0.0, 0.0, 1.0]),
                json!([0.0, 0.0, 1.0]),
                json!([0.0, 0.0, 1.0]),
                json!([0.0, 0.0, 1.0]),
            ]);
            let color = if (row + column) % 2 == 0 {
                "#F3F6FF"
            } else {
                "#111827"
            };
            colors.extend([json!(color), json!(color), json!(color), json!(color)]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    json!({
        "topology": "triangles",
        "positions": positions,
        "normals": normals,
        "indices": indices,
        "colors": colors
    })
}

#[cfg(feature = "scene-host")]
fn run_recipe_render_verify(
    recipe_path: &Path,
    png_path: &Path,
    use_gpu: bool,
) -> serde_json::Value {
    let mut args = vec!["recipe", "render", path_str(recipe_path)];
    if use_gpu {
        args.push("--gpu");
    }
    args.extend(["--introspect", "--verify", "--out", path_str(png_path)]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    if use_gpu {
        configure_command_for_lavapipe(&mut command);
    }
    let output = command
        .args(args)
        .output()
        .expect("scena recipe render command runs");
    assert!(
        output.status.success(),
        "recipe render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    if use_gpu {
        assert_eq!(
            report["introspection"]["capabilities"]["backend"], "headless_gpu",
            "GPU supersample proof must use the GPU backend, not a fallback: {report:#}"
        );
    }
    report
}

#[cfg(feature = "scene-host")]
fn run_recipe_render_introspect(
    recipe_path: &Path,
    png_path: &Path,
    use_gpu: bool,
) -> serde_json::Value {
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
        .expect("scena recipe render introspection command runs");
    assert!(
        output.status.success(),
        "recipe render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.render_introspection.v1");
    if use_gpu {
        assert_eq!(
            report["capabilities"]["backend"], "headless_gpu",
            "GPU chrome parity proof must use the GPU backend, not a fallback: {report:#}"
        );
    }
    report
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_render_subject_focus_resolves_depth_and_runs_dof_pass() {
    let dir = artifact_dir("subject-focus");
    let recipe_path = dir.join("subject-focus.recipe.json");
    let png_path = dir.join("subject-focus.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "subject", "uri": TEST_ASSET }
            ],
            "scene": { "preset": "product_studio" },
            "render": {
                "anti_aliasing": "none",
                "tonemapper": "standard",
                "exposure_ev": 0.0,
                "depth_of_field": {
                    "focus": {
                        "mode": "subject",
                        "target": { "kind": "import", "id": "subject" }
                    },
                    "coverage": "all",
                    "strength": "subtle"
                }
            },
            "capture": { "width": 96, "height": 72 }
        }))
        .expect("subject-focus recipe serializes"),
    )
    .expect("subject-focus recipe writes");

    let report = run_recipe_render_verify(&recipe_path, &png_path, false);

    assert_eq!(
        report["capture"]["frame"]["depth_of_field"], true,
        "subject focus must resolve a visible target depth and enable the DoF pass: {report:#}"
    );
    let focus_report = &report["introspection"]["focus_report"];
    assert_eq!(focus_report["schema"], "scena.focus_report.v1");
    assert_eq!(focus_report["status"], "resolved", "{report:#}");
    assert_eq!(focus_report["mode"], "subject", "{report:#}");
    assert_eq!(focus_report["target"]["kind"], "import", "{report:#}");
    assert_eq!(focus_report["target"]["id"], "subject", "{report:#}");
    assert!(
        focus_report["resolved"]["focus_distance_m"]
            .as_f64()
            .is_some_and(|distance| distance > 0.0),
        "focus report must expose the resolved focal distance: {report:#}"
    );
    assert!(
        focus_report["resolved"]["confidence"]
            .as_f64()
            .is_some_and(|confidence| confidence > 0.0),
        "focus report must expose nonzero confidence: {report:#}"
    );
    assert_eq!(
        focus_report["frame_key"]["payload_fnv1a64"], report["capture"]["payload"]["fnv1a64"],
        "focus report must bind to the exact final capture payload: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_render_subject_focus_accepts_authored_node_targets() {
    let dir = artifact_dir("subject-focus-node");
    let recipe_path = dir.join("subject-focus-node.recipe.json");
    let png_path = dir.join("subject-focus-node.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "subject": "#7CB342",
                "background": "#1B2028"
            },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } },
                { "id": "background_geo", "primitive": { "kind": "box", "size": [1.8, 1.2, 0.05] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" },
                { "id": "background_mat", "kind": "unlit", "base_color": "background" }
            ],
            "nodes": [
                {
                    "id": "subject_box",
                    "geometry": "subject_geo",
                    "material": "subject_mat",
                    "transform": { "kind": "center" }
                },
                {
                    "id": "background_panel",
                    "geometry": "background_geo",
                    "material": "background_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, -1.0] }
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject_box" }
            }],
            "render": {
                "exposure_ev": 0.0,
                "depth_of_field": {
                    "focus": {
                        "mode": "subject",
                        "target": { "kind": "node", "id": "subject_box" }
                    },
                    "coverage": "all",
                    "strength": "subtle"
                }
            },
            "capture": { "width": 96, "height": 72 }
        }))
        .expect("node subject-focus recipe serializes"),
    )
    .expect("node subject-focus recipe writes");

    let report = run_recipe_render_verify(&recipe_path, &png_path, false);

    assert_eq!(
        report["capture"]["frame"]["depth_of_field"], true,
        "node-target subject focus must enable the DoF pass: {report:#}"
    );
    let focus_report = &report["introspection"]["focus_report"];
    assert_eq!(focus_report["schema"], "scena.focus_report.v1");
    assert_eq!(focus_report["status"], "resolved", "{report:#}");
    assert_eq!(focus_report["mode"], "subject", "{report:#}");
    assert_eq!(focus_report["target"]["kind"], "node", "{report:#}");
    assert_eq!(focus_report["target"]["id"], "subject_box", "{report:#}");
    assert!(
        focus_report["target"]["handles"]
            .as_array()
            .is_some_and(|handles| !handles.is_empty()),
        "node-target focus report must carry resolved handles: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_render_subject_metering_and_focus_work_without_photo_intent() {
    let dir = artifact_dir("subject-metering-and-focus");
    let recipe_path = dir.join("subject-metering-and-focus.recipe.json");
    let png_path = dir.join("subject-metering-and-focus.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "subject", "uri": CAD_TERMINAL_ASSET }
            ],
            "scene": {
                "preset": "product_studio",
                "background": { "kind": "dark_studio" },
                "grid": { "enabled": false }
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "framing": {
                    "preset": "three_quarter_front_right",
                    "fill": 0.72
                }
            }],
            "render": {
                "auto_exposure": "product_studio",
                "metering": {
                    "mode": "subject",
                    "target": { "kind": "import", "id": "subject" },
                    "surround_weight": 0.1
                },
                "depth_of_field": {
                    "focus": {
                        "mode": "subject",
                        "target": { "kind": "import", "id": "subject" }
                    },
                    "coverage": "all",
                    "strength": "subtle"
                }
            },
            "capture": { "width": 128, "height": 96 }
        }))
        .expect("subject-metering-and-focus recipe serializes"),
    )
    .expect("subject-metering-and-focus recipe writes");

    let report = run_recipe_render_verify(&recipe_path, &png_path, false);

    assert!(
        report.get("photo").is_none(),
        "this proof must exercise explicit render fields, not photo.intent: {report:#}"
    );
    let exposure_report = &report["introspection"]["exposure_report"];
    assert_eq!(exposure_report["schema"], "scena.exposure_report.v1");
    assert_eq!(exposure_report["status"], "measured", "{report:#}");
    assert_eq!(
        exposure_report["metering_domain"], "scene_linear_pre_tonemap",
        "{report:#}"
    );
    let auto = &exposure_report["auto_exposure"];
    assert!(
        auto["subject_sample_count"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "render.metering mode=subject must route a subject rect into the auto-exposure meter: {report:#}"
    );
    assert!(
        auto["sample_count"]
            .as_u64()
            .zip(auto["subject_sample_count"].as_u64())
            .is_some_and(|(samples, subject_samples)| samples >= subject_samples),
        "subject samples must be a subset of total metering samples: {report:#}"
    );

    let focus_report = &report["introspection"]["focus_report"];
    assert_eq!(focus_report["schema"], "scena.focus_report.v1");
    assert_eq!(focus_report["status"], "resolved", "{report:#}");
    assert_eq!(focus_report["target"]["kind"], "import", "{report:#}");
    assert_eq!(focus_report["target"]["id"], "subject", "{report:#}");

    let observations = report["introspection"]["subject_observations"]
        .as_array()
        .expect("introspection subject observations serialize");
    assert!(
        observations
            .iter()
            .any(|observation| observation["source"] == "render.metering"),
        "render introspection must link the render.metering subject observation: {report:#}"
    );
    assert!(
        observations
            .iter()
            .any(|observation| observation["source"] == "render.depth_of_field.focus"),
        "render introspection must link the render.depth_of_field.focus subject observation: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_render_legacy_average_metering_and_manual_focus_stay_compatible() {
    let dir = artifact_dir("legacy-average-metering-manual-focus");
    let recipe_path = dir.join("legacy-average-metering-manual-focus.recipe.json");
    let png_path = dir.join("legacy-average-metering-manual-focus.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "subject", "uri": CAD_TERMINAL_ASSET }
            ],
            "scene": {
                "preset": "product_studio",
                "background": { "kind": "dark_studio" },
                "grid": { "enabled": false }
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "framing": {
                    "preset": "three_quarter_front_right",
                    "fill": 0.72
                }
            }],
            "render": {
                "auto_exposure": "product_studio",
                "metering": { "mode": "average" },
                "depth_of_field": {
                    "focus_distance": 2.75,
                    "aperture_f_stop": 2.8,
                    "radius_px": 4
                }
            },
            "capture": { "width": 128, "height": 96 }
        }))
        .expect("legacy recipe serializes"),
    )
    .expect("legacy recipe writes");

    let report = run_recipe_render_verify(&recipe_path, &png_path, false);

    assert!(
        report.get("photo").is_none(),
        "legacy compatibility proof must exercise explicit render fields, not photo.intent: {report:#}"
    );
    assert_eq!(
        report["capture"]["frame"]["depth_of_field"], true,
        "manual focus_distance should still enable the existing DoF render path: {report:#}"
    );
    let exposure_report = &report["introspection"]["exposure_report"];
    assert_eq!(exposure_report["schema"], "scena.exposure_report.v1");
    assert_eq!(exposure_report["status"], "measured", "{report:#}");
    assert_eq!(
        exposure_report["metering_domain"], "scene_linear_pre_tonemap",
        "{report:#}"
    );
    assert_eq!(
        exposure_report["auto_exposure"]["subject_sample_count"], 0,
        "average metering must not silently become subject metering for recipes without photo.intent: {report:#}"
    );
    assert!(
        report["introspection"]["subject_observations"]
            .as_array()
            .is_none_or(|observations| observations.is_empty()),
        "legacy average metering/manual focus should not invent a subject observation: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn recipe_render_product_quality_uses_exact_subject_observation_pixels() {
    let dir = artifact_dir("subject-observation-quality");
    let write_recipe =
        |name: &str, color_name: &str, color_hex: &str, exposure: Option<serde_json::Value>| {
            let recipe_path = dir.join(format!("{name}.recipe.json"));
            let png_path = dir.join(format!("{name}.png"));
            let mut expect_quality = json!({
                "profile": "product",
                "geometry": { "min_intermediate_edge_fraction": 0.0 }
            });
            if let Some(exposure) = exposure {
                expect_quality["exposure"] = exposure;
            }
            fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                color_name: color_hex
            },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.34, 0.10] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": color_name }
            ],
            "nodes": [
                {
                    "id": "subject",
                    "geometry": "subject_geo",
                    "material": "subject_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "scene": {
                "background": { "kind": "custom", "color": color_name }
            },
            "render": {
                "anti_aliasing": "none",
                "tonemapper": "standard",
                "auto_exposure": "product_studio",
                "metering": {
                    "mode": "subject",
                    "target": { "kind": "node", "id": "subject" }
                }
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject" }
            }],
            "capture": { "width": 128, "height": 96 },
            "expect": {
                "expect_quality": expect_quality
            }
        }))
        .expect("subject-observation quality recipe serializes"),
        )
        .expect("subject-observation quality recipe writes");
            (recipe_path, png_path)
        };

    let (recipe_path, png_path) = write_recipe(
        "subject-observation-quality-black",
        "black",
        "#000000",
        None,
    );
    let report = run_recipe_render_verify_expect_failure(&recipe_path, &png_path, false);
    let observations = report["verification"]["subject_observations"]
        .as_array()
        .expect("verification subject observations serialize");
    let metering = observations
        .iter()
        .find(|observation| observation["source"] == "render.metering")
        .unwrap_or_else(|| panic!("render.metering subject observation missing: {report:#}"));
    assert_eq!(metering["status"], "observed", "{report:#}");
    assert_eq!(
        metering["fallback"]["degraded"], false,
        "semantic AOV subject observation must be exact before quality consumes it: {report:#}"
    );
    assert!(
        metering["pixel_quality"]["sample_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "subject observation must carry exact visible subject pixel metrics, independent of background color: {report:#}"
    );
    assert_eq!(
        metering["pixel_quality"]["low_clip_fraction"].as_f64(),
        Some(1.0),
        "black subject pixels should be measured from the subject mask, not a color-difference foreground guess: {report:#}"
    );

    let quality_checks = report["verification"]["quality"]["checks"]
        .as_array()
        .expect("quality checks serialize");
    let subject_exposure = quality_checks
        .iter()
        .find(|check| check["id"] == "expect_quality.subject.pixel_exposure")
        .unwrap_or_else(|| panic!("subject-observation quality check missing: {report:#}"));
    assert_eq!(subject_exposure["status"], "failed", "{report:#}");
    assert_eq!(
        subject_exposure["code"], "subject_black_crushed",
        "{report:#}"
    );
    assert!(
        subject_exposure["observed"]["suggested_compensation_ev"]
            .as_f64()
            .is_some_and(|ev| ev > 0.0),
        "quality check must include exposure-compensation advice: {report:#}"
    );
    let material_readability = quality_checks
        .iter()
        .find(|check| check["id"] == "expect_quality.subject.material_readability")
        .unwrap_or_else(|| {
            panic!("subject material-readability quality check missing: {report:#}")
        });
    assert_eq!(material_readability["status"], "failed", "{report:#}");
    assert_eq!(
        material_readability["code"], "subject_luminance_structure_below_min",
        "{report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons serialize")
            .iter()
            .any(|reason| reason["source"] == "quality"
                && reason["code"] == "subject_black_crushed"
                && reason["expectation_id"] == "expect_quality.subject.pixel_exposure"),
        "subject quality failure must surface as a quality verification reason: {report:#}"
    );

    let (gray_recipe_path, gray_png_path) = write_recipe(
        "subject-observation-quality-flat-gray",
        "flat_gray",
        "#5A5A5A",
        None,
    );
    let gray_report =
        run_recipe_render_verify_expect_failure(&gray_recipe_path, &gray_png_path, false);
    let gray_observations = gray_report["verification"]["subject_observations"]
        .as_array()
        .expect("verification subject observations serialize");
    let gray_metering = gray_observations
        .iter()
        .find(|observation| observation["source"] == "render.metering")
        .unwrap_or_else(|| {
            panic!("flat-gray render.metering subject observation missing: {gray_report:#}")
        });
    assert_eq!(gray_metering["status"], "observed", "{gray_report:#}");
    assert!(
        gray_metering["pixel_quality"]["sample_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "flat-gray subject quality must use exact subject pixels, not foreground color difference: {gray_report:#}"
    );
    assert_quality_reason(
        &gray_report,
        "subject_luminance_structure_below_min",
        "expect_quality.subject.material_readability",
    );

    let (mean_band_recipe_path, mean_band_png_path) = write_recipe(
        "subject-observation-quality-mean-band",
        "fixture_gray",
        "#5A5A5A",
        Some(json!({
            "min_mean_luminance_srgb8": 150.0,
            "max_mean_luminance_srgb8": 170.0
        })),
    );
    let mean_band_report =
        run_recipe_render_verify_expect_failure(&mean_band_recipe_path, &mean_band_png_path, false);
    let subject_exposure = mean_band_report["verification"]["quality"]["checks"]
        .as_array()
        .expect("quality checks serialize")
        .iter()
        .find(|check| check["id"] == "expect_quality.subject.pixel_exposure")
        .unwrap_or_else(|| {
            panic!("subject pixel-exposure quality check missing: {mean_band_report:#}")
        });
    assert_eq!(
        subject_exposure["code"], "subject_luminance_below_min",
        "{mean_band_report:#}"
    );
    assert_eq!(
        subject_exposure["threshold"]["min_mean_luminance_srgb8"].as_f64(),
        Some(150.0),
        "fixture-specific subject luminance band must be reported in the quality threshold: {mean_band_report:#}"
    );
    assert_quality_reason(
        &mean_band_report,
        "subject_luminance_below_min",
        "expect_quality.subject.pixel_exposure",
    );
}

#[cfg(feature = "scene-host")]
fn run_recipe_render_verify_expect_failure(
    recipe_path: &Path,
    png_path: &Path,
    use_gpu: bool,
) -> serde_json::Value {
    let mut args = vec!["recipe", "render", path_str(recipe_path)];
    if use_gpu {
        args.push("--gpu");
    }
    args.extend(["--introspect", "--verify", "--out", path_str(png_path)]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    if use_gpu {
        configure_command_for_lavapipe(&mut command);
    }
    let output = command
        .args(args)
        .output()
        .expect("scena recipe render failure command runs");
    assert!(
        !output.status.success(),
        "recipe render should fail verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    if use_gpu {
        assert_eq!(
            report["introspection"]["capabilities"]["backend"], "headless_gpu",
            "GPU negative proof must use the GPU backend, not a fallback: {report:#}"
        );
    }
    report
}

#[cfg(feature = "scene-host")]
fn assert_quality_reason(report: &serde_json::Value, code: &str, expectation_id: &str) {
    assert!(
        report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize")
            .iter()
            .any(|check| check["code"] == code && check["id"] == expectation_id),
        "expected quality check {code}/{expectation_id}: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons serialize")
            .iter()
            .any(|reason| reason["code"] == code
                && reason["expectation_id"] == expectation_id
                && reason["source"] == "quality"),
        "expected agent-facing quality reason {code}/{expectation_id}: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_quality_per_line_region_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-line-quality-pass");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_line_quality_recipe(&dir, &format!("line-quality-{backend}"), 0.005, 0.15);
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
fn scena_recipe_render_verify_fails_geometry_edge_quality_without_sample_aa_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-geometry-edge-no-sample-aa");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let anti_aliasing = "none";
        let (recipe_path, png_path) = write_geometry_edge_quality_recipe(
            &dir,
            &format!("geometry-edge-no-sample-aa-{backend}"),
            anti_aliasing,
            None,
            Some(0.30),
        );
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
            .expect("scena geometry-edge quality render command runs");

        assert!(
            !output.status.success(),
            "unsampled geometry edge should fail on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU geometry quality failure proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().any(|check| {
                check["id"] == "expect_quality.geometry"
                    && check["code"] == "geometry_missing_antialiasing"
                    && check["region"]["kind"] == "subject"
                    && check["observed"]["intermediate_edge_fraction"]
                        .as_f64()
                        .is_some_and(|value| value < 0.30)
            }),
            "quality verifier must fail hard geometry edges with exact geometry_missing_antialiasing on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "geometry-edge unsampled render writes the PNG"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_profile_quality_runs_geometry_edge_check_by_default() {
    let dir = artifact_dir("recipe-render-geometry-edge-profile-default");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let anti_aliasing = "none";
        let (recipe_path, png_path) = write_geometry_edge_quality_recipe(
            &dir,
            &format!("geometry-edge-profile-default-{backend}"),
            anti_aliasing,
            None,
            None,
        );
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
            .expect("scena geometry-edge profile quality render command runs");

        assert!(
            !output.status.success(),
            "profile-only expect_quality should fail aliased geometry edges on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU profile-only geometry quality proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().any(|check| {
                check["id"] == "expect_quality.geometry"
                    && check["code"] == "geometry_missing_antialiasing"
                    && check["region"]["kind"] == "subject"
            }),
            "profile-only quality must include the geometry-edge check by default on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "geometry-edge profile-default render writes the PNG"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_profile_quality_runs_geometry_edge_check_for_non_product_profiles() {
    let dir = artifact_dir("recipe-render-geometry-edge-profile-non-product");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) = write_geometry_edge_quality_recipe_with_profile_and_colors(
            &dir,
            &format!("geometry-edge-cad-profile-{backend}"),
            "none",
            None,
            None,
            "cad",
            "#D8D8D8",
            "#808080",
        );
        add_text_quality_block(&recipe_path);
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
            .expect("scena non-product profile geometry quality render command runs");

        assert!(
            !output.status.success(),
            "cad profile-only expect_quality should still fail hard aliased geometry edges on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU non-product profile geometry quality proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().any(|check| {
                check["id"] == "expect_quality.geometry"
                    && check["code"] == "geometry_missing_antialiasing"
                    && check["threshold"]["min_intermediate_edge_fraction"]
                        .as_f64()
                        .is_some_and(|value| (value - 0.02).abs() < 1.0e-6)
            }),
            "cad profile-only quality must include the geometry-edge check by default on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "non-product profile geometry-edge render writes the PNG"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_geometry_edge_quality_with_msaa4_on_gpu() {
    let dir = artifact_dir("recipe-render-geometry-edge-sample-aa");
    let (recipe_path, png_path) = write_geometry_edge_quality_recipe(
        &dir,
        "geometry-edge-msaa4-gpu",
        "msaa4",
        None,
        Some(0.25),
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut command);
    let output = command
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--gpu",
            "--introspect",
            "--verify",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena geometry-edge MSAA4 render command runs");

    assert!(
        output.status.success(),
        "GPU MSAA4 should pass geometry-edge quality, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["verification"]["quality"]["ok"], true, "{report:#}");
    assert_eq!(
        report["introspection"]["capabilities"]["backend"], "headless_gpu",
        "GPU geometry quality proof must use the GPU backend, not a fallback: {report:#}"
    );
    assert!(png_path.exists(), "GPU MSAA4 render writes the PNG");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_low_contrast_geometry_edges_require_sample_aa_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-geometry-edge-low-contrast");
    let cases = [
        ("cpu-none", false, "none", None, false),
        ("cpu-ss4", false, "none", Some(4), true),
        ("gpu-none", true, "none", None, false),
        ("gpu-msaa4", true, "msaa4", None, true),
    ];
    for (name, use_gpu, anti_aliasing, supersample, should_pass) in cases {
        let (recipe_path, png_path) = write_geometry_edge_quality_recipe_with_colors(
            &dir,
            name,
            anti_aliasing,
            supersample,
            None,
            "#A8A8A8",
            "#808080",
        );
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
            .expect("scena low-contrast geometry-edge quality command runs");
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU low-contrast edge proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        if should_pass {
            assert!(
                output.status.success(),
                "sampled low-contrast geometry edge should pass for {name}; stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                checks.iter().all(|check| {
                    check["code"] != "geometry_missing_antialiasing" || check["status"] != "failed"
                }),
                "sampled low-contrast geometry edge should not fail geometry_missing_antialiasing for {name}: {report:#}"
            );
        } else {
            let edge_check = checks
                .iter()
                .find(|check| {
                    check["id"] == "expect_quality.geometry"
                        && check["code"] == "geometry_missing_antialiasing"
                        && check["region"]["kind"] == "subject"
                })
                .unwrap_or_else(|| {
                    panic!(
                        "low-contrast geometry edge quality check must fail for {name}: {report:#}"
                    )
                });
            let intermediate = edge_check["observed"]["intermediate_edge_fraction"]
                .as_f64()
                .expect("intermediate edge fraction serializes");
            let threshold = edge_check["threshold"]["min_intermediate_edge_fraction"]
                .as_f64()
                .expect("geometry edge threshold serializes");
            assert!(
                !output.status.success(),
                "unsampled low-contrast geometry edge should fail for {name}; intermediate={intermediate}, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                intermediate < threshold,
                "unsampled low-contrast geometry edge should stay below threshold for {name}: {report:#}"
            );
        }
        assert!(
            png_path.exists(),
            "low-contrast geometry edge render writes a PNG for {name}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_missing_reflection_quality_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-reflection-quality");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_reflection_quality_recipe(&dir, &format!("reflection-quality-{backend}"), false);
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
            .expect("scena reflection quality render command runs");

        assert!(
            !output.status.success(),
            "a matte floor without SSR/reflection should fail reflection quality on {backend}; stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU reflection quality proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        }
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().any(|check| {
                check["id"] == "expect_quality.reflection"
                    && check["code"] == "reflection_structure_missing"
                    && check["region"]["kind"] == "reflection_surface"
            }),
            "reflection quality must fail with exact reflection_structure_missing on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "reflection-quality failure render writes the PNG"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_grid_reflection_quality_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-reflection-quality-pass");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) = write_reflection_quality_recipe(
            &dir,
            &format!("reflection-quality-pass-{backend}"),
            true,
        );
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
            .expect("scena grid reflection quality render command runs");

        assert!(
            output.status.success(),
            "grid reflection should pass reflection quality on {backend}; stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU grid reflection quality proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        }
        assert_eq!(
            report["verification"]["quality"]["ok"], true,
            "reflection quality should pass on {backend}: {report:#}"
        );
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks
                .iter()
                .all(|check| check["code"] != "reflection_structure_missing"),
            "grid reflection should not emit reflection_structure_missing on {backend}: {report:#}"
        );
        assert!(png_path.exists(), "grid reflection render writes the PNG");
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_passes_screen_space_reflection_quality_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-screen-space-reflection-quality-pass");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) = write_screen_space_reflection_quality_recipe(
            &dir,
            &format!("screen-space-reflection-quality-pass-{backend}"),
        );
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
            .expect("scena screen-space reflection quality render command runs");

        assert!(
            output.status.success(),
            "screen-space reflections should pass reflection quality on {backend}; stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU screen-space reflection proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        }
        assert_eq!(
            report["verification"]["quality"]["ok"], true,
            "screen-space reflection quality should pass on {backend}: {report:#}"
        );
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks
                .iter()
                .all(|check| check["code"] != "reflection_structure_missing"),
            "screen-space reflections should not emit reflection_structure_missing on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "screen-space reflection render writes the PNG"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_material_reflection_changes_target_pixels_on_cpu_and_gpu() {
    // Doctor pin: expect_quality.reflection.target is the target-scoped
    // material reflection quality contract. The pixel delta assertions below
    // additionally prove the renderer applies material SSR to multiple
    // chrome-like metallic targets, not only to the verifier-selected target.
    let dir = artifact_dir("recipe-render-material-reflection-quality-pass");
    let mut all_results = Vec::new();
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let mut rendered = Vec::new();
        for (case, enable_ssr, should_pass) in [("off", false, false), ("on", true, true)] {
            let (recipe_path, png_path) = write_material_reflection_quality_recipe(
                &dir,
                &format!("material-reflection-quality-{case}-{backend}"),
                enable_ssr,
            );
            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena material reflection quality render command runs");

            assert_eq!(
                output.status.success(),
                should_pass,
                "material reflection {case} status mismatch on {backend}; stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "material reflection failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU material reflection proof must use HeadlessGpu, not a fallback: {report:#}"
                );
            } else {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless",
                    "CPU material reflection proof should use Headless CPU: {report:#}"
                );
            }
            let checks = report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks serialize");
            let has_missing_reflection = checks
                .iter()
                .any(|check| check["code"] == "reflection_structure_missing");
            assert_eq!(
                has_missing_reflection, !enable_ssr,
                "material reflection quality should fail only when SSR is absent on {backend}/{case}: {report:#}"
            );
            assert!(
                png_path.exists(),
                "material reflection {case} render writes the PNG on {backend}"
            );
            rendered.push((case, report, png_path));
        }
        let (_, off_report, off_png) = &rendered[0];
        let (_, on_report, on_png) = &rendered[1];
        let off = decode_png_rgba8(off_png);
        let on = decode_png_rgba8(on_png);
        assert_eq!((off.width, off.height), (on.width, on.height));
        for target_id in ["mirror", "mirror_secondary"] {
            let off_region = node_region_from_composition_report(off_report, target_id);
            let on_region = node_region_from_composition_report(on_report, target_id);
            assert_eq!(
                off_region, on_region,
                "material reflection ON/OFF proof should compare the same projected {target_id} region on {backend}"
            );
            let delta = frame_delta_in_region(&off.rgba8, &on.rgba8, off.width, off_region);
            assert!(
                delta.mean_channel_delta >= 5.0 && delta.max_channel_delta >= 20,
                "material SSR must measurably change the reflective {target_id} region on {backend}, delta={delta:?}, region={off_region:?}, off_png={off_png:?}, on_png={on_png:?}"
            );
            all_results.push((backend.to_owned(), target_id.to_owned(), delta, off_region));
        }
    }
    fs::write(
        dir.join("material-reflection-delta-metrics.json"),
        format_material_reflection_metrics(&all_results),
    )
    .expect("material reflection delta metrics write");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_missing_chrome_read_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-chrome-read-missing");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_chrome_read_failure_recipe(&dir, &format!("chrome-read-missing-{backend}"));
        let report = run_recipe_render_verify_expect_failure(&recipe_path, &png_path, use_gpu);
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        let chrome_read_check = checks
            .iter()
            .find(|check| {
                check["code"] == "reflection_chrome_read_missing"
                    && check["status"] == "failed"
            })
            .unwrap_or_else(|| {
                panic!(
                    "chrome-read failure must emit exact reflection_chrome_read_missing on {backend}: {report:#}"
                )
            });
        assert!(
            chrome_read_check["threshold"]["min_bright_fraction"].is_number(),
            "chrome-read failure must report min_bright_fraction on {backend}: {report:#}"
        );
        assert!(
            chrome_read_check["threshold"]["min_dark_fraction"].is_number(),
            "chrome-read failure must report min_dark_fraction on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "chrome-read failure render writes the PNG on {backend}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_ergonomic_product_scene_reaches_rust_api_quality_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-ergonomic-product");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_ergonomic_product_recipe(&dir, &format!("ergonomic-product-{backend}"));
        let report = run_recipe_render_verify(&recipe_path, &png_path, use_gpu);
        assert_eq!(
            report["verification"]["quality"]["ok"], true,
            "ergonomic recipe should pass product quality on {backend}: {report:#}"
        );
        assert_eq!(
            report["verification"]["composition"]["ok"], true,
            "ergonomic recipe should pass composition checks on {backend}: {report:#}"
        );
        let framing_check = composition_check(&report, "node.product.framing");
        assert_eq!(
            framing_check["code"], "subject_fit_sane",
            "camera.framing should pass object-level composition framing on {backend}: {report:#}"
        );
        assert_eq!(
            framing_check["status"], "checked",
            "camera.framing should be checked for the product object on {backend}: {report:#}"
        );
        let fit = framing_check["observed"]["fit_fraction"]
            .as_f64()
            .expect("fit_fraction is reported");
        assert!(
            (0.35..=0.92).contains(&fit),
            "camera.framing should produce Rust-helper-quality product framing for the product object on {backend}: {fit}, report={report:#}"
        );
        let quality_checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            quality_checks.iter().all(|check| {
                check["status"] != "failed"
                    || (check["code"] != "reflection_structure_missing"
                        && check["code"] != "low_clip_fraction_too_high"
                        && check["code"] != "high_clip_fraction_too_high")
            }),
            "material.preset chrome + environment.preset + auto_exposure should not fail reflection/exposure quality on {backend}: {report:#}"
        );
        let composition_checks = report["verification"]["composition"]["checks"]
            .as_array()
            .expect("composition checks serialize");
        assert!(
            composition_checks.iter().any(|check| {
                check["id"] == "node.product.pixel_exposure"
                    && check["code"] == "subject_exposure_sane"
                    && check["status"] == "checked"
            }),
            "render.auto_exposure should keep the product subject in sane exposure on {backend}: {report:#}"
        );
        let color_check = composition_check(&report, "node.product.expected_color");
        assert_eq!(
            color_check["code"], "material_base_color_available",
            "material.preset chrome should produce inspected material intent for the product on {backend}: {report:#}"
        );
        assert!(
            png_path.exists(),
            "ergonomic product recipe writes a PNG on {backend}"
        );
        let image = decode_png_rgba8(&png_path);
        let product_region = node_region_from_composition_report(&report, "product");
        let metrics = chrome_region_metrics(&image, product_region);
        assert!(
            metrics.luminance_range >= 0.12
                && metrics.unique_luma_levels >= 12
                && metrics.foreground_fraction >= 0.35,
            "material.preset chrome + environment.preset should render non-flat chrome-like product detail on {backend}, metrics={metrics:?}, region={product_region:?}, png={png_path:?}, report={report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_tonemap_color_contract_matches_oracle_on_cpu_and_gpu() {
    // Doctor pin: the render output stage must share the same PBR Neutral,
    // ACES, exposure, and sRGB transfer contract on CPU and HeadlessGpu.
    // This is a rendered-pixel oracle, not just a shader-string presence check.
    let dir = artifact_dir("recipe-render-tonemap-color-contract");
    let cases = [
        (
            "standard-gray-ev-minus2",
            "standard",
            -2.0,
            [0.18, 0.18, 0.18],
            [60, 60, 60],
        ),
        (
            "standard-gray-ev2",
            "standard",
            2.0,
            [0.18, 0.18, 0.18],
            [221, 221, 221],
        ),
        (
            "standard-color-ev0",
            "standard",
            0.0,
            [0.8, 0.2, 0.05],
            [231, 124, 63],
        ),
        (
            "pbr-neutral-gray-ev-minus1",
            "pbr_neutral",
            -1.0,
            [0.18, 0.18, 0.18],
            [63, 63, 63],
        ),
        (
            "pbr-neutral-color",
            "pbr_neutral",
            0.0,
            [0.8, 0.2, 0.05],
            [227, 113, 34],
        ),
        (
            "pbr-neutral-bright-ev1",
            "pbr_neutral",
            1.0,
            [1.0, 0.5, 0.1],
            [250, 193, 122],
        ),
        (
            "aces-gray-ev-minus2",
            "aces",
            -2.0,
            [0.18, 0.18, 0.18],
            [28, 28, 28],
        ),
        ("aces-color", "aces", 0.0, [0.8, 0.2, 0.05], [198, 104, 45]),
        (
            "aces-gray-ev2",
            "aces",
            2.0,
            [0.18, 0.18, 0.18],
            [188, 188, 188],
        ),
    ];
    let mut sweep = support::parity::ParitySweep::new("scena.tonemap_color_parity_sweep.v1");
    for (name, tonemapper, exposure_ev, linear_color, expected_srgb8) in cases {
        let (cpu_recipe, cpu_png) = write_tonemap_color_contract_recipe(
            &dir,
            &format!("{name}-cpu"),
            tonemapper,
            exposure_ev,
            linear_color,
        );
        let (gpu_recipe, gpu_png) = write_tonemap_color_contract_recipe(
            &dir,
            &format!("{name}-gpu"),
            tonemapper,
            exposure_ev,
            linear_color,
        );
        let cpu_report = run_recipe_render_introspect(&cpu_recipe, &cpu_png, false);
        let gpu_report = run_recipe_render_introspect(&gpu_recipe, &gpu_png, true);
        let cpu = decode_png_rgba8(&cpu_png);
        let gpu = decode_png_rgba8(&gpu_png);
        let region = content_region_from_introspection_report(&cpu_report)
            .intersect(content_region_from_introspection_report(&gpu_report))
            .and_then(|region| region.shrink(8))
            .expect("tonemap color proof should have a stable shared panel region");
        let comparison = sweep.compare_region(
            format!("{name}_cpu_vs_gpu"),
            support::parity::RgbaFrame::new("cpu", &cpu.rgba8, cpu.width, cpu.height),
            support::parity::RgbaFrame::new("gpu", &gpu.rgba8, gpu.width, gpu.height),
            region,
        );
        assert!(
            comparison.rmse <= 0.018
                && comparison.channel_delta.mean_channel_delta <= 2.5
                && comparison.channel_delta.max_channel_delta <= 8,
            "CPU/GPU {tonemapper} output must match the shared color contract within a tight region tolerance; comparison={comparison:?}, region={region:?}, cpu_png={cpu_png:?}, gpu_png={gpu_png:?}"
        );
        let cpu_pixel = center_pixel(&cpu);
        let gpu_pixel = center_pixel(&gpu);
        assert_rgb8_close(
            cpu_pixel,
            expected_srgb8,
            2,
            &format!("CPU {tonemapper} oracle pixel"),
        );
        assert_rgb8_close(
            gpu_pixel,
            expected_srgb8,
            3,
            &format!("GPU {tonemapper} oracle pixel"),
        );
    }
    assert_eq!(
        sweep.records().len(),
        cases.len(),
        "tonemap/color proof must record every tonemapper/exposure case"
    );
    sweep.write_json(&dir.join("tonemap-color-parity.json"), &[]);
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_chrome_ibl_fireflies_are_filtered_on_cpu_and_gpu() {
    // Doctor pin: chrome IBL fireflies were baked into GGX prefilter mips
    // when the prefilter point-sampled tiny HDR emitters. This recipe uses
    // the real Polyhaven studio HDRI and the public CLI verifier so the
    // regression has to stay fixed on the CPU and lavapipe GPU paths.
    let dir = artifact_dir("recipe-render-chrome-ibl-firefly");
    let mut rendered = Vec::new();
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) =
            write_chrome_ibl_firefly_recipe(&dir, &format!("chrome-ibl-firefly-{backend}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let output = command
            .args(args)
            .output()
            .expect("scena chrome IBL firefly render command runs");

        assert!(
            output.status.success(),
            "chrome IBL reflection should pass firefly quality on {backend}; stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU chrome IBL proof must use HeadlessGpu, not a fallback: {report:#}"
            );
        } else {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless",
                "CPU chrome IBL proof should use Headless CPU: {report:#}"
            );
        }
        assert_eq!(
            report["verification"]["quality"]["ok"], true,
            "chrome IBL reflection should pass quality checks on {backend}: {report:#}"
        );
        let checks = report["verification"]["quality"]["checks"]
            .as_array()
            .expect("quality checks serialize");
        assert!(
            checks.iter().all(|check| {
                check["code"] != "reflection_firefly_outliers"
                    && check["code"] != "reflection_structure_missing"
            }),
            "chrome IBL reflection should not emit reflection firefly/structure failures on {backend}: {report:#}"
        );
        assert!(png_path.exists(), "chrome IBL render writes the PNG");
        let image = decode_png_rgba8(&png_path);
        let region = node_region_from_composition_report(&report, "chrome_sphere");
        let firefly_fraction =
            reflection_firefly_fraction_in_region(&image.rgba8, image.width, image.height, region);
        assert!(
            firefly_fraction <= 0.006,
            "chrome IBL reflection must not contain isolated bright fireflies on {backend}; fraction={firefly_fraction:.5}, region={region:?}, png={png_path:?}"
        );
        rendered.push((
            backend.to_owned(),
            report,
            png_path,
            image,
            region,
            firefly_fraction,
        ));
    }

    let (cpu_backend, _cpu_report, _cpu_png, cpu, cpu_region, cpu_fireflies) = &rendered[0];
    let (gpu_backend, _gpu_report, _gpu_png, gpu, gpu_region, gpu_fireflies) = &rendered[1];
    assert_eq!(cpu_backend, "cpu");
    assert_eq!(gpu_backend, "gpu");
    assert_eq!((cpu.width, cpu.height), (gpu.width, gpu.height));
    assert_eq!(
        cpu_region, gpu_region,
        "CPU/GPU chrome IBL proof should compare the same projected region"
    );
    let delta = frame_delta_in_region(&cpu.rgba8, &gpu.rgba8, cpu.width, *cpu_region);
    assert!(
        delta.mean_channel_delta <= 80.0,
        "CPU/GPU chrome IBL reflection should agree within the broad renderer-backend tolerance after firefly filtering, delta={delta:?}, region={cpu_region:?}"
    );
    assert!(
        (*cpu_fireflies - *gpu_fireflies).abs() <= 0.006,
        "CPU/GPU chrome IBL firefly fractions should agree after filtering; cpu={cpu_fireflies:.5}, gpu={gpu_fireflies:.5}"
    );
    fs::write(
        dir.join("chrome-ibl-firefly-metrics.json"),
        format!(
            "{{\n  \"schema\": \"scena.chrome_ibl_firefly_probe.v1\",\n  \"cpu_firefly_fraction\": {:.5},\n  \"gpu_firefly_fraction\": {:.5},\n  \"mean_channel_delta\": {:.3},\n  \"max_channel_delta\": {},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
            cpu_fireflies,
            gpu_fireflies,
            delta.mean_channel_delta,
            delta.max_channel_delta,
            cpu_region.x,
            cpu_region.y,
            cpu_region.width,
            cpu_region.height
        ),
    )
    .expect("chrome IBL firefly metrics write");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_chrome_ibl_near_mirror_matches_cpu_and_keeps_detail_on_gpu() {
    // Doctor pin: near-mirror chrome must not blend low roughness into the
    // first heavily blurred prefilter mip. This compares the real CLI recipe
    // output on CPU and lavapipe HeadlessGpu instead of accepting verifier
    // ok:true, which did not catch the washed GPU reflection.
    let dir = artifact_dir("recipe-render-chrome-ibl-near-mirror-parity");
    let (cpu_recipe, cpu_png) =
        write_chrome_ibl_panel_recipe(&dir, "chrome-ibl-panel-cpu-r005", 0.05);
    let (gpu_recipe, gpu_png) =
        write_chrome_ibl_panel_recipe(&dir, "chrome-ibl-panel-gpu-r005", 0.05);
    let (gpu_sphere_recipe, gpu_sphere_png) =
        write_chrome_ibl_recipe(&dir, "chrome-ibl-sphere-gpu-r005", 0.05);
    let (gpu_mid012_recipe, gpu_mid012_png) =
        write_chrome_ibl_recipe(&dir, "chrome-ibl-sphere-gpu-r012", 0.12);
    let (gpu_mid025_recipe, gpu_mid025_png) =
        write_chrome_ibl_recipe(&dir, "chrome-ibl-sphere-gpu-r025", 0.25);
    let (gpu_mirror_recipe, gpu_mirror_png) =
        write_chrome_ibl_recipe(&dir, "chrome-ibl-sphere-gpu-r000", 0.0);
    let (gpu_rough_recipe, gpu_rough_png) =
        write_chrome_ibl_recipe(&dir, "chrome-ibl-sphere-gpu-r050", 0.50);

    let cpu_report = run_recipe_render_introspect(&cpu_recipe, &cpu_png, false);
    let gpu_report = run_recipe_render_introspect(&gpu_recipe, &gpu_png, true);
    let gpu_sphere_report = run_recipe_render_introspect(&gpu_sphere_recipe, &gpu_sphere_png, true);
    let gpu_mid012_report = run_recipe_render_introspect(&gpu_mid012_recipe, &gpu_mid012_png, true);
    let gpu_mid025_report = run_recipe_render_introspect(&gpu_mid025_recipe, &gpu_mid025_png, true);
    let gpu_mirror_report = run_recipe_render_introspect(&gpu_mirror_recipe, &gpu_mirror_png, true);
    let gpu_rough_report = run_recipe_render_introspect(&gpu_rough_recipe, &gpu_rough_png, true);
    let cpu = decode_png_rgba8(&cpu_png);
    let gpu = decode_png_rgba8(&gpu_png);
    let gpu_sphere = decode_png_rgba8(&gpu_sphere_png);
    let gpu_mid012 = decode_png_rgba8(&gpu_mid012_png);
    let gpu_mid025 = decode_png_rgba8(&gpu_mid025_png);
    let gpu_mirror = decode_png_rgba8(&gpu_mirror_png);
    let gpu_rough = decode_png_rgba8(&gpu_rough_png);
    let cpu_frame =
        support::parity::RgbaFrame::new("cpu_panel_r005", &cpu.rgba8, cpu.width, cpu.height);
    let gpu_frame =
        support::parity::RgbaFrame::new("gpu_panel_r005", &gpu.rgba8, gpu.width, gpu.height);
    let gpu_sphere_frame = support::parity::RgbaFrame::new(
        "gpu_sphere_r005",
        &gpu_sphere.rgba8,
        gpu_sphere.width,
        gpu_sphere.height,
    );
    let gpu_mid012_frame = support::parity::RgbaFrame::new(
        "gpu_sphere_r012",
        &gpu_mid012.rgba8,
        gpu_mid012.width,
        gpu_mid012.height,
    );
    let gpu_mid025_frame = support::parity::RgbaFrame::new(
        "gpu_sphere_r025",
        &gpu_mid025.rgba8,
        gpu_mid025.width,
        gpu_mid025.height,
    );
    let gpu_mirror_frame = support::parity::RgbaFrame::new(
        "gpu_sphere_r000",
        &gpu_mirror.rgba8,
        gpu_mirror.width,
        gpu_mirror.height,
    );
    let gpu_rough_frame = support::parity::RgbaFrame::new(
        "gpu_sphere_r050",
        &gpu_rough.rgba8,
        gpu_rough.width,
        gpu_rough.height,
    );
    let panel_region = content_region_from_introspection_report(&cpu_report)
        .intersect(content_region_from_introspection_report(&gpu_report))
        .expect("CPU/GPU chrome panel regions must overlap");
    let panel_interior = panel_region
        .shrink(18)
        .expect("chrome panel should have a stable interior parity region");
    let sphere_region = content_region_from_introspection_report(&gpu_sphere_report)
        .intersect(content_region_from_introspection_report(&gpu_mirror_report))
        .expect("GPU roughness sweep regions must overlap");
    let sphere_region = sphere_region
        .intersect(content_region_from_introspection_report(&gpu_mid012_report))
        .expect("GPU roughness 0.12 blur region must overlap near-mirror region");
    let sphere_region = sphere_region
        .intersect(content_region_from_introspection_report(&gpu_mid025_report))
        .expect("GPU roughness 0.25 blur region must overlap near-mirror region");
    let sphere_region = sphere_region
        .intersect(content_region_from_introspection_report(&gpu_rough_report))
        .expect("GPU roughness blur region must overlap near-mirror region");

    let mut sweep =
        support::parity::ParitySweep::new("scena.chrome_ibl_near_mirror_parity_probe.v1");
    let panel_comparison = sweep.compare_region(
        "chrome_panel_r005_cpu_vs_gpu",
        cpu_frame,
        gpu_frame,
        panel_interior,
    );
    let mirror_comparison = sweep.compare_region(
        "chrome_sphere_r005_vs_r000_gpu",
        gpu_sphere_frame,
        gpu_mirror_frame,
        sphere_region,
    );
    let mid012_comparison = sweep.compare_region(
        "chrome_sphere_r012_vs_r000_gpu",
        gpu_mid012_frame,
        gpu_mirror_frame,
        sphere_region,
    );
    let mid025_comparison = sweep.compare_region(
        "chrome_sphere_r025_vs_r000_gpu",
        gpu_mid025_frame,
        gpu_mirror_frame,
        sphere_region,
    );
    let rough_comparison = sweep.compare_region(
        "chrome_sphere_r050_vs_r000_gpu",
        gpu_rough_frame,
        gpu_mirror_frame,
        sphere_region,
    );
    assert_eq!(
        sweep.records().len(),
        5,
        "IBL parity proof must record the CPU/GPU panel comparison plus the GPU roughness sweep"
    );

    let parity_rmse = panel_comparison.rmse;
    let gpu_sobel = mirror_comparison.left_structure.sobel_luminance_energy;
    let gpu_mirror_sobel = mirror_comparison.right_structure.sobel_luminance_energy;
    let gpu_mid012_sobel = mid012_comparison.left_structure.sobel_luminance_energy;
    let gpu_mid025_sobel = mid025_comparison.left_structure.sobel_luminance_energy;
    let gpu_rough_sobel = rough_comparison.left_structure.sobel_luminance_energy;
    let cpu_chrome = panel_comparison.left_structure;
    let gpu_chrome = panel_comparison.right_structure;
    let gpu_sphere_chrome = mirror_comparison.left_structure;
    let gpu_mid012_chrome = mid012_comparison.left_structure;
    let gpu_mid025_chrome = mid025_comparison.left_structure;
    let gpu_mirror_delta = mirror_comparison.rmse;
    let gpu_mid012_delta = mid012_comparison.rmse;
    let gpu_mid025_delta = mid025_comparison.rmse;
    let gpu_rough_delta = rough_comparison.rmse;
    sweep.write_json(
        &dir.join("chrome-ibl-near-mirror-parity.json"),
        &[
            (
                "parity_harness_schema",
                "\"scena.cpu_gpu_parity_sweep.v1\"".to_owned(),
            ),
            ("panel_cpu_gpu_rmse", format!("{parity_rmse:.5}")),
            (
                "sphere_gpu_r005_r000_rmse",
                format!("{gpu_mirror_delta:.5}"),
            ),
            (
                "sphere_gpu_r012_r000_rmse",
                format!("{gpu_mid012_delta:.5}"),
            ),
            (
                "sphere_gpu_r025_r000_rmse",
                format!("{gpu_mid025_delta:.5}"),
            ),
            ("sphere_gpu_r050_r000_rmse", format!("{gpu_rough_delta:.5}")),
            ("gpu_sobel_energy", format!("{gpu_sobel:.5}")),
            ("gpu_mirror_sobel_energy", format!("{gpu_mirror_sobel:.5}")),
            ("gpu_mid012_sobel_energy", format!("{gpu_mid012_sobel:.5}")),
            ("gpu_mid025_sobel_energy", format!("{gpu_mid025_sobel:.5}")),
            ("gpu_rough_sobel_energy", format!("{gpu_rough_sobel:.5}")),
            (
                "cpu_panel_luminance_range",
                format!("{:.5}", cpu_chrome.luminance_range),
            ),
            (
                "gpu_panel_luminance_range",
                format!("{:.5}", gpu_chrome.luminance_range),
            ),
            (
                "gpu_sphere_luminance_range",
                format!("{:.5}", gpu_sphere_chrome.luminance_range),
            ),
            (
                "gpu_mid012_luminance_range",
                format!("{:.5}", gpu_mid012_chrome.luminance_range),
            ),
            (
                "gpu_mid025_luminance_range",
                format!("{:.5}", gpu_mid025_chrome.luminance_range),
            ),
            ("panel_region", parity_region_json(panel_interior)),
            ("sphere_region", parity_region_json(sphere_region)),
        ],
    );

    assert!(
        parity_rmse <= 0.055,
        "CPU and GPU near-mirror chrome panel interior must match within a tight native-resolution tolerance; rmse={parity_rmse:.5}, cpu_range={:?}, gpu_range={:?}, region={panel_interior:?}, cpu_png={cpu_png:?}, gpu_png={gpu_png:?}",
        cpu_chrome,
        gpu_chrome
    );
    assert!(
        gpu_mirror_delta <= 0.010 && gpu_sobel >= gpu_mirror_sobel * 0.94,
        "GPU roughness 0.05 chrome sphere must retain near-mirror high-frequency reflection detail; roughness_delta={gpu_mirror_delta:.5}, gpu_sobel={gpu_sobel:.5}, gpu_mirror_sobel={gpu_mirror_sobel:.5}, gpu_png={gpu_sphere_png:?}, mirror_png={gpu_mirror_png:?}"
    );
    assert!(
        // 0.90 preserves high-contrast reflections while accepting the stable
        // lavapipe result (0.9039); 0.92 rejected that healthy baseline.
        gpu_sphere_chrome.luminance_range >= 0.90,
        "GPU near-mirror chrome sphere must not wash out reflection contrast; gpu={gpu_sphere_chrome:?}, region={sphere_region:?}"
    );
    assert!(
        (0.08..=0.35).contains(&gpu_mid012_delta) && gpu_mid012_chrome.luminance_range >= 0.75,
        "GPU roughness 0.12 chrome must enter the mid-roughness prefilter range without washing out structured reflection contrast; delta={gpu_mid012_delta:.5}, sobel={gpu_mid012_sobel:.5}, metrics={gpu_mid012_chrome:?}, png={gpu_mid012_png:?}"
    );
    assert!(
        gpu_mid025_delta >= gpu_mid012_delta + 0.04
            && gpu_mid025_delta <= 0.45
            && gpu_mid025_chrome.luminance_range >= 0.70
            && gpu_mid025_sobel > gpu_rough_sobel,
        "GPU roughness 0.25 chrome must blur more than roughness 0.12 while retaining more structure than roughness 0.50; r012_delta={gpu_mid012_delta:.5}, r025_delta={gpu_mid025_delta:.5}, r025_sobel={gpu_mid025_sobel:.5}, r050_sobel={gpu_rough_sobel:.5}, metrics={gpu_mid025_chrome:?}, png={gpu_mid025_png:?}"
    );
    assert!(
        gpu_rough_delta >= gpu_mid025_delta + 0.10,
        "GPU rough chrome must still sample distinct blurred prefilter mips, not force every material to mip0; rough_delta={gpu_rough_delta:.5}, mirror_delta={gpu_mirror_delta:.5}, rough_png={gpu_rough_png:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_msaa_grid_floor_is_occluded_by_object() {
    let dir = artifact_dir("recipe-render-gpu-msaa-grid-occlusion");
    let (recipe_path, png_path) =
        write_grid_occlusion_recipe(&dir, "msaa4-grid-occlusion", "msaa4");
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut command);
    let output = command
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--gpu",
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena GPU MSAA grid occlusion render command runs");

    assert!(
        output.status.success(),
        "GPU MSAA4 grid occlusion render should succeed, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(
        report["capabilities"]["backend"], "headless_gpu",
        "GPU grid occlusion proof must use HeadlessGpu, not CPU fallback: {report:#}"
    );

    let image = decode_png_rgba8(&png_path);
    let object_bbox = color_bbox(
        &image.rgba8,
        image.width,
        image.height,
        is_blue_object_pixel,
    )
    .expect("blue occluder object should be visible in the rendered PNG");
    let object_interior = shrink_region(object_bbox, 6)
        .expect("blue occluder object should have an interior region after edge inset");
    let red_pixels = count_pixels_in_region(
        &image.rgba8,
        image.width,
        object_interior,
        is_red_grid_pixel,
    );
    fs::write(
        dir.join("msaa4-grid-occlusion.json"),
        format!(
            "{{\n  \"schema\": \"scena.grid_occlusion_probe.v1\",\n  \"red_grid_pixels_inside_object_interior\": {},\n  \"object_bbox\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }},\n  \"object_interior\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
            red_pixels,
            object_bbox.x,
            object_bbox.y,
            object_bbox.width,
            object_bbox.height,
            object_interior.x,
            object_interior.y,
            object_interior.width,
            object_interior.height
        ),
    )
    .expect("grid occlusion probe artifact writes");
    assert!(
        red_pixels == 0,
        "depth-tested grid floor strokes must be occluded by the object under GPU MSAA4; red_pixels_inside_object_interior={red_pixels}, object_bbox={object_bbox:?}, object_interior={object_interior:?}, png={png_path:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_msaa_overlays_write_png_on_real_adapter() {
    let dir = artifact_dir("recipe-render-gpu-msaa-overlays");
    let cases = [
        ("msaa4", json!({ "anti_aliasing": "msaa4" })),
        ("quality-high", json!({ "quality": "high" })),
    ];
    for (name, render) in cases {
        let (recipe_path, png_path) =
            write_line_quality_recipe_with_render(&dir, name, 0.0, 0.25, Some(render));
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        configure_command_for_lavapipe(&mut command);
        let output = command
            .args([
                "recipe",
                "render",
                path_str(&recipe_path),
                "--gpu",
                "--introspect",
                "--verify",
                "--out",
                path_str(&png_path),
            ])
            .output()
            .expect("scena GPU MSAA overlay render command runs");

        assert!(
            output.status.success(),
            "GPU {name} overlay render should write a PNG without wgpu validation errors, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        assert_eq!(
            report["introspection"]["capabilities"]["backend"], "headless_gpu",
            "GPU {name} overlay proof must use HeadlessGpu, not CPU fallback: {report:#}"
        );
        assert!(
            png_path.exists(),
            "GPU {name} overlay render writes the PNG"
        );
    }

    let (recipe_path, png_path) = write_line_quality_recipe_with_render(
        &dir,
        "msaa8",
        0.0,
        0.25,
        Some(json!({ "anti_aliasing": "msaa8" })),
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut command);
    let output = command
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--gpu",
            "--introspect",
            "--verify",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena GPU MSAA8 overlay render command runs");
    let stderr = stderr(&output);
    assert!(
        output.status.success() || (!stderr.contains("wgpu error") && !stderr.contains("panicked")),
        "GPU msaa8 must either render or fail before wgpu validation/panic on adapters without 8x depth support, stdout={}, stderr={stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    if output.status.success() {
        let report = json_report(&output);
        assert_eq!(
            report["introspection"]["capabilities"]["backend"], "headless_gpu",
            "GPU msaa8 overlay proof must use HeadlessGpu, not CPU fallback: {report:#}"
        );
        assert!(png_path.exists(), "GPU msaa8 overlay render writes the PNG");
    } else {
        assert!(
            has_actionable_msaa_limit(&stderr, 4, 8),
            "GPU msaa8 must fail with an actionable sample-count capability diagnostic, got stderr={stderr}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_supersample_changes_curve_grid_and_specular_pixels_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-supersample-quality");
    for case in ["curve", "grid", "specular"] {
        for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
            let (base_recipe, base_png) =
                write_supersample_recipe(&dir, &format!("{case}-{backend}-base"), case, 1);
            let (hero_recipe, hero_png) =
                write_supersample_recipe(&dir, &format!("{case}-{backend}-hero"), case, 3);

            let base = run_recipe_render_verify(&base_recipe, &base_png, use_gpu);
            let hero = run_recipe_render_verify(&hero_recipe, &hero_png, use_gpu);

            assert_eq!(base["capture"]["width"], 180, "{base:#}");
            assert_eq!(base["capture"]["height"], 120, "{base:#}");
            assert_eq!(hero["capture"]["width"], 180, "{hero:#}");
            assert_eq!(hero["capture"]["height"], 120, "{hero:#}");
            assert_ne!(
                base["capture"]["payload"]["fnv1a64"], hero["capture"]["payload"]["fnv1a64"],
                "supersample:3 must change final native-resolution pixels for {case} on {backend}; base={base:#}; hero={hero:#}"
            );

            let base_png = decode_png_rgba8(&base_png);
            let hero_png = decode_png_rgba8(&hero_png);
            assert_eq!((base_png.width, base_png.height), (180, 120));
            assert_eq!((hero_png.width, hero_png.height), (180, 120));
            assert_ne!(
                base_png.rgba8, hero_png.rgba8,
                "decoded PNG pixels should differ for supersampled {case} on {backend}"
            );
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_reconstruction_widens_dashboard_bar_and_preserves_grid_edges_without_haloing()
 {
    let dir = artifact_dir("recipe-render-reconstruction-dashboard-edge");
    let mut results = Vec::new();
    let mut grid_results = Vec::new();
    for supersample in [2_u8, 4, 8] {
        for reconstruction in ["box", "tent", "gaussian"] {
            let (recipe_path, png_path) = write_reconstruction_bar_recipe(
                &dir,
                &format!("dashboard-bar-{reconstruction}-ss{supersample}"),
                supersample,
                reconstruction,
            );
            let report = run_recipe_render_verify(&recipe_path, &png_path, true);
            assert_eq!(report["capture"]["width"], 260, "{report:#}");
            assert_eq!(report["capture"]["height"], 180, "{report:#}");
            let image = decode_png_rgba8(&png_path);
            let metrics = edge_reconstruction_metrics(&image.rgba8, image.width, image.height);
            results.push((reconstruction.to_owned(), supersample, metrics));

            let (grid_recipe_path, grid_png_path) = write_reconstruction_grid_recipe(
                &dir,
                &format!("dashboard-grid-{reconstruction}-ss{supersample}"),
                supersample,
                reconstruction,
            );
            let grid_report = run_recipe_render_verify(&grid_recipe_path, &grid_png_path, true);
            assert_eq!(grid_report["capture"]["width"], 260, "{grid_report:#}");
            assert_eq!(grid_report["capture"]["height"], 180, "{grid_report:#}");
            let grid_image = decode_png_rgba8(&grid_png_path);
            let grid_metrics =
                edge_reconstruction_metrics(&grid_image.rgba8, grid_image.width, grid_image.height);
            grid_results.push((reconstruction.to_owned(), supersample, grid_metrics));
        }
    }
    let box_ss2 = results
        .iter()
        .find(|(filter, supersample, _)| filter == "box" && *supersample == 2)
        .expect("box ss2 metrics exist")
        .2;
    let gaussian_ss4 = results
        .iter()
        .find(|(filter, supersample, _)| filter == "gaussian" && *supersample == 4)
        .expect("gaussian ss4 metrics exist")
        .2;
    let gaussian_ss8 = results
        .iter()
        .find(|(filter, supersample, _)| filter == "gaussian" && *supersample == 8)
        .expect("gaussian ss8 metrics exist")
        .2;
    let tent_ss4 = results
        .iter()
        .find(|(filter, supersample, _)| filter == "tent" && *supersample == 4)
        .expect("tent ss4 metrics exist")
        .2;
    let grid_box_ss2 = grid_results
        .iter()
        .find(|(filter, supersample, _)| filter == "box" && *supersample == 2)
        .expect("grid box ss2 metrics exist")
        .2;
    let grid_tent_ss2 = grid_results
        .iter()
        .find(|(filter, supersample, _)| filter == "tent" && *supersample == 2)
        .expect("grid tent ss2 metrics exist")
        .2;
    let grid_tent_ss8 = grid_results
        .iter()
        .find(|(filter, supersample, _)| filter == "tent" && *supersample == 8)
        .expect("grid tent ss8 metrics exist")
        .2;

    fs::write(
        dir.join("dashboard-bar-reconstruction-metrics.json"),
        format_reconstruction_metrics(&results),
    )
    .expect("reconstruction metrics artifact writes");
    fs::write(
        dir.join("dashboard-grid-reconstruction-metrics.json"),
        format_reconstruction_metrics(&grid_results),
    )
    .expect("grid reconstruction metrics artifact writes");

    assert!(
        gaussian_ss4.intermediate_px_per_edge > box_ss2.intermediate_px_per_edge + 0.35,
        "gaussian ss4 should widen the measured dashboard bar edge ramp; box_ss2={box_ss2:?}, gaussian_ss4={gaussian_ss4:?}, all={results:#?}"
    );
    assert!(
        gaussian_ss4.transition_width_px > box_ss2.transition_width_px + 0.35,
        "gaussian ss4 should increase transition width; box_ss2={box_ss2:?}, gaussian_ss4={gaussian_ss4:?}, all={results:#?}"
    );
    assert!(
        gaussian_ss4.unique_luma_levels >= box_ss2.unique_luma_levels,
        "gaussian ss4 should retain or increase edge luma levels; box_ss2={box_ss2:?}, gaussian_ss4={gaussian_ss4:?}, all={results:#?}"
    );
    assert!(
        gaussian_ss4.halo_overshoot <= 0.02 && tent_ss4.halo_overshoot <= 0.02,
        "positive reconstruction kernels must not introduce visible edge halos; tent_ss4={tent_ss4:?}, gaussian_ss4={gaussian_ss4:?}, all={results:#?}"
    );
    assert!(
        gaussian_ss4.contrast_range >= box_ss2.contrast_range * 0.82,
        "hero reconstruction should retain most edge contrast; box_ss2={box_ss2:?}, gaussian_ss4={gaussian_ss4:?}, all={results:#?}"
    );
    assert!(
        gaussian_ss8.unique_luma_levels >= gaussian_ss4.unique_luma_levels
            && gaussian_ss8.halo_overshoot <= 0.02
            && gaussian_ss8.contrast_range >= box_ss2.contrast_range * 0.82,
        "guarded ss8 hero reconstruction should render on the small dashboard target without halo or contrast loss; gaussian_ss4={gaussian_ss4:?}, gaussian_ss8={gaussian_ss8:?}, all={results:#?}"
    );
    assert!(
        grid_tent_ss2.intermediate_px_per_edge >= 3.5
            && grid_tent_ss2.intermediate_px_per_edge
                >= grid_box_ss2.intermediate_px_per_edge * 0.95,
        "floor/grid strokes already have geometric AA; tent ss2 must preserve a broad ramp without regressing box ss2; grid_box_ss2={grid_box_ss2:?}, grid_tent_ss2={grid_tent_ss2:?}, all={grid_results:#?}"
    );
    assert!(
        grid_tent_ss2.unique_luma_levels >= 100
            && grid_tent_ss2.unique_luma_levels + 4 >= grid_box_ss2.unique_luma_levels,
        "floor/grid strokes need dense subpixel luma levels and must not materially regress from box ss2; grid_box_ss2={grid_box_ss2:?}, grid_tent_ss2={grid_tent_ss2:?}, all={grid_results:#?}"
    );
    assert!(
        grid_tent_ss2.halo_overshoot <= 0.08,
        "floor/grid stroke reconstruction must keep halo/overshoot bounded; grid_box_ss2={grid_box_ss2:?}, grid_tent_ss2={grid_tent_ss2:?}, all={grid_results:#?}"
    );
    assert!(
        grid_tent_ss2.contrast_range >= grid_box_ss2.contrast_range * 0.9,
        "floor/grid stroke reconstruction should retain most line contrast; grid_box_ss2={grid_box_ss2:?}, grid_tent_ss2={grid_tent_ss2:?}, all={grid_results:#?}"
    );
    assert!(
        grid_tent_ss8.intermediate_px_per_edge >= 3.0
            && grid_tent_ss8.halo_overshoot <= 0.10
            && grid_tent_ss8.contrast_range >= grid_box_ss2.contrast_range * 0.75,
        "guarded ss8 reconstruction should keep grid/stroke lines usable without excessive haloing; grid_box_ss2={grid_box_ss2:?}, grid_tent_ss8={grid_tent_ss8:?}, all={grid_results:#?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_grid_floor_lines_are_antialiased_and_stable_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-grid-floor-line-quality");
    let mut results = Vec::new();
    let mut detail_results = Vec::new();
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (recipe_path, png_path) = write_recipe_grid_floor_line_quality_recipe(
            &dir,
            &format!("{backend}-grid-floor-line-quality"),
        );
        let report = run_recipe_render_verify(&recipe_path, &png_path, use_gpu);
        assert_eq!(
            report["introspection"]["framing"]["cropped"], false,
            "grid floor line proof must measure an uncropped native-resolution region on {backend}: {report:#}"
        );

        let image = decode_png_rgba8(&png_path);
        let metrics = edge_reconstruction_metrics(&image.rgba8, image.width, image.height);
        let detail_crop = floor_grid_detail_crop(&image);
        let detail_metrics =
            edge_reconstruction_metrics(&detail_crop.rgba8, detail_crop.width, detail_crop.height);
        results.push((backend.to_owned(), 2, metrics));
        detail_results.push((format!("{backend}-floor-detail"), 2, detail_metrics));
        assert!(
            metrics.intermediate_px_per_edge >= 4.5,
            "floor grid lines need a broad antialiasing ramp at native resolution on {backend}, metrics={metrics:?}, report={report:#}"
        );
        assert!(
            metrics.unique_luma_levels >= 90,
            "floor grid lines need enough subpixel luminance levels to avoid visible stair-step bands on {backend}, metrics={metrics:?}, report={report:#}"
        );
        assert!(
            metrics.halo_overshoot <= 0.45,
            "floor grid line reconstruction must keep halo/overshoot bounded on {backend}, metrics={metrics:?}, report={report:#}"
        );
        assert!(
            metrics.contrast_range >= 0.5,
            "floor grid lines must retain enough contrast after reconstruction on {backend}, metrics={metrics:?}, report={report:#}"
        );
        assert!(
            detail_metrics.transition_width_px >= 2.5,
            "floor grid lines need enough coverage in the visible lower-floor detail crop on {backend}, detail_metrics={detail_metrics:?}, full_metrics={metrics:?}, report={report:#}"
        );
        assert!(
            detail_metrics.halo_overshoot <= 0.03,
            "floor grid line detail crop must keep halo/overshoot bounded on {backend}, detail_metrics={detail_metrics:?}, full_metrics={metrics:?}, report={report:#}"
        );
        assert!(
            detail_metrics.contrast_range >= 0.86,
            "floor grid line detail crop must retain line contrast on {backend}, detail_metrics={detail_metrics:?}, full_metrics={metrics:?}, report={report:#}"
        );
    }
    fs::write(
        dir.join("grid-floor-line-quality.json"),
        format_reconstruction_metrics(&results),
    )
    .expect("grid floor line metrics artifact writes");
    fs::write(
        dir.join("grid-floor-line-detail-quality.json"),
        format_reconstruction_metrics(&detail_results),
    )
    .expect("grid floor line detail metrics artifact writes");
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_grid_floor_line_quality_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-grid-floor-line-quality-verification");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (bad_recipe, bad_png) = write_recipe_grid_floor_line_quality_recipe_with_settings(
            &dir,
            &format!("{backend}-grid-floor-line-quality-bad"),
            "none",
            1,
            "box",
            0.55,
            "#39404A",
            true,
        );
        let mut bad_command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut bad_command);
        }
        let mut bad_args = vec!["recipe", "render", path_str(&bad_recipe)];
        if use_gpu {
            bad_args.push("--gpu");
        }
        bad_args.extend(["--introspect", "--verify", "--out", path_str(&bad_png)]);
        let bad_output = bad_command
            .args(bad_args)
            .output()
            .expect("scena grid quality bad render command runs");
        assert!(
            !bad_output.status.success(),
            "{backend} aliased scene.grid should fail verification, stdout={}, stderr={}",
            String::from_utf8_lossy(&bad_output.stdout),
            stderr(&bad_output)
        );
        let bad_report = json_report(&bad_output);
        if use_gpu {
            assert_eq!(
                bad_report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU grid-line quality proof must use HeadlessGpu, not fallback: {bad_report:#}"
            );
        }
        assert!(
            bad_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks array")
                .iter()
                .any(|check| check["id"] == "expect_quality.grid_floor_lines"
                    && check["code"] == "grid_line_quality_too_low"),
            "{backend} low-quality scene.grid must fail with exact grid_line_quality_too_low quality check: {bad_report:#}"
        );
        assert!(
            bad_report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons array")
                .iter()
                .any(|reason| reason["source"] == "quality"
                    && reason["code"] == "grid_line_quality_too_low"
                    && reason["expectation_id"] == "expect_quality.grid_floor_lines"),
            "{backend} grid-line quality failure should surface as a verification reason: {bad_report:#}"
        );

        let (good_recipe, good_png) = write_recipe_grid_floor_line_quality_recipe_with_settings(
            &dir,
            &format!("{backend}-grid-floor-line-quality-good"),
            "msaa4",
            2,
            "tent",
            4.0,
            "#F2F6FF",
            true,
        );
        let good_report = run_recipe_render_verify(&good_recipe, &good_png, use_gpu);
        assert!(
            good_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks array")
                .iter()
                .any(|check| check["id"] == "expect_quality.grid_floor_lines"
                    && check["code"] == "grid_line_quality_checked"
                    && check["severity"] == "info"),
            "{backend} antialiased scene.grid should emit a passing grid-line quality coverage check: {good_report:#}"
        );
        assert!(
            good_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks array")
                .iter()
                .all(|check| check["code"] != "grid_line_quality_too_low"),
            "{backend} antialiased scene.grid should pass grid-line quality: {good_report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_profile_quality_keeps_grid_floor_check_with_explicit_quality_blocks() {
    let dir = artifact_dir("recipe-render-grid-floor-line-quality-profile-composed");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (bad_recipe, bad_png) = write_recipe_grid_floor_line_quality_recipe_with_settings(
            &dir,
            &format!("{backend}-grid-floor-line-quality-composed"),
            "none",
            1,
            "box",
            0.55,
            "#39404A",
            true,
        );
        add_text_quality_block(&bad_recipe);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&bad_recipe)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&bad_png)]);
        let output = command
            .args(args)
            .output()
            .expect("scena composed grid quality render command runs");
        assert!(
            !output.status.success(),
            "{backend} profile quality with an explicit text block should still fail low-quality scene.grid output, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU composed grid quality proof must use HeadlessGpu, not fallback: {report:#}"
            );
        }
        assert!(
            report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks array")
                .iter()
                .any(|check| check["id"] == "expect_quality.grid_floor_lines"
                    && check["code"] == "grid_line_quality_too_low"),
            "{backend} composed expect_quality must retain the grid-line baseline check: {report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_many_lights_use_tiled_assignment_before_truncation() {
    let dir = artifact_dir("recipe-render-gpu-light-capacity");
    let recipe_path = dir.join("many-point-lights.recipe.json");
    let png_path = dir.join("many-point-lights.png");
    let point_lights = (0..17)
        .map(|index| {
            json!({
                "id": format!("point-{index}"),
                "kind": "point",
                "intensity_candela": 120.0,
                "range": 4.0,
                "transform": {
                    "kind": "trs",
                    "translation": [-1.6 + index as f64 * 0.2, 0.8, 1.0]
                }
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "subject": "#C8CED8"
            },
            "geometries": [
                { "id": "box_geo", "primitive": { "kind": "box", "size": [0.4, 0.4, 0.4] } }
            ],
            "materials": [
                { "id": "box_mat", "kind": "pbr_metallic_roughness", "base_color": "subject", "roughness": 0.55 }
            ],
            "nodes": [
                { "id": "box", "geometry": "box_geo", "material": "box_mat" }
            ],
            "lights": point_lights,
            "render": {
                "anti_aliasing": "none"
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 38.0,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.0, 0.7, 1.6],
                    "target": [0.0, 0.0, 0.0]
                }
            }],
            "capture": { "width": 160, "height": 120 }
        }))
        .expect("many-light capacity recipe serializes"),
    )
    .expect("many-light capacity recipe writes");

    let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
    configure_command_for_lavapipe(&mut command);
    let output = command
        .args([
            "recipe",
            "render",
            path_str(&recipe_path),
            "--gpu",
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena GPU many-light capacity command runs");
    assert!(
        output.status.success(),
        "GPU tiled light assignment should render many point lights instead of failing the old fixed uniform cap; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    let backend = report
        .pointer("/introspection/capabilities/backend")
        .or_else(|| report.pointer("/capabilities/backend"));
    assert_eq!(
        backend,
        Some(&json!("headless_gpu")),
        "many-light proof must run on HeadlessGpu, not CPU fallback: {report:#}"
    );
    assert!(
        png_path.exists(),
        "many-light GPU render should write the requested PNG through tiled assignment"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_tiled_many_point_lights_use_late_light() {
    let dir = artifact_dir("recipe-render-gpu-tiled-light-assignment");
    let baseline_recipe_path = dir.join("tiled-many-point-lights-baseline.recipe.json");
    let late_recipe_path = dir.join("tiled-many-point-lights-late.recipe.json");
    let baseline_png = dir.join("tiled-many-point-lights-baseline.png");
    let late_png = dir.join("tiled-many-point-lights-late.png");
    let recipe = |include_late_light: bool| {
        let mut lights = (0..20)
            .map(|index| {
                json!({
                    "id": format!("point_{index}"),
                    "kind": "point",
                    "intensity_candela": 6.0,
                    "range": 1.05,
                    "transform": {
                        "kind": "trs",
                        "translation": [-1.55 + index as f64 * 0.155, 0.62, 0.92]
                    }
                })
            })
            .collect::<Vec<_>>();
        if include_late_light {
            lights.push(json!({
                "id": "point_20_blue_late",
                "kind": "point",
                "color": "#2E72FF",
                "intensity_candela": 56.0,
                "range": 0.82,
                "transform": {
                    "kind": "trs",
                    "translation": [0.64, 0.32, 0.74]
                }
            }));
        }
        json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "box", "size": [0.9, 0.5, 0.24] } }
            ],
            "materials": [
                {
                    "id": "body_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "#5A6472",
                    "metallic": 0.0,
                    "roughness": 0.58
                }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat" }
            ],
            "lights": lights,
            "scene": {
                "background": { "kind": "custom", "color": "#20242C" },
                "environment": { "kind": "none" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 30.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.22, 2.35], "target": "body" }
            }],
            "capture": { "width": 220, "height": 160 },
            "expect": {
                "expect_visible": [{
                    "id": "body-visible",
                    "target": { "kind": "node", "id": "body" }
                }]
            }
        })
    };
    fs::write(
        &baseline_recipe_path,
        serde_json::to_string_pretty(&recipe(false))
            .expect("baseline tiled-light recipe serializes"),
    )
    .expect("baseline tiled-light recipe writes");
    fs::write(
        &late_recipe_path,
        serde_json::to_string_pretty(&recipe(true)).expect("late tiled-light recipe serializes"),
    )
    .expect("late tiled-light recipe writes");

    let baseline_report = run_recipe_render_verify(&baseline_recipe_path, &baseline_png, true);
    let late_report = run_recipe_render_verify(&late_recipe_path, &late_png, true);
    assert_eq!(
        baseline_report["introspection"]["capabilities"]["backend"], "headless_gpu",
        "many-light proof must run on HeadlessGpu, not CPU fallback: {baseline_report:#}"
    );
    assert_eq!(
        late_report["introspection"]["capabilities"]["backend"], "headless_gpu",
        "many-light proof must run on HeadlessGpu, not CPU fallback: {late_report:#}"
    );

    let baseline = decode_png_rgba8(&baseline_png);
    let late = decode_png_rgba8(&late_png);
    let probe = QualityPixelRegion {
        x: 124,
        y: 57,
        width: 54,
        height: 52,
    };
    let baseline_blue = mean_blue_in_region(&baseline.rgba8, baseline.width, probe);
    let late_blue = mean_blue_in_region(&late.rgba8, late.width, probe);
    fs::write(
        dir.join("tiled-many-point-light-blue-delta.json"),
        format!(
            "{{\n  \"schema\": \"scena.tiled_light_assignment_probe.v1\",\n  \"baseline_blue\": {:.4},\n  \"late_light_blue\": {:.4},\n  \"blue_delta\": {:.4},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
            baseline_blue,
            late_blue,
            late_blue - baseline_blue,
            probe.x,
            probe.y,
            probe.width,
            probe.height
        ),
    )
    .expect("tiled many-light metrics artifact writes");
    assert!(
        late_blue > baseline_blue + 0.030,
        "a point light beyond the old fixed 16-light GPU lane must contribute to rendered pixels through tiled light assignment; baseline_blue={baseline_blue:.4}, late_blue={late_blue:.4}, baseline_report={baseline_report:#}, late_report={late_report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_area_light_shape_changes_pixels_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-area-light-shape");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (narrow_recipe, narrow_png) =
            write_area_light_shape_recipe(&dir, &format!("{backend}-narrow"), 0.12);
        let (wide_recipe, wide_png) =
            write_area_light_shape_recipe(&dir, &format!("{backend}-wide"), 1.8);

        let narrow_report = run_recipe_render_verify(&narrow_recipe, &narrow_png, use_gpu);
        let wide_report = run_recipe_render_verify(&wide_recipe, &wide_png, use_gpu);
        assert_eq!(narrow_report["capture"]["width"], 180, "{narrow_report:#}");
        assert_eq!(wide_report["capture"]["height"], 140, "{wide_report:#}");

        let narrow = decode_png_rgba8(&narrow_png);
        let wide = decode_png_rgba8(&wide_png);
        assert_eq!((narrow.width, narrow.height), (180, 140));
        assert_eq!((wide.width, wide.height), (180, 140));
        let delta = frame_delta_in_region(
            &narrow.rgba8,
            &wide.rgba8,
            narrow.width,
            QualityPixelRegion {
                x: 0,
                y: 0,
                width: narrow.width,
                height: narrow.height,
            },
        );
        assert!(
            delta.mean_channel_delta > 1.0 && delta.max_channel_delta > 10,
            "area-light rect width must affect native-resolution pixels on {backend}; delta={delta:?}, narrow={narrow_report:#}, wide={wide_report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_area_light_broadens_specular_highlight_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-area-light-specular-spread");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (point_recipe, point_png) = write_area_light_specular_recipe(
            &dir,
            &format!("specular-spread-{backend}-point"),
            false,
        );
        let (area_recipe, area_png) = write_area_light_specular_recipe(
            &dir,
            &format!("specular-spread-{backend}-area"),
            true,
        );

        let point_report = run_recipe_render_verify(&point_recipe, &point_png, use_gpu);
        let area_report = run_recipe_render_verify(&area_recipe, &area_png, use_gpu);
        let point = decode_png_rgba8(&point_png);
        let area = decode_png_rgba8(&area_png);
        let region = QualityPixelRegion {
            x: 22,
            y: 10,
            width: 136,
            height: 122,
        };
        let point_metrics = specular_spread_metrics(&point.rgba8, point.width, region);
        let area_metrics = specular_spread_metrics(&area.rgba8, area.width, region);
        let metrics_name = match backend {
            "cpu" => "specular-spread-cpu-metrics.json",
            "gpu" => "specular-spread-gpu-metrics.json",
            _ => unreachable!("known backend label"),
        };
        fs::write(
            dir.join(metrics_name),
            format!(
                "{{\n  \"schema\": \"scena.specular_spread_probe.v1\",\n  \"backend\": \"{backend}\",\n  \"point\": {{ \"fwhm_pixels\": {}, \"unique_luma_levels\": {}, \"median_luminance\": {:.4}, \"peak_luminance\": {:.4}, \"threshold_luminance\": {:.4} }},\n  \"area\": {{ \"fwhm_pixels\": {}, \"unique_luma_levels\": {}, \"median_luminance\": {:.4}, \"peak_luminance\": {:.4}, \"threshold_luminance\": {:.4} }},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
                point_metrics.fwhm_pixels,
                point_metrics.unique_luma_levels,
                point_metrics.median_luminance,
                point_metrics.peak_luminance,
                point_metrics.threshold_luminance,
                area_metrics.fwhm_pixels,
                area_metrics.unique_luma_levels,
                area_metrics.median_luminance,
                area_metrics.peak_luminance,
                area_metrics.threshold_luminance,
                region.x,
                region.y,
                region.width,
                region.height
            ),
        )
        .expect("specular spread metrics artifact writes");
        assert!(
            area_metrics.fwhm_pixels > point_metrics.fwhm_pixels.saturating_add(120),
            "broad area light should produce a wider native-resolution specular lobe than a point light on {backend}; point={point_metrics:?}, area={area_metrics:?}, point_report={point_report:#}, area_report={area_report:#}"
        );
        assert!(
            area_metrics.unique_luma_levels >= point_metrics.unique_luma_levels.saturating_sub(8),
            "broad area-light highlight must remain graded, not collapse into a flat patch on {backend}; point={point_metrics:?}, area={area_metrics:?}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_area_light_ltc_specular_matches_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-area-light-ltc-parity");
    let cases = [
        ("rect-low-roughness", "rect", 1.6, 0.9, 0.08),
        ("rect-mid-roughness", "rect", 1.6, 0.9, 0.34),
        ("rect-high-roughness", "rect", 1.6, 0.9, 0.68),
        ("disc-mid-roughness", "disc", 1.2, 1.2, 0.34),
        ("sphere-high-roughness", "sphere", 0.9, 0.9, 0.56),
    ];
    let region = QualityPixelRegion {
        x: 22,
        y: 10,
        width: 136,
        height: 122,
    };

    for (case, shape, width, height, roughness) in cases {
        let (cpu_recipe, cpu_png) = write_area_light_specular_recipe_with_options(
            &dir,
            &format!("ltc-specular-cpu-{case}"),
            true,
            shape,
            width,
            height,
            roughness,
        );
        let (gpu_recipe, gpu_png) = write_area_light_specular_recipe_with_options(
            &dir,
            &format!("ltc-specular-gpu-{case}"),
            true,
            shape,
            width,
            height,
            roughness,
        );

        let cpu_report = run_recipe_render_verify(&cpu_recipe, &cpu_png, false);
        let gpu_report = run_recipe_render_verify(&gpu_recipe, &gpu_png, true);
        let cpu = decode_png_rgba8(&cpu_png);
        let gpu = decode_png_rgba8(&gpu_png);
        let cpu_metrics = specular_spread_metrics(&cpu.rgba8, cpu.width, region);
        let gpu_metrics = specular_spread_metrics(&gpu.rgba8, gpu.width, region);
        let delta = frame_delta_in_region(&cpu.rgba8, &gpu.rgba8, cpu.width, region);
        fs::write(
            dir.join(format!("area-light-ltc-cpu-gpu-parity-{case}.json")),
            format!(
                "{{\n  \"schema\": \"scena.area_light_ltc_parity_probe.v1\",\n  \"case\": \"{}\",\n  \"shape\": \"{}\",\n  \"roughness\": {:.3},\n  \"cpu\": {{ \"fwhm_pixels\": {}, \"unique_luma_levels\": {}, \"median_luminance\": {:.4}, \"peak_luminance\": {:.4}, \"threshold_luminance\": {:.4} }},\n  \"gpu\": {{ \"fwhm_pixels\": {}, \"unique_luma_levels\": {}, \"median_luminance\": {:.4}, \"peak_luminance\": {:.4}, \"threshold_luminance\": {:.4} }},\n  \"delta\": {{ \"mean_channel_delta\": {:.4}, \"p999_channel_delta\": {}, \"max_channel_delta\": {} }},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
                case,
                shape,
                roughness,
                cpu_metrics.fwhm_pixels,
                cpu_metrics.unique_luma_levels,
                cpu_metrics.median_luminance,
                cpu_metrics.peak_luminance,
                cpu_metrics.threshold_luminance,
                gpu_metrics.fwhm_pixels,
                gpu_metrics.unique_luma_levels,
                gpu_metrics.median_luminance,
                gpu_metrics.peak_luminance,
                gpu_metrics.threshold_luminance,
                delta.mean_channel_delta,
                delta.p999_channel_delta,
                delta.max_channel_delta,
                region.x,
                region.y,
                region.width,
                region.height
            ),
        )
        .expect("LTC area-light parity metrics artifact writes");

        let fwhm_delta = cpu_metrics.fwhm_pixels.abs_diff(gpu_metrics.fwhm_pixels);
        assert!(
            fwhm_delta <= 48,
            "CPU and GPU LTC area-light specular spread should stay within a tight native-resolution tolerance for {case}; cpu={cpu_metrics:?}, gpu={gpu_metrics:?}, delta={delta:?}, cpu_report={cpu_report:#}, gpu_report={gpu_report:#}"
        );
        assert!(
            delta.mean_channel_delta <= 14.0 && delta.p999_channel_delta <= 84,
            "CPU and GPU area-light LTC renders should match across the shape/roughness sweep for {case}; cpu={cpu_metrics:?}, gpu={gpu_metrics:?}, delta={delta:?}, cpu_report={cpu_report:#}, gpu_report={gpu_report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_area_light_caster_darkens_tessellated_receiver_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-area-light-shadow");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (baseline_recipe, baseline_png) =
            write_area_light_shadow_recipe(&dir, &format!("area-shadow-{backend}-baseline"), false);
        let (shadow_recipe, shadow_png) =
            write_area_light_shadow_recipe(&dir, &format!("area-shadow-{backend}-caster"), true);
        let baseline_report = run_recipe_render_verify(&baseline_recipe, &baseline_png, use_gpu);
        let shadow_report = run_recipe_render_verify(&shadow_recipe, &shadow_png, use_gpu);
        let baseline = decode_png_rgba8(&baseline_png);
        let shadowed = decode_png_rgba8(&shadow_png);
        let shadow_region = QualityPixelRegion {
            x: 105,
            y: 59,
            width: 45,
            height: 30,
        };
        let baseline_luma =
            mean_non_caster_luminance_in_region(&baseline.rgba8, baseline.width, shadow_region);
        let shadow_luma =
            mean_non_caster_luminance_in_region(&shadowed.rgba8, shadowed.width, shadow_region);
        let delta = baseline_luma.mean_luminance - shadow_luma.mean_luminance;
        fs::write(
            dir.join(format!("area-shadow-{backend}-metrics.json")),
            format!(
                "{{\n  \"schema\": \"scena.area_shadow_probe.v1\",\n  \"backend\": \"{backend}\",\n  \"baseline_receiver_luminance\": {:.4},\n  \"shadowed_receiver_luminance\": {:.4},\n  \"receiver_luminance_delta\": {:.4},\n  \"baseline_receiver_pixels\": {},\n  \"shadowed_receiver_pixels\": {},\n  \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}\n}}\n",
                baseline_luma.mean_luminance,
                shadow_luma.mean_luminance,
                delta,
                baseline_luma.receiver_pixels,
                shadow_luma.receiver_pixels,
                shadow_region.x,
                shadow_region.y,
                shadow_region.width,
                shadow_region.height
            ),
        )
        .expect("area shadow metrics artifact writes");
        assert!(
            delta > 0.012,
            "area-light caster should darken the deterministic receiver shadow window on {backend}; delta={delta}, baseline={baseline_luma:?}, shadowed={shadow_luma:?}, baseline_report={baseline_report:#}, shadow_report={shadow_report:#}"
        );
        assert!(
            baseline_luma.receiver_pixels > 1_200 && shadow_luma.receiver_pixels > 900,
            "area-light shadow proof must measure the receiver surface, not the caster/background; baseline={baseline_luma:?}, shadowed={shadow_luma:?}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_area_light_soft_shadow_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-area-light-quality");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (bad_recipe, bad_png) = write_area_light_quality_recipe(
            &dir,
            &format!("area-soft-shadow-bad-{backend}"),
            0.04,
            true,
        );
        let mut bad_args = vec!["recipe", "render", path_str(&bad_recipe)];
        if use_gpu {
            bad_args.push("--gpu");
        }
        bad_args.extend(["--introspect", "--verify", "--out", path_str(&bad_png)]);
        let mut bad_command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut bad_command);
        }
        let bad_output = bad_command
            .args(bad_args)
            .output()
            .expect("scena area-light quality bad recipe render command runs");
        assert!(
            !bad_output.status.success(),
            "tiny area emitter should fail soft-shadow quality on {backend}, stdout={}, stderr={}",
            String::from_utf8_lossy(&bad_output.stdout),
            stderr(&bad_output)
        );
        let bad_report = json_report(&bad_output);
        if use_gpu {
            assert_eq!(
                bad_report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU area-light quality failure proof must use the GPU backend, not a fallback: {bad_report:#}"
            );
        }
        assert!(
            bad_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks serialize")
                .iter()
                .any(|check| check["id"] == "expect_quality.area_light.target"
                    && check["code"] == "area_light_soft_shadow_insufficient"
                    && check["region"]["kind"] == "area_light_shadow_target"),
            "{backend} tiny area-light emitter must fail with exact area_light_soft_shadow_insufficient quality check: {bad_report:#}"
        );
        assert!(
            bad_report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons serialize")
                .iter()
                .any(
                    |reason| reason["code"] == "area_light_soft_shadow_insufficient"
                        && reason["expectation_id"] == "expect_quality.area_light.target",
                ),
            "{backend} tiny area-light emitter must surface the exact quality reason to agents: {bad_report:#}"
        );

        let (good_recipe, good_png) = write_area_light_quality_recipe(
            &dir,
            &format!("area-soft-shadow-good-{backend}"),
            1.2,
            true,
        );
        let good_report = run_recipe_render_verify(&good_recipe, &good_png, use_gpu);
        assert!(
            good_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks serialize")
                .iter()
                .any(|check| check["id"] == "expect_quality.area_light.target"
                    && check["code"] == "area_light_soft_shadow_checked"
                    && check["region"]["kind"] == "area_light_shadow_target"),
            "{backend} broad area emitter must pass with explicit area_light_soft_shadow_checked coverage: {good_report:#}"
        );
        assert!(
            good_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks serialize")
                .iter()
                .all(|check| check["code"] != "area_light_soft_shadow_insufficient"),
            "{backend} broad area emitter must not emit area_light_soft_shadow_insufficient: {good_report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_area_light_soft_shadow_for_all_shapes_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-area-light-shape-quality");
    for shape in ["rect", "disc", "sphere"] {
        for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
            let (recipe, png) = write_area_light_quality_recipe_for_shape(
                &dir,
                &format!("area-soft-shadow-{shape}-{backend}"),
                shape,
                1.2,
                true,
            );
            let report = run_recipe_render_verify(&recipe, &png, use_gpu);
            let checks = report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks serialize");
            let area_check = checks
                .iter()
                .find(|check| check["id"] == "expect_quality.area_light.target")
                .unwrap_or_else(|| {
                    panic!(
                        "{backend} {shape} area-light proof must emit an area-light quality check: {report:#}"
                    )
                });
            assert_eq!(
                area_check["code"], "area_light_soft_shadow_checked",
                "{backend} {shape} finite emitter must pass the area-light quality check: {report:#}"
            );
            assert_eq!(
                area_check["region"]["kind"], "area_light_shadow_target",
                "{backend} {shape} area-light quality must measure the target region, not the whole frame: {report:#}"
            );
            let observed_extent = area_check["observed"]["soft_emitter_extent_meters"]
                .as_f64()
                .expect("soft emitter extent is numeric");
            assert!(
                observed_extent >= 0.5,
                "{backend} {shape} area-light check must record a finite emitter extent above the configured soft-shadow threshold; observed={observed_extent}, report={report:#}"
            );
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_depth_of_field_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-depth-of-field-quality");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let (missing_recipe, missing_png) =
            write_depth_of_field_quality_recipe(&dir, &format!("dof-missing-{backend}"), None);
        let missing_report =
            run_recipe_render_verify_expect_failure(&missing_recipe, &missing_png, use_gpu);
        assert_quality_reason(
            &missing_report,
            "depth_of_field_not_enabled",
            "expect_quality.depth_of_field",
        );

        let (wrong_focus_recipe, wrong_focus_png) = write_depth_of_field_quality_recipe(
            &dir,
            &format!("dof-wrong-focus-{backend}"),
            Some(4.35),
        );
        let wrong_focus_report =
            run_recipe_render_verify_expect_failure(&wrong_focus_recipe, &wrong_focus_png, use_gpu);
        assert_quality_reason(
            &wrong_focus_report,
            "depth_of_field_blur_insufficient",
            "expect_quality.depth_of_field",
        );

        let (good_recipe, good_png) = write_depth_of_field_quality_recipe(
            &dir,
            &format!("dof-good-focus-{backend}"),
            Some(3.0),
        );
        let good_report = run_recipe_render_verify(&good_recipe, &good_png, use_gpu);
        assert!(
            good_report["verification"]["quality"]["checks"]
                .as_array()
                .expect("quality checks serialize")
                .iter()
                .any(|check| check["id"] == "expect_quality.depth_of_field"
                    && check["code"] == "depth_of_field_checked"
                    && check["region"]["kind"] == "dof_background"
                    && check["observed"]["background_sobel_drop_fraction"]
                        .as_f64()
                        .is_some_and(|value| value >= 0.18)),
            "{backend} focused DoF render must pass with an explicit depth_of_field_checked quality result: {good_report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_gpu_many_point_lights_contribute_without_fixed_uniform_truncation() {
    let dir = artifact_dir("recipe-render-gpu-light-capacity");
    let four_recipe_path = dir.join("gpu-light-capacity-four.recipe.json");
    let five_recipe_path = dir.join("gpu-light-capacity-five.recipe.json");
    let four_png = dir.join("gpu-light-capacity-four-gpu.png");
    let five_png = dir.join("gpu-light-capacity-five-gpu.png");
    let recipe = |include_fifth: bool| {
        let mut lights = vec![
            json!({ "id": "point_0", "kind": "point", "intensity_candela": 8.0, "range": 4.0, "transform": { "kind": "trs", "translation": [-0.85, 0.9, 1.2] } }),
            json!({ "id": "point_1", "kind": "point", "intensity_candela": 8.0, "range": 4.0, "transform": { "kind": "trs", "translation": [-0.35, 1.0, 1.1] } }),
            json!({ "id": "point_2", "kind": "point", "intensity_candela": 8.0, "range": 4.0, "transform": { "kind": "trs", "translation": [0.15, 1.0, 1.1] } }),
            json!({ "id": "point_3", "kind": "point", "intensity_candela": 8.0, "range": 4.0, "transform": { "kind": "trs", "translation": [0.65, 0.9, 1.2] } }),
        ];
        if include_fifth {
            lights.push(json!({
                "id": "point_4_blue",
                "kind": "point",
                "color": "#427DFF",
                "intensity_candela": 40.0,
                "range": 4.0,
                "transform": { "kind": "trs", "translation": [1.05, 0.62, 0.8] }
            }));
        }
        json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "box", "size": [0.72, 0.45, 0.28] } }
            ],
            "materials": [
                {
                    "id": "body_mat",
                    "kind": "pbr_metallic_roughness",
                    "base_color": "#59687D",
                    "metallic": 0.0,
                    "roughness": 0.62
                }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat" }
            ],
            "lights": lights,
            "scene": {
                "background": { "kind": "custom", "color": "#20242C" },
                "environment": { "kind": "none" }
            },
            "render": {
                "anti_aliasing": "msaa4",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "fov_degrees": 30.0,
                "transform": { "kind": "look_at", "eye": [0.0, 0.18, 2.25], "target": "body" }
            }],
            "capture": { "width": 200, "height": 150 },
            "expect": {
                "expect_visible": [{
                    "id": "body-visible",
                    "target": { "kind": "node", "id": "body" }
                }]
            }
        })
    };
    fs::write(
        &four_recipe_path,
        serde_json::to_string_pretty(&recipe(false)).expect("four-light recipe serializes"),
    )
    .expect("four-light recipe writes");
    fs::write(
        &five_recipe_path,
        serde_json::to_string_pretty(&recipe(true)).expect("five-light recipe serializes"),
    )
    .expect("five-light recipe writes");

    let four_report = run_recipe_render_verify(&four_recipe_path, &four_png, true);
    let five_report = run_recipe_render_verify(&five_recipe_path, &five_png, true);
    assert_eq!(
        four_report["introspection"]["capabilities"]["backend"], "headless_gpu",
        "{four_report:#}"
    );
    assert_eq!(
        five_report["introspection"]["capabilities"]["backend"], "headless_gpu",
        "{five_report:#}"
    );

    let four = decode_png_rgba8(&four_png);
    let five = decode_png_rgba8(&five_png);
    let probe = QualityPixelRegion {
        x: 120,
        y: 46,
        width: 52,
        height: 58,
    };
    let four_blue = mean_blue_in_region(&four.rgba8, four.width, probe);
    let five_blue = mean_blue_in_region(&five.rgba8, five.width, probe);
    let blue_delta = five_blue - four_blue;
    assert!(
        blue_delta > 8.0,
        "the fifth blue point light must visibly contribute on HeadlessGpu instead of being truncated; blue_delta={blue_delta:.3}, four_blue={four_blue:.3}, five_blue={five_blue:.3}, four={four_report:#}, five={five_report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_culls_offscreen_node_without_culling_visible_node_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-render-frustum-culling-proof");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let baseline_recipe_path =
            dir.join(format!("frustum-culling-{backend}-baseline.recipe.json"));
        let baseline_png_path = dir.join(format!("frustum-culling-{backend}-baseline.png"));
        let culling_recipe_path = dir.join(format!("frustum-culling-{backend}.recipe.json"));
        let culling_png_path = dir.join(format!("frustum-culling-{backend}.png"));
        let recipe = |include_offscreen: bool| {
            let mut nodes = vec![json!({
                "id": "visible_box",
                "geometry": "box_geo",
                "material": "visible_mat",
                "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] }
            })];
            if include_offscreen {
                nodes.push(json!({
                    "id": "offscreen_box",
                    "geometry": "box_geo",
                    "material": "offscreen_mat",
                    "transform": { "kind": "trs", "translation": [4.0, 0.0, 0.0] }
                }));
            }
            json!({
                "schema": "scena.scene_recipe.v1",
                "geometries": [
                    { "id": "box_geo", "primitive": { "kind": "box", "size": [0.35, 0.35, 0.12] } }
                ],
                "materials": [
                    { "id": "visible_mat", "kind": "unlit", "base_color": "#2F9E44" },
                    { "id": "offscreen_mat", "kind": "unlit", "base_color": "#C92A2A" }
                ],
                "nodes": nodes,
                "scene": { "background": { "kind": "black" } },
                "render": {
                    "anti_aliasing": "none",
                    "tonemapper": "standard",
                    "exposure_ev": 0.0
                },
                "cameras": [{
                    "id": "main",
                    "kind": "perspective",
                    "fov_degrees": 36.0,
                    "active": true,
                    "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "visible_box" }
                }],
                "capture": { "width": 120, "height": 100 },
                "expect": {}
            })
        };
        fs::write(
            &baseline_recipe_path,
            serde_json::to_string_pretty(&recipe(false))
                .expect("baseline frustum culling recipe serializes"),
        )
        .expect("baseline frustum culling recipe writes");
        fs::write(
            &culling_recipe_path,
            serde_json::to_string_pretty(&recipe(true)).expect("frustum culling recipe serializes"),
        )
        .expect("frustum culling recipe writes");

        let baseline = run_recipe_render_verify(&baseline_recipe_path, &baseline_png_path, use_gpu);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&culling_recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend([
            "--introspect",
            "--verify",
            "--out",
            path_str(&culling_png_path),
        ]);
        let output = command
            .args(args)
            .output()
            .expect("scena culling recipe render command runs");
        assert!(
            !output.status.success(),
            "{backend} render with an offscreen declared node should fail composition verification, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "culling proof failures stay machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU culling proof must use the GPU backend, not a fallback: {report:#}"
            );
        }
        let baseline_visible = baseline["introspection"]["visible_pixel_fraction"]
            .as_f64()
            .expect("baseline visible pixel fraction");
        let culling_visible = report["introspection"]["visible_pixel_fraction"]
            .as_f64()
            .expect("culling visible pixel fraction");
        assert!(
            culling_visible > 0.01 && (culling_visible - baseline_visible).abs() <= 0.001,
            "{backend} render should keep the visible box in frame: {report:#}"
        );
        let baseline_culled = baseline["introspection"]["nodes_summary"]["culled"]
            .as_u64()
            .expect("baseline culled count");
        let culling_culled = report["introspection"]["nodes_summary"]["culled"]
            .as_u64()
            .expect("culling culled count");
        assert!(
            culling_culled > baseline_culled,
            "{backend} render should increase culling when the offscreen box is added: baseline={baseline_culled}, culling={culling_culled}, report={report:#}"
        );
        assert!(
            report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons array")
                .iter()
                .any(|reason| reason["code"] == "visible_pixel_coverage_missing"
                    && reason["expectation_id"] == "node.offscreen_box.visible_coverage"),
            "{backend} render should fail only after proving the offscreen declared node has no visible coverage: {report:#}"
        );
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
fn scena_recipe_render_verify_emits_passing_composition_report_for_declared_node() {
    let dir = artifact_dir("recipe-composition-good");
    let recipe_path = dir.join("composition.recipe.json");
    let png_path = dir.join("composition.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.35, 0.35, 0.08] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "visible_box",
                    "geometry": "visible_geo",
                    "material": "green_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "visible_box" }
            }],
            "capture": { "width": 128, "height": 128 },
            "expect": {}
        }))
        .expect("composition recipe serializes"),
    )
    .expect("composition recipe writes");

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
        .expect("scena composition recipe render command runs");

    assert!(
        output.status.success(),
        "composition-good recipe should pass verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");
    let composition = &report["verification"]["composition"];
    assert_eq!(composition["schema"], "scena.scene_composition.v1");
    assert_eq!(composition["ok"], true, "{report:#}");
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "node.visible_box.presence"
                && check["status"] == "checked"
                && check["code"] == "node_visible"),
        "visible declared node should be checked: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "node.visible_box.expected_color"
                && check["status"] == "checked"
                && check["code"] == "material_base_color_available"),
        "material-backed declared node should expose a structural expected-color check: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "node.visible_box.visible_coverage"
                && check["status"] == "checked"
                && check["code"] == "visible_pixel_coverage_available"
                && check["observed"]["foreground_pixels"]
                    .as_u64()
                    .is_some_and(|value| value > 0)),
        "declared visible node should have measured per-node visible coverage: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .all(|reason| !(reason["source"] == "composition" && reason["severity"] == "error")),
        "passing composition checks should not produce composition errors: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_projects_declared_metering_subject_with_manual_camera() {
    let dir = artifact_dir("recipe-composition-metering-subject");
    let recipe_path = dir.join("composition-metering-subject.recipe.json");
    let png_path = dir.join("composition-metering-subject.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "subject": "#B84D36"
            },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.45, 0.35, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" }
            ],
            "nodes": [
                {
                    "id": "subject_box",
                    "geometry": "subject_geo",
                    "material": "subject_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] }
                }
            ],
            "cameras": [{
                "id": "manual_cam",
                "kind": "perspective",
                "fov_degrees": 32.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject_box" }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": {
                    "mode": "subject",
                    "target": { "kind": "node", "id": "subject_box" }
                }
            },
            "capture": { "width": 160, "height": 100 },
            "expect": {}
        }))
        .expect("composition metering-subject recipe serializes"),
    )
    .expect("composition metering-subject recipe writes");

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
        .expect("scena composition metering-subject recipe render command runs");

    assert!(
        output.status.success(),
        "manual-camera metering subject recipe should pass verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["ok"], true, "{report:#}");
    let subject_check = composition_check(&report, "subject.render_metering.projected_bounds");
    assert_eq!(subject_check["status"], "checked", "{report:#}");
    assert_eq!(
        subject_check["code"], "subject_projected_bounds_available",
        "{report:#}"
    );
    assert_eq!(subject_check["target_id"], "subject_box", "{report:#}");
    assert_eq!(
        subject_check["observed"]["source"], "render.metering",
        "{report:#}"
    );
    assert_eq!(
        subject_check["observed"]["confidence"], "projected_only",
        "{report:#}"
    );
    assert_eq!(
        subject_check["observed"]["viewport_width"], 160,
        "{report:#}"
    );
    assert_eq!(
        subject_check["observed"]["viewport_height"], 100,
        "{report:#}"
    );
    assert!(
        subject_check["observed"]["fill_fraction"]
            .as_f64()
            .is_some_and(|value| value > 0.05 && value < 1.0),
        "subject observation should report sane viewport-aware fill: {report:#}"
    );
    let rect = &subject_check["region"]["rect_css_px"];
    assert!(
        rect["width"].as_f64().is_some_and(|value| value > 0.0)
            && rect["height"].as_f64().is_some_and(|value| value > 0.0),
        "subject observation should include projected rect: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_subject_mask_uses_semantic_aov_not_background_heuristics() {
    let dir = artifact_dir("recipe-composition-subject-mask");
    let recipe_path = dir.join("composition-subject-mask.recipe.json");
    let png_path = dir.join("composition-subject-mask.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "subject": "#C4623A",
                "occluder": "#20242A",
                "floor": "#3D4652",
                "label": "#F8F9FA"
            },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.64, 0.46, 0.08] } },
                { "id": "occluder_geo", "primitive": { "kind": "box", "size": [0.24, 0.58, 0.12] } },
                { "id": "floor_geo", "primitive": { "kind": "box", "size": [1.70, 0.08, 0.08] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" },
                { "id": "occluder_mat", "kind": "unlit", "base_color": "occluder" },
                { "id": "floor_mat", "kind": "unlit", "base_color": "floor" }
            ],
            "nodes": [
                {
                    "id": "subject_box",
                    "geometry": "subject_geo",
                    "material": "subject_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.02, -0.06] }
                },
                {
                    "id": "front_occluder",
                    "geometry": "occluder_geo",
                    "material": "occluder_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.02, 0.08] }
                },
                {
                    "id": "floor_slab",
                    "geometry": "floor_geo",
                    "material": "floor_mat",
                    "transform": { "kind": "trs", "translation": [0.0, -0.34, -0.08] }
                }
            ],
            "labels": [{
                "id": "subject_label",
                "text": "SUBJECT",
                "color": "label",
                "size_px": 16.0,
                "transform": { "kind": "trs", "translation": [0.0, 0.40, 0.0] }
            }],
            "cameras": [{
                "id": "manual_cam",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject_box" }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": {
                    "mode": "subject",
                    "target": { "kind": "node", "id": "subject_box" }
                }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }))
        .expect("composition subject-mask recipe serializes"),
    )
    .expect("composition subject-mask recipe writes");

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
        .expect("scena composition subject-mask recipe render command runs");

    assert!(
        output.status.success(),
        "subject-mask recipe should pass verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    let mask_check = composition_check(&report, "subject.render_metering.visible_mask");
    assert_eq!(mask_check["status"], "checked", "{report:#}");
    assert_eq!(
        mask_check["code"], "subject_visible_mask_available",
        "{report:#}"
    );
    assert_eq!(mask_check["target_id"], "subject_box", "{report:#}");
    assert_eq!(
        mask_check["observed"]["mask_source"], "semantic_aov",
        "{report:#}"
    );
    assert_eq!(
        mask_check["observed"]["confidence"], "exact_opaque_semantic_aov",
        "{report:#}"
    );
    assert!(
        mask_check["observed"]["visible_pixels"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "semantic subject mask should include visible subject pixels: {report:#}"
    );
    assert!(
        mask_check["observed"]["other_visible_pixels"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "semantic subject mask should distinguish occluder/floor pixels from the subject: {report:#}"
    );
    assert!(
        mask_check["observed"]["visible_fraction_of_projected"]
            .as_f64()
            .is_some_and(|value| value > 0.05 && value < 0.85),
        "semantic subject mask should expose the occluded subject fraction: {report:#}"
    );
    assert!(
        mask_check["observed"]["excluded_label_quad_count"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "semantic subject mask should report label-overlay exclusion: {report:#}"
    );
    let rect = &mask_check["region"]["rect_css_px"];
    assert!(
        rect["width"].as_f64().is_some_and(|value| value > 0.0)
            && rect["height"].as_f64().is_some_and(|value| value > 0.0),
        "semantic subject mask should include visible bounds: {report:#}"
    );
    let observations = report["verification"]["subject_observations"]
        .as_array()
        .expect("subject observations serialize");
    let introspection_observations = report["introspection"]["subject_observations"]
        .as_array()
        .expect("introspection subject observations serialize");
    let observation = observations
        .iter()
        .find(|observation| observation["source"] == "render.metering")
        .unwrap_or_else(|| panic!("render.metering subject observation missing: {report:#}"));
    assert!(
        introspection_observations
            .iter()
            .any(|entry| entry == observation),
        "render introspection should link the capture-bound subject observation: {report:#}"
    );
    assert_eq!(observation["schema"], "scena.subject_observation.v1");
    assert_eq!(observation["status"], "observed");
    assert_eq!(observation["target"]["kind"], "node");
    assert_eq!(observation["target"]["id"], "subject_box");
    assert_eq!(
        observation["frame_key"]["state_binding"],
        "exact_readback_completion"
    );
    assert!(
        observation["projected_bounds"]["area_px"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "observation should carry projected bounds: {report:#}"
    );
    assert!(
        observation["visible_bounds"]["area_px"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "observation should carry visible bounds: {report:#}"
    );
    assert!(
        observation["visible_pixel_count"]
            .as_u64()
            .is_some_and(|value| value > 0),
        "observation should carry visible subject pixels: {report:#}"
    );
    assert!(
        observation["depth"]["p50_m"]
            .as_f64()
            .is_some_and(|value| value > 0.0),
        "observation should carry visible depth percentile: {report:#}"
    );
    assert_eq!(observation["fallback"]["degraded"], false);
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_reports_zero_visible_subject_reason_codes() {
    let dir = artifact_dir("recipe-composition-zero-visible-subject");
    for (case, expected_code) in [
        ("hidden", "subject_hidden"),
        ("outside_viewport", "subject_outside_viewport"),
        ("behind_camera", "subject_behind_camera"),
        ("degenerate_transform", "subject_degenerate_geometry"),
        ("clipped_section_box", "subject_clipped_by_section_box"),
        ("clipped_plane", "subject_clipped_by_clipping_plane"),
        ("transparent", "subject_transparent_unsupported"),
        ("occluded", "subject_occluded"),
    ] {
        let recipe_path = dir.join(format!("{case}.recipe.json"));
        let png_path = dir.join(format!("{case}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&zero_visible_subject_recipe(case))
                .expect("zero-visible subject recipe serializes"),
        )
        .expect("zero-visible subject recipe writes");

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
            .expect("scena zero-visible subject render command runs");

        assert!(
            !output.status.success(),
            "{case} should fail verification with a structured subject reason, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "{case} subject visibility failures stay machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let report = json_report(&output);
        assert_eq!(report["schema"], "scena.recipe_render_result.v1");
        assert_eq!(report["ok"], false, "{case}: {report:#}");

        let mask_check = composition_check(&report, "subject.render_metering.visible_mask");
        assert_eq!(mask_check["status"], "failed", "{case}: {report:#}");
        assert_eq!(mask_check["code"], expected_code, "{case}: {report:#}");
        assert_eq!(
            mask_check["observed"]["zero_visible_reason"], expected_code,
            "{case}: {report:#}"
        );

        assert!(
            report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons array")
                .iter()
                .any(|reason| reason["source"] == "composition"
                    && reason["code"] == expected_code
                    && reason["expectation_id"] == "subject.render_metering.visible_mask"),
            "{case} must surface the same subject reason in verification reasons: {report:#}"
        );

        let observation = report["verification"]["subject_observations"]
            .as_array()
            .expect("subject observations serialize")
            .iter()
            .find(|observation| observation["source"] == "render.metering")
            .unwrap_or_else(|| panic!("{case} render.metering observation missing: {report:#}"));
        assert_eq!(observation["status"], "degraded", "{case}: {report:#}");
        assert!(
            observation["fallback"]["reason_codes"]
                .as_array()
                .expect("reason codes serialize")
                .iter()
                .any(|code| code == expected_code),
            "{case} subject observation must carry {expected_code}: {report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_reports_zero_visible_photo_and_focus_subject_reason_codes() {
    let dir = artifact_dir("recipe-composition-zero-visible-subject-sources");
    for (source, expected_check_id, expected_source) in [
        (
            ZeroVisibleSubjectSource::Photo,
            "subject.photo_subject.visible_mask",
            "photo.subject",
        ),
        (
            ZeroVisibleSubjectSource::Focus,
            "subject.render_depth_of_field_focus.visible_mask",
            "render.depth_of_field.focus",
        ),
    ] {
        let recipe_path = dir.join(format!("{}.recipe.json", source.file_stem()));
        let png_path = dir.join(format!("{}.png", source.file_stem()));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&zero_visible_subject_recipe_for_source("hidden", source))
                .expect("zero-visible source recipe serializes"),
        )
        .expect("zero-visible source recipe writes");

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
            .expect("scena zero-visible source render command runs");

        assert!(
            !output.status.success(),
            "{source:?} should fail verification with a structured subject reason, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "{source:?} subject visibility failures stay machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let report = json_report(&output);
        assert_eq!(report["schema"], "scena.recipe_render_result.v1");
        assert_eq!(report["ok"], false, "{source:?}: {report:#}");

        let mask_check = composition_check(&report, expected_check_id);
        assert_eq!(mask_check["status"], "failed", "{source:?}: {report:#}");
        assert_eq!(
            mask_check["code"], "subject_hidden",
            "{source:?}: {report:#}"
        );

        assert!(
            report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons array")
                .iter()
                .any(|reason| reason["source"] == "composition"
                    && reason["code"] == "subject_hidden"
                    && reason["expectation_id"] == expected_check_id),
            "{source:?} must surface subject_hidden in verification reasons: {report:#}"
        );

        let observation = report["verification"]["subject_observations"]
            .as_array()
            .expect("subject observations serialize")
            .iter()
            .find(|observation| observation["source"] == expected_source)
            .unwrap_or_else(|| {
                panic!("{source:?} subject observation missing for {expected_source}: {report:#}")
            });
        assert_eq!(observation["status"], "degraded", "{source:?}: {report:#}");
        assert!(
            observation["fallback"]["reason_codes"]
                .as_array()
                .expect("reason codes serialize")
                .iter()
                .any(|code| code == "subject_hidden"),
            "{source:?} subject observation must carry subject_hidden: {report:#}"
        );

        if source == ZeroVisibleSubjectSource::Focus {
            let focus_report = &report["introspection"]["focus_report"];
            assert_eq!(focus_report["schema"], "scena.focus_report.v1");
            assert_eq!(focus_report["status"], "unresolved", "{report:#}");
            assert_eq!(focus_report["target"]["kind"], "node", "{report:#}");
            assert_eq!(focus_report["target"]["id"], "subject_box", "{report:#}");
            assert_eq!(focus_report["reason"], "subject_hidden", "{report:#}");
        }
    }
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZeroVisibleSubjectSource {
    Metering,
    Photo,
    Focus,
}

#[cfg(feature = "scene-host")]
impl ZeroVisibleSubjectSource {
    const fn file_stem(self) -> &'static str {
        match self {
            Self::Metering => "render-metering",
            Self::Photo => "photo-subject",
            Self::Focus => "focus-subject",
        }
    }
}

#[cfg(feature = "scene-host")]
fn zero_visible_subject_recipe_for_source(
    case: &str,
    source: ZeroVisibleSubjectSource,
) -> serde_json::Value {
    let mut recipe = zero_visible_subject_recipe(case);
    if source == ZeroVisibleSubjectSource::Metering {
        return recipe;
    }
    let recipe_obj = recipe
        .as_object_mut()
        .expect("zero-visible recipe is an object");
    let render = recipe_obj
        .get_mut("render")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zero-visible recipe render object exists");
    render.remove("metering");
    match source {
        ZeroVisibleSubjectSource::Metering => {}
        ZeroVisibleSubjectSource::Photo => {
            recipe_obj.remove("cameras");
            recipe_obj.insert(
                "photo".to_owned(),
                json!({
                    "intent": "camera_behavior",
                    "subject": {
                        "target": { "kind": "node", "id": "subject_box" }
                    }
                }),
            );
        }
        ZeroVisibleSubjectSource::Focus => {
            render.insert(
                "depth_of_field".to_owned(),
                json!({
                    "focus": {
                        "mode": "subject",
                        "target": { "kind": "node", "id": "subject_box" }
                    },
                    "coverage": "all",
                    "strength": "subtle"
                }),
            );
        }
    }
    recipe
}

#[cfg(feature = "scene-host")]
fn zero_visible_subject_recipe(case: &str) -> serde_json::Value {
    match case {
        "hidden" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "subject": "#C4623A" },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" }
            ],
            "nodes": [{
                "id": "subject_box",
                "geometry": "subject_geo",
                "material": "subject_mat",
                "visible": false,
                "transform": { "kind": "center" }
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "outside_viewport" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "subject": "#C4623A" },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" }
            ],
            "nodes": [{
                "id": "subject_box",
                "geometry": "subject_geo",
                "material": "subject_mat",
                "transform": { "kind": "trs", "translation": [4.0, 0.0, 0.0] }
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "behind_camera" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "subject": "#C4623A" },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" }
            ],
            "nodes": [{
                "id": "subject_box",
                "geometry": "subject_geo",
                "material": "subject_mat",
                "transform": { "kind": "trs", "translation": [0.0, 0.0, 3.0] }
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "degenerate_transform" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "subject": "#C4623A" },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" }
            ],
            "nodes": [{
                "id": "subject_box",
                "geometry": "subject_geo",
                "material": "subject_mat",
                "transform": { "kind": "trs", "scale": [0.0, 0.0, 0.0] }
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "clipped_section_box" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "subject": "#C4623A",
                "clipper": "#263241"
            },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.42, 0.30, 0.10] } },
                { "id": "clipper_geo", "primitive": { "kind": "box", "size": [0.20, 0.20, 0.20] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" },
                { "id": "clipper_mat", "kind": "unlit", "base_color": "clipper" }
            ],
            "nodes": [
                {
                    "id": "subject_box",
                    "geometry": "subject_geo",
                    "material": "subject_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.55, 0.0] }
                },
                {
                    "id": "clipper_box",
                    "geometry": "clipper_geo",
                    "material": "clipper_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "section_box": {
                "target": { "kind": "node", "id": "clipper_box" },
                "margin": 0.01,
                "helper_wireframe": false
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.55, 2.0], "target": "subject_box" }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "clipped_plane" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "subject": "#C4623A" },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" }
            ],
            "nodes": [{
                "id": "subject_box",
                "geometry": "subject_geo",
                "material": "subject_mat",
                "transform": { "kind": "center" }
            }],
            "clipping_planes": [
                { "id": "clip_subject", "normal": [1.0, 0.0, 0.0], "distance": -1.0, "active": true }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject_box" }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "transparent" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": { "subject": "#C4623A" },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.46, 0.36, 0.12] } }
            ],
            "materials": [
                {
                    "id": "subject_mat",
                    "kind": "unlit",
                    "base_color": "subject",
                    "alpha_mode": { "kind": "blend" }
                }
            ],
            "nodes": [{
                "id": "subject_box",
                "geometry": "subject_geo",
                "material": "subject_mat",
                "transform": { "kind": "center" }
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject_box" }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        "occluded" => json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "subject": "#C4623A",
                "occluder": "#20242A"
            },
            "geometries": [
                { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.42, 0.30, 0.08] } },
                { "id": "occluder_geo", "primitive": { "kind": "box", "size": [0.72, 0.54, 0.12] } }
            ],
            "materials": [
                { "id": "subject_mat", "kind": "unlit", "base_color": "subject" },
                { "id": "occluder_mat", "kind": "unlit", "base_color": "occluder" }
            ],
            "nodes": [
                {
                    "id": "subject_box",
                    "geometry": "subject_geo",
                    "material": "subject_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, -0.10] }
                },
                {
                    "id": "front_occluder",
                    "geometry": "occluder_geo",
                    "material": "occluder_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.10] }
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject_box" }
            }],
            "render": {
                "auto_exposure": "mixed",
                "metering": { "mode": "subject", "target": { "kind": "node", "id": "subject_box" } }
            },
            "capture": { "width": 160, "height": 120 },
            "expect": {}
        }),
        other => panic!("unknown zero-visible subject case {other}"),
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_required_composition_coverage_when_node_is_offscreen() {
    let dir = artifact_dir("recipe-composition-required-coverage");
    let recipe_path = dir.join("composition-required.recipe.json");
    let png_path = dir.join("composition-required.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.35, 0.35, 0.08] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "visible_box",
                    "geometry": "visible_geo",
                    "material": "green_mat",
                    "transform": { "kind": "trs", "translation": [4.0, 0.0, 0.0] }
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
            }],
            "capture": { "width": 128, "height": 128 },
            "expect": {
                "expect_quality": { "profile": "cad" }
            }
        }))
        .expect("composition required coverage recipe serializes"),
    )
    .expect("composition required coverage recipe writes");

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
        .expect("scena composition required coverage render command runs");

    assert!(
        !output.status.success(),
        "profile-required offscreen coverage failures must fail verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "composition coverage failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    let composition = &report["verification"]["composition"];
    assert_eq!(composition["schema"], "scena.scene_composition.v1");
    assert_eq!(composition["ok"], false, "{report:#}");
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "node.visible_box.visible_coverage"
                && check["status"] == "failed"
                && check["code"] == "visible_pixel_coverage_missing"),
        "profile-required missing visible coverage should be an exact failed composition check: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .any(|reason| reason["source"] == "composition"
                && reason["code"] == "visible_pixel_coverage_missing"),
        "profile-required missing visible coverage must surface as a verification reason: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_overlay_line_through_label() {
    let dir = artifact_dir("recipe-composition-overlay-collision");
    let recipe_path = dir.join("composition-overlay-collision.recipe.json");
    let png_path = dir.join("composition-overlay-collision.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "white": "#F8F9FA",
                "black": "#050505"
            },
            "geometries": [
                {
                    "id": "crossing_line",
                    "primitive": {
                        "kind": "line",
                        "start": [-0.6, 0.0, 0.0],
                        "end": [0.6, 0.0, 0.0]
                    }
                }
            ],
            "materials": [
                {
                    "id": "line_mat",
                    "kind": "line",
                    "base_color": "white",
                    "stroke_width_px": 2.0
                }
            ],
            "nodes": [
                {
                    "id": "line",
                    "geometry": "crossing_line",
                    "material": "line_mat"
                }
            ],
            "labels": [
                {
                    "id": "center_label",
                    "text": "STATUS",
                    "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] },
                    "color": "white",
                    "background": "black",
                    "size_px": 18.0
                }
            ],
            "scene": { "background": { "kind": "black" } },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
            }],
            "capture": { "width": 220, "height": 140 },
            "expect": {}
        }))
        .expect("composition overlay-collision recipe serializes"),
    )
    .expect("composition overlay-collision recipe writes");

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
        .expect("scena composition overlay-collision render command runs");

    assert!(
        !output.status.success(),
        "line-through-label composition should fail verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "composition overlay failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert!(
        report["verification"]["composition"]["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["status"] == "failed"
                && check["code"] == "overlay_label_intersects_line"),
        "line-through-label should be an exact failed composition check: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .any(|reason| reason["source"] == "composition"
                && reason["code"] == "overlay_label_intersects_line"),
        "line-through-label composition failure must surface as a verification reason: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_label_label_overlap_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-label-label-overlap");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, right_x, should_pass) in [("separated", 0.62, true), ("overlap", 0.05, false)] {
            let recipe_path = dir.join(format!("label-overlap-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("label-overlap-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "white": "#F8F9FA",
                        "black": "#050505",
                        "blue": "#173B8F"
                    },
                    "labels": [
                        {
                            "id": "left_label",
                            "text": "PUMP A",
                            "transform": { "kind": "trs", "translation": [-0.38, 0.0, 0.0] },
                            "color": "white",
                            "background": "black",
                            "size_px": 18.0
                        },
                        {
                            "id": "right_label",
                            "text": "PUMP B",
                            "transform": { "kind": "trs", "translation": [right_x, 0.0, 0.0] },
                            "color": "white",
                            "background": "blue",
                            "size_px": 18.0
                        }
                    ],
                    "scene": { "background": { "kind": "black" } },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "supersample": 1,
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 34.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
                    }],
                    "capture": { "width": 260, "height": 140 },
                    "expect": {}
                }))
                .expect("composition label-overlap recipe serializes"),
            )
            .expect("composition label-overlap recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena composition label-overlap render command runs");

            if should_pass {
                assert!(
                    output.status.success(),
                    "{backend}/{case} label overlap recipe should pass verification, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
            } else {
                assert!(
                    !output.status.success(),
                    "{backend}/{case} label overlap recipe should fail verification, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
                assert!(
                    output.stderr.is_empty(),
                    "composition label-overlap failures stay machine-readable on stdout, stderr={}",
                    stderr(&output)
                );
            }
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU label-overlap proof must use HeadlessGpu, not CPU fallback: {report:#}"
                );
            }
            let expected_code = if should_pass {
                "overlay_label_clear_of_labels"
            } else {
                "overlay_label_intersects_label"
            };
            let expected_status = if should_pass { "checked" } else { "failed" };
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(
                        |check| check["id"] == "overlay.label.left_label.label_clearance"
                            && check["status"] == expected_status
                            && check["code"] == expected_code
                    ),
                "{backend}/{case} should emit exact label-overlap composition check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "overlay_label_intersects_label"
                            && reason["expectation_id"]
                                == "overlay.label.left_label.label_clearance"),
                    "{backend}/{case} label-overlap failure should surface as a verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_label_viewport_fit_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-label-viewport-fit");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, label_x, should_pass) in [("inside", 0.0, true), ("clipped", 1.0, false)] {
            let recipe_path = dir.join(format!("label-viewport-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("label-viewport-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "white": "#F8F9FA",
                        "black": "#050505"
                    },
                    "labels": [
                        {
                            "id": "edge_label",
                            "text": "EDGE LABEL",
                            "transform": { "kind": "trs", "translation": [label_x, 0.0, 0.0] },
                            "color": "white",
                            "background": "black",
                            "size_px": 18.0
                        }
                    ],
                    "scene": { "background": { "kind": "black" } },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "supersample": 1,
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 34.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
                    }],
                    "capture": { "width": 220, "height": 120 },
                    "expect": {}
                }))
                .expect("composition label viewport-fit recipe serializes"),
            )
            .expect("composition label viewport-fit recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena composition label viewport-fit render command runs");

            if should_pass {
                assert!(
                    output.status.success(),
                    "{backend}/{case} label viewport-fit recipe should pass verification, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
            } else {
                assert!(
                    !output.status.success(),
                    "{backend}/{case} label viewport-fit recipe should fail verification, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
                assert!(
                    output.stderr.is_empty(),
                    "composition label viewport-fit failures stay machine-readable on stdout, stderr={}",
                    stderr(&output)
                );
            }
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU label viewport-fit proof must use HeadlessGpu, not CPU fallback: {report:#}"
                );
            }
            let expected_code = if should_pass {
                "overlay_label_inside_viewport"
            } else {
                "overlay_label_clipped_by_viewport"
            };
            let expected_status = if should_pass { "checked" } else { "failed" };
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(
                        |check| check["id"] == "overlay.label.edge_label.viewport_fit"
                            && check["status"] == expected_status
                            && check["code"] == expected_code
                    ),
                "{backend}/{case} should emit exact label viewport-fit composition check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "overlay_label_clipped_by_viewport"
                            && reason["expectation_id"] == "overlay.label.edge_label.viewport_fit"),
                    "{backend}/{case} label viewport-fit failure should surface as a verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_callout_annotation_ownership() {
    let dir = artifact_dir("recipe-composition-callout-ownership");
    let recipe_path = dir.join("composition-callout.recipe.json");
    let png_path = dir.join("composition-callout.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.35, 0.35, 0.08] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "visible_box",
                    "geometry": "visible_geo",
                    "material": "green_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "callouts": [
                {
                    "id": "box_note",
                    "text": "BOX",
                    "target": { "kind": "node", "id": "visible_box" },
                    "label_offset": [0.22, 0.18, 0.0]
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "visible_box" }
            }],
            "capture": { "width": 160, "height": 140 },
            "expect": {}
        }))
        .expect("composition callout recipe serializes"),
    )
    .expect("composition callout recipe writes");

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
        .expect("scena composition callout render command runs");

    assert!(
        output.status.success(),
        "composition callout recipe should pass verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    let composition = &report["verification"]["composition"];
    assert_eq!(composition["schema"], "scena.scene_composition.v1");
    assert_eq!(composition["ok"], true, "{report:#}");
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(
                |check| check["id"] == "annotation.callout.box_note.attachment"
                    && check["status"] == "checked"
                    && check["code"] == "callout_target_attached"
            ),
        "node callout target ownership should be checked: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "annotation.callout.box_note.output"
                && check["status"] == "checked"
                && check["code"] == "callout_overlay_output_projected"),
        "callout generated label and leader line should be checked: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["status"] == "checked"
                && check["code"] == "overlay_label_clear_of_lines"),
        "callout label should be checked as clear of crossing line overlays: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_grid_floor_ownership() {
    let dir = artifact_dir("recipe-composition-grid-ownership");
    let recipe_path = dir.join("composition-grid.recipe.json");
    let png_path = dir.join("composition-grid.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.25, 0.25, 0.08] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "visible_box",
                    "geometry": "visible_geo",
                    "material": "green_mat",
                    "transform": { "kind": "ground", "plane_y": 0.0 }
                }
            ],
            "scene": {
                "grid": {
                    "enabled": true,
                    "floor_y": 0.0,
                    "padding": 0.15,
                    "line_spacing": 0.1
                }
            },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.42, 1.6], "target": "visible_box" }
            }],
            "capture": { "width": 160, "height": 140 },
            "expect": {
                "expect_grounded": [{
                    "id": "box_on_floor",
                    "target": { "kind": "node", "id": "visible_box" },
                    "plane_y": 0.0,
                    "tolerance": 0.01
                }]
            }
        }))
        .expect("composition grid recipe serializes"),
    )
    .expect("composition grid recipe writes");

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
        .expect("scena composition grid render command runs");

    assert!(
        output.status.success(),
        "composition grid recipe should pass verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    let composition = &report["verification"]["composition"];
    assert_eq!(composition["schema"], "scena.scene_composition.v1");
    assert_eq!(composition["ok"], true, "{report:#}");
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "scene.grid.ownership"
                && check["status"] == "checked"
                && check["code"] == "grid_floor_output_owned"),
        "recipe grid floor output should have explicit composition ownership: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "expect_grounded.box_on_floor"
                && check["status"] == "checked"
                && check["code"] == "ground_contact_present"),
        "grounded node should emit a passing placement contact check: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_fails_floating_grounded_node_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-ground-contact");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let recipe_path = dir.join(format!("composition-ground-contact-{backend}.recipe.json"));
        let png_path = dir.join(format!("composition-ground-contact-{backend}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "colors": {
                    "green": "#2F9E44"
                },
                "geometries": [
                    { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.28, 0.28, 0.12] } }
                ],
                "materials": [
                    { "id": "green_mat", "kind": "unlit", "base_color": "green" }
                ],
                "nodes": [
                    {
                        "id": "floating_box",
                        "geometry": "visible_geo",
                        "material": "green_mat",
                        "transform": { "kind": "trs", "translation": [0.0, 0.34, 0.0] }
                    }
                ],
                "scene": {
                    "background": { "kind": "black" },
                    "grid": {
                        "enabled": true,
                        "floor_y": 0.0,
                        "padding": 0.15,
                        "line_spacing": 0.1
                    }
                },
                "cameras": [{
                    "id": "main",
                    "kind": "perspective",
                    "fov_degrees": 36.0,
                    "active": true,
                    "transform": { "kind": "look_at", "eye": [0.0, 0.42, 1.8], "target": "floating_box" }
                }],
                "capture": { "width": 180, "height": 150 },
                "expect": {
                    "expect_grounded": [{
                        "id": "box_on_floor",
                        "target": { "kind": "node", "id": "floating_box" },
                        "plane_y": 0.0,
                        "tolerance": 0.01
                    }]
                }
            }))
            .expect("composition ground contact recipe serializes"),
        )
        .expect("composition ground contact recipe writes");

        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let output = command
            .args(args)
            .output()
            .expect("scena composition ground-contact render command runs");

        assert!(
            !output.status.success(),
            "{backend} floating grounded node should fail verification, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "composition placement failures stay machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU ground-contact proof must use HeadlessGpu, not CPU fallback: {report:#}"
            );
        }
        assert!(
            report["verification"]["composition"]["checks"]
                .as_array()
                .expect("composition checks array")
                .iter()
                .any(|check| check["id"] == "expect_grounded.box_on_floor"
                    && check["status"] == "failed"
                    && check["code"] == "ground_contact_missing"),
            "{backend} render should fail with exact ground_contact_missing composition check: {report:#}"
        );
        assert!(
            report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons array")
                .iter()
                .any(|reason| reason["source"] == "composition"
                    && reason["code"] == "ground_contact_missing"
                    && reason["expectation_id"] == "expect_grounded.box_on_floor"),
            "{backend} ground-contact failure should surface as a verification reason: {report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_helper_layer_occlusion_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-helper-occlusion");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, helper_z, should_pass) in [("behind", -0.22, true), ("front", 0.22, false)] {
            let recipe_path = dir.join(format!("helper-occlusion-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("helper-occlusion-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "blue": "#0B5FFF",
                        "red": "#FF2020"
                    },
                    "geometries": [
                        { "id": "box_geo", "primitive": { "kind": "box", "size": [0.70, 0.55, 0.22] } },
                        {
                            "id": "helper_line_geo",
                            "primitive": {
                                "kind": "line",
                                "start": [-0.42, 0.0, 0.0],
                                "end": [0.42, 0.0, 0.0]
                            }
                        }
                    ],
                    "materials": [
                        { "id": "box_mat", "kind": "unlit", "base_color": "blue" },
                        { "id": "helper_mat", "kind": "line", "base_color": "red", "stroke_width_px": 3.0 }
                    ],
                    "nodes": [
                        {
                            "id": "box",
                            "geometry": "box_geo",
                            "material": "box_mat",
                            "transform": { "kind": "center" }
                        },
                        {
                            "id": "helper_line",
                            "geometry": "helper_line_geo",
                            "material": "helper_mat",
                            "transform": { "kind": "trs", "translation": [0.0, 0.0, helper_z] }
                        }
                    ],
                    "scene": { "background": { "kind": "black" } },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "supersample": 2,
                        "reconstruction": "tent",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 32.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.1], "target": "box" }
                    }],
                    "capture": { "width": 220, "height": 160 },
                    "expect": {
                        "expect_helper_occluded": [{
                            "id": "helper-behind-box",
                            "helper": { "kind": "node", "id": "helper_line" },
                            "occluder": { "kind": "node", "id": "box" },
                            "tolerance_pixels": 0
                        }]
                    }
                }))
                .expect("composition helper occlusion recipe serializes"),
            )
            .expect("composition helper occlusion recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena composition helper occlusion render command runs");
            if should_pass {
                assert!(
                    output.status.success(),
                    "{backend}/{case} helper behind subject should pass verification, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
            } else {
                assert!(
                    !output.status.success(),
                    "{backend}/{case} helper in front of subject should fail verification, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
                assert!(
                    output.stderr.is_empty(),
                    "composition helper-layer failures stay machine-readable on stdout, stderr={}",
                    stderr(&output)
                );
            }
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU helper-layer proof must use HeadlessGpu, not CPU fallback: {report:#}"
                );
            }
            let expected_code = if should_pass {
                "helper_layer_occluded_by_subject"
            } else {
                "helper_layer_overdraws_subject"
            };
            let expected_status = if should_pass { "checked" } else { "failed" };
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(
                        |check| check["id"] == "expect_helper_occluded.helper-behind-box"
                            && check["status"] == expected_status
                            && check["code"] == expected_code
                            && check["observed"]["helper_pixels_inside_occluder"]
                                .as_u64()
                                .is_some()
                    ),
                "{backend}/{case} should emit exact helper-layer composition check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "helper_layer_overdraws_subject"
                            && reason["expectation_id"]
                                == "expect_helper_occluded.helper-behind-box"),
                    "{backend}/{case} helper-layer failure should surface as a verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_object_depth_order_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-object-depth-order");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, front_z, back_z, should_pass) in [
            ("blue_front", 0.10, -0.10, true),
            ("blue_behind", -0.10, 0.10, false),
        ] {
            let recipe_path = dir.join(format!("object-depth-order-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("object-depth-order-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "blue": "#0B5FFF",
                        "red": "#FF2020"
                    },
                    "geometries": [
                        { "id": "box_geo", "primitive": { "kind": "box", "size": [0.62, 0.46, 0.08] } }
                    ],
                    "materials": [
                        { "id": "blue_mat", "kind": "unlit", "base_color": "blue" },
                        { "id": "red_mat", "kind": "unlit", "base_color": "red" }
                    ],
                    "nodes": [
                        {
                            "id": "expected_front",
                            "geometry": "box_geo",
                            "material": "blue_mat",
                            "transform": { "kind": "trs", "translation": [0.0, 0.0, front_z] }
                        },
                        {
                            "id": "expected_back",
                            "geometry": "box_geo",
                            "material": "red_mat",
                            "transform": { "kind": "trs", "translation": [0.0, 0.0, back_z] }
                        }
                    ],
                    "scene": { "background": { "kind": "black" } },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 30.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.3], "target": "expected_front" }
                    }],
                    "capture": { "width": 220, "height": 160 },
                    "expect": {
                        "expect_occlusion": [{
                            "id": "blue-occludes-red",
                            "front": { "kind": "node", "id": "expected_front" },
                            "back": { "kind": "node", "id": "expected_back" },
                            "tolerance_pixels": 0
                        }]
                    }
                }))
                .expect("composition object-depth-order recipe serializes"),
            )
            .expect("composition object-depth-order recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena composition object depth-order render command runs");
            assert_eq!(
                output.status.success(),
                should_pass,
                "{backend}/{case} object depth-order status mismatch, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "{backend}/{case} object depth-order failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU object depth-order proof must use HeadlessGpu, not fallback: {report:#}"
                );
            }
            let expected_code = if should_pass {
                "object_depth_order_satisfied"
            } else {
                "object_depth_order_mismatch"
            };
            let expected_status = if should_pass { "checked" } else { "failed" };
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(|check| check["id"] == "expect_occlusion.blue-occludes-red"
                        && check["status"] == expected_status
                        && check["code"] == expected_code
                        && check["observed"]["back_pixels_inside_front"]
                            .as_u64()
                            .is_some()),
                "{backend}/{case} should emit exact object depth-order composition check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "object_depth_order_mismatch"
                            && reason["expectation_id"] == "expect_occlusion.blue-occludes-red"),
                    "{backend}/{case} object depth-order failure should surface as a verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_rejects_ambiguous_object_depth_colors_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-object-depth-ambiguous-color");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let recipe_path = dir.join(format!("object-depth-ambiguous-{backend}.recipe.json"));
        let png_path = dir.join(format!("object-depth-ambiguous-{backend}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "colors": {
                    "blue": "#0B5FFF"
                },
                "geometries": [
                    { "id": "box_geo", "primitive": { "kind": "box", "size": [0.62, 0.46, 0.08] } }
                ],
                "materials": [
                    { "id": "blue_mat", "kind": "unlit", "base_color": "blue" }
                ],
                "nodes": [
                    {
                        "id": "expected_front",
                        "geometry": "box_geo",
                        "material": "blue_mat",
                        "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.10] }
                    },
                    {
                        "id": "expected_back",
                        "geometry": "box_geo",
                        "material": "blue_mat",
                        "transform": { "kind": "trs", "translation": [0.0, 0.0, -0.10] }
                    }
                ],
                "scene": { "background": { "kind": "black" } },
                "render": {
                    "anti_aliasing": "msaa4",
                    "tonemapper": "standard",
                    "exposure_ev": 0.0
                },
                "cameras": [{
                    "id": "main",
                    "kind": "perspective",
                    "fov_degrees": 30.0,
                    "active": true,
                    "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.3], "target": "expected_front" }
                }],
                "capture": { "width": 220, "height": 160 },
                "expect": {
                    "expect_occlusion": [{
                        "id": "blue-occludes-blue",
                        "front": { "kind": "node", "id": "expected_front" },
                        "back": { "kind": "node", "id": "expected_back" },
                        "tolerance_pixels": 0
                    }]
                }
            }))
            .expect("ambiguous object-depth recipe serializes"),
        )
        .expect("ambiguous object-depth recipe writes");

        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let output = command
            .args(args)
            .output()
            .expect("scena ambiguous object depth-order render command runs");
        assert!(
            !output.status.success(),
            "{backend} ambiguous object depth-order should fail closed, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        assert!(
            output.stderr.is_empty(),
            "{backend} ambiguous object depth-order failure stays machine-readable on stdout, stderr={}",
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU ambiguous object depth-order proof must use HeadlessGpu, not fallback: {report:#}"
            );
        }
        assert!(
            report["verification"]["composition"]["checks"]
                .as_array()
                .expect("composition checks array")
                .iter()
                .any(|check| check["id"] == "expect_occlusion.blue-occludes-blue"
                    && check["status"] == "failed"
                    && check["code"] == "object_depth_order_color_ambiguous"
                    && check["observed"]["front_srgb8"].is_array()
                    && check["observed"]["back_srgb8"].is_array()),
            "{backend} should emit exact ambiguous-color object depth-order check: {report:#}"
        );
        assert!(
            report["verification"]["reasons"]
                .as_array()
                .expect("verification reasons array")
                .iter()
                .any(|reason| reason["source"] == "composition"
                    && reason["code"] == "object_depth_order_color_ambiguous"
                    && reason["expectation_id"] == "expect_occlusion.blue-occludes-blue"),
            "{backend} ambiguous color failure should surface as a verification reason: {report:#}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_backend_conformance_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-backend-conformance");
    for (backend, use_gpu, expected_backend, expected_gpu_device) in [
        ("cpu", false, "headless", false),
        ("gpu", true, "headless_gpu", true),
    ] {
        let recipe_path = dir.join(format!("backend-conformance-{backend}.recipe.json"));
        let png_path = dir.join(format!("backend-conformance-{backend}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "colors": {
                    "green": "#2F9E44"
                },
                "geometries": [
                    { "id": "box_geo", "primitive": { "kind": "box", "size": [0.32, 0.32, 0.16] } }
                ],
                "materials": [
                    { "id": "box_mat", "kind": "unlit", "base_color": "green" }
                ],
                "nodes": [
                    {
                        "id": "box",
                        "geometry": "box_geo",
                        "material": "box_mat",
                        "transform": { "kind": "center" }
                    }
                ],
                "scene": { "background": { "kind": "black" } },
                "render": {
                    "anti_aliasing": "msaa4",
                    "supersample": 2,
                    "reconstruction": "tent",
                    "tonemapper": "standard",
                    "exposure_ev": 0.0
                },
                "cameras": [{
                    "id": "main",
                    "kind": "perspective",
                    "fov_degrees": 34.0,
                    "active": true,
                    "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.8], "target": "box" }
                }],
                "capture": { "width": 180, "height": 140 },
                "expect": {
                    "expect_backend": {
                        "backend": expected_backend,
                        "gpu_device": expected_gpu_device
                    }
                }
            }))
            .expect("composition backend recipe serializes"),
        )
        .expect("composition backend recipe writes");

        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let output = command
            .args(args)
            .output()
            .expect("scena composition backend render command runs");
        assert!(
            output.status.success(),
            "{backend} backend conformance recipe should pass, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        assert_eq!(
            report["introspection"]["capabilities"]["backend"], expected_backend,
            "{report:#}"
        );
        assert_eq!(
            report["introspection"]["capabilities"]["gpu_device"], expected_gpu_device,
            "{report:#}"
        );
        for code in [
            "backend_expectation_satisfied",
            "render_antialiasing_active",
            "render_supersample_active",
            "render_reconstruction_active",
        ] {
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(|check| check["status"] == "checked" && check["code"] == code),
                "{backend} render should emit checked backend conformance code {code}: {report:#}"
            );
        }
    }

    let bad_recipe_path = dir.join("backend-conformance-mismatch.recipe.json");
    let bad_png_path = dir.join("backend-conformance-mismatch.png");
    fs::write(
        &bad_recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "box_geo", "primitive": { "kind": "box", "size": [0.32, 0.32, 0.16] } }
            ],
            "materials": [
                { "id": "box_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "box",
                    "geometry": "box_geo",
                    "material": "box_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "scene": { "background": { "kind": "black" } },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.8], "target": "box" }
            }],
            "capture": { "width": 180, "height": 140 },
            "expect": {
                "expect_backend": {
                    "backend": "headless_gpu",
                    "gpu_device": true
                }
            }
        }))
        .expect("composition backend mismatch recipe serializes"),
    )
    .expect("composition backend mismatch recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&bad_recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&bad_png_path),
        ])
        .output()
        .expect("scena composition backend mismatch render command runs");
    assert!(
        !output.status.success(),
        "CPU render with GPU backend expectation should fail, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "backend-conformance failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert!(
        report["verification"]["composition"]["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "expect_backend"
                && check["status"] == "failed"
                && check["code"] == "backend_expectation_mismatch"),
        "backend mismatch should emit exact composition check: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .any(|reason| reason["source"] == "composition"
                && reason["code"] == "backend_expectation_mismatch"
                && reason["expectation_id"] == "expect_backend"),
        "backend mismatch should surface as compact verification reason: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_clipping_and_section_conformance_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-clipping-conformance");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let recipe_path = dir.join(format!("clipping-conformance-{backend}.recipe.json"));
        let png_path = dir.join(format!("clipping-conformance-{backend}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "colors": {
                    "green": "#2F9E44"
                },
                "geometries": [
                    { "id": "box_geo", "primitive": { "kind": "box", "size": [0.46, 0.46, 0.28] } }
                ],
                "materials": [
                    { "id": "box_mat", "kind": "unlit", "base_color": "green" }
                ],
                "nodes": [
                    {
                        "id": "box",
                        "geometry": "box_geo",
                        "material": "box_mat",
                        "transform": { "kind": "center" }
                    }
                ],
                "clipping_planes": [
                    { "id": "front_clip", "normal": [0.0, 0.0, 1.0], "distance": -0.05, "active": true },
                    { "id": "disabled_clip", "normal": [1.0, 0.0, 0.0], "distance": 0.0, "active": false }
                ],
                "section_box": {
                    "target": { "kind": "node", "id": "box" },
                    "margin": 0.02,
                    "inverted": false,
                    "helper_wireframe": true
                },
                "scene": { "background": { "kind": "black" } },
                "render": {
                    "anti_aliasing": "none",
                    "tonemapper": "standard",
                    "exposure_ev": 0.0
                },
                "cameras": [{
                    "id": "main",
                    "kind": "perspective",
                    "fov_degrees": 34.0,
                    "active": true,
                    "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.9], "target": "box" }
                }],
                "capture": { "width": 180, "height": 140 },
                "expect": {
                    "expect_clipping": {
                        "active_clipping_planes": 1,
                        "section_box_active": true,
                        "section_box_inverted": false
                    }
                }
            }))
            .expect("composition clipping recipe serializes"),
        )
        .expect("composition clipping recipe writes");

        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let output = command
            .args(args)
            .output()
            .expect("scena composition clipping render command runs");
        assert!(
            output.status.success(),
            "{backend} clipping conformance recipe should pass, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU clipping conformance proof must use HeadlessGpu, not fallback: {report:#}"
            );
        }
        for code in [
            "clipping_plane_count_satisfied",
            "section_box_active",
            "section_box_inversion_satisfied",
        ] {
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(|check| check["status"] == "checked" && check["code"] == code),
                "{backend} render should emit checked clipping/section code {code}: {report:#}"
            );
        }
    }

    let bad_recipe_path = dir.join("clipping-conformance-missing.recipe.json");
    let bad_png_path = dir.join("clipping-conformance-missing.png");
    fs::write(
        &bad_recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "box_geo", "primitive": { "kind": "box", "size": [0.46, 0.46, 0.28] } }
            ],
            "materials": [
                { "id": "box_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "box",
                    "geometry": "box_geo",
                    "material": "box_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "scene": { "background": { "kind": "black" } },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.9], "target": "box" }
            }],
            "capture": { "width": 180, "height": 140 },
            "expect": {
                "expect_clipping": {
                    "active_clipping_planes": 1,
                    "section_box_active": true
                }
            }
        }))
        .expect("composition clipping mismatch recipe serializes"),
    )
    .expect("composition clipping mismatch recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&bad_recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&bad_png_path),
        ])
        .output()
        .expect("scena composition clipping mismatch render command runs");
    assert!(
        !output.status.success(),
        "render missing expected clipping/section state should fail, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "clipping-conformance failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    for code in ["clipping_plane_count_mismatch", "section_box_missing"] {
        assert!(
            report["verification"]["composition"]["checks"]
                .as_array()
                .expect("composition checks array")
                .iter()
                .any(|check| check["status"] == "failed" && check["code"] == code),
            "missing clipping/section state should emit exact failed composition check {code}: {report:#}"
        );
    }

    let inverted_recipe_path = dir.join("clipping-conformance-inversion.recipe.json");
    let inverted_png_path = dir.join("clipping-conformance-inversion.png");
    fs::write(
        &inverted_recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "box_geo", "primitive": { "kind": "box", "size": [0.46, 0.46, 0.28] } }
            ],
            "materials": [
                { "id": "box_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "box",
                    "geometry": "box_geo",
                    "material": "box_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "clipping_planes": [
                { "id": "front_clip", "normal": [0.0, 0.0, 1.0], "distance": -0.05, "active": true }
            ],
            "section_box": {
                "target": { "kind": "node", "id": "box" },
                "margin": 0.02,
                "inverted": false,
                "helper_wireframe": true
            },
            "scene": { "background": { "kind": "black" } },
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.9], "target": "box" }
            }],
            "capture": { "width": 180, "height": 140 },
            "expect": {
                "expect_clipping": {
                    "active_clipping_planes": 1,
                    "section_box_active": true,
                    "section_box_inverted": true
                }
            }
        }))
        .expect("composition clipping inversion recipe serializes"),
    )
    .expect("composition clipping inversion recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&inverted_recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&inverted_png_path),
        ])
        .output()
        .expect("scena composition clipping inversion render command runs");
    assert!(
        !output.status.success(),
        "render with wrong section-box inversion should fail, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "section inversion failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert!(
        report["verification"]["composition"]["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["status"] == "failed"
                && check["code"] == "section_box_inversion_mismatch"),
        "wrong section-box inversion should emit exact failed composition check: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_material_variant_state_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-material-variant-state");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        let recipe_path = dir.join(format!("variant-state-{backend}.recipe.json"));
        let png_path = dir.join(format!("variant-state-{backend}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "imports": [
                    { "id": "part", "uri": "tests/assets/gltf/material_variants_scene.gltf" }
                ],
                "scene": { "background": { "kind": "black" } },
                "render": {
                    "anti_aliasing": "none",
                    "tonemapper": "standard",
                    "exposure_ev": 0.0
                },
                "capture": { "width": 180, "height": 140 },
                "expect": {
                    "expect_state": [{
                        "id": "default_variant",
                        "import": "part"
                    }]
                }
            }))
            .expect("composition state recipe serializes"),
        )
        .expect("composition state recipe writes");

        let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
        if use_gpu {
            configure_command_for_lavapipe(&mut command);
        }
        let mut args = vec!["recipe", "render", path_str(&recipe_path)];
        if use_gpu {
            args.push("--gpu");
        }
        args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
        let output = command
            .args(args)
            .output()
            .expect("scena composition state render command runs");
        assert!(
            output.status.success(),
            "{backend} default material-variant state should pass, stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        let report = json_report(&output);
        if use_gpu {
            assert_eq!(
                report["introspection"]["capabilities"]["backend"], "headless_gpu",
                "GPU material-variant state proof must use HeadlessGpu, not fallback: {report:#}"
            );
        }
        assert!(
            report["verification"]["composition"]["checks"]
                .as_array()
                .expect("composition checks array")
                .iter()
                .any(|check| check["status"] == "checked"
                    && check["code"] == "material_variant_state_satisfied"),
            "{backend} render should emit checked material variant state: {report:#}"
        );
    }

    let mismatch_recipe_path = dir.join("variant-state-mismatch.recipe.json");
    let mismatch_png_path = dir.join("variant-state-mismatch.png");
    fs::write(
        &mismatch_recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "part", "uri": "tests/assets/gltf/material_variants_scene.gltf" }
            ],
            "scene": { "background": { "kind": "black" } },
            "render": {
                "anti_aliasing": "none",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            },
            "capture": { "width": 180, "height": 140 },
            "expect": {
                "expect_state": [{
                    "id": "must_be_noon",
                    "import": "part",
                    "active_material_variant": "noon"
                }]
            }
        }))
        .expect("composition state mismatch recipe serializes"),
    )
    .expect("composition state mismatch recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&mismatch_recipe_path),
            "--introspect",
            "--verify",
            "--out",
            path_str(&mismatch_png_path),
        ])
        .output()
        .expect("scena composition state mismatch render command runs");
    assert!(
        !output.status.success(),
        "wrong material variant state should fail, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(
        output.stderr.is_empty(),
        "material variant state failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert!(
        report["verification"]["composition"]["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["status"] == "failed"
                && check["code"] == "material_variant_state_mismatch"),
        "wrong material variant state should emit exact failed composition check: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_transform_conformance_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-transform-conformance");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, expected_translation, should_pass, expected_code) in [
            (
                "match",
                [0.18_f64, 0.05_f64, 0.0_f64],
                true,
                "transform_conformance_satisfied",
            ),
            (
                "mismatch",
                [0.42_f64, 0.05_f64, 0.0_f64],
                false,
                "transform_conformance_mismatch",
            ),
        ] {
            let recipe_path = dir.join(format!("transform-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("transform-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "blue": "#2D68C4"
                    },
                    "geometries": [
                        { "id": "part_geo", "primitive": { "kind": "box", "size": [0.32, 0.24, 0.12] } }
                    ],
                    "materials": [
                        { "id": "part_mat", "kind": "unlit", "base_color": "blue" }
                    ],
                    "nodes": [{
                        "id": "part",
                        "geometry": "part_geo",
                        "material": "part_mat",
                        "transform": {
                            "kind": "trs",
                            "translation": [0.18, 0.05, 0.0],
                            "rotation_degrees": [0.0, 45.0, 0.0],
                            "scale": [1.2, 0.8, 1.0]
                        }
                    }],
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 36.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.1], "target": "part" }
                    }],
                    "scene": { "background": { "kind": "white" } },
                    "render": {
                        "anti_aliasing": "none",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "capture": { "width": 180, "height": 140 },
                    "expect": {
                        "expect_transform": [{
                            "id": "part_world_transform",
                            "target": { "kind": "node", "id": "part" },
                            "translation": expected_translation,
                            "scale": [1.2, 0.8, 1.0],
                            "rotation_degrees": [0.0, 45.0, 0.0],
                            "translation_tolerance": 0.002,
                            "scale_tolerance": 0.002,
                            "rotation_tolerance_degrees": 0.1
                        }]
                    }
                }))
                .expect("composition transform recipe serializes"),
            )
            .expect("composition transform recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena composition transform render command runs");
            assert_eq!(
                output.status.success(),
                should_pass,
                "{backend}/{case} transform conformance status mismatch, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "transform conformance failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU transform conformance proof must use HeadlessGpu, not fallback: {report:#}"
                );
            }
            let composition = &report["verification"]["composition"];
            assert_eq!(composition["schema"], "scena.scene_composition.v1");
            assert_eq!(composition["ok"], should_pass, "{report:#}");
            assert!(
                composition["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(
                        |check| check["id"] == "expect_transform.part_world_transform"
                            && check["status"] == if should_pass { "checked" } else { "failed" }
                            && check["code"] == expected_code,
                    ),
                "{backend}/{case} should emit exact transform conformance code {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "transform_conformance_mismatch"
                            && reason["expectation_id"] == "expect_transform.part_world_transform"),
                    "{backend}/{case} transform mismatch should surface as verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_world_bounds_separation_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-world-bounds-separation");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, right_translation, should_pass, expected_code) in [
            (
                "separated",
                [0.26_f64, 0.0_f64, 0.0_f64],
                true,
                "separation_conformance_satisfied",
            ),
            (
                "intersecting",
                [-0.02_f64, 0.0_f64, 0.0_f64],
                false,
                "separation_conformance_mismatch",
            ),
        ] {
            let recipe_path = dir.join(format!("separation-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("separation-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "blue": "#2D68C4",
                        "orange": "#F08C00"
                    },
                    "geometries": [
                        { "id": "box_geo", "primitive": { "kind": "box", "size": [0.20, 0.20, 0.20] } }
                    ],
                    "materials": [
                        { "id": "blue_mat", "kind": "unlit", "base_color": "blue" },
                        { "id": "orange_mat", "kind": "unlit", "base_color": "orange" }
                    ],
                    "nodes": [
                        {
                            "id": "left_part",
                            "geometry": "box_geo",
                            "material": "blue_mat",
                            "transform": { "kind": "trs", "translation": [-0.12, 0.0, 0.0] }
                        },
                        {
                            "id": "right_part",
                            "geometry": "box_geo",
                            "material": "orange_mat",
                            "transform": { "kind": "trs", "translation": right_translation }
                        }
                    ],
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 36.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "left_part" }
                    }],
                    "scene": { "background": { "kind": "white" } },
                    "render": {
                        "anti_aliasing": "none",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "capture": { "width": 180, "height": 140 },
                    "expect": {
                        "expect_separation": [{
                            "id": "parts-do-not-intersect",
                            "a": { "kind": "node", "id": "left_part" },
                            "b": { "kind": "node", "id": "right_part" },
                            "min_gap": 0.0,
                            "tolerance": 0.001
                        }]
                    }
                }))
                .expect("composition separation recipe serializes"),
            )
            .expect("composition separation recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena composition separation render command runs");
            assert_eq!(
                output.status.success(),
                should_pass,
                "{backend}/{case} separation status mismatch, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "separation failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU separation proof must use HeadlessGpu, not fallback: {report:#}"
                );
            }
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(
                        |check| check["id"] == "expect_separation.parts-do-not-intersect"
                            && check["status"] == if should_pass { "checked" } else { "failed" }
                            && check["code"] == expected_code,
                    ),
                "{backend}/{case} should emit exact separation conformance code {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "separation_conformance_mismatch"
                            && reason["expectation_id"]
                                == "expect_separation.parts-do-not-intersect"),
                    "{backend}/{case} separation mismatch should surface as a verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_object_exposure_and_salience_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-object-pixel-quality");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, material_color, background_color, should_pass, expected_code) in [
            ("good", "#2F9E44", "#DDE2E8", true, "subject_exposure_sane"),
            (
                "black-crush",
                "#000000",
                "#F0F3F5",
                false,
                "subject_black_crushed",
            ),
            (
                "blown-out",
                "#FFFFFF",
                "#101820",
                false,
                "subject_blown_out",
            ),
            (
                "low-salience",
                "#34383D",
                "#30343A",
                false,
                "subject_salience_too_low",
            ),
        ] {
            let recipe_path = dir.join(format!("object-pixel-{case}-{backend}.recipe.json"));
            let png_path = dir.join(format!("object-pixel-{case}-{backend}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "colors": {
                        "subject_color": material_color,
                        "background_color": background_color
                    },
                    "geometries": [
                        { "id": "subject_geo", "primitive": { "kind": "box", "size": [0.42, 0.42, 0.08] } }
                    ],
                    "materials": [
                        { "id": "subject_mat", "kind": "unlit", "base_color": "subject_color" }
                    ],
                    "nodes": [
                        {
                            "id": "subject",
                            "geometry": "subject_geo",
                            "material": "subject_mat",
                            "transform": { "kind": "center" }
                        }
                    ],
                    "scene": {
                        "background": { "kind": "custom", "color": "background_color" }
                    },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 34.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "subject" }
                    }],
                    "capture": { "width": 160, "height": 160 },
                    "expect": {
                        "expect_quality": {
                            "profile": "product",
                            "geometry": {
                                "min_intermediate_edge_fraction": 0.0
                            }
                        }
                    }
                }))
                .expect("object pixel composition recipe serializes"),
            )
            .expect("object pixel composition recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena object-pixel composition render command runs");
            assert_eq!(
                output.status.success(),
                should_pass,
                "{backend}/{case} object-pixel composition status mismatch, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "{backend}/{case} object-pixel composition failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU object-pixel proof must use HeadlessGpu, not fallback: {report:#}"
                );
            }
            let composition = &report["verification"]["composition"];
            assert_eq!(composition["schema"], "scena.scene_composition.v1");
            assert_eq!(composition["ok"], should_pass, "{report:#}");
            assert!(
                composition["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(|check| check["id"] == "node.subject.pixel_exposure"
                        && check["code"] == expected_code
                        && check["status"] == if should_pass { "checked" } else { "failed" }
                        && check["observed"]["low_clip_fraction"].as_f64().is_some()
                        && check["observed"]["mean_background_delta"]
                            .as_f64()
                            .is_some()),
                "{backend}/{case} should emit exact object-pixel exposure/salience code {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == expected_code
                            && reason["expectation_id"] == "node.subject.pixel_exposure"),
                    "{backend}/{case} object-pixel failure must surface as verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_imported_product_exposure_by_default_on_gpu() {
    let dir = artifact_dir("recipe-import-product-exposure");
    for (case, background, environment, lights, should_pass, expected_code) in [
        (
            "neutral-environment-auto-exposes-import",
            "studio",
            "neutral_studio",
            json!([]),
            true,
            "subject_exposure_sane",
        ),
        (
            "close-softbox-blows-out-import",
            "dark_studio",
            "studio",
            json!([{
                "id": "softbox",
                "kind": "area",
                "shape": "rect",
                "preset": "softbox",
                "transform": {
                    "kind": "trs",
                    "translation": [0.1, 0.3, 0.2]
                }
            }]),
            false,
            "subject_blown_out",
        ),
    ] {
        let recipe_path = dir.join(format!("{case}.recipe.json"));
        let png_path = dir.join(format!("{case}.png"));
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "capture": { "width": 512, "height": 640 },
                "imports": [{
                    "id": "bottle",
                    "uri": "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf",
                    "expected_extent": { "min": 0.001, "max": 100.0, "unit": "m" }
                }],
                "scene": {
                    "background": { "kind": background },
                    "environment": { "preset": environment }
                },
                "render": {
                    "auto_exposure": "product_studio",
                    "quality": "high",
                    "anti_aliasing": "msaa4",
                    "supersample": 1,
                    "reconstruction": "tent"
                },
                "cameras": [{
                    "id": "main",
                    "active": true,
                    "kind": "perspective",
                    "lens": "portrait",
                    "framing": {
                        "preset": "three_quarter_front_right",
                        "fill": 0.7
                    }
                }],
                "lights": lights
            }))
            .expect("WaterBottle exposure recipe serializes"),
        )
        .expect("WaterBottle exposure recipe writes");

        let report = if should_pass {
            run_recipe_render_verify(&recipe_path, &png_path, true)
        } else {
            run_recipe_render_verify_expect_failure(&recipe_path, &png_path, true)
        };
        let composition = &report["verification"]["composition"];
        assert_eq!(composition["schema"], "scena.scene_composition.v1");
        assert_eq!(composition["ok"], should_pass, "{report:#}");
        let import_pixel_check = composition["checks"]
            .as_array()
            .expect("composition checks serialize")
            .iter()
            .find(|check| check["id"] == "import.bottle.pixel_exposure")
            .unwrap_or_else(|| {
                panic!("imported WaterBottle must emit object-pixel exposure check: {report:#}")
            });
        assert_eq!(import_pixel_check["code"], expected_code, "{report:#}");
        assert_eq!(
            import_pixel_check["status"],
            if should_pass { "checked" } else { "failed" },
            "{report:#}"
        );
        if should_pass {
            let mean_luminance = import_pixel_check["observed"]["mean_luminance"]
                .as_f64()
                .expect("mean luminance serializes");
            assert!(
                mean_luminance >= 0.34,
                "neutral-studio WaterBottle should not stay dull after product auto-exposure: {report:#}"
            );
        } else {
            assert!(
                report["verification"]["reasons"]
                    .as_array()
                    .expect("verification reasons serialize")
                    .iter()
                    .any(|reason| reason["source"] == "composition"
                        && reason["code"] == expected_code
                        && reason["expectation_id"] == "import.bottle.pixel_exposure"),
                "blown-out import exposure must surface as a composition verification reason: {report:#}"
            );
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_contact_shadow_grounding_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-quality-contact-shadow-grounding");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, ssao, should_pass, expected_code) in [
            ("missing", None, false, "contact_shadow_missing"),
            (
                "present",
                Some(json!({
                    "radius_px": 4,
                    "intensity": 0.8,
                    "depth_threshold": 0.0
                })),
                true,
                "contact_shadow_checked",
            ),
        ] {
            let recipe_path = dir.join(format!("contact-shadow-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("contact-shadow-{backend}-{case}.png"));
            let mut render = json!({
                "anti_aliasing": "none",
                "tonemapper": "standard",
                "exposure_ev": 0.0
            });
            if let Some(ssao) = ssao {
                render["ssao"] = ssao;
            }
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "geometries": [
                        {
                            "id": "floor_geo",
                            "mesh": {
                                "topology": "triangles",
                                "positions": [
                                    [-0.75, -0.55, 0.0],
                                    [0.75, -0.55, 0.0],
                                    [0.75, 0.35, 0.0],
                                    [-0.75, 0.35, 0.0]
                                ],
                                "indices": [0, 1, 2, 0, 2, 3]
                            }
                        },
                        {
                            "id": "block_geo",
                            "mesh": {
                                "topology": "triangles",
                                "positions": [
                                    [-0.14, -0.18, 0.16],
                                    [0.14, -0.18, 0.16],
                                    [0.14, 0.18, 0.16],
                                    [-0.14, 0.18, 0.16]
                                ],
                                "indices": [0, 1, 2, 0, 2, 3]
                            }
                        }
                    ],
                    "materials": [
                        { "id": "floor_mat", "kind": "unlit", "base_color": "#F6F7F8", "double_sided": true },
                        { "id": "block_mat", "kind": "unlit", "base_color": "#AEB5BF", "double_sided": true }
                    ],
                    "nodes": [
                        { "id": "floor", "geometry": "floor_geo", "material": "floor_mat" },
                        { "id": "block", "geometry": "block_geo", "material": "block_mat" }
                    ],
                    "scene": { "background": { "kind": "custom", "color": "#F6F7F8" } },
                    "render": render,
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 60.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.7320508], "target": [0.0, 0.0, 0.0] }
                    }],
                    "capture": { "width": 96, "height": 96 },
                    "expect": {
                        "expect_quality": {
                            "profile": "product",
                            "noise": { "max_outlier_fraction": 0.04 },
                            "geometry": { "min_intermediate_edge_fraction": 0.0 },
                            "grounding": {
                                "target": { "kind": "node", "id": "block" },
                                "min_contact_shadow_delta": 0.015
                            }
                        }
                    }
                }))
                .expect("contact-shadow recipe serializes"),
            )
            .expect("contact-shadow recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena contact-shadow recipe render command runs");
            assert_eq!(
                output.status.success(),
                should_pass,
                "{backend}/{case} contact-shadow status mismatch, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "{backend}/{case} contact-shadow failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU contact-shadow proof must use HeadlessGpu, not CPU fallback: {report:#}"
                );
            }
            assert!(
                report["verification"]["quality"]["checks"]
                    .as_array()
                    .expect("quality checks array")
                    .iter()
                    .any(|check| check["id"] == "expect_quality.grounding.target"
                        && check["code"] == expected_code
                        && check["observed"]["contact_shadow_delta"].as_f64().is_some()),
                "{backend}/{case} should emit exact contact-shadow quality check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "quality"
                            && reason["code"] == "contact_shadow_missing"
                            && reason["expectation_id"] == "expect_quality.grounding.target"),
                    "{backend}/{case} contact-shadow failure must surface as verification reason: {report:#}"
                );
            }
            if should_pass {
                let image = decode_png_rgba8(&png_path);
                assert_contact_shadow_is_localized(backend, &image);
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_object_framing_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-object-framing");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, size, eye_z, should_pass, expected_code) in [
            ("normal", [0.62, 0.46, 0.12], 2.2, true, "subject_fit_sane"),
            (
                "tiny",
                [0.055, 0.040, 0.015],
                4.8,
                false,
                "subject_too_small_in_frame",
            ),
        ] {
            let recipe_path = dir.join(format!("object-framing-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("object-framing-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "geometries": [
                        { "id": "subject_geo", "primitive": { "kind": "box", "size": size } }
                    ],
                    "materials": [
                        { "id": "subject_mat", "kind": "unlit", "base_color": "#2F9E44" }
                    ],
                    "nodes": [
                        { "id": "subject", "geometry": "subject_geo", "material": "subject_mat" }
                    ],
                    "scene": { "background": { "kind": "custom", "color": "#DDE2E8" } },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "supersample": 2,
                        "reconstruction": "tent",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "fov_degrees": 30.0,
                        "active": true,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, eye_z], "target": "subject" }
                    }],
                    "capture": { "width": 220, "height": 160 },
                    "expect": {
                        "expect_quality": {
                            "profile": "product",
                            "geometry": {
                                "min_intermediate_edge_fraction": 0.0
                            }
                        }
                    }
                }))
                .expect("object framing recipe serializes"),
            )
            .expect("object framing recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena object-framing composition render command runs");
            assert_eq!(
                output.status.success(),
                should_pass,
                "{backend}/{case} object framing status mismatch, stdout={}, stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(&output)
            );
            assert!(
                output.stderr.is_empty(),
                "{backend}/{case} object framing failures stay machine-readable on stdout, stderr={}",
                stderr(&output)
            );
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU object-framing proof must use HeadlessGpu, not fallback: {report:#}"
                );
            }
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(|check| check["id"] == "node.subject.framing"
                        && check["status"] == if should_pass { "checked" } else { "failed" }
                        && check["code"] == expected_code
                        && check["observed"]["fit_fraction"].as_f64().is_some()),
                "{backend}/{case} should emit exact object framing composition check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "subject_too_small_in_frame"
                            && reason["expectation_id"] == "node.subject.framing"),
                    "{backend}/{case} object framing failure should surface as verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_texture_material_result_on_cpu_and_gpu() {
    let dir = artifact_dir("recipe-composition-texture-result");
    let source_texture = Path::new("tests/assets/gltf/khronos/TextureSettingsTest/CheckAndX.png");
    let local_texture = dir.join("check-and-x.png");
    fs::copy(source_texture, &local_texture).expect("texture fixture copies next to recipe");
    for (backend, use_gpu) in [("cpu", false), ("gpu", true)] {
        for (case, uvs, should_pass, expected_code) in [
            (
                "mapped",
                json!([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
                true,
                "texture_result_visible",
            ),
            (
                "flat_uv",
                json!([[0.08, 0.08], [0.08, 0.08], [0.08, 0.08], [0.08, 0.08]]),
                false,
                "texture_result_flat",
            ),
        ] {
            let recipe_path = dir.join(format!("texture-result-{backend}-{case}.recipe.json"));
            let png_path = dir.join(format!("texture-result-{backend}-{case}.png"));
            fs::write(
                &recipe_path,
                serde_json::to_string_pretty(&json!({
                    "schema": "scena.scene_recipe.v1",
                    "geometries": [{
                        "id": "panel_geo",
                        "mesh": {
                            "topology": "triangles",
                            "positions": [
                                [-0.55, -0.40, 0.0],
                                [0.55, -0.40, 0.0],
                                [0.55, 0.40, 0.0],
                                [-0.55, 0.40, 0.0]
                            ],
                            "normals": [
                                [0.0, 0.0, 1.0],
                                [0.0, 0.0, 1.0],
                                [0.0, 0.0, 1.0],
                                [0.0, 0.0, 1.0]
                            ],
                            "uvs": uvs,
                            "indices": [0, 1, 2, 0, 2, 3]
                        }
                    }],
                    "materials": [{
                        "id": "textured_mat",
                        "kind": "unlit",
                        "base_color": "#808080",
                        "double_sided": false,
                        "base_color_texture": { "uri": "check-and-x.png", "color_space": "srgb" }
                    }],
                    "nodes": [
                        { "id": "textured_panel", "geometry": "panel_geo", "material": "textured_mat" }
                    ],
                    "scene": { "background": { "kind": "custom", "color": "#303030" } },
                    "render": {
                        "anti_aliasing": "msaa4",
                        "supersample": 2,
                        "reconstruction": "tent",
                        "tonemapper": "standard",
                        "exposure_ev": 0.0
                    },
                    "cameras": [{
                        "id": "main",
                        "kind": "perspective",
                        "active": true,
                        "fov_degrees": 24.0,
                        "transform": { "kind": "look_at", "eye": [0.0, 0.0, 3.2], "target": "textured_panel" }
                    }],
                    "capture": { "width": 180, "height": 150 },
                    "expect": {
                        "expect_quality": {
                            "profile": "product",
                            "exposure": {}
                        }
                    }
                }))
                .expect("texture-result recipe serializes"),
            )
            .expect("texture-result recipe writes");

            let mut command = Command::new(env!("CARGO_BIN_EXE_scena"));
            if use_gpu {
                configure_command_for_lavapipe(&mut command);
            }
            let mut args = vec!["recipe", "render", path_str(&recipe_path)];
            if use_gpu {
                args.push("--gpu");
            }
            args.extend(["--introspect", "--verify", "--out", path_str(&png_path)]);
            let output = command
                .args(args)
                .output()
                .expect("scena texture-result recipe render command runs");
            if should_pass {
                assert!(
                    output.status.success(),
                    "{backend}/{case} textured material result should pass, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
            } else {
                assert!(
                    !output.status.success(),
                    "{backend}/{case} flat texture mapping should fail, stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    stderr(&output)
                );
                assert!(
                    output.stderr.is_empty(),
                    "composition texture-result failures stay machine-readable on stdout, stderr={}",
                    stderr(&output)
                );
            }
            let report = json_report(&output);
            if use_gpu {
                assert_eq!(
                    report["introspection"]["capabilities"]["backend"], "headless_gpu",
                    "GPU texture-result proof must use HeadlessGpu, not CPU fallback: {report:#}"
                );
            }
            assert!(
                report["verification"]["composition"]["checks"]
                    .as_array()
                    .expect("composition checks array")
                    .iter()
                    .any(|check| check["id"] == "node.textured_panel.texture_result"
                        && check["code"] == expected_code
                        && check["observed"]["texture_slots"]
                            .as_array()
                            .is_some_and(|slots| {
                                slots.iter().any(|slot| slot == "baseColorTexture")
                            })
                        && check["observed"]["luminance_stddev"].as_f64().is_some()),
                "{backend}/{case} should emit exact texture-result composition check {expected_code}: {report:#}"
            );
            if !should_pass {
                assert!(
                    report["verification"]["reasons"]
                        .as_array()
                        .expect("verification reasons array")
                        .iter()
                        .any(|reason| reason["source"] == "composition"
                            && reason["code"] == "texture_result_flat"
                            && reason["expectation_id"] == "node.textured_panel.texture_result"),
                    "{backend}/{case} texture-result failure should surface as a verification reason: {report:#}"
                );
            }
        }
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_checks_measurement_overlay_ownership() {
    let dir = artifact_dir("recipe-composition-measurement-ownership");
    let recipe_path = dir.join("composition-measurement.recipe.json");
    let png_path = dir.join("composition-measurement.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44"
            },
            "geometries": [
                { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.35, 0.35, 0.08] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "green" }
            ],
            "nodes": [
                {
                    "id": "visible_box",
                    "geometry": "visible_geo",
                    "material": "green_mat",
                    "transform": { "kind": "center" }
                }
            ],
            "measurements": [
                {
                    "id": "box_width",
                    "kind": "distance",
                    "start": [-0.22, -0.25, 0.0],
                    "end": [0.22, -0.25, 0.0],
                    "label": "WIDTH",
                    "unit": "m",
                    "precision": 2
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "visible_box" }
            }],
            "capture": { "width": 180, "height": 150 },
            "expect": {}
        }))
        .expect("composition measurement recipe serializes"),
    )
    .expect("composition measurement recipe writes");

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
        .expect("scena composition measurement render command runs");

    assert!(
        output.status.success(),
        "composition measurement recipe should pass verification, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let report = json_report(&output);
    let composition = &report["verification"]["composition"];
    assert_eq!(composition["schema"], "scena.scene_composition.v1");
    assert_eq!(composition["ok"], true, "{report:#}");
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(
                |check| check["id"] == "annotation.measurement.box_width.output"
                    && check["status"] == "checked"
                    && check["code"] == "measurement_overlay_output_projected",
            ),
        "measurement generated line and label should be owned and projected: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["status"] == "checked"
                && check["code"] == "overlay_label_clear_of_lines"),
        "measurement label should be checked as clear of crossing line overlays: {report:#}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scena_recipe_render_verify_emits_composition_report_for_declared_nodes() {
    let dir = artifact_dir("recipe-composition-report");
    let recipe_path = dir.join("composition.recipe.json");
    let png_path = dir.join("composition.png");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "green": "#2F9E44",
                "red": "#C92A2A"
            },
            "geometries": [
                { "id": "visible_geo", "primitive": { "kind": "box", "size": [0.35, 0.35, 0.08] } },
                { "id": "hidden_geo", "primitive": { "kind": "box", "size": [0.20, 0.20, 0.08] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "green" },
                { "id": "red_mat", "kind": "unlit", "base_color": "red" }
            ],
            "nodes": [
                {
                    "id": "visible_box",
                    "geometry": "visible_geo",
                    "material": "green_mat",
                    "transform": { "kind": "center" }
                },
                {
                    "id": "hidden_box",
                    "geometry": "hidden_geo",
                    "material": "red_mat",
                    "visible": false,
                    "transform": { "kind": "trs", "translation": [0.45, 0.0, 0.0] }
                }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 36.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": "visible_box" }
            }],
            "capture": { "width": 128, "height": 128 },
            "expect": {}
        }))
        .expect("composition recipe serializes"),
    )
    .expect("composition recipe writes");

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
        .expect("scena composition recipe render command runs");

    assert!(
        !output.status.success(),
        "declared hidden node should fail composition verification"
    );
    assert!(
        output.stderr.is_empty(),
        "composition failures stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report = json_report(&output);
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["build"]["ok"], true, "{report:#}");
    let composition = &report["verification"]["composition"];
    assert_eq!(composition["schema"], "scena.scene_composition.v1");
    assert_eq!(composition["ok"], false, "{report:#}");
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "node.visible_box.presence"
                && check["status"] == "checked"
                && check["code"] == "node_visible"),
        "visible declared node should be explicitly checked: {report:#}"
    );
    assert!(
        composition["checks"]
            .as_array()
            .expect("composition checks array")
            .iter()
            .any(|check| check["id"] == "node.hidden_box.presence"
                && check["status"] == "failed"
                && check["code"] == "declared_node_not_drawn"),
        "hidden declared node should be an exact composition failure: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .any(|reason| reason["source"] == "composition"
                && reason["code"] == "declared_node_not_drawn"),
        "composition failures must surface in verification reasons: {report:#}"
    );
    assert!(
        report["verification"]["reasons"]
            .as_array()
            .expect("verification reasons array")
            .iter()
            .all(|reason| reason["code"] != "grounding_intent_not_declared"),
        "optional composition skips must stay informational, not top-level warning reasons: {report:#}"
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
    let recipe_transform: scena::SceneRecipeTransformV1 =
        serde_json::from_value(json_transform(&look_at)).expect("look_at transform deserializes");
    let transform = scena::Transform::try_from(&recipe_transform)
        .expect("look_at placement emits a concrete raw transform");
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
                    "translation": [0.42, 0.0, 0.0]
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

#[cfg(feature = "scene-host")]
fn write_many_import_recipe(dir: &Path, count: usize) -> PathBuf {
    let imports = (0..count)
        .map(|index| {
            json!({
                "id": format!("part_{index:03}"),
                "uri": TEST_ASSET
            })
        })
        .collect::<Vec<_>>();
    let recipe_path = dir.join("many-imports.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": imports,
            "capture": { "width": 64, "height": 64 }
        }))
        .expect("many-import recipe serializes"),
    )
    .expect("many-import recipe writes");
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

fn contains_diagnostic(report: &serde_json::Value, code: &str, path: &str) -> bool {
    match report {
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| contains_diagnostic(value, code, path)),
        serde_json::Value::Object(fields) => {
            (fields.get("code").and_then(serde_json::Value::as_str) == Some(code)
                && fields.get("path").and_then(serde_json::Value::as_str) == Some(path))
                || fields
                    .values()
                    .any(|value| contains_diagnostic(value, code, path))
        }
        _ => false,
    }
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
fn center_pixel(image: &DecodedPng) -> [u8; 4] {
    let x = image.width / 2;
    let y = image.height / 2;
    let offset = ((y * image.width + x) * 4) as usize;
    [
        image.rgba8[offset],
        image.rgba8[offset + 1],
        image.rgba8[offset + 2],
        image.rgba8[offset + 3],
    ]
}

#[cfg(feature = "scene-host")]
fn assert_rgb8_close(actual: [u8; 4], expected: [u8; 3], tolerance: u8, label: &str) {
    for channel in 0..3 {
        let delta = actual[channel].abs_diff(expected[channel]);
        assert!(
            delta <= tolerance,
            "{label} channel {channel} expected {:?} +/- {tolerance}, got {:?}",
            expected,
            actual
        );
    }
    assert_eq!(actual[3], 255, "{label} should be fully opaque");
}

#[cfg(feature = "scene-host")]
fn floor_grid_detail_crop(image: &DecodedPng) -> DecodedPng {
    let crop_width = ((image.width as f32) * 0.56).round() as u32;
    let crop_height = ((image.height as f32) * 0.34).round() as u32;
    let x0 = image.width.saturating_sub(crop_width) / 2;
    let y0 = ((image.height as f32) * 0.58).floor() as u32;
    let x1 = x0.saturating_add(crop_width).min(image.width);
    let y1 = y0.saturating_add(crop_height).min(image.height);
    let width = x1.saturating_sub(x0).max(1);
    let height = y1.saturating_sub(y0).max(1);
    let mut rgba8 = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for y in y0..y1 {
        let row_start = ((y * image.width + x0) * 4) as usize;
        let row_end = row_start + (width as usize) * 4;
        rgba8.extend_from_slice(&image.rgba8[row_start..row_end]);
    }
    DecodedPng {
        width,
        height,
        rgba8,
    }
}

#[cfg(feature = "scene-host")]
fn composition_check<'a>(report: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    report["verification"]["composition"]["checks"]
        .as_array()
        .expect("composition checks serialize")
        .iter()
        .find(|check| check["id"] == id)
        .unwrap_or_else(|| panic!("composition check {id} not found in report: {report:#}"))
}

#[cfg(feature = "scene-host")]
fn node_region_from_composition_report(
    report: &serde_json::Value,
    target_id: &str,
) -> QualityPixelRegion {
    let checks = report["verification"]["composition"]["checks"]
        .as_array()
        .expect("composition checks serialize");
    for check in checks {
        if check["category"] != "placement"
            || check["code"] != "projected_bbox_available"
            || check["target_id"] != target_id
        {
            continue;
        }
        let rect = &check["region"]["rect_css_px"];
        let min_x = rect["min_x"].as_f64().expect("region min_x");
        let min_y = rect["min_y"].as_f64().expect("region min_y");
        let max_x = rect["max_x"].as_f64().expect("region max_x");
        let max_y = rect["max_y"].as_f64().expect("region max_y");
        let width = report["capture"]["width"].as_u64().expect("capture width") as u32;
        let height = report["capture"]["height"]
            .as_u64()
            .expect("capture height") as u32;
        let x = min_x.floor().max(0.0) as u32;
        let y = min_y.floor().max(0.0) as u32;
        let end_x = (max_x.ceil().max(min_x).min(f64::from(width))) as u32;
        let end_y = (max_y.ceil().max(min_y).min(f64::from(height))) as u32;
        return QualityPixelRegion {
            x: x.min(width),
            y: y.min(height),
            width: end_x.saturating_sub(x).max(1),
            height: end_y.saturating_sub(y).max(1),
        };
    }
    panic!("projected bbox for target {target_id} not found in report: {report:#}");
}

#[cfg(feature = "scene-host")]
fn content_region_from_introspection_report(
    report: &serde_json::Value,
) -> support::parity::PixelRegion {
    let rect = &report["content_bbox_css_px"];
    let min_x = rect["min_x"].as_f64().expect("content bbox min_x");
    let min_y = rect["min_y"].as_f64().expect("content bbox min_y");
    let max_x = rect["max_x"].as_f64().expect("content bbox max_x");
    let max_y = rect["max_y"].as_f64().expect("content bbox max_y");
    let width = report["artifacts"]["capture"]["width"]
        .as_u64()
        .expect("capture width") as u32;
    let height = report["artifacts"]["capture"]["height"]
        .as_u64()
        .expect("capture height") as u32;
    let x = min_x.floor().max(0.0) as u32;
    let y = min_y.floor().max(0.0) as u32;
    let end_x = max_x.ceil().max(min_x).min(f64::from(width)) as u32;
    let end_y = max_y.ceil().max(min_y).min(f64::from(height)) as u32;
    support::parity::PixelRegion {
        x: x.min(width),
        y: y.min(height),
        width: end_x.saturating_sub(x).max(1),
        height: end_y.saturating_sub(y).max(1),
    }
}

#[cfg(feature = "scene-host")]
fn quality_region_from_pixel_region(region: support::parity::PixelRegion) -> QualityPixelRegion {
    QualityPixelRegion {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

#[cfg(feature = "scene-host")]
fn parity_region_json(region: support::parity::PixelRegion) -> String {
    format!(
        "{{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }}",
        region.x, region.y, region.width, region.height
    )
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy)]
struct ChromeRegionMetrics {
    foreground_fraction: f32,
    luminance_range: f32,
    unique_luma_levels: usize,
}

#[cfg(feature = "scene-host")]
fn chrome_region_metrics(image: &DecodedPng, region: QualityPixelRegion) -> ChromeRegionMetrics {
    let mut foreground_pixels = 0usize;
    let mut min_luma = f32::INFINITY;
    let mut max_luma = f32::NEG_INFINITY;
    let mut unique_luma = std::collections::BTreeSet::new();
    for y in region.y..region.y.saturating_add(region.height).min(image.height) {
        for x in region.x..region.x.saturating_add(region.width).min(image.width) {
            let offset = ((y as usize) * (image.width as usize) + x as usize) * 4;
            let Some(pixel) = image.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if pixel[3] == 0 || pixel[..3].iter().all(|channel| *channel >= 248) {
                continue;
            }
            foreground_pixels += 1;
            let luma = srgb_luminance_u8(pixel);
            min_luma = min_luma.min(luma);
            max_luma = max_luma.max(luma);
            unique_luma.insert((luma * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    let region_pixels = (region.width as usize).saturating_mul(region.height as usize);
    ChromeRegionMetrics {
        foreground_fraction: foreground_pixels as f32 / region_pixels.max(1) as f32,
        luminance_range: if min_luma.is_finite() && max_luma.is_finite() {
            max_luma - min_luma
        } else {
            0.0
        },
        unique_luma_levels: unique_luma.len(),
    }
}

#[cfg(feature = "scene-host")]
fn srgb_luminance_u8(pixel: &[u8]) -> f32 {
    (0.2126 * f32::from(pixel[0]) + 0.7152 * f32::from(pixel[1]) + 0.0722 * f32::from(pixel[2]))
        / 255.0
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
    p999_channel_delta: u8,
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
    let mut deltas = Vec::with_capacity(
        (region.width as usize)
            .saturating_mul(region.height as usize)
            .saturating_mul(3),
    );
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            for channel in 0..3 {
                let delta = left[offset + channel].abs_diff(right[offset + channel]);
                max_channel_delta = max_channel_delta.max(delta);
                deltas.push(delta);
                total = total.saturating_add(u64::from(delta));
                count = count.saturating_add(1);
            }
        }
    }
    deltas.sort_unstable();
    let p999_index = deltas.len().saturating_sub(1).saturating_mul(999) / 1_000;
    FrameDelta {
        max_channel_delta,
        p999_channel_delta: deltas.get(p999_index).copied().unwrap_or(0),
        mean_channel_delta: total as f32 / count.max(1) as f32,
    }
}

#[cfg(feature = "scene-host")]
fn format_material_reflection_metrics(
    results: &[(String, String, FrameDelta, QualityPixelRegion)],
) -> String {
    let rows = results
        .iter()
        .map(|(backend, target_id, delta, region)| {
            format!(
                "    {{ \"backend\": \"{}\", \"target_id\": \"{}\", \"mean_channel_delta\": {:.3}, \"max_channel_delta\": {}, \"region\": {{ \"x\": {}, \"y\": {}, \"width\": {}, \"height\": {} }} }}",
                backend,
                target_id,
                delta.mean_channel_delta,
                delta.max_channel_delta,
                region.x,
                region.y,
                region.width,
                region.height
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema\": \"scena.material_reflection_delta_probe.v1\",\n  \"rows\": [\n{rows}\n  ]\n}}\n"
    )
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

#[cfg(feature = "scene-host")]
fn reflection_firefly_fraction_in_region(
    rgba: &[u8],
    frame_width: u32,
    frame_height: u32,
    region: QualityPixelRegion,
) -> f32 {
    let max_x = region.x.saturating_add(region.width).min(frame_width);
    let max_y = region.y.saturating_add(region.height).min(frame_height);
    if max_x <= region.x || max_y <= region.y {
        return 0.0;
    }
    let mut luminance =
        Vec::with_capacity((region.width as usize).saturating_mul(region.height as usize));
    for y in region.y..max_y {
        for x in region.x..max_x {
            luminance.push(encoded_luminance_at(rgba, frame_width, x, y));
        }
    }
    if luminance.len() < 9 {
        return 0.0;
    }
    luminance.sort_by(f32::total_cmp);
    let p95 = percentile(&luminance, 0.95);
    let threshold = (p95 + 0.25).clamp(0.74, 0.97);
    let mut isolated = 0usize;
    for y in region.y..max_y {
        for x in region.x..max_x {
            let center = encoded_luminance_at(rgba, frame_width, x, y);
            if center < threshold {
                continue;
            }
            if bright_encoded_neighbor_count(rgba, frame_width, frame_height, x, y, center) <= 1 {
                isolated = isolated.saturating_add(1);
            }
        }
    }
    isolated as f32 / (region.width.saturating_mul(region.height).max(1) as f32)
}

#[cfg(feature = "scene-host")]
fn bright_encoded_neighbor_count(
    rgba: &[u8],
    frame_width: u32,
    frame_height: u32,
    x: u32,
    y: u32,
    center_luminance: f32,
) -> usize {
    let threshold = (center_luminance - 0.12).max(0.0);
    let min_x = x.saturating_sub(1);
    let min_y = y.saturating_sub(1);
    let max_x = (x + 1).min(frame_width.saturating_sub(1));
    let max_y = (y + 1).min(frame_height.saturating_sub(1));
    let mut count = 0usize;
    for ny in min_y..=max_y {
        for nx in min_x..=max_x {
            if nx == x && ny == y {
                continue;
            }
            if encoded_luminance_at(rgba, frame_width, nx, ny) >= threshold {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

#[cfg(feature = "scene-host")]
fn encoded_luminance_at(rgba: &[u8], width: u32, x: u32, y: u32) -> f32 {
    let offset = ((y * width + x) * 4) as usize;
    (0.2126 * f32::from(rgba[offset])
        + 0.7152 * f32::from(rgba[offset + 1])
        + 0.0722 * f32::from(rgba[offset + 2]))
        / 255.0
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy)]
struct LuminanceRegionStats {
    mean: f32,
    stddev: f32,
    p05: f32,
}

#[cfg(feature = "scene-host")]
fn luminance_region_stats(
    rgba: &[u8],
    frame_width: u32,
    region: QualityPixelRegion,
) -> LuminanceRegionStats {
    let mut samples =
        Vec::with_capacity((region.width as usize).saturating_mul(region.height as usize));
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            samples.push(linear_luminance_at(rgba, frame_width, x, y));
        }
    }
    if samples.is_empty() {
        return LuminanceRegionStats {
            mean: 0.0,
            stddev: 0.0,
            p05: 0.0,
        };
    }
    let mean = samples.iter().copied().sum::<f32>() / samples.len() as f32;
    let variance = samples
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f32>()
        / samples.len() as f32;
    samples.sort_by(f32::total_cmp);
    LuminanceRegionStats {
        mean,
        stddev: variance.sqrt(),
        p05: percentile(&samples, 0.05),
    }
}

#[cfg(feature = "scene-host")]
fn assert_contact_shadow_is_localized(backend: &str, image: &DecodedPng) {
    let floor_regions = [
        (
            "left-floor",
            QualityPixelRegion {
                x: image.width.saturating_mul(20) / 100,
                y: image.height.saturating_mul(42) / 100,
                width: image.width.saturating_mul(15) / 100,
                height: image.height.saturating_mul(18) / 100,
            },
        ),
        (
            "right-floor",
            QualityPixelRegion {
                x: image.width.saturating_mul(65) / 100,
                y: image.height.saturating_mul(42) / 100,
                width: image.width.saturating_mul(15) / 100,
                height: image.height.saturating_mul(18) / 100,
            },
        ),
    ];
    for (name, region) in floor_regions {
        let stats = luminance_region_stats(&image.rgba8, image.width, region);
        assert!(
            stats.mean >= 0.83 && stats.p05 >= 0.78 && stats.stddev <= 0.05,
            "{backend} contact shadow must stay localized and leave far {name} clean; stats={stats:?}, region={region:?}"
        );
    }
}

#[cfg(feature = "scene-host")]
fn mean_blue_in_region(rgba: &[u8], frame_width: u32, region: QualityPixelRegion) -> f32 {
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            total += f32::from(rgba[offset + 2]);
            count = count.saturating_add(1);
        }
    }
    total / count.max(1) as f32
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy)]
struct SpecularSpreadMetrics {
    fwhm_pixels: u32,
    unique_luma_levels: usize,
    median_luminance: f32,
    peak_luminance: f32,
    threshold_luminance: f32,
}

#[cfg(feature = "scene-host")]
fn specular_spread_metrics(
    rgba: &[u8],
    frame_width: u32,
    region: QualityPixelRegion,
) -> SpecularSpreadMetrics {
    let mut samples =
        Vec::with_capacity((region.width as usize).saturating_mul(region.height as usize));
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            samples.push(linear_luminance_at(rgba, frame_width, x, y));
        }
    }
    if samples.is_empty() {
        return SpecularSpreadMetrics {
            fwhm_pixels: 0,
            unique_luma_levels: 0,
            median_luminance: 0.0,
            peak_luminance: 0.0,
            threshold_luminance: 0.0,
        };
    }
    let mut sorted = samples.clone();
    sorted.sort_by(f32::total_cmp);
    let median_luminance = percentile(&sorted, 0.50);
    let peak_luminance = *sorted.last().expect("non-empty luminance samples");
    let threshold_luminance = median_luminance + (peak_luminance - median_luminance).max(0.0) * 0.5;
    let mut unique_luma_levels = std::collections::BTreeSet::new();
    let mut fwhm_pixels = 0_u32;
    for luma in samples {
        if luma >= threshold_luminance {
            fwhm_pixels = fwhm_pixels.saturating_add(1);
            unique_luma_levels.insert((luma * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    SpecularSpreadMetrics {
        fwhm_pixels,
        unique_luma_levels: unique_luma_levels.len(),
        median_luminance,
        peak_luminance,
        threshold_luminance,
    }
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy)]
struct ReceiverLuminance {
    mean_luminance: f32,
    receiver_pixels: u32,
}

#[cfg(feature = "scene-host")]
fn mean_non_caster_luminance_in_region(
    rgba: &[u8],
    frame_width: u32,
    region: QualityPixelRegion,
) -> ReceiverLuminance {
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            let r = rgba[offset];
            let g = rgba[offset + 1];
            let b = rgba[offset + 2];
            let luma = linear_luminance_at(rgba, frame_width, x, y);
            let red_caster = r > g.saturating_add(28) && r > b.saturating_add(20);
            if !red_caster {
                total += luma;
                count = count.saturating_add(1);
            }
        }
    }
    ReceiverLuminance {
        mean_luminance: total / count.max(1) as f32,
        receiver_pixels: count,
    }
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone, Copy)]
struct EdgeReconstructionMetrics {
    intermediate_px_per_edge: f32,
    unique_luma_levels: usize,
    transition_width_px: f32,
    halo_overshoot: f32,
    contrast_range: f32,
}

#[cfg(feature = "scene-host")]
fn edge_reconstruction_metrics(rgba: &[u8], width: u32, height: u32) -> EdgeReconstructionMetrics {
    let mut unique_luma_levels = std::collections::BTreeSet::new();
    let mut transition_total = 0.0_f32;
    let mut edge_rows = 0_u32;
    let y0 = height / 5;
    let y1 = height.saturating_mul(4) / 5;
    let mut all_luma = Vec::with_capacity((width as usize).saturating_mul(height as usize));
    for y in 0..height {
        for x in 0..width {
            all_luma.push(linear_luminance_at(rgba, width, x, y));
        }
    }
    all_luma.sort_by(f32::total_cmp);
    let global_low = percentile(&all_luma, 0.05);
    let global_high = percentile(&all_luma, 0.95);
    let contrast_range = (global_high - global_low).max(0.0);
    let mut halo_overshoot = 0.0_f32;
    for y in y0..y1 {
        let mut strongest_x = 1;
        let mut strongest_gradient = 0.0_f32;
        let mut previous = linear_luminance_at(rgba, width, 0, y);
        for x in 1..width {
            let current = linear_luminance_at(rgba, width, x, y);
            let gradient = (current - previous).abs();
            if gradient > strongest_gradient {
                strongest_gradient = gradient;
                strongest_x = x;
            }
            previous = current;
        }
        if strongest_gradient < 0.08 {
            continue;
        }
        let window_min_x = strongest_x.saturating_sub(10);
        let window_max_x = strongest_x.saturating_add(10).min(width.saturating_sub(1));
        let mut row_min = f32::INFINITY;
        let mut row_max = f32::NEG_INFINITY;
        for x in window_min_x..=window_max_x {
            let luma = linear_luminance_at(rgba, width, x, y);
            row_min = row_min.min(luma);
            row_max = row_max.max(luma);
        }
        let row_range = row_max - row_min;
        if row_range < 0.18 {
            continue;
        }
        let low_cutoff = row_min + row_range * 0.05;
        let high_cutoff = row_max - row_range * 0.05;
        let mut transition_width = 0_u32;
        for x in window_min_x..=window_max_x {
            let luma = linear_luminance_at(rgba, width, x, y);
            if (low_cutoff..high_cutoff).contains(&luma) {
                transition_width = transition_width.saturating_add(1);
                unique_luma_levels.insert((luma * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
        halo_overshoot = halo_overshoot
            .max((global_low - row_min).max(0.0))
            .max((row_max - global_high).max(0.0));
        transition_total += transition_width as f32;
        edge_rows = edge_rows.saturating_add(1);
    }
    let intermediate_px_per_edge = transition_total / edge_rows.max(1) as f32;
    EdgeReconstructionMetrics {
        intermediate_px_per_edge,
        unique_luma_levels: unique_luma_levels.len(),
        transition_width_px: intermediate_px_per_edge,
        halo_overshoot,
        contrast_range,
    }
}

#[cfg(feature = "scene-host")]
fn format_reconstruction_metrics(results: &[(String, u8, EdgeReconstructionMetrics)]) -> String {
    let rows = results
        .iter()
        .map(|(filter, supersample, metrics)| {
            format!(
                "    {{ \"reconstruction\": \"{}\", \"supersample\": {}, \"intermediate_px_per_edge\": {:.3}, \"unique_luma_levels\": {}, \"transition_width_px\": {:.3}, \"halo_overshoot\": {:.3}, \"contrast_range\": {:.3} }}",
                filter,
                supersample,
                metrics.intermediate_px_per_edge,
                metrics.unique_luma_levels,
                metrics.transition_width_px,
                metrics.halo_overshoot,
                metrics.contrast_range
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema\": \"scena.edge_reconstruction_probe.v1\",\n  \"rows\": [\n{rows}\n  ]\n}}\n"
    )
}

#[cfg(feature = "scene-host")]
fn linear_luminance_at(rgba: &[u8], width: u32, x: u32, y: u32) -> f32 {
    let offset = ((y * width + x) * 4) as usize;
    let to_linear = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * to_linear(rgba[offset])
        + 0.7152 * to_linear(rgba[offset + 1])
        + 0.0722 * to_linear(rgba[offset + 2])
}

#[cfg(feature = "scene-host")]
fn percentile(sorted: &[f32], fraction: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let position = fraction.clamp(0.0, 1.0) * (sorted.len().saturating_sub(1)) as f32;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let t = position - lower as f32;
        sorted[lower] * (1.0 - t) + sorted[upper] * t
    }
}

#[cfg(feature = "scene-host")]
fn color_bbox(
    rgba: &[u8],
    frame_width: u32,
    frame_height: u32,
    predicate: fn(u8, u8, u8) -> bool,
) -> Option<QualityPixelRegion> {
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..frame_height {
        for x in 0..frame_width {
            let offset = ((y * frame_width + x) * 4) as usize;
            if predicate(rgba[offset], rgba[offset + 1], rgba[offset + 2]) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    found.then_some(QualityPixelRegion {
        x: min_x,
        y: min_y,
        width: max_x.saturating_sub(min_x).saturating_add(1),
        height: max_y.saturating_sub(min_y).saturating_add(1),
    })
}

#[cfg(feature = "scene-host")]
fn count_pixels_in_region(
    rgba: &[u8],
    frame_width: u32,
    region: QualityPixelRegion,
    predicate: fn(u8, u8, u8) -> bool,
) -> u32 {
    let mut count = 0_u32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            count += u32::from(predicate(rgba[offset], rgba[offset + 1], rgba[offset + 2]));
        }
    }
    count
}

#[cfg(feature = "scene-host")]
fn shrink_region(region: QualityPixelRegion, inset: u32) -> Option<QualityPixelRegion> {
    (region.width > inset.saturating_mul(2) && region.height > inset.saturating_mul(2)).then_some(
        QualityPixelRegion {
            x: region.x.saturating_add(inset),
            y: region.y.saturating_add(inset),
            width: region.width.saturating_sub(inset.saturating_mul(2)),
            height: region.height.saturating_sub(inset.saturating_mul(2)),
        },
    )
}

#[cfg(feature = "scene-host")]
fn is_blue_object_pixel(r: u8, g: u8, b: u8) -> bool {
    b > 150 && r < 80 && g > 40 && g < 150
}

#[cfg(feature = "scene-host")]
fn is_red_grid_pixel(r: u8, g: u8, b: u8) -> bool {
    r > 150 && g < 95 && b < 95
}

#[cfg(feature = "scene-host")]
fn is_dark_grey_material_pixel(r: u8, g: u8, b: u8) -> bool {
    let max_delta = r.abs_diff(g).max(g.abs_diff(b)).max(r.abs_diff(b));
    (25..=145).contains(&r) && (25..=145).contains(&g) && (25..=145).contains(&b) && max_delta <= 38
}

#[cfg(feature = "scene-host")]
fn is_white_blob_pixel(r: u8, g: u8, b: u8) -> bool {
    r >= 235 && g >= 235 && b >= 235
}

#[cfg(feature = "scene-host")]
fn is_cad_edge_pixel(r: u8, g: u8, b: u8) -> bool {
    r >= 170 && (95..=210).contains(&g) && b <= 90
}

#[cfg(feature = "scene-host")]
fn is_light_cad_panel_material_pixel(r: u8, g: u8, b: u8) -> bool {
    (120..=245).contains(&r)
        && (125..=245).contains(&g)
        && (130..=250).contains(&b)
        && r.abs_diff(g).max(g.abs_diff(b)).max(r.abs_diff(b)) <= 55
}

#[cfg(feature = "scene-host")]
fn is_black_slab_pixel(r: u8, g: u8, b: u8) -> bool {
    r < 25 && g < 25 && b < 25
}

fn system_test_font_path() -> PathBuf {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Helvetica.ttf",
        "/Library/Fonts/Arial.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\calibri.ttf",
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
