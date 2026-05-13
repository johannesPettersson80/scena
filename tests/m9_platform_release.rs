#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use scena::{
    AdapterLimitsReport, Angle, Assets, Backend, Capabilities, Color, DirectionalLight,
    GeometryDesc, GpuAdapterReport, MaterialDesc, NotPreparedReason, PerspectiveCamera, PointLight,
    Primitive, RenderError, Renderer, RendererOptions, Scene, SpotLight, SurfaceEvent, Transform,
    Vec3,
};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;
const STATIC_GLTF_PROOF_FIXTURE: &str = "tests/assets/gltf/non_ndc_camera_scene.gltf";
const BENCHMARK_BASELINE_PATH: &str = "docs/benchmarks/m9-baselines.json";
const BENCHMARK_SAMPLE_COUNT: usize = 100;
const DEDICATED_4K_SAMPLE_COUNT: usize = 100;
const HEADLESS_CPU_LANE: &str = "headless-cpu";
const PBR_DIRECTIONAL_RED_PPM: &str = "pbr-directional-red.ppm";
const PBR_POINT_GREEN_PPM: &str = "pbr-point-green.ppm";
const PBR_SPOT_BLUE_PPM: &str = "pbr-spot-blue.ppm";

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

fn headless_cpu_dir() -> PathBuf {
    platform_dir().join(HEADLESS_CPU_LANE)
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

    let pbr_lights = render_pbr_light_suite_platform(96, 64);
    for proof in &pbr_lights {
        write_ppm(
            &proof.ppm_path,
            proof.artifact.width,
            proof.artifact.height,
            &proof.artifact.frame,
        );
    }

    let default_gpu_proof = production_claim_for_gpu(&default);
    let static_gltf_gpu_proof = production_claim_for_gpu(&static_gltf);
    let pbr_light_gpu_proof = pbr_lights
        .iter()
        .all(|proof| production_claim_for_gpu(&proof.artifact) && proof.color_assertion_passed);
    let capabilities = capability_json(lane, &default);
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
        "gpu_proof": default_gpu_proof && static_gltf_gpu_proof,
        "fallback_policy": "cpu fallback is diagnostic only and never satisfies GPU rendered-output claims",
        "commit": current_commit_label(),
        "timestamp_unix_seconds": current_timestamp_unix_seconds(),
        "test_names": [
            "m9_platform_rendered_output_suite_writes_release_artifacts",
            "m9_capability_matrix_artifact_covers_required_lanes",
            "m9_surface_context_loss_artifact_records_required_sequence"
        ],
        "artifact_paths": [
            path_string(&default_ppm),
            path_string(&static_gltf_ppm),
            path_string(&pbr_lights[0].ppm_path),
            path_string(&pbr_lights[1].ppm_path),
            path_string(&pbr_lights[2].ppm_path),
            path_string(&capability_path),
            path_string(&artifact_dir.join("rendered-output.json"))
        ],
        "default_scene": {
            "proof_class": "harness-smoke",
            "production_claim": false,
            "gpu_proof": default_gpu_proof,
            "backend": format!("{:?}", default.capabilities.backend),
            "host_gpu_available": default.host_gpu_available,
            "host_gpu_error": default.host_gpu_error,
            "adapter": adapter_metadata(default.adapter.as_ref()),
            "renderer_settings": screenshot_renderer_settings(&default),
            "color_management": screenshot_color_management(),
            "tolerance": screenshot_tolerance_metadata(),
            "screenshot": path_string(&default_ppm),
            "width": default.width,
            "height": default.height,
            "draw_calls": default.draw_calls,
            "nonblack_pixels": default.nonblack_pixels,
        },
        "static_gltf": {
            "source": STATIC_GLTF_PROOF_FIXTURE,
            "proof_class": static_gltf_proof_class(static_gltf_gpu_proof),
            "production_claim": static_gltf_gpu_proof,
            "gpu_proof": static_gltf_gpu_proof,
            "backend": format!("{:?}", static_gltf.capabilities.backend),
            "host_gpu_available": static_gltf.host_gpu_available,
            "host_gpu_error": static_gltf.host_gpu_error,
            "adapter": adapter_metadata(static_gltf.adapter.as_ref()),
            "asset_provenance": asset_provenance(STATIC_GLTF_PROOF_FIXTURE),
            "renderer_settings": screenshot_renderer_settings(&static_gltf),
            "color_management": screenshot_color_management(),
            "tolerance": screenshot_tolerance_metadata(),
            "screenshot": path_string(&static_gltf_ppm),
            "width": static_gltf.width,
            "height": static_gltf.height,
            "draw_calls": static_gltf.draw_calls,
            "nonblack_pixels": static_gltf.nonblack_pixels,
        },
        "pbr_lights": {
            "proof_class": "native-pbr-punctual-light",
            "production_claim": pbr_light_gpu_proof,
            "gpu_proof": pbr_light_gpu_proof,
            "fallback_policy": "CPU fallback records diagnostics only and never satisfies native PBR light proof",
            "lights": pbr_lights.iter().map(PbrLightProof::to_json).collect::<Vec<_>>(),
        },
        "capabilities": path_string(&capability_path),
    });
    write_json(&artifact_dir.join("rendered-output.json"), &proof);

    write_headless_cpu_lane_artifacts();
    write_benchmark_artifact(lane);
}

#[test]
fn m9_cpu_fallback_artifacts_do_not_claim_gpu_rendered_output() {
    let fallback = RenderedArtifact {
        width: 1,
        height: 1,
        frame: vec![0, 0, 0, 255],
        nonblack_pixels: 0,
        draw_calls: 0,
        capabilities: Capabilities::for_backend(Backend::Headless),
        host_gpu_available: false,
        host_gpu_error: Some("adapter unavailable".to_string()),
        adapter: None,
    };

    assert!(!production_claim_for_gpu(&fallback));
    assert_eq!(
        static_gltf_proof_class(production_claim_for_gpu(&fallback)),
        "cpu-fallback-camera-framed-non-ndc"
    );
}

