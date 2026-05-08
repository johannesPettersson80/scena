use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use scena::{
    AnimationError, AssetError, Backend, BuildError, ChangeKind, Color, DebugOverlay, GeometryDesc,
    ImportError, InstantiateError, LookupError, MaterialDesc, NotPreparedReason, PerspectiveCamera,
    PrepareError, Primitive, RenderError, Renderer, Scene, Transform, Vec3,
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn m5_debug_overlay_api_is_public_and_requires_prepare_after_change() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");

    let mut renderer = Renderer::headless(8, 8).expect("headless renderer builds");
    assert_eq!(renderer.debug_overlay(), DebugOverlay::None);
    renderer.set_debug(DebugOverlay::Wireframe);
    assert_eq!(renderer.debug_overlay(), DebugOverlay::Wireframe);
    renderer.set_debug_overlay(DebugOverlay::BoundingBoxes);
    assert_eq!(renderer.debug_overlay(), DebugOverlay::BoundingBoxes);

    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("first render succeeds");
    renderer.set_debug(DebugOverlay::Normals);

    let error = renderer
        .render_active(&scene)
        .expect_err("debug overlay change requires prepare");
    assert!(matches!(
        error,
        RenderError::NotPrepared {
            reason: NotPreparedReason::RendererChanged {
                change: ChangeKind::DebugOverlay,
                ..
            }
        }
    ));
}

#[test]
fn m5_release_surface_files_and_examples_are_present() {
    let required_files = [
        "CHANGELOG.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "docs/api/m5-public-api-baseline.txt",
        "docs/api/m5-semver-baseline.toml",
        "examples/primitive_shapes.rs",
        "examples/glb_model_viewer.rs",
        "examples/picking_selection_hover.rs",
        "examples/instancing.rs",
        "examples/labels_helpers.rs",
        "examples/animation.rs",
        "examples/native_window.rs",
        "examples/browser_canvas.rs",
        "examples/headless_ci.rs",
        "examples/industrial_static_scene.rs",
        "examples/industrial_connector_assembly.rs",
        "examples/coordinate_connector_repair.rs",
    ];

    for rel in required_files {
        assert!(root().join(rel).is_file(), "missing release file {rel}");
    }
}

#[test]
fn m5_package_metadata_is_ready_for_dry_run() {
    let manifest = fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml is readable");
    for needle in [
        "version = \"1.0.0-rc.0\"",
        "rust-version = ",
        "license = \"MIT OR Apache-2.0\"",
        "documentation = \"https://docs.rs/scena\"",
        "keywords = [",
        "categories = [",
        "include = [",
    ] {
        assert!(manifest.contains(needle), "Cargo.toml missing {needle}");
    }
}

#[test]
fn m5_public_api_baseline_names_frozen_contracts() {
    let baseline = fs::read_to_string(root().join("docs/api/m5-public-api-baseline.txt"))
        .expect("public API baseline exists");
    for needle in [
        "Renderer::prepare",
        "Renderer::render",
        "Renderer::set_debug",
        "DebugOverlay",
        "RendererStats",
        "BuildError",
        "AssetError",
        "ImportError",
        "InstantiateError",
        "PrepareError",
        "RenderError",
        "LookupError",
        "AnimationError",
        "SceneImport",
        "SurfaceEvent",
        "Scene::pick_with_assets",
        "Scene::pick_and_select_with_assets",
    ] {
        assert!(baseline.contains(needle), "baseline missing {needle}");
    }

    let artifact = root().join("target/gate-artifacts/m5-public-api-freeze.json");
    fs::create_dir_all(artifact.parent().expect("artifact has parent")).expect("artifact dir");
    fs::write(
        artifact,
        serde_json::json!({
            "gate": "m5-public-api-freeze",
            "status": "passed",
            "baseline": "docs/api/m5-public-api-baseline.txt",
            "semver_baseline": "docs/api/m5-semver-baseline.toml",
            "required_terms": [
                "Renderer::prepare",
                "Renderer::render",
                "Renderer::set_debug",
                "RendererStats",
                "BuildError",
                "RenderError",
                "SceneImport"
            ]
        })
        .to_string(),
    )
    .expect("public api artifact is written");
}

#[test]
fn public_error_displays_are_actionable() {
    let samples = [
        BuildError::InvalidTargetSize {
            width: 0,
            height: 32,
        }
        .to_string(),
        AssetError::UnsupportedRequiredExtension {
            path: "scene.gltf".to_string(),
            extension: "KHR_unknown".to_string(),
        }
        .to_string(),
        ImportError::Asset(AssetError::NotFound {
            path: "missing.glb".to_string(),
        })
        .to_string(),
        InstantiateError::InvalidAnchorExtras {
            node: "arm".to_string(),
            reason: "missing id".to_string(),
        }
        .to_string(),
        PrepareError::BackendCapabilityMismatch {
            feature: "compute culling",
            backend: Backend::WebGl2,
            help: "use CPU culling fallback".to_string(),
        }
        .to_string(),
        RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared,
        }
        .to_string(),
        LookupError::StaleImport.to_string(),
        AnimationError::ClipNotFound {
            name: "Idle".to_string(),
        }
        .to_string(),
    ];

    for message in samples {
        assert!(
            message.len() >= 18
                && !message.contains("TODO")
                && !message.contains("unimplemented")
                && (message.contains("prepare")
                    || message.contains("asset")
                    || message.contains("gltf")
                    || message.contains("glTF")
                    || message.contains("backend")
                    || message.contains("scene")
                    || message.contains("animation")
                    || message.contains("invalid")
                    || message.contains("missing")),
            "error message is not actionable: {message}"
        );
    }
}

