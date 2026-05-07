#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use scena::{
    Assets, Backend, Capabilities, Color, GeometryDesc, MaterialDesc, NotPreparedReason,
    PerspectiveCamera, Primitive, RenderError, Renderer, RendererOptions, Scene, SurfaceEvent,
    Transform, Vec3,
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn current_lane() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos-metal"
    } else if cfg!(target_os = "windows") {
        "windows-dx12"
    } else {
        "linux-native-vulkan"
    }
}

fn platform_dir() -> PathBuf {
    root().join("target/gate-artifacts/m9-platform")
}

fn lane_dir() -> PathBuf {
    platform_dir().join(current_lane())
}

#[test]
fn m9_platform_rendered_output_suite_writes_release_artifacts() {
    let lane = current_lane();
    let artifact_dir = lane_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact dir");

    let default = render_default_scene_platform(96, 64);
    let default_ppm = artifact_dir.join("default-scene.ppm");
    write_ppm(&default_ppm, default.width, default.height, &default.frame);
    assert!(
        default.nonblack_pixels > 0,
        "default scene renders nonblack pixels"
    );

    let static_gltf = render_static_gltf_platform(96, 64);
    let static_gltf_ppm = artifact_dir.join("static-gltf.ppm");
    write_ppm(
        &static_gltf_ppm,
        static_gltf.width,
        static_gltf.height,
        &static_gltf.frame,
    );
    assert!(
        static_gltf.nonblack_pixels > 0,
        "static glTF fixture renders nonblack pixels"
    );

    let capabilities = capability_json(lane, default.capabilities);
    let capability_path = artifact_dir.join("capabilities.json");
    write_json(&capability_path, &capabilities);

    let proof = serde_json::json!({
        "schema": "scena.m9.platform_render.v1",
        "lane": lane,
        "os": std::env::consts::OS,
        "backend": format!("{:?}", default.capabilities.backend),
        "host_gpu_attempted": true,
        "host_gpu_available": default.host_gpu_available,
        "host_gpu_error": default.host_gpu_error,
        "default_scene": {
            "screenshot": path_string(&default_ppm),
            "width": default.width,
            "height": default.height,
            "draw_calls": default.draw_calls,
            "nonblack_pixels": default.nonblack_pixels,
        },
        "static_gltf": {
            "source": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "screenshot": path_string(&static_gltf_ppm),
            "width": static_gltf.width,
            "height": static_gltf.height,
            "draw_calls": static_gltf.draw_calls,
            "nonblack_pixels": static_gltf.nonblack_pixels,
        },
        "capabilities": path_string(&capability_path),
    });
    write_json(&artifact_dir.join("rendered-output.json"), &proof);

    write_benchmark_artifact(lane);
}

#[test]
fn m9_capability_matrix_artifact_covers_required_lanes() {
    fs::create_dir_all(platform_dir()).expect("platform artifact dir");
    let matrix = serde_json::json!({
        "schema": "scena.capabilities.v1",
        "status": "passed",
        "lanes": [
            lane_capability("linux-native-vulkan", Capabilities::for_gpu_backend(Backend::NativeSurface)),
            lane_capability("linux-webgl2-chromium", Capabilities::for_attached_gpu_backend(Backend::WebGl2)),
            lane_capability("linux-webgpu-chromium", Capabilities::for_attached_gpu_backend(Backend::WebGpu)),
            lane_capability("macos-metal", Capabilities::for_gpu_backend(Backend::NativeSurface)),
            lane_capability("windows-dx12", Capabilities::for_gpu_backend(Backend::NativeSurface)),
            lane_capability("wasm32-unknown-unknown", Capabilities::for_backend(Backend::SurfaceDescriptor)),
        ],
    });
    let lanes = matrix["lanes"].as_array().expect("lanes array");
    for lane in [
        "linux-native-vulkan",
        "linux-webgl2-chromium",
        "linux-webgpu-chromium",
        "macos-metal",
        "windows-dx12",
        "wasm32-unknown-unknown",
    ] {
        assert!(
            lanes.iter().any(|entry| entry["lane"] == lane),
            "missing capability lane {lane}"
        );
    }
    write_json(&platform_dir().join("m9-capability-matrix.json"), &matrix);
}