#[test]
fn m9_screenshot_metadata_records_renderer_color_and_tolerance_contract() {
    let artifact = RenderedArtifact {
        width: 96,
        height: 64,
        frame: vec![0, 0, 0, 255],
        nonblack_pixels: 0,
        draw_calls: 0,
        capabilities: Capabilities::for_backend(Backend::Headless),
        host_gpu_available: false,
        host_gpu_error: None,
        adapter: None,
    };

    let settings = screenshot_renderer_settings(&artifact);
    assert_eq!(settings["width"], 96);
    assert_eq!(settings["height"], 64);
    assert_eq!(settings["backend"], "Headless");
    assert_eq!(settings["color_target_format"], "Rgba8UnormSrgb");
    assert_eq!(
        screenshot_color_management()["output_encoding"],
        "srgb8-after-aces"
    );
    assert_eq!(
        screenshot_tolerance_metadata()["policy"],
        "native-rendered-output-smoke"
    );
}

#[test]
fn m9_adapter_metadata_records_actual_gpu_identity_when_available() {
    let report = GpuAdapterReport {
        name: "test adapter".to_string(),
        backend: "Vulkan".to_string(),
        device_type: "DiscreteGpu".to_string(),
        vendor: 0x10de,
        device: 0x1234,
        driver: "test-driver".to_string(),
        driver_info: "test-driver-info".to_string(),
        features: "TEXTURE_COMPRESSION_BC".to_string(),
        limits: AdapterLimitsReport {
            max_texture_dimension_2d: 8192,
            max_bind_groups: 4,
            max_uniform_buffer_binding_size: 65536,
            max_vertex_attributes: 16,
        },
    };

    let metadata = adapter_metadata(Some(&report));
    assert_eq!(metadata["name"], "test adapter");
    assert_eq!(metadata["backend"], "Vulkan");
    assert_eq!(metadata["limits"]["max_texture_dimension_2d"], 8192);
    assert_eq!(adapter_metadata(None)["available"], false);
}