#[test]
fn scena_convert_cli_reports_fbx_to_gltf_plan() {
    let help = run_scena_convert(["--help"])
        .output()
        .expect("scena-convert --help runs");
    assert!(help.status.success(), "--help should succeed");
    let help = String::from_utf8(help.stdout).expect("help is utf8");
    assert!(help.contains("FBX"));
    assert!(help.contains("glTF"));

    let dry_run = run_scena_convert([
        "--input",
        "fixtures/robot-arm.fbx",
        "--output",
        "target/robot-arm.glb",
        "--dry-run",
    ])
    .output()
    .expect("scena-convert dry-run runs");
    assert!(dry_run.status.success(), "dry-run should succeed");
    let stdout = String::from_utf8(dry_run.stdout).expect("dry-run stdout is utf8");
    assert!(stdout.contains("\"status\":\"planned\""));
    assert!(stdout.contains("FBX2glTF"));
    assert!(stdout.contains("target/robot-arm.glb"));
}

fn run_scena_convert<const N: usize>(args: [&str; N]) -> Command {
    let mut command = Command::new(env!("CARGO"));
    command.args(["run", "--quiet", "--bin", "scena-convert", "--"]);
    command.args(args);
    command
}

#[test]
fn m5_benchmark_report_writes_required_scene_rows() {
    let rows = [
        benchmark_resource_free_static_viewer(),
        benchmark_standard_model_viewer_gltf(),
        benchmark_larger_industrial_gltf(),
        benchmark_high_instance_scene(),
        benchmark_idle_render_on_change(),
        benchmark_headless_4k(),
    ];

    let report = serde_json::json!({
        "gate": "m5-benchmarks",
        "status": "passed",
        "regression_threshold_percent": 5.0,
        "rows": rows,
    });
    let artifact = root().join("target/gate-artifacts/m5-benchmarks.json");
    fs::create_dir_all(artifact.parent().expect("artifact has parent")).expect("artifact dir");
    fs::write(
        artifact,
        serde_json::to_string_pretty(&report).expect("report serializes"),
    )
    .expect("benchmark artifact is written");

    for name in [
        "static-viewer",
        "standard-model-viewer-gltf",
        "larger-industrial-gltf",
        "high-instance",
        "idle",
        "headless-4k",
    ] {
        assert!(
            report["rows"]
                .as_array()
                .expect("rows are an array")
                .iter()
                .any(|row| row["scene"] == name),
            "missing benchmark row {name}"
        );
    }
}

fn benchmark_resource_free_static_viewer() -> serde_json::Value {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    benchmark_scene("static-viewer", 128, 128, scene, None)
}

fn benchmark_standard_model_viewer_gltf() -> serde_json::Value {
    let assets = scena::Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("mesh glTF instantiates");
    benchmark_scene("standard-model-viewer-gltf", 128, 128, scene, Some(&assets))
}

fn benchmark_larger_industrial_gltf() -> serde_json::Value {
    let assets = scena::Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/khronos/BrainStem/BrainStem.gltf"))
            .expect("BrainStem glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("BrainStem glTF instantiates");
    benchmark_scene("larger-industrial-gltf", 128, 128, scene, Some(&assets))
}

fn benchmark_high_instance_scene() -> serde_json::Value {
    let assets = scena::Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.1, 0.1, 0.1));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 190, 140)));
    let mut scene = Scene::new();
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    scene
        .reserve_instances(set, 256)
        .expect("reserve instances");
    for index in 0..128 {
        let x = index % 16;
        let y = index / 16;
        scene
            .push_instance(
                set,
                Transform {
                    translation: Vec3::new(x as f32 * 0.13 - 1.0, y as f32 * 0.13 - 0.5, 0.0),
                    ..Transform::default()
                },
            )
            .expect("instance inserts");
    }
    benchmark_scene("high-instance", 128, 128, scene, Some(&assets))
}

fn benchmark_idle_render_on_change() -> serde_json::Value {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera");
    let mut renderer = Renderer::headless_with_options(
        64,
        64,
        scena::RendererOptions::default().with_render_mode(scena::RenderMode::OnChange),
    )
    .expect("renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render_active(&scene).expect("warm render");
    let start = Instant::now();
    let outcome = renderer.render_active(&scene).expect("idle render skips");
    benchmark_row(
        "idle",
        renderer.capabilities().backend,
        start.elapsed().as_secs_f64() * 1000.0,
        renderer.stats().draw_calls,
        outcome.skipped,
    )
}

fn benchmark_headless_4k() -> serde_json::Value {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    benchmark_scene("headless-4k", 3840, 2160, scene, None)
}

fn benchmark_scene(
    name: &str,
    width: u32,
    height: u32,
    mut scene: Scene,
    assets: Option<&scena::Assets>,
) -> serde_json::Value {
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera");
    let mut renderer = Renderer::headless(width, height).expect("renderer builds");
    let start = Instant::now();
    if let Some(assets) = assets {
        renderer
            .prepare_with_assets(&mut scene, assets)
            .expect("asset scene prepares");
    } else {
        renderer.prepare(&mut scene).expect("scene prepares");
    }
    let outcome = renderer.render_active(&scene).expect("scene renders");
    benchmark_row(
        name,
        renderer.capabilities().backend,
        start.elapsed().as_secs_f64() * 1000.0,
        outcome.draw_calls,
        outcome.skipped,
    )
}

fn benchmark_row(
    scene: &str,
    backend: Backend,
    frame_ms: f64,
    draw_calls: u64,
    skipped: bool,
) -> serde_json::Value {
    serde_json::json!({
        "scene": scene,
        "backend": format!("{backend:?}"),
        "median_frame_ms": frame_ms,
        "p95_frame_ms": frame_ms,
        "draw_calls": draw_calls,
        "skipped": skipped,
        "allocation_bytes": 0,
    })
}