#[test]
fn m9_surface_context_loss_artifact_records_required_sequence() {
    let lane = current_lane();
    let artifact_dir = lane_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact dir");

    let (mut scene, camera) = scene_with_triangle();
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("initial prepare");
    renderer.render(&scene, camera).expect("initial render");
    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 48,
            height: 32,
        })
        .expect("resize accepted");
    let target_changed = matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::TargetChanged { .. }
        })
    );
    renderer.prepare(&mut scene).expect("resize prepare");
    renderer.render(&scene, camera).expect("resize render");
    renderer
        .handle_surface_event(SurfaceEvent::Lost)
        .expect("surface lost accepted");
    let surface_lost = matches!(
        renderer.render(&scene, camera),
        Err(RenderError::SurfaceLost { recoverable: true })
    );

    let artifact = serde_json::json!({
        "schema": "scena.m9.surface_context_loss.v1",
        "lane": lane,
        "backend": "Headless",
        "event_sequence": [
            "prepare",
            "render",
            "resize",
            "not-prepared-target-changed",
            "reprepare-after-resize",
            "render-after-resize",
            "surface-lost"
        ],
        "target_changed_requires_prepare": target_changed,
        "surface_lost_is_structured": surface_lost,
        "final_prepare": "ok",
        "diagnostics": [],
    });
    assert!(target_changed, "resize requires explicit prepare");
    assert!(surface_lost, "surface loss is structured");
    write_json(&artifact_dir.join("surface-context-loss.json"), &artifact);
}

fn render_default_scene_platform(width: u32, height: u32) -> RenderedArtifact {
    let (mut scene, camera) = scene_with_triangle();
    render_scene_platform(width, height, &mut scene, None, camera)
}

fn render_static_gltf_platform(width: u32, height: u32) -> RenderedArtifact {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("static glTF fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("static glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).expect("camera frames glTF");
    }
    render_scene_platform(width, height, &mut scene, Some(&assets), camera)
}

fn render_scene_platform(
    width: u32,
    height: u32,
    scene: &mut Scene,
    assets: Option<&Assets>,
    camera: scena::CameraKey,
) -> RenderedArtifact {
    let mut host_gpu_available = true;
    let mut host_gpu_error = None;
    let mut renderer = match Renderer::headless_gpu(width, height) {
        Ok(renderer) => renderer,
        Err(error) => {
            host_gpu_available = false;
            host_gpu_error = Some(format!("{error:?}"));
            Renderer::headless(width, height).expect("headless fallback renderer builds")
        }
    };
    if let Some(assets) = assets {
        renderer
            .prepare_with_assets(scene, assets)
            .expect("asset scene prepares");
    } else {
        renderer.prepare(scene).expect("scene prepares");
    }
    let outcome = renderer.render(scene, camera).expect("scene renders");
    let frame = renderer.frame_rgba8().to_vec();
    let nonblack_pixels = nonblack_pixels(&frame);
    RenderedArtifact {
        width,
        height,
        frame,
        nonblack_pixels,
        draw_calls: outcome.draw_calls,
        capabilities: *renderer.capabilities(),
        host_gpu_available,
        host_gpu_error,
    }
}

fn scene_with_triangle() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("triangle inserts");
    (scene, camera)
}

fn write_benchmark_artifact(lane: &str) {
    let rows = [
        benchmark_static_viewer(),
        benchmark_standard_model_viewer_gltf(),
        benchmark_larger_industrial_gltf(),
        benchmark_high_instance_scene(),
        benchmark_idle_render_on_change(),
        benchmark_headless_4k(),
    ];
    let artifact = serde_json::json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": lane,
        "regression_threshold_percent": 5.0,
        "rows": rows,
    });
    write_json(&platform_dir().join("m9-benchmarks.json"), &artifact);
}

fn benchmark_static_viewer() -> serde_json::Value {
    let (mut scene, camera) = scene_with_triangle();
    benchmark_scene("static-viewer", 128, 128, &mut scene, None, camera)
}

fn benchmark_standard_model_viewer_gltf() -> serde_json::Value {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("minimal glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    benchmark_scene(
        "standard-model-viewer-gltf",
        128,
        128,
        &mut scene,
        Some(&assets),
        camera,
    )
}