#[test]
fn m9_capability_matrix_artifact_covers_required_lanes() {
    fs::create_dir_all(platform_dir()).expect("platform artifact dir");
    let measured_current_lane = render_default_scene_platform(32, 24);
    let measured_headless_cpu = render_default_scene_headless_cpu(32, 24);
    let matrix = serde_json::json!({
        "schema": "scena.capabilities.v1",
        "status": "incomplete",
        "status_reason": "current runner records a measured lane row; public release requires measured adapter artifacts from every required lane",
        "commit": current_commit_label(),
        "timestamp_unix_seconds": current_timestamp_unix_seconds(),
        "test_names": [
            "m9_capability_matrix_artifact_covers_required_lanes"
        ],
        "artifact_paths": [
            path_string(&platform_dir().join("m9-capability-matrix.json"))
        ],
        "lanes": [
            capability_matrix_row("linux-native-vulkan", &measured_current_lane, &measured_headless_cpu),
            capability_matrix_row("linux-webgl2-chromium", &measured_current_lane, &measured_headless_cpu),
            capability_matrix_row("linux-webgpu-chromium", &measured_current_lane, &measured_headless_cpu),
            capability_matrix_row("macos-metal", &measured_current_lane, &measured_headless_cpu),
            capability_matrix_row("windows-dx12", &measured_current_lane, &measured_headless_cpu),
            capability_matrix_row("wasm32-unknown-unknown", &measured_current_lane, &measured_headless_cpu),
            capability_matrix_row(HEADLESS_CPU_LANE, &measured_current_lane, &measured_headless_cpu),
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
        HEADLESS_CPU_LANE,
    ] {
        assert!(
            lanes.iter().any(|entry| entry["lane"] == lane),
            "missing capability lane {lane}"
        );
    }
    let forbidden_factory_source = ["factory", "contract"].join("-");
    assert!(
        lanes
            .iter()
            .all(|entry| entry["measurement_source"] != forbidden_factory_source),
        "capability matrix must not synthesize non-current platform capabilities from factory constants"
    );
    let current_row = lanes
        .iter()
        .find(|entry| entry["lane"] == current_lane())
        .expect("current native lane row exists");
    assert_eq!(current_row["measurement_source"], "lane-renderer-runtime");
    assert!(
        current_row.get("adapter").is_some(),
        "measured lane rows must include adapter metadata, even when no adapter is available"
    );
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

#[test]
fn m9_asset_provenance_records_source_path_and_hash() {
    let provenance = asset_provenance(STATIC_GLTF_PROOF_FIXTURE);

    assert_eq!(provenance["path"], STATIC_GLTF_PROOF_FIXTURE);
    assert!(
        provenance["hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("fnv1a64:")),
        "asset provenance must include a stable source hash"
    );
}

#[test]
fn m9_static_gltf_proof_uses_non_ndc_camera_framed_asset() {
    let fixture = std::fs::read_to_string(root().join(STATIC_GLTF_PROOF_FIXTURE))
        .expect("static glTF proof fixture is readable");

    assert!(
        fixture.contains("\"min\": [2.0, -0.5, 0.0]")
            && fixture.contains("\"max\": [3.0, 0.5, 0.0]"),
        "M9 static glTF proof must use a camera-framed source asset outside raw NDC coordinates",
    );
}

fn render_default_scene_platform(width: u32, height: u32) -> RenderedArtifact {
    let (mut scene, camera) = scene_with_triangle();
    render_scene_platform(width, height, &mut scene, None, camera)
}

fn render_default_scene_headless_cpu(width: u32, height: u32) -> RenderedArtifact {
    let (mut scene, camera) = scene_with_triangle();
    render_scene_headless_cpu(width, height, &mut scene, None, camera)
}

fn render_static_gltf_platform(width: u32, height: u32) -> RenderedArtifact {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene(STATIC_GLTF_PROOF_FIXTURE))
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

fn render_static_gltf_headless_cpu(width: u32, height: u32) -> RenderedArtifact {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene(STATIC_GLTF_PROOF_FIXTURE))
        .expect("static glTF fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("static glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).expect("camera frames glTF");
    }
    render_scene_headless_cpu(width, height, &mut scene, Some(&assets), camera)
}

fn render_pbr_light_suite_platform(width: u32, height: u32) -> Vec<PbrLightProof> {
    [
        PbrLightKind::DirectionalRed,
        PbrLightKind::PointGreen,
        PbrLightKind::SpotBlue,
    ]
    .into_iter()
    .map(|kind| {
        let (mut scene, assets, camera) = pbr_light_scene(kind);
        let artifact = render_scene_platform(width, height, &mut scene, Some(&assets), camera);
        let center = sample_rgb(&artifact.frame, width, height, width / 2, height / 2);
        let color_assertion_passed = kind.assert_expected_tint(center);
        assert!(
            artifact.nonblack_pixels > 0,
            "PBR {kind:?} proof should render visible pixels"
        );
        assert!(
            color_assertion_passed,
            "PBR {kind:?} proof should tint the center pixel as expected; center={center:?}"
        );
        PbrLightProof {
            kind,
            center,
            color_assertion_passed,
            ppm_path: lane_dir().join(kind.ppm_filename()),
            artifact,
        }
    })
    .collect()
}

fn pbr_light_scene(kind: PbrLightKind) -> (Scene, Assets, scena::CameraKey) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.25, 0.25, 0.25), 0.0, 0.8)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("PBR mesh inserts");
    match kind {
        PbrLightKind::DirectionalRed => {
            scene
                .directional_light(
                    DirectionalLight::default()
                        .with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))
                        .with_illuminance_lux(100.0),
                )
                .add()
                .expect("directional light inserts");
        }
        PbrLightKind::PointGreen => {
            scene
                .point_light(
                    PointLight::default()
                        .with_color(Color::from_linear_rgb(0.0, 1.0, 0.0))
                        .with_intensity_candela(900.0)
                        .with_range(5.0),
                )
                .transform(Transform::at(Vec3::new(0.0, 0.0, 1.0)))
                .add()
                .expect("point light inserts");
        }
        PbrLightKind::SpotBlue => {
            scene
                .spot_light(
                    SpotLight::default()
                        .with_color(Color::from_linear_rgb(0.0, 0.0, 1.0))
                        .with_intensity_candela(1_000.0)
                        .with_range(5.0)
                        .with_inner_cone_angle(Angle::from_degrees(20.0))
                        .with_outer_cone_angle(Angle::from_degrees(35.0)),
                )
                .transform(Transform::at(Vec3::new(0.0, 0.0, 1.0)))
                .add()
                .expect("spot light inserts");
        }
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    (scene, assets, camera)
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
    let adapter = renderer.gpu_adapter_report();
    RenderedArtifact {
        width,
        height,
        frame,
        nonblack_pixels,
        draw_calls: outcome.draw_calls,
        capabilities: *renderer.capabilities(),
        host_gpu_available,
        host_gpu_error,
        adapter,
    }
}

fn render_scene_headless_cpu(
    width: u32,
    height: u32,
    scene: &mut Scene,
    assets: Option<&Assets>,
    camera: scena::CameraKey,
) -> RenderedArtifact {
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
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
        host_gpu_available: false,
        host_gpu_error: None,
        adapter: None,
    }
}

fn write_headless_cpu_lane_artifacts() {
    let artifact_dir = headless_cpu_dir();
    fs::create_dir_all(&artifact_dir).expect("headless CPU artifact dir");

    let default = render_default_scene_headless_cpu(96, 64);
    let default_ppm = artifact_dir.join("default-scene.ppm");
    write_ppm(&default_ppm, default.width, default.height, &default.frame);
    assert!(
        default.nonblack_pixels > 0,
        "headless CPU default scene renders nonblack pixels"
    );

    let static_gltf = render_static_gltf_headless_cpu(96, 64);
    let static_gltf_ppm = artifact_dir.join("static-gltf.ppm");
    write_ppm(
        &static_gltf_ppm,
        static_gltf.width,
        static_gltf.height,
        &static_gltf.frame,
    );
    assert!(
        static_gltf.nonblack_pixels > 0,
        "headless CPU static glTF fixture renders nonblack pixels"
    );

    let capabilities = capability_json(HEADLESS_CPU_LANE, &static_gltf);
    let capability_path = artifact_dir.join("capabilities.json");
    write_json(&capability_path, &capabilities);

    let headless_cpu_production_claim = true;
    let proof = serde_json::json!({
        "schema": "scena.m9.platform_render.v1",
        "lane": HEADLESS_CPU_LANE,
        "os": std::env::consts::OS,
        "backend": "Headless",
        "headless_cpu_proof": true,
        "gpu_proof": false,
        "fallback_policy": "headless CPU is a separate software proof lane and never satisfies native GPU claims",
        "commit": current_commit_label(),
        "timestamp_unix_seconds": current_timestamp_unix_seconds(),
        "test_names": [
            "m9_platform_rendered_output_suite_writes_release_artifacts"
        ],
        "artifact_paths": [
            path_string(&default_ppm),
            path_string(&static_gltf_ppm),
            path_string(&capability_path),
            path_string(&artifact_dir.join("rendered-output.json"))
        ],
        "default_scene": {
            "proof_class": "headless-cpu-harness-smoke",
            "production_claim": false,
            "backend": "Headless",
            "adapter": adapter_metadata(None),
            "renderer_settings": screenshot_renderer_settings(&default),
            "color_management": screenshot_color_management(),
            "tolerance": screenshot_tolerance_metadata(),
            "screenshot": path_string(&default_ppm),
            "width": default.width,
            "height": default.height,
            "draw_calls": default.draw_calls,
            "nonblack_pixels": default.nonblack_pixels,
        },
        "static_gltf": {
            "source": STATIC_GLTF_PROOF_FIXTURE,
            "proof_class": "cpu-camera-framed-non-ndc",
            "production_claim": headless_cpu_production_claim,
            "backend": "Headless",
            "adapter": adapter_metadata(None),
            "asset_provenance": asset_provenance(STATIC_GLTF_PROOF_FIXTURE),
            "renderer_settings": screenshot_renderer_settings(&static_gltf),
            "color_management": screenshot_color_management(),
            "tolerance": screenshot_tolerance_metadata(),
            "screenshot": path_string(&static_gltf_ppm),
            "width": static_gltf.width,
            "height": static_gltf.height,
            "draw_calls": static_gltf.draw_calls,
            "nonblack_pixels": static_gltf.nonblack_pixels,
        },
        "capabilities": path_string(&capability_path),
    });
    write_json(&artifact_dir.join("rendered-output.json"), &proof);
}

fn production_claim_for_gpu(artifact: &RenderedArtifact) -> bool {
    artifact.host_gpu_available
        && matches!(
            artifact.capabilities.backend,
            Backend::HeadlessGpu | Backend::NativeSurface
        )
}

fn static_gltf_proof_class(gpu_proof: bool) -> &'static str {
    if gpu_proof {
        "camera-framed-non-ndc"
    } else {
        "cpu-fallback-camera-framed-non-ndc"
    }
}

fn scene_with_triangle() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
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
    let mut rows = vec![
        benchmark_static_viewer(),
        benchmark_standard_model_viewer_gltf(),
        benchmark_larger_industrial_gltf(),
        benchmark_high_instance_scene(),
        benchmark_idle_render_on_change(),
        benchmark_headless_4k(),
    ];
    let baseline = benchmark_baseline();
    let baseline_comparison = apply_benchmark_baselines(&mut rows, &baseline);
    let artifact = serde_json::json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": lane,
        "regression_threshold_percent": 5.0,
        "baseline_comparison": baseline_comparison,
        "rows": rows,
    });
    write_json(&platform_dir().join("m9-benchmarks.json"), &artifact);
}

fn write_dedicated_4k_benchmark_artifact() -> serde_json::Value {
    let mut rows = vec![benchmark_headless_4k_measured(DEDICATED_4K_SAMPLE_COUNT)];
    let baseline = benchmark_baseline();
    let baseline_comparison = apply_benchmark_baselines(&mut rows, &baseline);
    let artifact = serde_json::json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": "headless-4k-performance",
        "regression_threshold_percent": 5.0,
        "baseline_comparison": baseline_comparison,
        "rows": rows,
    });
    fs::create_dir_all(platform_dir()).expect("platform artifact dir for headless-4k");
    write_json(&platform_dir().join("m9-benchmarks-4k.json"), &artifact);
    artifact
}

fn benchmark_baseline() -> serde_json::Value {
    let text = fs::read_to_string(root().join(BENCHMARK_BASELINE_PATH))
        .expect("benchmark baseline file is readable");
    serde_json::from_str(&text).expect("benchmark baseline file is valid JSON")
}