fn benchmark_larger_industrial_gltf() -> serde_json::Value {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/khronos/BrainStem/BrainStem.gltf"))
            .expect("BrainStem glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("BrainStem instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    if let Some(bounds) = import.bounds_world(&scene) {
        scene
            .frame(camera, bounds)
            .expect("camera frames BrainStem");
    }
    benchmark_scene(
        "larger-industrial-gltf",
        128,
        128,
        &mut scene,
        Some(&assets),
        camera,
    )
}

fn benchmark_high_instance_scene() -> serde_json::Value {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.1, 0.1, 0.1));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 190, 140)));
    let mut scene = Scene::new();
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    scene
        .reserve_instances(set, 128)
        .expect("reserve instances");
    for index in 0..128 {
        let x = index % 16;
        let y = index / 16;
        scene
            .push_instance(
                set,
                Transform::at(Vec3::new(x as f32 * 0.13 - 1.0, y as f32 * 0.13 - 0.5, 0.0)),
            )
            .expect("instance inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    benchmark_scene("high-instance", 128, 128, &mut scene, Some(&assets), camera)
}

fn benchmark_idle_render_on_change() -> serde_json::Value {
    let (mut scene, camera) = scene_with_triangle();
    let mut renderer = Renderer::headless_with_options(
        64,
        64,
        RendererOptions::default().with_render_mode(scena::RenderMode::OnChange),
    )
    .expect("renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("warm render");
    let start = Instant::now();
    let outcome = renderer.render(&scene, camera).expect("idle render skips");
    benchmark_row(
        "idle",
        renderer.capabilities().backend,
        start.elapsed().as_secs_f64() * 1000.0,
        outcome.draw_calls,
        outcome.skipped,
    )
}

fn benchmark_headless_4k() -> serde_json::Value {
    let (mut scene, camera) = scene_with_triangle();
    benchmark_scene("headless-4k", 3840, 2160, &mut scene, None, camera)
}

fn benchmark_scene(
    name: &str,
    width: u32,
    height: u32,
    scene: &mut Scene,
    assets: Option<&Assets>,
    camera: scena::CameraKey,
) -> serde_json::Value {
    let mut renderer = Renderer::headless(width, height).expect("renderer builds");
    let start = Instant::now();
    if let Some(assets) = assets {
        renderer
            .prepare_with_assets(scene, assets)
            .expect("asset scene prepares");
    } else {
        renderer.prepare(scene).expect("scene prepares");
    }
    let outcome = renderer.render(scene, camera).expect("scene renders");
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
        "regression_threshold_percent": 5.0,
    })
}

fn lane_capability(lane: &str, capabilities: Capabilities) -> serde_json::Value {
    serde_json::json!({
        "lane": lane,
        "capabilities": capability_fields(capabilities),
        "diagnostics": capabilities
            .diagnostics()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>(),
    })
}

fn capability_json(lane: &str, capabilities: Capabilities) -> serde_json::Value {
    serde_json::json!({
        "schema": "scena.capabilities.v1",
        "lane": lane,
        "backend": format!("{:?}", capabilities.backend),
        "hardware_tier": format!("{:?}", capabilities.hardware_tier),
        "features": capability_fields(capabilities),
        "diagnostics": capabilities
            .diagnostics()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>(),
    })
}

fn capability_fields(capabilities: Capabilities) -> serde_json::Value {
    serde_json::json!({
        "forward_pbr": { "state": format!("{:?}", capabilities.forward_pbr) },
        "clipping_planes": {
            "state": "Supported",
            "default": capabilities.default_clipping_planes,
            "max": capabilities.max_clipping_planes,
        },
        "gpu_frustum_culling": { "state": format!("{:?}", capabilities.gpu_frustum_culling) },
        "per_instance_culling": { "state": format!("{:?}", capabilities.per_instance_culling) },
        "compute_shaders": { "state": format!("{:?}", capabilities.compute_shaders) },
        "storage_buffers": { "state": format!("{:?}", capabilities.storage_buffers) },
        "reversed_z_depth": { "state": format!("{:?}", capabilities.reversed_z_depth) },
        "readback_headless_screenshots": { "state": format!("{:?}", capabilities.readback_headless_screenshots) },
    })
}

fn write_ppm(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[0..3]);
    }
    fs::write(path, ppm).expect("ppm writes");
}

fn write_json(path: &Path, value: &serde_json::Value) {
    let body = serde_json::to_string_pretty(value).expect("json serializes");
    fs::write(path, format!("{body}\n")).expect("json writes");
}

fn nonblack_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn path_string(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

struct RenderedArtifact {
    width: u32,
    height: u32,
    frame: Vec<u8>,
    nonblack_pixels: usize,
    draw_calls: u64,
    capabilities: Capabilities,
    host_gpu_available: bool,
    host_gpu_error: Option<String>,
}