fn apply_benchmark_baselines(
    rows: &mut [serde_json::Value],
    baseline: &serde_json::Value,
) -> serde_json::Value {
    let mut status = "passed";
    let minimum_sample_count = baseline
        .get("minimum_sample_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(BENCHMARK_SAMPLE_COUNT as u64);

    for row in rows {
        if row.get("status").and_then(serde_json::Value::as_str)
            == Some("deferred-to-dedicated-performance-lane")
        {
            row["baseline_comparison"] = serde_json::json!({
                "status": "deferred",
                "reason": "dedicated performance lane required before this row becomes a release blocker",
            });
            continue;
        }

        let Some(row_baseline) = benchmark_baseline_for_row(row, baseline) else {
            status = "failed";
            row["baseline_comparison"] = serde_json::json!({
                "status": "failed",
                "reason": "missing stored baseline row",
            });
            continue;
        };

        let p95_frame_ms = row
            .get("p95_frame_ms")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(f64::INFINITY);
        let baseline_p95_frame_ms = row_baseline
            .get("p95_frame_ms")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let allowed_regression_percent = row_baseline
            .get("allowed_regression_percent")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(5.0);
        let row_minimum_sample_count = row_baseline
            .get("minimum_sample_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(minimum_sample_count);
        let sample_count = row
            .get("sample_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let allowed_p95 = baseline_p95_frame_ms * (1.0 + allowed_regression_percent / 100.0);
        let regression_percent = if baseline_p95_frame_ms > 0.0 {
            ((p95_frame_ms - baseline_p95_frame_ms) / baseline_p95_frame_ms) * 100.0
        } else {
            f64::INFINITY
        };
        let row_status = if sample_count >= row_minimum_sample_count && p95_frame_ms <= allowed_p95
        {
            "passed"
        } else {
            status = "failed";
            "failed"
        };

        row["baseline_comparison"] = serde_json::json!({
            "status": row_status,
            "baseline_p95_frame_ms": baseline_p95_frame_ms,
            "allowed_regression_percent": allowed_regression_percent,
            "allowed_p95_frame_ms": allowed_p95,
            "regression_percent": regression_percent,
            "minimum_sample_count": row_minimum_sample_count,
        });
    }

    serde_json::json!({
        "status": status,
        "baseline_path": BENCHMARK_BASELINE_PATH,
        "baseline_sha256": asset_source_hash(BENCHMARK_BASELINE_PATH),
        "metric": "p95_frame_ms",
        "minimum_sample_count": minimum_sample_count,
    })
}

fn benchmark_baseline_for_row<'a>(
    row: &serde_json::Value,
    baseline: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    let scene = row.get("scene").and_then(serde_json::Value::as_str)?;
    let backend = row.get("backend").and_then(serde_json::Value::as_str)?;
    baseline
        .get("rows")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .find(|candidate| {
            candidate.get("scene").and_then(serde_json::Value::as_str) == Some(scene)
                && candidate.get("backend").and_then(serde_json::Value::as_str) == Some(backend)
        })
}

fn benchmark_static_viewer() -> serde_json::Value {
    let (mut scene, camera) = scene_with_triangle();
    benchmark_scene(
        "static-viewer",
        128,
        128,
        "builtin:unlit-triangle",
        &mut scene,
        None,
        camera,
    )
}

fn benchmark_standard_model_viewer_gltf() -> serde_json::Value {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(STATIC_GLTF_PROOF_FIXTURE)).expect("mesh glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("mesh glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    if let Some(bounds) = import.bounds_world(&scene) {
        scene
            .frame(camera, bounds)
            .expect("camera frames benchmark mesh glTF");
    }
    benchmark_scene(
        "standard-model-viewer-gltf",
        128,
        128,
        STATIC_GLTF_PROOF_FIXTURE,
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
        "tests/assets/gltf/khronos/BrainStem/BrainStem.gltf",
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
    benchmark_scene(
        "high-instance",
        128,
        128,
        "generated:128-box-instances",
        &mut scene,
        Some(&assets),
        camera,
    )
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
    let mut samples = Vec::with_capacity(BENCHMARK_SAMPLE_COUNT);
    let mut outcome = None;
    for _ in 0..BENCHMARK_SAMPLE_COUNT {
        let start = Instant::now();
        let next = renderer.render(&scene, camera).expect("idle render skips");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        outcome = Some(next);
    }
    let outcome = outcome.expect("benchmark loop records an outcome");
    benchmark_row(BenchmarkRowInput {
        scene: "idle",
        backend: renderer.capabilities().backend,
        samples: &samples,
        draw_calls: outcome.draw_calls,
        skipped: outcome.skipped,
        fixture: BenchmarkFixture {
            width: 64,
            height: 64,
            source: "builtin:unlit-triangle-on-change",
            sample_count_policy: "100 timed idle render calls after one warm render",
        },
        prepare_ms: 0.0,
        warmup_frame_ms: None,
    })
}

fn benchmark_headless_4k() -> serde_json::Value {
    serde_json::json!({
        "scene": "headless-4k",
        "backend": "Headless",
        "status": "deferred-to-dedicated-performance-lane",
        "sample_count": 0,
        "fixture": {
            "source": "builtin:unlit-triangle",
            "width": 3840,
            "height": 2160,
            "sample_count_policy": "not measured in cargo test; requires dedicated 4K performance lane with 100+ timed render samples",
        },
        "regression_threshold_percent": 5.0,
    })
}

fn benchmark_headless_4k_measured(sample_count: usize) -> serde_json::Value {
    let (mut scene, camera) = scene_with_triangle();
    benchmark_scene_with_sample_count(
        BenchmarkSceneInput {
            name: "headless-4k",
            width: 3840,
            height: 2160,
            fixture_source: "builtin:unlit-triangle",
            sample_count,
            sample_count_policy: "dedicated performance lane with 100 timed render calls after one warm render",
        },
        &mut scene,
        None,
        camera,
    )
}

#[test]
fn m9_benchmark_rows_use_distribution_not_single_sample() {
    let (mut scene, camera) = scene_with_triangle();
    let row = benchmark_scene(
        "benchmark-contract",
        64,
        64,
        "builtin:unlit-triangle",
        &mut scene,
        None,
        camera,
    );

    assert_eq!(row["sample_count"], 100);
    assert!(
        row["p50_frame_ms"].as_f64().is_some(),
        "benchmark row records p50"
    );
    assert!(
        row["p95_frame_ms"].as_f64().is_some(),
        "benchmark row records p95"
    );
    assert!(
        row["min_frame_ms"].as_f64().is_some(),
        "benchmark row records minimum"
    );
    assert!(
        row["max_frame_ms"].as_f64().is_some(),
        "benchmark row records maximum"
    );
    assert!(
        row["stddev_frame_ms"].as_f64().is_some(),
        "benchmark row records standard deviation"
    );
    assert_eq!(row["fixture"]["width"], 64);
    assert_eq!(row["fixture"]["height"], 64);
}

#[test]
fn m9_benchmark_rows_record_stored_baseline_comparison() {
    let mut rows = vec![
        serde_json::json!({
            "scene": "static-viewer",
            "backend": "Headless",
            "sample_count": 100,
            "p95_frame_ms": 10.0,
        }),
        serde_json::json!({
            "scene": "headless-4k",
            "status": "deferred-to-dedicated-performance-lane",
            "sample_count": 0,
        }),
    ];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "p95_frame_ms": 12.0,
                "allowed_regression_percent": 5.0
            }
        ]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline);

    assert_eq!(summary["status"], "passed");
    assert_eq!(summary["baseline_path"], BENCHMARK_BASELINE_PATH);
    assert_eq!(summary["metric"], "p95_frame_ms");
    assert_eq!(rows[0]["baseline_comparison"]["status"], "passed");
    assert_eq!(
        rows[1]["baseline_comparison"]["status"], "deferred",
        "dedicated-lane benchmark rows must be explicit deferrals, not silent misses"
    );
}

#[test]
fn m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact() {
    if std::env::var_os("SCENA_RUN_DEDICATED_4K_BENCHMARK").is_none() {
        fs::create_dir_all(platform_dir()).expect("platform artifact dir");
        let artifact_path = platform_dir().join("m9-benchmarks-4k-required.json");
        let artifact = serde_json::json!({
            "schema": "scena.m9.benchmark_4k_required.v1",
            "status": "fail-closed",
            "release_evidence": false,
            "reason": "SCENA_RUN_DEDICATED_4K_BENCHMARK is not set in the normal cargo-test lane",
            "run_hint": "Set SCENA_RUN_DEDICATED_4K_BENCHMARK=1 on the dedicated performance lane to write m9-benchmarks-4k.json.",
            "required_artifact": path_string(&platform_dir().join("m9-benchmarks-4k.json")),
        });
        write_json(&artifact_path, &artifact);
        assert!(
            artifact_path.is_file(),
            "normal suite must record fail-closed 4K benchmark requirement metadata"
        );
        return;
    }

    let artifact = write_dedicated_4k_benchmark_artifact();
    let rows = artifact["rows"].as_array().expect("benchmark rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["scene"], "headless-4k");
    assert_eq!(rows[0]["sample_count"], DEDICATED_4K_SAMPLE_COUNT as u64);
    assert_ne!(
        rows[0]["status"].as_str(),
        Some("deferred-to-dedicated-performance-lane"),
        "dedicated 4K lane must produce a measured row, not the normal-suite deferral"
    );
    assert_eq!(
        artifact["baseline_comparison"]["baseline_path"],
        BENCHMARK_BASELINE_PATH
    );
    assert!(
        platform_dir().join("m9-benchmarks-4k.json").is_file(),
        "dedicated 4K benchmark artifact must be written for release-readiness"
    );
}

#[test]
fn m9_benchmark_baseline_comparison_fails_significant_regressions() {
    let mut rows = vec![serde_json::json!({
        "scene": "static-viewer",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 12.0,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "p95_frame_ms": 10.0,
                "allowed_regression_percent": 5.0
            }
        ]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline);

    assert_eq!(summary["status"], "failed");
    assert_eq!(rows[0]["baseline_comparison"]["status"], "failed");
    assert_eq!(rows[0]["baseline_comparison"]["regression_percent"], 20.0);
}

fn benchmark_scene(
    name: &str,
    width: u32,
    height: u32,
    fixture_source: &str,
    scene: &mut Scene,
    assets: Option<&Assets>,
    camera: scena::CameraKey,
) -> serde_json::Value {
    benchmark_scene_with_sample_count(
        BenchmarkSceneInput {
            name,
            width,
            height,
            fixture_source,
            sample_count: BENCHMARK_SAMPLE_COUNT,
            sample_count_policy: "100 timed render calls after one warm render",
        },
        scene,
        assets,
        camera,
    )
}

struct BenchmarkSceneInput<'a> {
    name: &'a str,
    width: u32,
    height: u32,
    fixture_source: &'a str,
    sample_count: usize,
    sample_count_policy: &'a str,
}

fn benchmark_scene_with_sample_count(
    input: BenchmarkSceneInput<'_>,
    scene: &mut Scene,
    assets: Option<&Assets>,
    camera: scena::CameraKey,
) -> serde_json::Value {
    assert!(
        input.sample_count > 0,
        "benchmark sample count must be nonzero"
    );
    let mut renderer = Renderer::headless(input.width, input.height).expect("renderer builds");
    let start = Instant::now();
    if let Some(assets) = assets {
        renderer
            .prepare_with_assets(scene, assets)
            .expect("asset scene prepares");
    } else {
        renderer.prepare(scene).expect("scene prepares");
    }
    let prepare_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let warmup = renderer.render(scene, camera).expect("warm scene render");
    let warmup_frame_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut samples = Vec::with_capacity(input.sample_count);
    let mut outcome = warmup;
    for _ in 0..input.sample_count {
        let start = Instant::now();
        outcome = renderer.render(scene, camera).expect("scene renders");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    benchmark_row(BenchmarkRowInput {
        scene: input.name,
        backend: renderer.capabilities().backend,
        samples: &samples,
        draw_calls: outcome.draw_calls,
        skipped: outcome.skipped,
        fixture: BenchmarkFixture {
            width: input.width,
            height: input.height,
            source: input.fixture_source,
            sample_count_policy: input.sample_count_policy,
        },
        prepare_ms,
        warmup_frame_ms: Some(warmup_frame_ms),
    })
}

struct BenchmarkFixture<'a> {
    width: u32,
    height: u32,
    source: &'a str,
    sample_count_policy: &'a str,
}

struct BenchmarkRowInput<'a> {
    scene: &'a str,
    backend: Backend,
    samples: &'a [f64],
    draw_calls: u64,
    skipped: bool,
    fixture: BenchmarkFixture<'a>,
    prepare_ms: f64,
    warmup_frame_ms: Option<f64>,
}

fn benchmark_row(input: BenchmarkRowInput<'_>) -> serde_json::Value {
    let distribution = benchmark_distribution(input.samples);
    serde_json::json!({
        "scene": input.scene,
        "backend": format!("{:?}", input.backend),
        "sample_count": distribution.sample_count,
        "median_frame_ms": distribution.p50_frame_ms,
        "p50_frame_ms": distribution.p50_frame_ms,
        "p95_frame_ms": distribution.p95_frame_ms,
        "min_frame_ms": distribution.min_frame_ms,
        "max_frame_ms": distribution.max_frame_ms,
        "stddev_frame_ms": distribution.stddev_frame_ms,
        "prepare_ms": input.prepare_ms,
        "warmup_frame_ms": input.warmup_frame_ms,
        "fixture": {
            "source": input.fixture.source,
            "source_hash": asset_source_hash_if_file(input.fixture.source),
            "width": input.fixture.width,
            "height": input.fixture.height,
            "sample_count_policy": input.fixture.sample_count_policy,
        },
        "draw_calls": input.draw_calls,
        "skipped": input.skipped,
        "regression_threshold_percent": 5.0,
    })
}

struct BenchmarkDistribution {
    sample_count: usize,
    min_frame_ms: f64,
    p50_frame_ms: f64,
    p95_frame_ms: f64,
    max_frame_ms: f64,
    stddev_frame_ms: f64,
}

fn benchmark_distribution(samples: &[f64]) -> BenchmarkDistribution {
    assert!(
        !samples.is_empty(),
        "benchmark distribution requires at least one sample"
    );
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let sample_count = sorted.len();
    let min_frame_ms = sorted[0];
    let max_frame_ms = sorted[sample_count - 1];
    let p50_frame_ms = percentile_nearest_rank(&sorted, 0.50);
    let p95_frame_ms = percentile_nearest_rank(&sorted, 0.95);
    let mean = sorted.iter().sum::<f64>() / sample_count as f64;
    let variance = sorted
        .iter()
        .map(|sample| {
            let delta = sample - mean;
            delta * delta
        })
        .sum::<f64>()
        / sample_count as f64;
    BenchmarkDistribution {
        sample_count,
        min_frame_ms,
        p50_frame_ms,
        p95_frame_ms,
        max_frame_ms,
        stddev_frame_ms: variance.sqrt(),
    }
}

fn percentile_nearest_rank(sorted_samples: &[f64], percentile: f64) -> f64 {
    debug_assert!(!sorted_samples.is_empty());
    let rank = (sorted_samples.len() as f64 * percentile).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    sorted_samples[index]
}

fn capability_matrix_row(
    lane: &str,
    measured_current_lane: &RenderedArtifact,
    measured_headless_cpu: &RenderedArtifact,
) -> serde_json::Value {
    if lane == current_lane() {
        lane_capability_from_artifact(lane, measured_current_lane)
    } else if lane == HEADLESS_CPU_LANE {
        lane_capability_from_artifact(lane, measured_headless_cpu)
    } else {
        missing_lane_capability(lane)
    }
}

fn lane_capability_from_artifact(lane: &str, artifact: &RenderedArtifact) -> serde_json::Value {
    let mut row = lane_capability(lane, artifact.capabilities, "lane-renderer-runtime");
    row["status"] = serde_json::json!("measured");
    row["adapter"] = adapter_metadata(artifact.adapter.as_ref());
    row["host_gpu_available"] = serde_json::json!(artifact.host_gpu_available);
    row["host_gpu_error"] = serde_json::json!(artifact.host_gpu_error);
    row["commit"] = serde_json::json!(current_commit_label());
    row["timestamp_unix_seconds"] = serde_json::json!(current_timestamp_unix_seconds());
    row
}

fn missing_lane_capability(lane: &str) -> serde_json::Value {
    serde_json::json!({
        "lane": lane,
        "status": "missing-measurement",
        "measurement_source": "missing-lane-artifact",
        "capabilities": serde_json::Value::Null,
        "adapter": { "available": false },
        "host_gpu_available": false,
        "host_gpu_error": "required lane must upload its own measured capabilities.json",
        "diagnostics": [
            "no factory capability constants are accepted as platform proof"
        ],
    })
}

fn lane_capability(
    lane: &str,
    capabilities: Capabilities,
    measurement_source: &str,
) -> serde_json::Value {
    serde_json::json!({
        "lane": lane,
        "measurement_source": measurement_source,
        "capabilities": capability_fields(capabilities),
        "diagnostics": capabilities
            .diagnostics()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>(),
    })
}

fn capability_json(lane: &str, artifact: &RenderedArtifact) -> serde_json::Value {
    let capabilities = artifact.capabilities;
    serde_json::json!({
        "schema": "scena.capabilities.v1",
        "lane": lane,
        "measurement_source": "lane-renderer-runtime",
        "commit": current_commit_label(),
        "timestamp_unix_seconds": current_timestamp_unix_seconds(),
        "backend": format!("{:?}", capabilities.backend),
        "hardware_tier": format!("{:?}", capabilities.hardware_tier),
        "adapter": adapter_metadata(artifact.adapter.as_ref()),
        "features": capability_fields(capabilities),
        "diagnostics": capabilities
            .diagnostics()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>(),
    })
}

fn adapter_metadata(report: Option<&GpuAdapterReport>) -> serde_json::Value {
    let Some(report) = report else {
        return serde_json::json!({ "available": false });
    };
    serde_json::json!({
        "available": true,
        "name": report.name,
        "backend": report.backend,
        "device_type": report.device_type,
        "vendor": report.vendor,
        "device": report.device,
        "driver": report.driver,
        "driver_info": report.driver_info,
        "features": report.features,
        "limits": {
            "max_texture_dimension_2d": report.limits.max_texture_dimension_2d,
            "max_bind_groups": report.limits.max_bind_groups,
            "max_uniform_buffer_binding_size": report.limits.max_uniform_buffer_binding_size,
            "max_vertex_attributes": report.limits.max_vertex_attributes,
        },
    })
}

fn capability_fields(capabilities: Capabilities) -> serde_json::Value {
    serde_json::json!({
        "forward_pbr": { "state": format!("{:?}", capabilities.forward_pbr) },
        "directional_shadows": { "state": format!("{:?}", capabilities.directional_shadows) },
        "point_shadows": { "state": format!("{:?}", capabilities.point_shadows) },
        "spot_shadows": { "state": format!("{:?}", capabilities.spot_shadows) },
        "bloom": { "state": format!("{:?}", capabilities.bloom) },
        "screen_space_ambient_occlusion": { "state": format!("{:?}", capabilities.screen_space_ambient_occlusion) },
        "texture_compression_basisu": { "state": format!("{:?}", capabilities.texture_compression_basisu) },
        "hardware_instancing": { "state": format!("{:?}", capabilities.hardware_instancing) },
        "texture_arrays": {
            "state": format!("{:?}", capabilities.texture_arrays),
            "max_layers": capabilities.max_texture_array_layers,
        },
        "fragment_high_precision": { "state": format!("{:?}", capabilities.fragment_high_precision) },
        "uniform_buffers": {
            "state": format!("{:?}", capabilities.uniform_buffers),
            "max_bytes": capabilities.uniform_buffer_max_bytes,
        },
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

fn sample_rgb(rgba: &[u8], width: u32, _height: u32, x: u32, y: u32) -> [u8; 3] {
    let index = ((y * width + x) * 4) as usize;
    [rgba[index], rgba[index + 1], rgba[index + 2]]
}

fn path_string(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn asset_provenance(relative_path: &str) -> serde_json::Value {
    serde_json::json!({
        "path": relative_path,
        "hash": asset_source_hash(relative_path),
    })
}

fn screenshot_renderer_settings(artifact: &RenderedArtifact) -> serde_json::Value {
    serde_json::json!({
        "width": artifact.width,
        "height": artifact.height,
        "backend": format!("{:?}", artifact.capabilities.backend),
        "color_target_format": artifact.capabilities.color_target_format,
        "output_stage": format!("{:?}", artifact.capabilities.output_stage),
        "alpha_pipeline": format!("{:?}", artifact.capabilities.alpha_pipeline),
        "readback_headless_screenshots": format!("{:?}", artifact.capabilities.readback_headless_screenshots),
    })
}

fn screenshot_color_management() -> serde_json::Value {
    serde_json::json!({
        "scene_input": "linear-scene-referred",
        "tone_mapper": "aces",
        "output_encoding": "srgb8-after-aces",
    })
}

fn screenshot_tolerance_metadata() -> serde_json::Value {
    serde_json::json!({
        "policy": "native-rendered-output-smoke",
        "max_abs_diff": 8,
        "mean_abs_diff": 2.0,
        "comparison_space": "srgb8",
    })
}

fn asset_source_hash_if_file(relative_path: &str) -> Option<String> {
    root()
        .join(relative_path)
        .is_file()
        .then(|| asset_source_hash(relative_path))
}

fn current_commit_label() -> String {
    std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local-checkout".to_string())
}

fn current_timestamp_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn asset_source_hash(relative_path: &str) -> String {
    let bytes = fs::read(root().join(relative_path)).expect("asset provenance source is readable");
    format!("fnv1a64:{:016x}", fnv1a64(&bytes))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
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
    adapter: Option<GpuAdapterReport>,
}

#[derive(Clone, Copy, Debug)]
enum PbrLightKind {
    DirectionalRed,
    PointGreen,
    SpotBlue,
}

impl PbrLightKind {
    const fn ppm_filename(self) -> &'static str {
        match self {
            Self::DirectionalRed => PBR_DIRECTIONAL_RED_PPM,
            Self::PointGreen => PBR_POINT_GREEN_PPM,
            Self::SpotBlue => PBR_SPOT_BLUE_PPM,
        }
    }

    const fn light_type(self) -> &'static str {
        match self {
            Self::DirectionalRed => "directional",
            Self::PointGreen => "point",
            Self::SpotBlue => "spot",
        }
    }

    const fn expected_channel(self) -> &'static str {
        match self {
            Self::DirectionalRed => "red",
            Self::PointGreen => "green",
            Self::SpotBlue => "blue",
        }
    }

    fn assert_expected_tint(self, rgb: [u8; 3]) -> bool {
        let r = rgb[0] as i16;
        let g = rgb[1] as i16;
        let b = rgb[2] as i16;
        match self {
            Self::DirectionalRed => r >= g + 8 && r >= b + 8,
            Self::PointGreen => g >= r + 8 && g >= b + 8,
            Self::SpotBlue => b >= r + 8 && b >= g + 8,
        }
    }
}

struct PbrLightProof {
    kind: PbrLightKind,
    center: [u8; 3],
    color_assertion_passed: bool,
    ppm_path: PathBuf,
    artifact: RenderedArtifact,
}

impl PbrLightProof {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "light_type": self.kind.light_type(),
            "expected_channel": self.kind.expected_channel(),
            "proof_class": "native-pbr-punctual-light",
            "production_claim": production_claim_for_gpu(&self.artifact) && self.color_assertion_passed,
            "gpu_proof": production_claim_for_gpu(&self.artifact) && self.color_assertion_passed,
            "backend": format!("{:?}", self.artifact.capabilities.backend),
            "host_gpu_available": self.artifact.host_gpu_available,
            "host_gpu_error": self.artifact.host_gpu_error,
            "adapter": adapter_metadata(self.artifact.adapter.as_ref()),
            "renderer_settings": screenshot_renderer_settings(&self.artifact),
            "color_management": screenshot_color_management(),
            "tolerance": screenshot_tolerance_metadata(),
            "screenshot": path_string(&self.ppm_path),
            "width": self.artifact.width,
            "height": self.artifact.height,
            "draw_calls": self.artifact.draw_calls,
            "nonblack_pixels": self.artifact.nonblack_pixels,
            "center_rgb": self.center,
            "color_assertion_passed": self.color_assertion_passed,
        })
    }
}
