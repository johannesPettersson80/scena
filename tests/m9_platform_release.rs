#![cfg(not(target_arch = "wasm32"))]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use scena::{
    AdapterLimitsReport, Angle, AnimationChannel, AnimationClip, AnimationInterpolation,
    AnimationLoopMode, AnimationOutput, AnimationTarget, AntiAliasing, AreaLight, Assets, Backend,
    Capabilities, Color, CursorPosition, DepthOfFieldConfig, DirectionalLight, EnvironmentDesc,
    EnvironmentSidecarProfile, GeometryDesc, GeometryMorphTarget, GeometryTopology, GeometryVertex,
    GpuAdapterReport, MaterialDesc, NotPreparedReason, OrthographicCamera, PerspectiveCamera,
    PointLight, PostBloomConfig, Primitive, ReconstructionFilter, RenderError, Renderer,
    RendererOptions, Scene, ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig,
    SpotLight, SurfaceEvent, TextureColorSpace, Transform, Vec3, Viewport,
};
use sha2::{Digest, Sha256};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;
const STATIC_GLTF_PROOF_FIXTURE: &str = "tests/assets/gltf/non_ndc_camera_scene.gltf";
const BENCHMARK_BASELINE_PATH: &str = "tests/fixtures/m9-baselines.json";
const BENCHMARK_SAMPLE_COUNT: usize = 100;
const DEDICATED_4K_SAMPLE_COUNT: usize = 100;
const PF00_HARDWARE_SUMMARY_PATH: &str =
    "target/gate-artifacts/windows-complete-hardware-proof/proof-summary.json";
const PF00_REQUIRED_HARDWARE_ARTIFACTS: [&str; 4] = [
    "target/gate-artifacts/pf01-output-toggle/browser/browser-output-toggle.json",
    "target/gate-artifacts/fr06-semantic-aov/browser/semantic-aov-browser-proof.json",
    "target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json",
    "target/gate-artifacts/fr06-semantic-aov/native/native-semantic-aov-proof.json",
];
const PF00_WORKLOAD_ARTIFACTS: [(&str, &str); 10] = [
    (
        "pick-100k-triangle-deformed-undeformed",
        "pick-100k-triangle-deformed-undeformed.json",
    ),
    (
        "animation-many-channels-keyframes-weights",
        "animation-many-channels-keyframes-weights.json",
    ),
    (
        "tangent-generation-static-deformed",
        "tangent-generation-static-deformed.json",
    ),
    (
        "shadow-scaling-directional-area",
        "shadow-scaling-directional-area.json",
    ),
    (
        "cpu-texture-bake-qualifying-nonqualifying",
        "cpu-texture-bake-qualifying-nonqualifying.json",
    ),
    (
        "one-node-transform-prepare-render",
        "one-node-transform-prepare-render.json",
    ),
    (
        "environment-bake-cold-sidecar-hit",
        "environment-bake-cold-sidecar-hit.json",
    ),
    (
        "native-present-capture-sync-async",
        "native-present-capture-sync-async.json",
    ),
    (
        "gpu-first-render-output-settings",
        "gpu-first-render-output-settings.json",
    ),
    (
        "draw-uniform-indexing-many-unique-transforms",
        "draw-uniform-indexing-many-unique-transforms.json",
    ),
];
const HEADLESS_CPU_LANE: &str = "headless-cpu";
const PBR_DIRECTIONAL_RED_PPM: &str = "pbr-directional-red.ppm";
const PBR_POINT_GREEN_PPM: &str = "pbr-point-green.ppm";
const PBR_SPOT_BLUE_PPM: &str = "pbr-spot-blue.ppm";
const ROUND_E_MATERIAL_PRESETS: &[&str] = &[
    "matte",
    "plastic",
    "metal",
    "rough_metal",
    "chrome",
    "brushed_steel",
    "clearcoat_plastic",
    "satin",
    "leather",
    "clear_glass",
    "frosted_glass",
    "rubber",
];
const ROUND_E_MATERIAL_LANES: &[&str] = &[
    "cpu-reference",
    "webgl2-desktop-chromium",
    "webgpu-desktop-chromium",
    "native-headless-gpu",
    "ios-safari",
    "android-chrome",
];

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

thread_local! {
    static COUNT_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    static ALLOCATION_BYTES: Cell<usize> = const { Cell::new(0) };
    static ALLOCATION_SIZES: RefCell<[usize; 64]> = const { RefCell::new([0; 64]) };
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: this allocator delegates with the original layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && COUNT_ALLOCATIONS.with(Cell::get) {
            ALLOCATION_COUNT.set(ALLOCATION_COUNT.get().saturating_add(1));
            ALLOCATION_BYTES.set(ALLOCATION_BYTES.get().saturating_add(layout.size()));
            record_allocation_size(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer and layout are forwarded unchanged to the
        // allocator that created the allocation.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: this allocator delegates with the original layout.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() && COUNT_ALLOCATIONS.with(Cell::get) {
            ALLOCATION_COUNT.set(ALLOCATION_COUNT.get().saturating_add(1));
            ALLOCATION_BYTES.set(ALLOCATION_BYTES.get().saturating_add(layout.size()));
            record_allocation_size(layout.size());
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the pointer, old layout, and requested size are forwarded unchanged.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() && COUNT_ALLOCATIONS.with(Cell::get) {
            ALLOCATION_COUNT.set(ALLOCATION_COUNT.get().saturating_add(1));
            ALLOCATION_BYTES.set(ALLOCATION_BYTES.get().saturating_add(new_size));
            record_allocation_size(new_size);
        }
        resized
    }
}

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

fn pf00_dir() -> PathBuf {
    root().join("target/gate-artifacts/pf00")
}

fn pf03_dir() -> PathBuf {
    root().join("target/gate-artifacts/pf03")
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
        "required_parity": {
            "enabled": required_gpu_parity_enabled(),
            "status": if default_gpu_proof && static_gltf_gpu_proof && pbr_light_gpu_proof { "passed" } else { "failed" },
        },
        "fallback_policy": "cpu fallback is diagnostic only and never satisfies GPU rendered-output claims",
        "commit": current_commit_label(),
        "commit_sha": current_commit_label(),
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

    if required_gpu_parity_enabled() {
        assert!(
            default_gpu_proof && static_gltf_gpu_proof && pbr_light_gpu_proof,
            "SCENA_REQUIRE_PARITY=1 requires native GPU output for the default scene, camera-framed glTF, and all PBR light proofs; host_gpu_available={} backend={:?} error={:?}",
            default.host_gpu_available,
            default.capabilities.backend,
            default.host_gpu_error,
        );
    }

    write_headless_cpu_lane_artifacts();
}

#[test]
fn m9_platform_benchmark_writes_release_artifact() {
    let required_path = platform_dir().join("m9-benchmarks-required.json");
    if std::env::var_os("SCENA_RUN_M9_PLATFORM_BENCHMARK").is_none() {
        write_json(
            &required_path,
            &serde_json::json!({
                "schema": "scena.m9.platform_benchmark_required.v1",
                "status": "incomplete",
                "release_evidence": false,
                "reason": "SCENA_RUN_M9_PLATFORM_BENCHMARK is not set in the broad parallel test suite",
                "run_hint": "Run SCENA_RUN_M9_PLATFORM_BENCHMARK=1 cargo test --test m9_platform_release m9_platform_benchmark_writes_release_artifact -- --exact --test-threads=1 in the isolated release lane.",
                "required_artifact": path_string(&platform_dir().join("m9-benchmarks.json")),
            }),
        );
        return;
    }
    let _ = fs::remove_file(required_path);
    write_benchmark_artifact(current_lane());
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
    let browser_results = read_browser_probe_results();
    let wasm_size_artifact = read_wasm_size_artifact();
    let lanes = [
        "linux-native-vulkan",
        "linux-webgl2-chromium",
        "linux-webgpu-chromium",
        "macos-metal",
        "windows-dx12",
        "wasm32-unknown-unknown",
        HEADLESS_CPU_LANE,
    ]
    .into_iter()
    .map(|lane| {
        capability_matrix_row(
            lane,
            &measured_current_lane,
            &measured_headless_cpu,
            &browser_results,
            wasm_size_artifact.as_ref(),
        )
    })
    .collect::<Vec<_>>();
    let status = if lanes.iter().all(|entry| entry["status"] == "measured") {
        "passed"
    } else {
        // The local artifact must still write `"status": "incomplete"` when
        // required host lanes are missing; doctor pins this fail-closed contract.
        "incomplete"
    };
    let status_reason = if status == "passed" {
        "current runner records measured artifacts from every required release lane"
    } else {
        "current runner records measured local/browser lanes; final release still requires measured artifacts from missing host lanes"
    };
    let matrix = serde_json::json!({
        "schema": "scena.capabilities.v1",
        "status": status,
        "status_reason": status_reason,
        "commit": current_commit_label(),
        "commit_sha": current_commit_label(),
        "timestamp_unix_seconds": current_timestamp_unix_seconds(),
        "test_names": [
            "m9_capability_matrix_artifact_covers_required_lanes"
        ],
        "artifact_paths": [
            path_string(&platform_dir().join("m9-capability-matrix.json"))
        ],
        "lanes": lanes,
        "material_preset_lanes": material_preset_capability_rows(),
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
    let current_row_expected_gpu_status =
        if current_row["host_gpu_available"].as_bool().unwrap_or(false) {
            "Supported"
        } else {
            "Degraded"
        };
    assert_eq!(
        current_row["capabilities"]["forward_pbr"]["state"], current_row_expected_gpu_status,
        "M9 capability rows must promote forward PBR only when the measured lane owns a GPU device"
    );
    assert_eq!(
        current_row["capabilities"]["physical_glass_transmission"]["state"],
        current_row_expected_gpu_status,
        "M9 capability rows must expose physical glass status separately from forward_pbr and promote it only on measured GPU lanes"
    );
    assert_eq!(
        current_row["capabilities"]["auto_exposure_metering"]["average"]["state"], "Supported",
        "M9 capability rows must expose supported average metering explicitly"
    );
    assert_eq!(
        current_row["capabilities"]["auto_exposure_metering"]["subject"]["state"], "Degraded",
        "M9 capability rows must not claim subject metering support before the E02 meter-routing proof lands"
    );
    assert_eq!(
        current_row["capabilities"]["auto_exposure_metering"]["spot"]["state"], "FeatureDisabled",
        "M9 capability rows must expose unsupported spot metering explicitly"
    );
    for (lane, backend) in [
        ("linux-webgl2-chromium", "WebGl2"),
        ("linux-webgpu-chromium", "WebGpu"),
    ] {
        if browser_probe_has_passed_backend(&browser_results, backend) {
            let row = lanes
                .iter()
                .find(|entry| entry["lane"] == lane)
                .expect("browser lane row exists");
            assert_eq!(
                row["status"], "measured",
                "browser proof artifact must be folded into the M9 matrix for {lane}"
            );
            assert_eq!(row["measurement_source"], "browser-probe-runtime");
        }
    }
    if wasm_size_artifact.is_some() {
        let row = lanes
            .iter()
            .find(|entry| entry["lane"] == "wasm32-unknown-unknown")
            .expect("wasm lane row exists");
        assert_eq!(row["status"], "measured");
        assert_eq!(row["measurement_source"], "wasm-size-gate-runtime");
    }
    let material_rows = matrix["material_preset_lanes"]
        .as_array()
        .expect("material_preset_lanes array");
    for preset in ROUND_E_MATERIAL_PRESETS {
        for lane in ROUND_E_MATERIAL_LANES {
            assert!(
                material_rows.iter().any(|entry| {
                    entry["preset"] == *preset
                        && entry["lane"] == *lane
                        && entry["status"].as_str().is_some()
                }),
                "M9 material capability matrix must include explicit row for {preset}/{lane}"
            );
        }
    }
    assert!(
        material_rows.iter().any(|entry| {
            entry["preset"] == "chrome"
                && entry["lane"] == "ios-safari"
                && entry["status"] == "proof-gap"
        }),
        "mobile material lanes must stay explicit proof-gap rows until mobile artifacts exist"
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

fn render_pbr_light_suite_headless_cpu(width: u32, height: u32) -> Vec<PbrLightProof> {
    [
        PbrLightKind::DirectionalRed,
        PbrLightKind::PointGreen,
        PbrLightKind::SpotBlue,
    ]
    .into_iter()
    .map(|kind| {
        let (mut scene, assets, camera) = pbr_light_scene(kind);
        let artifact = render_scene_headless_cpu(width, height, &mut scene, Some(&assets), camera);
        let center = sample_rgb(&artifact.frame, width, height, width / 2, height / 2);
        let color_assertion_passed = kind.assert_expected_tint(center);
        let ppm_path = headless_cpu_dir().join(kind.ppm_filename());
        write_ppm(&ppm_path, artifact.width, artifact.height, &artifact.frame);
        PbrLightProof {
            kind,
            center,
            color_assertion_passed,
            ppm_path,
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
                        .with_illuminance_lux(12_000.0),
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
        "commit_sha": current_commit_label(),
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
        "pbr_lights": {
            "proof_class": "headless-cpu-pbr-punctual-light",
            "production_claim": headless_cpu_production_claim,
            "gpu_proof": false,
            "lights": render_pbr_light_suite_headless_cpu(96, 64)
                .iter()
                .map(PbrLightProof::to_json)
                .collect::<Vec<_>>(),
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

fn required_gpu_parity_enabled() -> bool {
    std::env::var("SCENA_REQUIRE_PARITY").as_deref() == Ok("1")
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
    let timing_policy = m9_timing_policy();
    let baseline_comparison =
        apply_benchmark_baselines_with_policy(&mut rows, &baseline, lane, timing_policy);
    let artifact = serde_json::json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": lane,
        "timing_policy": timing_policy.as_str(),
        "performance_environment": performance_environment_metadata(lane),
        "regression_threshold_percent": 5.0,
        "baseline_comparison": baseline_comparison,
        "rows": rows,
    });
    write_json(&platform_dir().join("m9-benchmarks.json"), &artifact);
    write_feature_matrix_required_artifact(lane);
    assert_eq!(
        artifact["baseline_comparison"]["status"], "passed",
        "M9 benchmark artifact must fail the gate when a stored frame-time or allocation budget regresses: {:#}",
        artifact["baseline_comparison"]
    );
}

fn write_dedicated_4k_benchmark_artifact() -> serde_json::Value {
    let mut rows = vec![benchmark_headless_4k_measured(DEDICATED_4K_SAMPLE_COUNT)];
    let matrix_rows = benchmark_feature_matrix_measured_rows(DEDICATED_4K_SAMPLE_COUNT);
    rows.extend(matrix_rows.clone());
    let baseline = benchmark_baseline();
    let timing_policy = m9_timing_policy();
    let baseline_comparison = apply_benchmark_baselines_with_policy(
        &mut rows,
        &baseline,
        "headless-4k-performance",
        timing_policy,
    );
    let artifact = serde_json::json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": "headless-4k-performance",
        "timing_policy": timing_policy.as_str(),
        "performance_environment": performance_environment_metadata("headless-4k-performance"),
        "regression_threshold_percent": 5.0,
        "baseline_comparison": baseline_comparison,
        "rows": rows,
    });
    fs::create_dir_all(platform_dir()).expect("platform artifact dir for headless-4k");
    write_json(&platform_dir().join("m9-benchmarks-4k.json"), &artifact);
    let matrix_artifact = feature_matrix_artifact("headless-4k-performance", matrix_rows);
    write_json(
        &platform_dir().join("m9-benchmarks-feature-matrix.json"),
        &matrix_artifact,
    );
    assert_eq!(
        artifact["baseline_comparison"]["status"], "passed",
        "dedicated 4K benchmark artifact must fail the gate when a stored frame-time or allocation budget regresses: {:#}",
        artifact["baseline_comparison"]
    );
    artifact
}

fn write_feature_matrix_required_artifact(lane: &str) {
    let artifact = feature_matrix_artifact(lane, benchmark_feature_matrix_deferred_rows());
    write_json(
        &platform_dir().join("m9-benchmarks-feature-matrix.json"),
        &artifact,
    );
}

fn feature_matrix_artifact(lane: &str, mut rows: Vec<serde_json::Value>) -> serde_json::Value {
    let baseline = benchmark_baseline();
    let timing_policy = m9_timing_policy();
    let baseline_comparison =
        apply_benchmark_baselines_with_policy(&mut rows, &baseline, lane, timing_policy);
    serde_json::json!({
        "schema": "scena.m9.benchmarks.feature_matrix.v1",
        "lane": lane,
        "timing_policy": timing_policy.as_str(),
        "performance_environment": performance_environment_metadata(lane),
        "matrix_contract": "resolution x feature set cost reporting for Part A visual features",
        "regression_threshold_percent": 5.0,
        "baseline_comparison": baseline_comparison,
        "rows": rows,
    })
}

fn performance_environment_metadata(lane: &str) -> serde_json::Value {
    let optimized = !cfg!(debug_assertions);
    let profile = std::env::var("SCENA_BENCHMARK_PROFILE").unwrap_or_else(|_| {
        if optimized {
            "optimized-unspecified".to_owned()
        } else {
            "unoptimized-test".to_owned()
        }
    });
    let command = std::env::var("SCENA_BENCHMARK_COMMAND").unwrap_or_else(|_| {
        "unavailable: set SCENA_BENCHMARK_COMMAND in the benchmark lane".to_owned()
    });
    serde_json::json!({
        "lane": lane,
        "optimized": optimized,
        "profile": profile,
        "toolchain": command_output("rustc", &["-Vv"]),
        "cpu": cpu_model(),
        "gpu": {
            "status": "not-applicable",
            "reason": "these M9 rows use Renderer::headless on the CPU",
        },
        "driver": {
            "status": "not-applicable",
            "reason": "the headless CPU renderer does not use a GPU driver",
        },
        "commit": current_commit_label(),
        "command": command,
        "sidecar_cache_state": "not-applicable-to-current-m9-rows",
        "distribution": {
            "percentile_method": "nearest-rank",
            "reported": ["sample_count", "min", "p50", "p95", "max", "population_stddev"],
        },
        "confidence": {
            "status": "distribution-only",
            "reason": "no parametric confidence interval is claimed; compare complete same-host distributions",
        },
    })
}

fn performance_environment_metadata_with_renderer(
    lane: &str,
    renderer: &Renderer,
) -> serde_json::Value {
    let report = renderer.capability_report().to_schema_json();
    let mut metadata = performance_environment_metadata(lane);
    let adapter = report["adapter"].clone();
    metadata["gpu"] = if adapter.is_null() {
        serde_json::json!({
            "status": "unavailable",
            "reason": "the active renderer did not report an adapter",
        })
    } else {
        adapter.clone()
    };
    metadata["driver"] = if adapter.is_null() {
        serde_json::json!({
            "status": "unavailable",
            "reason": "the active renderer did not report a GPU driver",
        })
    } else {
        serde_json::json!({
            "name": adapter["driver"].clone(),
            "info": adapter["driver_info"].clone(),
            "backend": adapter["backend"].clone(),
        })
    };
    metadata["capability_report"] = report;
    metadata
}

fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|output| !output.is_empty())
        .unwrap_or_else(|| format!("unavailable: could not run {program}"))
}

fn cpu_model() -> String {
    std::env::var("SCENA_BENCHMARK_CPU").unwrap_or_else(|_| {
        fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|cpuinfo| {
                cpuinfo.lines().find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    (key.trim() == "model name").then(|| value.trim().to_owned())
                })
            })
            .unwrap_or_else(|| format!("{}-unknown-model", std::env::consts::ARCH))
    })
}

fn benchmark_baseline() -> serde_json::Value {
    let text = fs::read_to_string(root().join(BENCHMARK_BASELINE_PATH))
        .expect("benchmark baseline file is readable");
    serde_json::from_str(&text).expect("benchmark baseline file is valid JSON")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M9TimingPolicy {
    StrictControlled,
    ReportOnlyHosted,
}

impl M9TimingPolicy {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value {
            None | Some("strict-controlled") => Ok(Self::StrictControlled),
            Some("report-only-hosted") => Ok(Self::ReportOnlyHosted),
            Some(value) => Err(format!(
                "unsupported SCENA_M9_TIMING_POLICY={value:?}; expected strict-controlled or report-only-hosted"
            )),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::StrictControlled => "strict-controlled",
            Self::ReportOnlyHosted => "report-only-hosted",
        }
    }

    const fn timing_enforced(self) -> bool {
        matches!(self, Self::StrictControlled)
    }
}

fn m9_timing_policy() -> M9TimingPolicy {
    let value = std::env::var("SCENA_M9_TIMING_POLICY").ok();
    M9TimingPolicy::parse(value.as_deref()).unwrap_or_else(|error| panic!("{error}"))
}

fn apply_benchmark_baselines(
    rows: &mut [serde_json::Value],
    baseline: &serde_json::Value,
    lane: &str,
) -> serde_json::Value {
    apply_benchmark_baselines_with_policy(rows, baseline, lane, M9TimingPolicy::StrictControlled)
}

fn apply_benchmark_baselines_with_policy(
    rows: &mut [serde_json::Value],
    baseline: &serde_json::Value,
    lane: &str,
    timing_policy: M9TimingPolicy,
) -> serde_json::Value {
    let mut status = "passed";
    let mut reported_timing_regressions = 0_u64;
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
                "timing_policy": timing_policy.as_str(),
                "timing_enforced": timing_policy.timing_enforced(),
            });
            continue;
        }

        let Some(row_baseline) = benchmark_baseline_for_row(row, baseline, lane) else {
            status = "failed";
            row["baseline_comparison"] = serde_json::json!({
                "status": "failed",
                "reason": "missing stored baseline row",
                "timing_policy": timing_policy.as_str(),
                "timing_enforced": timing_policy.timing_enforced(),
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
        let max_allocations_per_frame = row
            .get("max_allocations_per_frame")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(u64::MAX);
        let p95_allocations_per_frame = row
            .get("p95_allocations_per_frame")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(max_allocations_per_frame);
        let Some(allowed_max_allocations_per_frame) = row_baseline
            .get("max_allocations_per_frame")
            .and_then(serde_json::Value::as_u64)
        else {
            status = "failed";
            row["baseline_comparison"] = serde_json::json!({
                "status": "failed",
                "reason": "missing stored allocation budget",
                "baseline_p95_frame_ms": baseline_p95_frame_ms,
                "allowed_regression_percent": allowed_regression_percent,
                "allowed_p95_frame_ms": allowed_p95,
                "regression_percent": regression_percent,
                "minimum_sample_count": row_minimum_sample_count,
                "p95_allocations_per_frame": p95_allocations_per_frame,
                "max_allocations_per_frame": max_allocations_per_frame,
            });
            continue;
        };
        let p95_prepare_ms = row
            .get("p95_prepare_ms")
            .and_then(serde_json::Value::as_f64);
        let baseline_p95_prepare_ms = row_baseline
            .get("p95_prepare_ms")
            .and_then(serde_json::Value::as_f64);
        let allowed_p95_prepare_ms = baseline_p95_prepare_ms
            .map(|baseline| baseline * (1.0 + allowed_regression_percent / 100.0));
        let prepare_measurement_valid = match (p95_prepare_ms, allowed_p95_prepare_ms) {
            (Some(measured), Some(allowed)) => measured.is_finite() && allowed.is_finite(),
            (None, None) => true,
            _ => false,
        };
        let prepare_status = prepare_measurement_valid
            && match (p95_prepare_ms, allowed_p95_prepare_ms) {
                (Some(measured), Some(allowed)) => measured <= allowed,
                (None, None) => true,
                _ => false,
            };
        let max_allocated_bytes_per_frame = row
            .get("max_allocated_bytes_per_frame")
            .and_then(serde_json::Value::as_u64);
        let allowed_max_allocated_bytes_per_frame = row_baseline
            .get("max_allocated_bytes_per_frame")
            .and_then(serde_json::Value::as_u64);
        let allocation_bytes_status = match (
            max_allocated_bytes_per_frame,
            allowed_max_allocated_bytes_per_frame,
        ) {
            (Some(measured), Some(allowed)) => measured <= allowed,
            (None, None) => true,
            _ => false,
        };
        let sample_count_status = sample_count >= row_minimum_sample_count;
        let frame_measurement_valid = p95_frame_ms.is_finite()
            && baseline_p95_frame_ms.is_finite()
            && baseline_p95_frame_ms > 0.0
            && allowed_p95.is_finite();
        let frame_status = frame_measurement_valid && p95_frame_ms <= allowed_p95;
        let allocation_status = p95_allocations_per_frame <= allowed_max_allocations_per_frame;
        let measurement_status =
            sample_count_status && frame_measurement_valid && prepare_measurement_valid;
        let timing_observation_status = frame_status && prepare_status;
        if measurement_status && !timing_observation_status {
            reported_timing_regressions += 1;
        }
        let timing_gate_status = !timing_policy.timing_enforced() || timing_observation_status;
        let row_status = if measurement_status
            && timing_gate_status
            && allocation_status
            && allocation_bytes_status
        {
            "passed"
        } else {
            status = "failed";
            "failed"
        };

        row["baseline_comparison"] = serde_json::json!({
            "status": row_status,
            "timing_policy": timing_policy.as_str(),
            "timing_enforced": timing_policy.timing_enforced(),
            "timing_gate_status": if timing_policy.timing_enforced() {
                if timing_observation_status { "passed" } else { "failed" }
            } else {
                "reported-only"
            },
            "measurement_status": if measurement_status { "passed" } else { "failed" },
            "sample_count_status": if sample_count_status { "passed" } else { "failed" },
            "baseline_lane": row_baseline
                .get("lane")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("generic"),
            "frame_time_status": if frame_status { "passed" } else { "failed" },
            "prepare_time_status": if prepare_status { "passed" } else { "failed" },
            "allocation_status": if allocation_status { "passed" } else { "failed" },
            "allocation_count_status": if allocation_status { "passed" } else { "failed" },
            "allocation_bytes_status": if allocation_bytes_status { "passed" } else { "failed" },
            "baseline_p95_frame_ms": baseline_p95_frame_ms,
            "allowed_regression_percent": allowed_regression_percent,
            "allowed_p95_frame_ms": allowed_p95,
            "regression_percent": regression_percent,
            "p95_prepare_ms": p95_prepare_ms,
            "baseline_p95_prepare_ms": baseline_p95_prepare_ms,
            "allowed_p95_prepare_ms": allowed_p95_prepare_ms,
            "minimum_sample_count": row_minimum_sample_count,
            "p95_allocations_per_frame": p95_allocations_per_frame,
            "allowed_p95_allocations_per_frame": allowed_max_allocations_per_frame,
            "max_allocations_per_frame": max_allocations_per_frame,
            "allowed_max_allocations_per_frame": allowed_max_allocations_per_frame,
            "max_allocated_bytes_per_frame": max_allocated_bytes_per_frame,
            "allowed_max_allocated_bytes_per_frame": allowed_max_allocated_bytes_per_frame,
        });
    }

    serde_json::json!({
        "status": status,
        "lane": lane,
        "timing_policy": timing_policy.as_str(),
        "timing_enforced": timing_policy.timing_enforced(),
        "reported_timing_regressions": reported_timing_regressions,
        "blocking_contract": if timing_policy.timing_enforced() {
            "sample validity, wall-clock timing, and allocations"
        } else {
            "sample validity and allocations; wall-clock timing is reported only on shared hosted hardware"
        },
        "baseline_path": BENCHMARK_BASELINE_PATH,
        "baseline_sha256": asset_source_hash(BENCHMARK_BASELINE_PATH),
        "metrics": [
            "p95_frame_ms",
            "p95_prepare_ms",
            "p95_allocations_per_frame",
            "max_allocated_bytes_per_frame"
        ],
        "minimum_sample_count": minimum_sample_count,
    })
}

fn benchmark_baseline_for_row<'a>(
    row: &serde_json::Value,
    baseline: &'a serde_json::Value,
    lane: &str,
) -> Option<&'a serde_json::Value> {
    let scene = row.get("scene").and_then(serde_json::Value::as_str)?;
    let backend = row.get("backend").and_then(serde_json::Value::as_str)?;
    let candidates = baseline.get("rows").and_then(serde_json::Value::as_array)?;
    let matches_row = |candidate: &&serde_json::Value| {
        candidate.get("scene").and_then(serde_json::Value::as_str) == Some(scene)
            && candidate.get("backend").and_then(serde_json::Value::as_str) == Some(backend)
    };
    candidates
        .iter()
        .filter(matches_row)
        .find(|candidate| candidate.get("lane").and_then(serde_json::Value::as_str) == Some(lane))
        .or_else(|| {
            candidates.iter().filter(matches_row).find(|candidate| {
                candidate
                    .get("lane")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
            })
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
    let prepare_start = Instant::now();
    renderer.prepare(&mut scene).expect("scene prepares");
    let prepare_samples = [prepare_start.elapsed().as_secs_f64() * 1000.0];
    renderer.render(&scene, camera).expect("warm render");
    let mut samples = Vec::with_capacity(BENCHMARK_SAMPLE_COUNT);
    let mut allocation_samples = Vec::with_capacity(BENCHMARK_SAMPLE_COUNT);
    let mut allocation_byte_samples = Vec::with_capacity(BENCHMARK_SAMPLE_COUNT);
    let mut outcome = None;
    for _ in 0..BENCHMARK_SAMPLE_COUNT {
        let start = Instant::now();
        start_allocation_counting();
        let next = renderer.render(&scene, camera);
        stop_allocation_counting();
        let allocation_count = allocation_count();
        let allocation_bytes = allocation_bytes();
        let next = next.expect("idle render skips");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        allocation_samples.push(allocation_count);
        allocation_byte_samples.push(allocation_bytes);
        outcome = Some(next);
    }
    let outcome = outcome.expect("benchmark loop records an outcome");
    benchmark_row(BenchmarkRowInput {
        scene: "idle",
        backend: renderer.capabilities().backend,
        samples: &samples,
        allocation_samples: &allocation_samples,
        allocation_byte_samples: &allocation_byte_samples,
        prepare_samples: &prepare_samples,
        draw_calls: outcome.draw_calls,
        skipped: outcome.skipped,
        fixture: BenchmarkFixture {
            width: 64,
            height: 64,
            source: "builtin:unlit-triangle-on-change",
            sample_count_policy: "100 timed idle render calls after one warm render",
        },
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

#[derive(Clone, Copy)]
struct FeatureMatrixResolution {
    id: &'static str,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
enum FeatureMatrixFeature {
    AaOff,
    Msaa4,
    Ssaa2,
    Ssr,
    AreaLights,
    DepthOfField,
}

const FEATURE_MATRIX_RESOLUTIONS: &[FeatureMatrixResolution] = &[
    FeatureMatrixResolution {
        id: "1080p",
        width: 1920,
        height: 1080,
    },
    FeatureMatrixResolution {
        id: "4k",
        width: 3840,
        height: 2160,
    },
];

const FEATURE_MATRIX_FEATURES: &[FeatureMatrixFeature] = &[
    FeatureMatrixFeature::AaOff,
    FeatureMatrixFeature::Msaa4,
    FeatureMatrixFeature::Ssaa2,
    FeatureMatrixFeature::Ssr,
    FeatureMatrixFeature::AreaLights,
    FeatureMatrixFeature::DepthOfField,
];
const REQUIRED_RELEASE_FEATURE_MATRIX_ROWS: &[&str] = &[
    "headless-feature-matrix-4k-ssaa2",
    "headless-feature-matrix-1080p-ssr-on",
];

impl FeatureMatrixFeature {
    const fn id(self) -> &'static str {
        match self {
            Self::AaOff => "aa-off",
            Self::Msaa4 => "msaa4",
            Self::Ssaa2 => "ssaa2",
            Self::Ssr => "ssr-on",
            Self::AreaLights => "area-lights",
            Self::DepthOfField => "dof-on",
        }
    }

    const fn part_a_feature(self) -> &'static str {
        match self {
            Self::AaOff | Self::Msaa4 | Self::Ssaa2 => "A1 anti-aliasing and supersampling",
            Self::Ssr => "A2 screen-space reflections",
            Self::AreaLights => "A3 area lights",
            Self::DepthOfField => "A4 depth of field",
        }
    }

    fn configure_renderer(self, renderer: &mut Renderer) {
        match self {
            Self::AaOff => renderer.set_anti_aliasing(AntiAliasing::None),
            Self::Msaa4 => renderer.set_anti_aliasing(AntiAliasing::Msaa4),
            Self::Ssaa2 => {
                renderer.set_anti_aliasing(AntiAliasing::None);
                renderer
                    .set_supersample_factor(2)
                    .expect("feature-matrix ssaa2 target is valid");
                renderer.set_reconstruction_filter(ReconstructionFilter::Tent);
            }
            Self::Ssr => {
                renderer.set_anti_aliasing(AntiAliasing::None);
                renderer.set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::default()));
            }
            Self::AreaLights => renderer.set_anti_aliasing(AntiAliasing::None),
            Self::DepthOfField => {
                renderer.set_anti_aliasing(AntiAliasing::None);
                renderer.set_depth_of_field(Some(DepthOfFieldConfig::new(2.5, 1.2, 6)));
            }
        }
    }
}

fn benchmark_feature_matrix_deferred_rows() -> Vec<serde_json::Value> {
    FEATURE_MATRIX_RESOLUTIONS
        .iter()
        .flat_map(|resolution| {
            FEATURE_MATRIX_FEATURES
                .iter()
                .copied()
                .map(move |feature| benchmark_feature_matrix_deferred_row(*resolution, feature))
        })
        .collect()
}

fn benchmark_feature_matrix_deferred_row(
    resolution: FeatureMatrixResolution,
    feature: FeatureMatrixFeature,
) -> serde_json::Value {
    serde_json::json!({
        "scene": feature_matrix_scene_name(resolution, feature),
        "backend": "Headless",
        "status": "deferred-to-dedicated-performance-lane",
        "sample_count": 0,
        "feature_matrix": feature_matrix_metadata(resolution, feature),
        "fixture": {
            "source": "generated:feature-matrix-product-scene",
            "width": resolution.width,
            "height": resolution.height,
            "sample_count_policy": "not measured in cargo test; requires dedicated 4K feature-matrix performance lane with 100+ timed render samples",
        },
        "regression_threshold_percent": 5.0,
    })
}

fn benchmark_feature_matrix_measured_rows(sample_count: usize) -> Vec<serde_json::Value> {
    FEATURE_MATRIX_RESOLUTIONS
        .iter()
        .flat_map(|resolution| {
            FEATURE_MATRIX_FEATURES.iter().copied().map(move |feature| {
                benchmark_feature_matrix_measured_row(*resolution, feature, sample_count)
            })
        })
        .collect()
}

fn benchmark_feature_matrix_measured_row(
    resolution: FeatureMatrixResolution,
    feature: FeatureMatrixFeature,
    sample_count: usize,
) -> serde_json::Value {
    let (mut scene, camera, assets) = feature_matrix_scene(feature);
    let scene_name = feature_matrix_scene_name(resolution, feature);
    let mut row = benchmark_scene_with_renderer_setup(
        BenchmarkSceneInput {
            name: &scene_name,
            width: resolution.width,
            height: resolution.height,
            fixture_source: "generated:feature-matrix-product-scene",
            sample_count,
            sample_count_policy: "dedicated performance lane with 100 timed render calls after one warm render",
        },
        &mut scene,
        Some(&assets),
        camera,
        |renderer| feature.configure_renderer(renderer),
    );
    row["feature_matrix"] = feature_matrix_metadata(resolution, feature);
    row
}

fn feature_matrix_scene_name(
    resolution: FeatureMatrixResolution,
    feature: FeatureMatrixFeature,
) -> String {
    format!("headless-feature-matrix-{}-{}", resolution.id, feature.id())
}

fn feature_matrix_metadata(
    resolution: FeatureMatrixResolution,
    feature: FeatureMatrixFeature,
) -> serde_json::Value {
    serde_json::json!({
        "resolution": resolution.id,
        "width": resolution.width,
        "height": resolution.height,
        "feature_set": feature.id(),
        "part_a_feature": feature.part_a_feature(),
        "reports_frame_time_cost": true,
    })
}

fn feature_matrix_scene(feature: FeatureMatrixFeature) -> (Scene, scena::CameraKey, Assets) {
    let assets = Assets::new();
    let body_geometry = assets.create_geometry(GeometryDesc::sphere(0.45, 32, 16));
    let floor_geometry = assets.create_geometry(GeometryDesc::box_xyz(2.8, 0.04, 2.8));
    let body_material = assets.create_material(MaterialDesc::chrome().with_double_sided(false));
    let floor_material =
        assets.create_material(MaterialDesc::rough_metal(Color::from_srgb_u8(90, 94, 105)));
    let mut scene = Scene::new();
    scene
        .add_studio_lighting()
        .expect("feature matrix studio lighting inserts");
    if matches!(feature, FeatureMatrixFeature::AreaLights) {
        scene
            .area_light(AreaLight::softbox())
            .transform(Transform::at(Vec3::new(0.0, 1.6, 1.1)))
            .add()
            .expect("feature matrix area light inserts");
    }
    scene
        .mesh(body_geometry, body_material)
        .transform(Transform::at(Vec3::new(0.0, 0.45, 0.0)))
        .add()
        .expect("feature matrix body inserts");
    scene
        .mesh(floor_geometry, floor_material)
        .transform(Transform::at(Vec3::new(0.0, -0.02, 0.0)))
        .add()
        .expect("feature matrix floor inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let camera_node = scene.camera_node(camera).expect("camera node resolves");
    scene
        .set_transform(
            camera_node,
            Transform::at(Vec3::new(0.0, 1.1, 2.9)).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
        )
        .expect("feature matrix camera positions");
    (scene, camera, assets)
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
    assert!(
        row["p95_allocations_per_frame"].as_u64().is_some(),
        "benchmark row records p95 per-frame allocation count"
    );
    assert!(
        row["max_allocations_per_frame"].as_u64().is_some(),
        "benchmark row records max per-frame allocation count"
    );
    assert_eq!(row["fixture"]["width"], 64);
    assert_eq!(row["fixture"]["height"], 64);
}

#[test]
fn pf00_benchmark_rows_measure_allocation_bytes_and_prepare_distributions() {
    let (mut scene, camera) = scene_with_triangle();
    let row = benchmark_scene(
        "pf00-contract",
        32,
        32,
        "builtin:unlit-triangle",
        &mut scene,
        None,
        camera,
    );

    for metric in [
        "p50_allocated_bytes_per_frame",
        "p95_allocated_bytes_per_frame",
        "max_allocated_bytes_per_frame",
        "p50_prepare_ms",
        "p95_prepare_ms",
        "max_prepare_ms",
    ] {
        assert!(
            row[metric].as_f64().is_some(),
            "PF00 row must contain measured {metric}: {row:#}"
        );
    }
    assert!(
        row["p95_allocated_bytes_per_frame"].as_u64().is_some(),
        "allocation bytes must be an integer measurement: {row:#}"
    );
    assert_eq!(row["prepare_sample_count"], 100);
}

#[test]
fn pf00_stored_baselines_match_the_advertised_five_percent_policy() {
    let baseline = benchmark_baseline();
    for row in baseline["rows"].as_array().expect("baseline rows") {
        assert_eq!(
            row["allowed_regression_percent"], 5.0,
            "stored baseline row {} must enforce the advertised 5% policy",
            row["scene"]
        );
    }
}

#[test]
fn pf00_baseline_comparison_gates_prepare_p95_and_allocation_bytes() {
    let mut rows = vec![serde_json::json!({
        "scene": "pf00-gated-row",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 10.0,
        "p95_prepare_ms": 20.0,
        "max_allocations_per_frame": 2,
        "max_allocated_bytes_per_frame": 2_048,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [{
            "scene": "pf00-gated-row",
            "backend": "Headless",
            "p95_frame_ms": 10.0,
            "p95_prepare_ms": 10.0,
            "allowed_regression_percent": 5.0,
            "max_allocations_per_frame": 4,
            "max_allocated_bytes_per_frame": 1_024,
        }]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "test-lane");

    assert_eq!(summary["status"], "failed");
    assert_eq!(
        rows[0]["baseline_comparison"]["frame_time_status"],
        "passed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["prepare_time_status"],
        "failed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["allocation_count_status"],
        "passed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["allocation_bytes_status"],
        "failed"
    );
}

#[test]
fn m9_benchmark_baseline_prefers_an_exact_lane_over_the_generic_fallback() {
    let mut rows = vec![serde_json::json!({
        "scene": "lane-specific-row",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 10.0,
        "p95_prepare_ms": 15.0,
        "max_allocations_per_frame": 0,
        "max_allocated_bytes_per_frame": 0,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [
            {
                "scene": "lane-specific-row",
                "backend": "Headless",
                "p95_frame_ms": 10.0,
                "p95_prepare_ms": 10.0,
                "allowed_regression_percent": 5.0,
                "max_allocations_per_frame": 0,
                "max_allocated_bytes_per_frame": 0
            },
            {
                "scene": "lane-specific-row",
                "backend": "Headless",
                "lane": "macos-metal",
                "p95_frame_ms": 10.0,
                "p95_prepare_ms": 15.0,
                "allowed_regression_percent": 5.0,
                "max_allocations_per_frame": 0,
                "max_allocated_bytes_per_frame": 0
            }
        ]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "macos-metal");

    assert_eq!(summary["status"], "passed");
    assert_eq!(
        rows[0]["baseline_comparison"]["baseline_lane"],
        "macos-metal"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["baseline_p95_prepare_ms"],
        15.0
    );
}

#[test]
fn m9_linux_native_baseline_covers_exact_source_hosted_measurement() {
    let mut rows = vec![serde_json::json!({
        "scene": "larger-industrial-gltf",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 56.560595,
        "p95_prepare_ms": 1930.651602,
        "max_allocations_per_frame": 0,
        "max_allocated_bytes_per_frame": 0,
    })];
    let baseline = benchmark_baseline();

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "linux-native-vulkan");

    assert_eq!(summary["status"], "passed");
    assert_eq!(
        rows[0]["baseline_comparison"]["baseline_lane"],
        "linux-native-vulkan"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["baseline_p95_frame_ms"],
        60.0
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["baseline_p95_prepare_ms"],
        1950.0
    );
}

#[test]
fn pf00_profiled_pick_benchmark_row_records_work_and_byte_distributions() {
    let row = benchmark_profiled_pick_workload(32, 3);

    assert_eq!(row["id"], "pick-100k-triangle-deformed-undeformed");
    assert_eq!(row["fixture"]["triangle_count"], 32);
    assert_eq!(row["fixture"]["release_triangle_count"], 100_000);
    assert_eq!(
        row["distributions"]["undeformed_pick_ms"]["sample_count"],
        3
    );
    assert_eq!(row["distributions"]["deformed_pick_ms"]["sample_count"], 3);
    assert_eq!(row["counters"]["undeformed"]["triangles_considered"], 32);
    assert_eq!(row["counters"]["deformed"]["triangles_considered"], 32);
    assert_eq!(
        row["counters"]["undeformed"]["ray_triangle_intersection_tests"],
        32
    );
    assert_eq!(
        row["counters"]["deformed"]["ray_triangle_intersection_tests"],
        32
    );
    assert_eq!(
        row["counters"]["undeformed"]["deformed_vertex_bytes_materialized"],
        0
    );
    assert_eq!(
        row["counters"]["deformed"]["deformed_vertices_materialized"],
        96
    );
    assert!(
        row["allocations"]["deformed"]["max_allocated_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "morph picking must report measured allocation bytes: {row:#}"
    );
}

#[test]
fn pf00_animation_benchmark_row_records_work_and_byte_distributions() {
    let row = benchmark_profiled_animation_workload(4, 8, 4, 3);

    assert_eq!(row["id"], "animation-many-channels-keyframes-weights");
    assert_eq!(row["fixture"]["channel_count"], 4);
    assert_eq!(row["fixture"]["keyframe_count"], 8);
    assert_eq!(row["fixture"]["weight_width"], 4);
    assert_eq!(row["distributions"]["advance_ms"]["sample_count"], 3);
    assert_eq!(row["counters"]["channels_scanned"], 4);
    assert_eq!(row["counters"]["weight_values_written"], 8);
    assert_eq!(row["counters"]["clip_clone_bytes"], 0);
    assert!(
        row["counters"]["keyframe_intervals_tested"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "profiled animation must expose actual keyframe search work: {row:#}"
    );
    assert!(
        row["counters"]["bytes_cloned_or_copied"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "profiled animation must expose actual copied bytes: {row:#}"
    );
    assert_eq!(row["allocations"]["max_allocation_count"], 0);
    assert_eq!(row["allocations"]["max_allocated_bytes"], 0);
}

#[test]
fn pf00_tangent_benchmark_row_records_static_and_deformed_work() {
    let row = benchmark_profiled_tangent_workload(16, 3);

    assert_eq!(row["id"], "tangent-generation-static-deformed");
    assert_eq!(row["fixture"]["triangle_count"], 16);
    assert_eq!(row["distributions"]["static_tangent_ms"]["sample_count"], 3);
    assert_eq!(
        row["distributions"]["deformed_tangent_ms"]["sample_count"],
        3
    );
    assert_eq!(
        row["counters"]["static_cold"]["generated_tangent_triangles"],
        16
    );
    assert_eq!(row["counters"]["static"]["generated_tangent_triangles"], 0);
    assert_eq!(
        row["counters"]["deformed"]["generated_tangent_triangles"],
        16
    );
    assert_eq!(
        row["counters"]["static_cold"]["generated_tangent_vertices"],
        48
    );
    assert_eq!(row["counters"]["static"]["generated_tangent_vertices"], 0);
    assert_eq!(row["counters"]["static"]["generated_tangent_cache_hits"], 1);
    assert_eq!(
        row["counters"]["static_cold"]["generated_tangent_cache_misses"],
        1
    );
    assert_eq!(
        row["counters"]["deformed"]["generated_tangent_cache_hits"],
        0
    );
    assert_eq!(
        row["counters"]["deformed"]["generated_tangent_vertices"],
        48
    );
    assert_eq!(
        row["counters"]["static"]["deformed_vertex_bytes_materialized"],
        0
    );
    assert!(
        row["counters"]["deformed"]["deformed_vertex_bytes_materialized"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "deformed tangent preparation must report materialized vertex bytes: {row:#}"
    );
    assert!(
        row["allocations"]["deformed"]["max_allocated_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "deformed tangent preparation must report measured allocation bytes: {row:#}"
    );
    assert!(
        row["counters"]["static"]["asset_storage_lock_acquisitions"]
            .as_u64()
            .is_some_and(|locks| locks > 0),
        "profiled asset preparation must expose actual storage-lock acquisitions: {row:#}"
    );
}

#[test]
fn pf00_shadow_benchmark_row_records_scaling_and_intersection_work() {
    let row = benchmark_profiled_shadow_workload(&[1, 4], 3);
    let scales = row["scales"].as_array().expect("shadow scale rows");

    assert_eq!(row["id"], "shadow-scaling-directional-area");
    assert_eq!(scales.len(), 2);
    assert_eq!(scales[1]["triangle_count"], 4);
    assert_eq!(scales[1]["directional"]["prepare_ms"]["sample_count"], 3);
    assert_eq!(scales[1]["area"]["prepare_ms"]["sample_count"], 3);
    assert!(
        scales[1]["directional"]["counters"]["shadow_rays"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        scales[1]["area"]["counters"]["area_light_samples"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    let area_bounds = scales[1]["area"]["counters"]["bvh_node_bounds_tests"]
        .as_u64()
        .expect("area-light preparation reports BVH bounds work");
    let area_exact = scales[1]["area"]["counters"]["ray_triangle_intersection_tests"]
        .as_u64()
        .expect("area-light preparation reports exact intersection work");
    let area_rays = scales[1]["area"]["counters"]["shadow_rays"]
        .as_u64()
        .expect("area-light preparation reports shadow rays");
    assert!(
        area_bounds > 0,
        "area-light BVH traversal must be measured: {row:#}"
    );
    assert!(
        area_exact < area_rays * scales[1]["triangle_count"].as_u64().unwrap(),
        "BVH should reject work before exact intersections; zero is an honest measured result: {row:#}"
    );
}

#[test]
fn pf00_cpu_texture_bake_row_separates_qualifying_and_nonqualifying_work() {
    let row = benchmark_profiled_cpu_texture_bake_workload(3);

    assert_eq!(row["id"], "cpu-texture-bake-qualifying-nonqualifying");
    assert_eq!(
        row["distributions"]["qualifying_bake_ms"]["sample_count"],
        3
    );
    assert_eq!(
        row["distributions"]["nonqualifying_bake_ms"]["sample_count"],
        3
    );
    let texture_roles = row["texture_roles"].as_object().expect("texture role rows");
    assert_eq!(texture_roles.len(), 15);
    for (role, metrics) in texture_roles {
        let triangles = metrics["counters"]["cpu_bake_subdivided_triangles"]
            .as_u64()
            .expect("role triangle counter");
        assert!(
            triangles > 1 && triangles <= 48 * 48,
            "{role} must use bounded adaptive subdivision: {metrics:#}"
        );
        assert!(
            metrics["counters"]["texture_samples"]
                .as_u64()
                .is_some_and(|samples| samples > 0),
            "{role} must exercise its texture sampling path: {metrics:#}"
        );
    }
    assert_eq!(
        row["counters"]["nonqualifying"]["cpu_bake_subdivided_triangles"],
        1
    );
    assert_eq!(row["counters"]["nonqualifying"]["texture_samples"], 0);
    assert!(
        row["allocations"]["qualifying"]["max_allocated_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0),
        "qualifying CPU texture bake must report measured allocation bytes: {row:#}"
    );
}

#[test]
fn pf00_remaining_transform_draw_environment_and_gpu_contract_rows_are_fail_closed() {
    let transform = benchmark_profiled_one_node_transform_workload(3);
    assert_eq!(transform["id"], "one-node-transform-prepare-render");
    assert_eq!(
        transform["cpu"]["distributions"]["prepare_ms"]["sample_count"],
        3
    );
    assert_eq!(
        transform["cpu"]["distributions"]["render_ms"]["sample_count"],
        3
    );
    assert_eq!(transform["cpu"]["counters"]["render"]["readback_copies"], 0);

    let draw_uniforms = benchmark_profiled_draw_uniform_workload(16, 3);
    assert_eq!(
        draw_uniforms["id"],
        "draw-uniform-indexing-many-unique-transforms"
    );
    if draw_uniforms["status"] != "hardware-unavailable" {
        assert_eq!(draw_uniforms["counters"]["draw_uniform_unique_values"], 16);
        assert!(
            draw_uniforms["counters"]["draw_uniform_lookup_probes"]
                .as_u64()
                .is_some_and(|probes| (16..=32).contains(&probes)),
            "16 unique transforms must use near-linear indexed lookup work: {draw_uniforms:#}"
        );
        assert!(
            draw_uniforms["counters"]["draw_uniform_bytes_copied"]
                .as_u64()
                .is_some_and(|bytes| bytes > 0)
        );
    }

    let environment = benchmark_profiled_environment_bake_workload(1);
    assert_eq!(environment["id"], "environment-bake-cold-sidecar-hit");
    assert_eq!(
        environment["distributions"]["cold_bake_ms"]["sample_count"],
        1
    );
    assert_eq!(
        environment["distributions"]["sidecar_hit_ms"]["sample_count"],
        1
    );
    assert!(
        environment["counters"]["source_texture_samples"]
            .as_u64()
            .is_some_and(|samples| samples > 0),
        "profiled environment baking must expose actual cubemap sample calls: {environment:#}"
    );
    assert!(
        environment["counters"]["brdf_integration_samples"]
            .as_u64()
            .is_some_and(|samples| samples > 0),
        "profiled environment baking must expose actual BRDF integration work: {environment:#}"
    );

    let native_capture = benchmark_profiled_native_capture_workload(3);
    if native_capture["status"] != "hardware-unavailable" {
        assert_eq!(
            native_capture["distributions"]["present_only_ms"]["sample_count"],
            3
        );
        assert_eq!(
            native_capture["distributions"]["synchronous_capture_ms"]["sample_count"],
            3
        );
        assert_eq!(
            native_capture["distributions"]["asynchronous_capture_ms"]["sample_count"],
            3
        );
        assert_eq!(
            native_capture["counters"]["asynchronous_batch"]["peak_readbacks_in_flight"],
            2
        );
    }
    let output_settings = benchmark_profiled_gpu_output_settings_workload(1);
    if output_settings["status"] != "hardware-unavailable" {
        let settings = output_settings["settings"]
            .as_array()
            .expect("output-setting rows");
        assert_eq!(settings.len(), 8);
        for setting in settings
            .iter()
            .filter(|setting| setting["status"] == "contract-scale-measured")
        {
            assert_eq!(
                setting["distributions"]["prepare_output_ms"]["sample_count"],
                1
            );
            assert_eq!(
                setting["distributions"]["first_prepared_render_ms"]["sample_count"],
                1
            );
            for metric in [
                "gpu_buffer_creations",
                "gpu_texture_creations",
                "gpu_pipeline_creations",
                "gpu_bind_group_creations",
                "gpu_shader_module_creations",
                "readback_copies",
                "blocking_polls",
            ] {
                assert_eq!(
                    setting["counters"]["render"][metric], 0,
                    "prepared present-only render must not create resources or read back: {setting:#}"
                );
            }
        }
    }
    assert_eq!(native_capture["release_evidence"], false);
    assert!(matches!(
        native_capture["status"].as_str(),
        Some("contract-scale-measured") | Some("hardware-unavailable")
    ));
    assert_eq!(output_settings["release_evidence"], false);
    assert!(matches!(
        output_settings["status"].as_str(),
        Some("contract-scale-or-unsupported") | Some("hardware-unavailable")
    ));
}

#[test]
fn m9_benchmark_rows_record_stored_baseline_comparison() {
    let mut rows = vec![
        serde_json::json!({
            "scene": "static-viewer",
            "backend": "Headless",
            "sample_count": 100,
            "p95_frame_ms": 10.0,
            "p95_prepare_ms": 8.0,
            "max_allocations_per_frame": 2,
            "max_allocated_bytes_per_frame": 512,
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
                "p95_prepare_ms": 10.0,
                "allowed_regression_percent": 5.0,
                "max_allocations_per_frame": 4,
                "max_allocated_bytes_per_frame": 1024
            }
        ]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "test-lane");

    assert_eq!(summary["status"], "passed");
    assert_eq!(summary["baseline_path"], BENCHMARK_BASELINE_PATH);
    assert_eq!(
        summary["metrics"],
        serde_json::json!([
            "p95_frame_ms",
            "p95_prepare_ms",
            "p95_allocations_per_frame",
            "max_allocated_bytes_per_frame"
        ])
    );
    assert_eq!(rows[0]["baseline_comparison"]["status"], "passed");
    assert_eq!(
        rows[1]["baseline_comparison"]["status"], "deferred",
        "dedicated-lane benchmark rows must be explicit deferrals, not silent misses"
    );
}

#[test]
#[ignore = "requires the serial M9 allocation measurement lane"]
fn m9_parallel_cpu_render_has_low_steady_state_allocations() {
    let (mut scene, camera, assets) = feature_matrix_scene(FeatureMatrixFeature::AaOff);
    let mut renderer = Renderer::headless(1024, 768).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer
        .render(&scene, camera)
        .expect("warm render initializes workers");

    start_allocation_counting();
    let outcome = renderer.render(&scene, camera);
    stop_allocation_counting();
    let allocations = allocation_count();

    outcome.expect("steady render succeeds");
    assert!(
        allocations <= 16,
        "warm parallel CPU render should reuse worker resources; observed {allocations} allocations of sizes {:?}",
        allocation_size_trace()
    );
}

#[test]
#[ignore = "requires the serial M9 allocation measurement lane"]
fn m9_parallel_cpu_ssr_render_reuses_steady_state_row_band_scratch() {
    let (mut scene, camera, assets) = feature_matrix_scene(FeatureMatrixFeature::Ssr);
    let mut renderer = Renderer::headless(1024, 768).expect("renderer builds");
    FeatureMatrixFeature::Ssr.configure_renderer(&mut renderer);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer
        .render(&scene, camera)
        .expect("warm render initializes SSR row-band scratch");

    start_allocation_counting();
    let outcome = renderer.render(&scene, camera);
    stop_allocation_counting();
    let allocations = allocation_count();

    outcome.expect("steady SSR render succeeds");
    assert!(
        allocations <= 16,
        "warm parallel CPU SSR render should reuse row-band scratch; observed {allocations} allocations of sizes {:?}",
        allocation_size_trace()
    );
}

#[test]
#[ignore = "requires the serial M9 allocation measurement lane"]
fn m9_cpu_supersample_render_reuses_steady_state_scratch_buffers() {
    let (mut scene, camera, assets) = feature_matrix_scene(FeatureMatrixFeature::Msaa4);
    let mut renderer = Renderer::headless(1024, 768).expect("renderer builds");
    FeatureMatrixFeature::Msaa4.configure_renderer(&mut renderer);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer
        .render(&scene, camera)
        .expect("warm render initializes supersample scratch");

    start_allocation_counting();
    let outcome = renderer.render(&scene, camera);
    stop_allocation_counting();
    let allocations = allocation_count();

    outcome.expect("steady supersample render succeeds");
    assert!(
        allocations <= 16,
        "warm CPU supersample render should reuse supersample scratch buffers; observed {allocations} allocations of sizes {:?}",
        allocation_size_trace()
    );
}

#[test]
fn m9_feature_matrix_declares_resolution_feature_cost_rows() {
    let rows = benchmark_feature_matrix_deferred_rows();
    assert_eq!(
        rows.len(),
        FEATURE_MATRIX_RESOLUTIONS.len() * FEATURE_MATRIX_FEATURES.len()
    );
    for resolution in FEATURE_MATRIX_RESOLUTIONS {
        for feature in FEATURE_MATRIX_FEATURES {
            let name = feature_matrix_scene_name(*resolution, *feature);
            let row = rows
                .iter()
                .find(|row| {
                    row.get("scene")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|scene| scene == name)
                })
                .unwrap_or_else(|| panic!("feature matrix row missing {name}"));
            assert_eq!(
                row["status"], "deferred-to-dedicated-performance-lane",
                "normal M9 test lane must be explicit about unmeasured feature-matrix rows"
            );
            assert_eq!(row["feature_matrix"]["resolution"], resolution.id);
            assert_eq!(row["feature_matrix"]["feature_set"], feature.id());
            assert_eq!(
                row["feature_matrix"]["part_a_feature"],
                feature.part_a_feature()
            );
            assert_eq!(row["feature_matrix"]["reports_frame_time_cost"], true);
            assert_eq!(row["fixture"]["width"], resolution.width);
            assert_eq!(row["fixture"]["height"], resolution.height);
        }
    }
    for required in REQUIRED_RELEASE_FEATURE_MATRIX_ROWS {
        assert!(
            rows.iter()
                .any(|row| row["scene"].as_str() == Some(required)),
            "release-pinned M9 feature matrix row missing: {required}"
        );
    }
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
            "run_hint": "Run SCENA_RUN_DEDICATED_4K_BENCHMARK=1 cargo test --profile perf-test --test m9_platform_release m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact -- --nocapture on the dedicated performance lane to write m9-benchmarks-4k.json.",
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
    assert_eq!(
        rows.len(),
        1 + FEATURE_MATRIX_RESOLUTIONS.len() * FEATURE_MATRIX_FEATURES.len()
    );
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

fn validate_pf00_complete_hardware_summary(summary: &serde_json::Value) -> Result<(), String> {
    if summary["schema"] != "scena.windows_complete_hardware_proof.v1" {
        return Err("PF00 hardware summary has the wrong schema".to_owned());
    }
    if summary["status"] != "passed" || summary["hardware_evidence"] != true {
        return Err("PF00 hardware summary is not passing hardware evidence".to_owned());
    }
    let mut browser_backends = summary["coverage"]["browser_backends"]
        .as_array()
        .ok_or_else(|| "PF00 hardware summary is missing browser backends".to_owned())?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    browser_backends.sort_unstable();
    if browser_backends != ["webgl2", "webgpu"] {
        return Err("PF00 hardware summary must prove both WebGL2 and WebGPU".to_owned());
    }
    if summary["coverage"]["native_surface"] != true {
        return Err("PF00 hardware summary must prove native surface presentation".to_owned());
    }
    if summary["coverage"]["native_semantic_aov"] != true {
        return Err("PF00 hardware summary must prove native semantic AOV execution".to_owned());
    }
    let hashes = summary["artifact_sha256"]
        .as_object()
        .ok_or_else(|| "PF00 hardware summary is missing artifact hashes".to_owned())?;
    for relative in PF00_REQUIRED_HARDWARE_ARTIFACTS {
        let hash = hashes
            .get(relative)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("PF00 hardware summary is missing {relative}"))?;
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "PF00 hardware summary has an invalid hash for {relative}"
            ));
        }
    }
    Ok(())
}

fn pf00_release_evidence_ready(
    measurements_complete: bool,
    hardware_complete: bool,
    source_checksums_current: bool,
    commit: &str,
) -> bool {
    measurements_complete
        && hardware_complete
        && source_checksums_current
        && commit.len() == 40
        && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        && commit.bytes().any(|byte| byte != b'0')
}

fn classify_pf00_workload_provenance(workload: &mut serde_json::Value, commit: &str) {
    let measurement_evidence = workload["release_evidence"] == true;
    let release_evidence = pf00_release_evidence_ready(measurement_evidence, true, true, commit);
    workload["measurement_evidence"] = serde_json::json!(measurement_evidence);
    workload["release_evidence"] = serde_json::json!(release_evidence);
    workload["release_provenance"] = if release_evidence {
        serde_json::json!({
            "status": "exact-commit",
            "commit_sha": commit,
        })
    } else {
        serde_json::json!({
            "status": "unavailable",
            "reason": if measurement_evidence {
                "the measured workload is not bound to one exact source commit"
            } else {
                "the workload is not complete at release measurement scale"
            },
        })
    };
}

fn validate_pf00_existing_measurement_artifact(
    workload: &serde_json::Value,
    expected_id: &str,
) -> Result<serde_json::Value, String> {
    if workload["schema"] != "scena.performance_workload.v1" {
        return Err(format!("PF00 workload {expected_id} has the wrong schema"));
    }
    if workload["id"] != expected_id {
        return Err(format!("PF00 workload {expected_id} has the wrong id"));
    }
    if !matches!(
        workload["status"].as_str(),
        Some("measured") | Some("measured-headless-cpu-and-gpu")
    ) {
        return Err(format!("PF00 workload {expected_id} is not measured"));
    }
    let measurement_evidence =
        workload["measurement_evidence"] == true || workload["release_evidence"] == true;
    if !measurement_evidence {
        return Err(format!(
            "PF00 workload {expected_id} is not complete measurement evidence"
        ));
    }
    let commit = workload["commit_sha"]
        .as_str()
        .filter(|commit| !commit.trim().is_empty())
        .ok_or_else(|| format!("PF00 workload {expected_id} is missing commit_sha"))?;
    let timestamp = workload["timestamp_unix_seconds"]
        .as_u64()
        .filter(|timestamp| *timestamp > 0)
        .ok_or_else(|| format!("PF00 workload {expected_id} is missing timestamp_unix_seconds"))?;
    let checksums = workload["source_checksums"]
        .as_array()
        .filter(|checksums| checksums.len() >= 2)
        .ok_or_else(|| format!("PF00 workload {expected_id} is missing source checksums"))?;
    let mut source_checksums_current = true;
    for checksum in checksums {
        let relative = checksum["path"]
            .as_str()
            .filter(|relative| !relative.trim().is_empty())
            .ok_or_else(|| format!("PF00 workload {expected_id} has a checksum without a path"))?;
        let expected = checksum["sha256"]
            .as_str()
            .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| format!("PF00 workload {expected_id} has an invalid source checksum"))?;
        source_checksums_current &=
            sha256_file_hex(&root().join(relative)).is_ok_and(|actual| actual == expected);
    }
    let mut distribution_sample_counts = Vec::new();
    collect_pf00_distribution_sample_counts(workload, &mut distribution_sample_counts);
    if distribution_sample_counts.is_empty()
        || distribution_sample_counts
            .iter()
            .any(|sample_count| *sample_count < BENCHMARK_SAMPLE_COUNT as u64)
    {
        return Err(format!(
            "PF00 workload {expected_id} does not contain only release-scale distributions"
        ));
    }
    let release_evidence =
        pf00_release_evidence_ready(measurement_evidence, true, source_checksums_current, commit);
    let release_rejection_code = if release_evidence {
        serde_json::Value::Null
    } else if commit == "local-checkout" {
        serde_json::json!("LOCAL_CHECKOUT_COMMIT")
    } else if !source_checksums_current {
        serde_json::json!("SOURCE_CHECKSUM_MISMATCH")
    } else {
        serde_json::json!("SOURCE_PROVENANCE_UNAVAILABLE")
    };
    Ok(serde_json::json!({
        "id": expected_id,
        "measurement_evidence": true,
        "release_evidence": release_evidence,
        "release_rejection_code": release_rejection_code,
        "source_commit_sha": commit,
        "source_timestamp_unix_seconds": timestamp,
        "source_checksums_current": source_checksums_current,
        "distribution_count": distribution_sample_counts.len(),
        "minimum_sample_count": distribution_sample_counts.into_iter().min(),
    }))
}

fn collect_pf00_distribution_sample_counts(value: &serde_json::Value, counts: &mut Vec<u64>) {
    match value {
        serde_json::Value::Object(object) => {
            if object.contains_key("p50_ms")
                && object.contains_key("p95_ms")
                && let Some(sample_count) = object
                    .get("sample_count")
                    .and_then(serde_json::Value::as_u64)
            {
                counts.push(sample_count);
            }
            for child in object.values() {
                collect_pf00_distribution_sample_counts(child, counts);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                collect_pf00_distribution_sample_counts(child, counts);
            }
        }
        _ => {}
    }
}

fn sha256_file_hex(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn load_pf00_complete_hardware_proof() -> Result<serde_json::Value, String> {
    let proof_root = std::env::var_os("SCENA_HARDWARE_PROOF_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().to_path_buf());
    let summary_path = proof_root.join(PF00_HARDWARE_SUMMARY_PATH);
    let summary_text = fs::read_to_string(&summary_path)
        .map_err(|error| format!("could not read {}: {error}", summary_path.display()))?;
    let summary: serde_json::Value = serde_json::from_str(&summary_text)
        .map_err(|error| format!("could not parse {}: {error}", summary_path.display()))?;
    validate_pf00_complete_hardware_summary(&summary)?;
    for relative in PF00_REQUIRED_HARDWARE_ARTIFACTS {
        let expected = summary["artifact_sha256"][relative]
            .as_str()
            .expect("validated hardware hash exists");
        let actual = sha256_file_hex(&proof_root.join(relative))?;
        if actual != expected {
            return Err(format!(
                "PF00 hardware artifact {relative} hash mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    Ok(serde_json::json!({
        "status": "passed",
        "hardware_evidence": summary["hardware_evidence"].clone(),
        "release_evidence": summary["release_evidence"].clone(),
        "release_provenance": summary["release_provenance"].clone(),
        "summary_path": PF00_HARDWARE_SUMMARY_PATH,
        "summary_sha256": sha256_file_hex(&summary_path)?,
        "coverage": summary["coverage"].clone(),
        "adapters": summary["adapters"].clone(),
        "artifact_sha256": summary["artifact_sha256"].clone(),
    }))
}

#[test]
fn m9_pf00_representative_performance_artifact() {
    fs::create_dir_all(pf00_dir()).expect("PF00 artifact directory");
    let enabled = std::env::var_os("SCENA_RUN_PF00_BENCHMARK").is_some();
    let artifact = if enabled {
        let commit = current_commit_label();
        let mut pick = benchmark_profiled_pick_workload(100_000, BENCHMARK_SAMPLE_COUNT);
        pick["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut pick, &commit);
        write_json(
            &pf00_dir().join("pick-100k-triangle-deformed-undeformed.json"),
            &pick,
        );
        assert_eq!(pick["measurement_evidence"], true);
        let mut animation =
            benchmark_profiled_animation_workload(64, 256, 16, BENCHMARK_SAMPLE_COUNT);
        animation["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut animation, &commit);
        write_json(
            &pf00_dir().join("animation-many-channels-keyframes-weights.json"),
            &animation,
        );
        assert_eq!(animation["measurement_evidence"], true);
        let mut tangents = benchmark_profiled_tangent_workload(33_334, BENCHMARK_SAMPLE_COUNT);
        tangents["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut tangents, &commit);
        write_json(
            &pf00_dir().join("tangent-generation-static-deformed.json"),
            &tangents,
        );
        assert_eq!(tangents["measurement_evidence"], true);
        let mut shadows = benchmark_profiled_shadow_workload(&[1, 8, 32], BENCHMARK_SAMPLE_COUNT);
        shadows["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut shadows, &commit);
        write_json(
            &pf00_dir().join("shadow-scaling-directional-area.json"),
            &shadows,
        );
        assert_eq!(shadows["measurement_evidence"], true);
        let mut cpu_texture_bake =
            benchmark_profiled_cpu_texture_bake_workload(BENCHMARK_SAMPLE_COUNT);
        cpu_texture_bake["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut cpu_texture_bake, &commit);
        write_json(
            &pf00_dir().join("cpu-texture-bake-qualifying-nonqualifying.json"),
            &cpu_texture_bake,
        );
        assert_eq!(cpu_texture_bake["measurement_evidence"], true);
        let mut transform = benchmark_profiled_one_node_transform_workload(BENCHMARK_SAMPLE_COUNT);
        transform["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut transform, &commit);
        write_json(
            &pf00_dir().join("one-node-transform-prepare-render.json"),
            &transform,
        );
        let mut environment = benchmark_profiled_environment_bake_workload(BENCHMARK_SAMPLE_COUNT);
        environment["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut environment, &commit);
        write_json(
            &pf00_dir().join("environment-bake-cold-sidecar-hit.json"),
            &environment,
        );
        let mut native_capture = benchmark_profiled_native_capture_workload(BENCHMARK_SAMPLE_COUNT);
        native_capture["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut native_capture, &commit);
        write_json(
            &pf00_dir().join("native-present-capture-sync-async.json"),
            &native_capture,
        );
        let mut output_settings =
            benchmark_profiled_gpu_output_settings_workload(BENCHMARK_SAMPLE_COUNT);
        output_settings["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut output_settings, &commit);
        write_json(
            &pf00_dir().join("gpu-first-render-output-settings.json"),
            &output_settings,
        );
        let mut draw_uniforms =
            benchmark_profiled_draw_uniform_workload(512, BENCHMARK_SAMPLE_COUNT);
        draw_uniforms["schema"] = serde_json::json!("scena.performance_workload.v1");
        classify_pf00_workload_provenance(&mut draw_uniforms, &commit);
        write_json(
            &pf00_dir().join("draw-uniform-indexing-many-unique-transforms.json"),
            &draw_uniforms,
        );
        let workloads = [
            &pick,
            &animation,
            &tangents,
            &shadows,
            &cpu_texture_bake,
            &transform,
            &environment,
            &native_capture,
            &output_settings,
            &draw_uniforms,
        ];
        let measured_workloads = workloads
            .iter()
            .filter(|workload| workload["measurement_evidence"] == true)
            .filter_map(|workload| workload["id"].as_str())
            .collect::<Vec<_>>();
        let partial_or_required_workloads = workloads
            .iter()
            .filter(|workload| workload["measurement_evidence"] != true)
            .filter_map(|workload| workload["id"].as_str())
            .collect::<Vec<_>>();
        let hardware_proof = load_pf00_complete_hardware_proof();
        let measurements_complete = measured_workloads.len() == workloads.len();
        let hardware_complete = hardware_proof.is_ok();
        let release_evidence =
            pf00_release_evidence_ready(measurements_complete, hardware_complete, true, &commit);
        let hardware_proof = hardware_proof.unwrap_or_else(|error| {
            serde_json::json!({
                "status": "required",
                "release_evidence": false,
                "reason": error,
            })
        });
        let artifact = serde_json::json!({
            "schema": "scena.performance_evidence.v1",
            "status": if measurements_complete && hardware_complete { "measured" } else { "partial" },
            "measurement_evidence": measurements_complete,
            "hardware_evidence": hardware_complete,
            "release_evidence": release_evidence,
            "release_provenance": if release_evidence {
                serde_json::json!({"status": "exact-commit", "commit_sha": commit})
            } else {
                serde_json::json!({
                    "status": "unavailable",
                    "reason": "the measurement and hardware artifacts are not bound to one exact source commit",
                })
            },
            "reason": if measurements_complete && hardware_complete {
                "all ten PF00 workloads have optimized 100-sample distributions and measured work counters; the separate complete real-GPU summary binds native, WebGPU, and WebGL2 behavior without asserting release provenance"
            } else {
                "one or more PF00 distributions or required real-GPU proof artifacts are incomplete"
            },
            "measured_workloads": measured_workloads,
            "partial_or_required_workloads": partial_or_required_workloads,
            "required_workload_count": 10,
            "hardware_proof": hardware_proof,
            "provenance": performance_environment_metadata("pf00-headless-cpu"),
        });
        assert_eq!(
            artifact["measurement_evidence"], true,
            "dedicated PF00 lane must fail closed unless all ten distributions and the complete hardware proof are present: {artifact:#}"
        );
        assert_eq!(artifact["hardware_evidence"], true);
        artifact
    } else {
        serde_json::json!({
            "schema": "scena.performance_evidence.v1",
            "status": "required",
            "release_evidence": false,
            "reason": "SCENA_RUN_PF00_BENCHMARK is not set in the normal test lane",
            "run_hint": "Set SCENA_RUN_PF00_BENCHMARK=1 and use --profile perf-test to collect the dedicated representative workload distributions.",
            "required_workload_count": 10,
        })
    };
    write_json(&pf00_dir().join("performance-evidence.json"), &artifact);
}

fn reaggregate_pf00_existing_measurements() -> Result<serde_json::Value, String> {
    let mut workload_classifications = Vec::with_capacity(PF00_WORKLOAD_ARTIFACTS.len());
    let mut raw_artifact_sha256 = serde_json::Map::new();
    for (expected_id, file_name) in PF00_WORKLOAD_ARTIFACTS {
        let path = pf00_dir().join(file_name);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let workload: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
        let mut classification =
            validate_pf00_existing_measurement_artifact(&workload, expected_id)?;
        let relative = format!("target/gate-artifacts/pf00/{file_name}");
        let hash = sha256_file_hex(&path)?;
        classification["artifact"] = serde_json::json!(relative.clone());
        classification["artifact_sha256"] = serde_json::json!(hash.clone());
        classification["source_release_claim"] = workload["release_evidence"].clone();
        raw_artifact_sha256.insert(relative, serde_json::json!(hash));
        workload_classifications.push(classification);
    }
    let hardware_proof = load_pf00_complete_hardware_proof()?;
    let measurements_complete = workload_classifications.len() == PF00_WORKLOAD_ARTIFACTS.len();
    let hardware_complete = hardware_proof["hardware_evidence"] == true;
    let release_evidence = workload_classifications
        .iter()
        .all(|classification| classification["release_evidence"] == true)
        && hardware_proof["release_evidence"] == true;
    let measured_workloads = workload_classifications
        .iter()
        .filter_map(|classification| classification["id"].as_str())
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "schema": "scena.performance_evidence.v1",
        "status": if measurements_complete && hardware_complete { "measured" } else { "partial" },
        "measurement_evidence": measurements_complete,
        "hardware_evidence": hardware_complete,
        "release_evidence": release_evidence,
        "release_provenance": if release_evidence {
            serde_json::json!({"status": "exact-commit"})
        } else {
            serde_json::json!({
                "status": "unavailable",
                "reason": "the preserved measurements and hardware proof are not bound to one exact source commit; local-checkout release claims are rejected rather than rewritten",
            })
        },
        "classification_policy": {
            "source_artifacts_are_immutable": true,
            "local_checkout_is_measurement_only": true,
            "source_commit_and_timestamp_are_not_rewritten": true,
            "minimum_sample_count": BENCHMARK_SAMPLE_COUNT,
        },
        "measured_workloads": measured_workloads,
        "partial_or_required_workloads": [],
        "required_workload_count": PF00_WORKLOAD_ARTIFACTS.len(),
        "workload_classifications": workload_classifications,
        "raw_artifact_sha256": raw_artifact_sha256,
        "hardware_proof": hardware_proof,
        "provenance": performance_environment_metadata("pf00-measurement-reaggregation"),
    }))
}

#[test]
fn m9_pf00_reaggregate_existing_measurements_without_rerunning_benchmarks() {
    if std::env::var_os("SCENA_REAGGREGATE_PF00").is_none() {
        return;
    }
    let artifact = reaggregate_pf00_existing_measurements()
        .expect("preserved PF00 measurement artifacts validate");
    assert_eq!(artifact["status"], "measured");
    assert_eq!(artifact["measurement_evidence"], true);
    assert_eq!(artifact["hardware_evidence"], true);
    assert_eq!(artifact["release_evidence"], false);
    write_json(&pf00_dir().join("performance-evidence.json"), &artifact);
}

#[test]
fn pf00_complete_hardware_summary_requires_every_real_gpu_lane() {
    let complete = serde_json::json!({
        "schema": "scena.windows_complete_hardware_proof.v1",
        "status": "passed",
        "hardware_evidence": true,
        "release_evidence": false,
        "coverage": {
            "browser_backends": ["webgl2", "webgpu"],
            "native_surface": true,
            "native_semantic_aov": true
        },
        "artifact_sha256": {
            "target/gate-artifacts/pf01-output-toggle/browser/browser-output-toggle.json": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "target/gate-artifacts/fr06-semantic-aov/browser/semantic-aov-browser-proof.json": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "target/gate-artifacts/fr06-semantic-aov/native/native-semantic-aov-proof.json": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
        }
    });
    assert!(validate_pf00_complete_hardware_summary(&complete).is_ok());

    let mut missing_webgpu = complete.clone();
    missing_webgpu["coverage"]["browser_backends"] = serde_json::json!(["webgl2"]);
    assert!(
        validate_pf00_complete_hardware_summary(&missing_webgpu)
            .expect_err("missing WebGPU must fail")
            .contains("WebGPU")
    );

    let mut partial = complete;
    partial["hardware_evidence"] = serde_json::json!(false);
    assert!(validate_pf00_complete_hardware_summary(&partial).is_err());
}

#[test]
fn pf00_release_claim_requires_exact_source_provenance() {
    let commit = "0123456789abcdef0123456789abcdef01234567";
    assert!(pf00_release_evidence_ready(true, true, true, commit));
    assert!(!pf00_release_evidence_ready(
        true,
        true,
        true,
        "local-checkout"
    ));
    assert!(!pf00_release_evidence_ready(true, true, false, commit));
    assert!(!pf00_release_evidence_ready(true, false, true, commit));
    assert!(!pf00_release_evidence_ready(false, true, true, commit));
    assert!(!pf00_release_evidence_ready(
        true,
        true,
        true,
        "0000000000000000000000000000000000000000"
    ));
    assert!(!pf00_release_evidence_ready(true, true, true, "01234567"));
}

#[test]
fn pf00_workload_classification_separates_measurement_from_release_provenance() {
    let mut local = serde_json::json!({
        "status": "measured",
        "release_evidence": true,
    });
    classify_pf00_workload_provenance(&mut local, "local-checkout");
    assert_eq!(local["measurement_evidence"], true);
    assert_eq!(local["release_evidence"], false);
    assert_eq!(local["release_provenance"]["status"], "unavailable");

    let mut exact = serde_json::json!({
        "status": "measured",
        "release_evidence": true,
    });
    classify_pf00_workload_provenance(&mut exact, "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(exact["measurement_evidence"], true);
    assert_eq!(exact["release_evidence"], true);
    assert_eq!(exact["release_provenance"]["status"], "exact-commit");

    let mut incomplete = serde_json::json!({
        "status": "contract-scale-measured",
        "release_evidence": false,
    });
    classify_pf00_workload_provenance(&mut incomplete, "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(incomplete["measurement_evidence"], false);
    assert_eq!(incomplete["release_evidence"], false);
}

#[test]
fn pf00_existing_local_artifact_is_measurement_evidence_not_release_evidence() {
    let complete = serde_json::json!({
        "schema": "scena.performance_workload.v1",
        "id": "pick-100k-triangle-deformed-undeformed",
        "status": "measured",
        "release_evidence": true,
        "commit_sha": "local-checkout",
        "timestamp_unix_seconds": 1_784_410_000_u64,
        "source_checksums": [
            {"path": "Cargo.lock", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
            {"path": "tests/m9_platform_release.rs", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
        ],
        "distributions": {
            "undeformed_pick_ms": {"sample_count": 100, "p50_ms": 1.0, "p95_ms": 2.0},
            "deformed_pick_ms": {"sample_count": 100, "p50_ms": 2.0, "p95_ms": 3.0}
        }
    });
    let classification = validate_pf00_existing_measurement_artifact(
        &complete,
        "pick-100k-triangle-deformed-undeformed",
    )
    .expect("complete local workload remains valid measurement evidence");
    assert_eq!(classification["measurement_evidence"], true);
    assert_eq!(classification["release_evidence"], false);
    assert_eq!(
        classification["release_rejection_code"],
        "LOCAL_CHECKOUT_COMMIT"
    );

    let mut too_few_samples = complete.clone();
    too_few_samples["distributions"]["deformed_pick_ms"]["sample_count"] = serde_json::json!(99);
    assert!(
        validate_pf00_existing_measurement_artifact(
            &too_few_samples,
            "pick-100k-triangle-deformed-undeformed",
        )
        .is_err()
    );

    let mut missing_provenance = complete;
    missing_provenance["source_checksums"] = serde_json::json!([]);
    assert!(
        validate_pf00_existing_measurement_artifact(
            &missing_provenance,
            "pick-100k-triangle-deformed-undeformed",
        )
        .is_err()
    );
}

#[test]
fn m9_pf03_release_scale_prepared_storage_artifact() {
    if std::env::var_os("SCENA_RUN_PF03_STORAGE_BENCHMARK").is_none() {
        return;
    }
    const TRIANGLE_COUNT: usize = 33_334;
    const SAMPLE_COUNT: usize = 10;
    let assets = Assets::new();
    let geometry = assets.create_geometry(tangent_benchmark_geometry(TRIANGLE_COUNT, false));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = tangent_benchmark_scene(geometry, material, false);
    let mut renderer = Renderer::headless(1, 1).expect("PF03 renderer creates");
    let samples = measure_profiled_prepares(&mut renderer, &mut scene, &assets, SAMPLE_COUNT);
    let metrics = samples.metrics;
    assert_eq!(metrics.prepared_triangle_count, TRIANGLE_COUNT as u64);
    assert_eq!(metrics.prepared_model_vertex_buffer_count, 1);
    assert_eq!(metrics.prepared_unique_draw_transforms, 1);
    assert_eq!(metrics.prepared_list_copy_bytes, 0);
    assert!(metrics.prepared_model_vertex_bytes > 0);
    assert!(
        metrics.prepared_triangle_reference_bytes
            < TRIANGLE_COUNT as u64 * 4 * 16 * size_of::<f32>() as u64
    );
    let artifact = serde_json::json!({
        "schema": "scena.pf03.prepared_storage.v1",
        "status": "measured",
        "release_evidence": true,
        "fixture": {
            "triangle_count": TRIANGLE_COUNT,
            "vertex_count": TRIANGLE_COUNT * 3,
            "node_count": 1,
            "sample_count": SAMPLE_COUNT,
        },
        "distributions": {
            "prepare_ms": duration_distribution_json(&samples.duration_ms),
        },
        "allocations": allocation_measurement_json(
            &samples.allocation_counts,
            &samples.allocation_bytes,
        ),
        "counters": prepare_work_metrics_json(metrics),
        "provenance": performance_environment_metadata("cpu-pf03-prepared-storage"),
    });
    fs::create_dir_all(pf03_dir()).expect("PF03 artifact directory creates");
    write_json(
        &pf03_dir().join("prepared-storage-100k-triangles.json"),
        &artifact,
    );
}

#[test]
fn m9_benchmark_baseline_comparison_fails_significant_regressions() {
    let mut rows = vec![serde_json::json!({
        "scene": "static-viewer",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 12.0,
        "max_allocations_per_frame": 2,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "p95_frame_ms": 10.0,
                "allowed_regression_percent": 5.0,
                "max_allocations_per_frame": 4
            }
        ]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "test-lane");

    assert_eq!(summary["status"], "failed");
    assert_eq!(rows[0]["baseline_comparison"]["status"], "failed");
    assert_eq!(
        rows[0]["baseline_comparison"]["frame_time_status"],
        "failed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["allocation_status"],
        "passed"
    );
    assert_eq!(rows[0]["baseline_comparison"]["regression_percent"], 20.0);
}

#[test]
fn m9_benchmark_baseline_comparison_fails_allocation_regressions() {
    let mut rows = vec![serde_json::json!({
        "scene": "static-viewer",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 10.0,
        "max_allocations_per_frame": 12,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "p95_frame_ms": 10.0,
                "allowed_regression_percent": 5.0,
                "max_allocations_per_frame": 4
            }
        ]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "test-lane");

    assert_eq!(summary["status"], "failed");
    assert_eq!(rows[0]["baseline_comparison"]["status"], "failed");
    assert_eq!(
        rows[0]["baseline_comparison"]["max_allocations_per_frame"],
        12
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["allowed_max_allocations_per_frame"],
        4
    );
}

#[test]
fn m9_benchmark_allocation_gate_uses_p95_and_reports_isolated_maximums() {
    let mut rows = vec![serde_json::json!({
        "scene": "static-viewer",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 10.0,
        "p95_allocations_per_frame": 16,
        "max_allocations_per_frame": 17,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [{
            "scene": "static-viewer",
            "backend": "Headless",
            "p95_frame_ms": 10.0,
            "allowed_regression_percent": 5.0,
            "max_allocations_per_frame": 16
        }]
    });

    let summary = apply_benchmark_baselines(&mut rows, &baseline, "test-lane");

    assert_eq!(summary["status"], "passed");
    assert_eq!(summary["metrics"][2], "p95_allocations_per_frame");
    assert_eq!(
        rows[0]["baseline_comparison"]["allocation_status"],
        "passed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["p95_allocations_per_frame"],
        16
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["max_allocations_per_frame"],
        17
    );
}

#[test]
fn m9_hosted_timing_policy_reports_wall_clock_regressions_without_hiding_them() {
    let mut rows = vec![serde_json::json!({
        "scene": "static-viewer",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 12.0,
        "p95_prepare_ms": 24.0,
        "max_allocations_per_frame": 2,
        "max_allocated_bytes_per_frame": 64,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [{
            "scene": "static-viewer",
            "backend": "Headless",
            "p95_frame_ms": 10.0,
            "p95_prepare_ms": 20.0,
            "allowed_regression_percent": 5.0,
            "max_allocations_per_frame": 4,
            "max_allocated_bytes_per_frame": 128
        }]
    });

    let summary = apply_benchmark_baselines_with_policy(
        &mut rows,
        &baseline,
        "github-hosted-test",
        M9TimingPolicy::ReportOnlyHosted,
    );

    assert_eq!(summary["status"], "passed");
    assert_eq!(summary["timing_policy"], "report-only-hosted");
    assert_eq!(summary["timing_enforced"], false);
    assert_eq!(summary["reported_timing_regressions"], 1);
    assert_eq!(rows[0]["baseline_comparison"]["status"], "passed");
    assert_eq!(
        rows[0]["baseline_comparison"]["frame_time_status"],
        "failed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["prepare_time_status"],
        "failed"
    );
    assert_eq!(
        rows[0]["baseline_comparison"]["timing_gate_status"],
        "reported-only"
    );
}

#[test]
fn m9_hosted_timing_policy_keeps_allocation_regressions_blocking() {
    let mut rows = vec![serde_json::json!({
        "scene": "static-viewer",
        "backend": "Headless",
        "sample_count": 100,
        "p95_frame_ms": 10.0,
        "max_allocations_per_frame": 12,
    })];
    let baseline = serde_json::json!({
        "minimum_sample_count": 100,
        "rows": [{
            "scene": "static-viewer",
            "backend": "Headless",
            "p95_frame_ms": 10.0,
            "allowed_regression_percent": 5.0,
            "max_allocations_per_frame": 4
        }]
    });

    let summary = apply_benchmark_baselines_with_policy(
        &mut rows,
        &baseline,
        "github-hosted-test",
        M9TimingPolicy::ReportOnlyHosted,
    );

    assert_eq!(summary["status"], "failed");
    assert_eq!(
        rows[0]["baseline_comparison"]["allocation_status"],
        "failed"
    );
}

#[test]
fn m9_timing_policy_rejects_unknown_values() {
    let error = M9TimingPolicy::parse(Some("make-it-green")).expect_err("unknown policy fails");
    assert!(error.contains("unsupported SCENA_M9_TIMING_POLICY"));
}

fn benchmark_profiled_pick_workload(
    triangle_count: usize,
    sample_count: usize,
) -> serde_json::Value {
    assert!(triangle_count > 0, "pick benchmark needs triangles");
    assert!(sample_count > 0, "pick benchmark needs samples");
    let assets = Assets::new();
    let static_geometry = assets.create_geometry(overlapping_pick_geometry(triangle_count, false));
    let deformed_geometry = assets.create_geometry(overlapping_pick_geometry(triangle_count, true));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let (static_scene, static_camera) = profiled_pick_scene(static_geometry, material, false);
    let (deformed_scene, deformed_camera) = profiled_pick_scene(deformed_geometry, material, true);
    let viewport = Viewport::new(200, 200, 1.0).expect("pick benchmark viewport validates");
    let cursor = CursorPosition::physical(100.0, 100.0);
    let static_samples = measure_profiled_picks(
        &static_scene,
        &assets,
        static_camera,
        cursor,
        viewport,
        sample_count,
    );
    let deformed_samples = measure_profiled_picks(
        &deformed_scene,
        &assets,
        deformed_camera,
        cursor,
        viewport,
        sample_count,
    );
    let release_scale = triangle_count == 100_000 && sample_count >= BENCHMARK_SAMPLE_COUNT;

    serde_json::json!({
        "id": "pick-100k-triangle-deformed-undeformed",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "triangle_count": triangle_count,
            "release_triangle_count": 100_000,
            "vertex_count": triangle_count.saturating_mul(3),
            "geometry": "overlapping indexed triangles so every triangle reaches the ray/triangle test",
            "deformation": "one morph target materialized at weight 1.0",
        },
        "distributions": {
            "undeformed_pick_ms": duration_distribution_json(&static_samples.duration_ms),
            "deformed_pick_ms": duration_distribution_json(&deformed_samples.duration_ms),
        },
        "allocations": {
            "undeformed": allocation_measurement_json(
                &static_samples.allocation_counts,
                &static_samples.allocation_bytes,
            ),
            "deformed": allocation_measurement_json(
                &deformed_samples.allocation_counts,
                &deformed_samples.allocation_bytes,
            ),
        },
        "counters": {
            "undeformed": picking_metrics_json(static_samples.metrics),
            "deformed": picking_metrics_json(deformed_samples.metrics),
        },
        "provenance": performance_environment_metadata("headless-cpu-picking"),
    })
}

fn benchmark_profiled_animation_workload(
    channel_count: usize,
    keyframe_count: usize,
    weight_width: usize,
    sample_count: usize,
) -> serde_json::Value {
    assert!(channel_count >= 2 && channel_count.is_multiple_of(2));
    assert!(keyframe_count >= 2);
    assert!(weight_width > 0);
    assert!(sample_count > 0);

    let mut scene = Scene::new();
    let times = (0..keyframe_count)
        .map(|index| index as f32 / (keyframe_count - 1) as f32)
        .collect::<Vec<_>>();
    let weight_channel_count = channel_count / 2;
    let mut channels = Vec::with_capacity(channel_count);
    for channel_index in 0..channel_count {
        let node = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("animation benchmark node inserts");
        if channel_index < weight_channel_count {
            scene
                .set_morph_weights(node, vec![0.0; weight_width])
                .expect("animation benchmark morph weights initialize");
            let values = times
                .iter()
                .copied()
                .map(|time| {
                    (0..weight_width)
                        .map(|weight| (time + weight as f32 * 0.01).fract())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            channels.push(AnimationChannel::new(
                node,
                AnimationTarget::Weights,
                times.clone(),
                AnimationOutput::Weights(values),
                AnimationInterpolation::Linear,
            ));
        } else {
            let values = times
                .iter()
                .copied()
                .map(|time| Vec3::new(time, channel_index as f32, 0.0))
                .collect::<Vec<_>>();
            channels.push(AnimationChannel::new(
                node,
                AnimationTarget::Translation,
                times.clone(),
                AnimationOutput::Vec3(values),
                AnimationInterpolation::Linear,
            ));
        }
    }
    let clip = AnimationClip::authored(
        Some("pf00-animation-many-channels-keyframes-weights".to_owned()),
        channels,
        1.0,
    )
    .expect("animation benchmark clip validates");
    let mixer = scene
        .play_authored_animation(clip)
        .expect("animation benchmark mixer starts");
    scene
        .set_animation_loop_mode(mixer, AnimationLoopMode::Repeat)
        .expect("animation benchmark loop mode sets");
    scene
        .seek_animation(mixer, 0.6)
        .expect("animation benchmark seeks to a representative interior keyframe interval");

    let mut duration_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_metrics = None;
    for _ in 0..sample_count {
        start_allocation_counting();
        let start = Instant::now();
        let result = scene.update_animation_profiled(mixer, 0.0);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let metrics = result.expect("profiled animation benchmark update succeeds");
        if let Some(expected) = expected_metrics {
            assert_eq!(
                metrics, expected,
                "profiled animation counters must be deterministic"
            );
        } else {
            expected_metrics = Some(metrics);
        }
        duration_ms.push(elapsed_ms);
        allocation_counts.push(allocation_count());
        allocated_bytes.push(allocation_bytes());
    }
    let metrics = expected_metrics.expect("profiled animation records metrics");
    let release_scale = channel_count == 64
        && keyframe_count == 256
        && weight_width == 16
        && sample_count >= BENCHMARK_SAMPLE_COUNT;

    serde_json::json!({
        "id": "animation-many-channels-keyframes-weights",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "channel_count": channel_count,
            "translation_channel_count": channel_count - weight_channel_count,
            "weight_channel_count": weight_channel_count,
            "keyframe_count": keyframe_count,
            "weight_width": weight_width,
            "release_channel_count": 64,
            "release_keyframe_count": 256,
            "release_weight_width": 16,
            "sample_time_seconds": 0.6,
        },
        "distributions": {
            "advance_ms": duration_distribution_json(&duration_ms),
        },
        "allocations": allocation_measurement_json(&allocation_counts, &allocated_bytes),
        "counters": {
            "channels_scanned": metrics.channels_scanned,
            "keyframe_intervals_tested": metrics.keyframe_intervals_tested,
            "weight_values_written": metrics.weight_values_written,
            "weight_bytes_written": metrics.weight_bytes_written,
            "clip_clone_bytes": metrics.clip_clone_bytes,
            "bytes_cloned_or_copied": metrics
                .clip_clone_bytes
                .saturating_add(metrics.weight_bytes_written),
        },
        "provenance": performance_environment_metadata("cpu-animation"),
    })
}

fn benchmark_profiled_tangent_workload(
    triangle_count: usize,
    sample_count: usize,
) -> serde_json::Value {
    assert!(triangle_count > 0);
    assert!(sample_count > 0);
    let assets = Assets::new();
    let static_geometry = assets.create_geometry(tangent_benchmark_geometry(triangle_count, false));
    let deformed_geometry =
        assets.create_geometry(tangent_benchmark_geometry(triangle_count, true));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut static_scene = tangent_benchmark_scene(static_geometry, material, false);
    let mut deformed_scene = tangent_benchmark_scene(deformed_geometry, material, true);
    let mut static_renderer = Renderer::headless(1, 1).expect("static tangent renderer creates");
    let mut deformed_renderer =
        Renderer::headless(1, 1).expect("deformed tangent renderer creates");
    let static_cold = static_renderer
        .prepare_with_assets_profiled(&mut static_scene, &assets)
        .expect("cold static tangent prepare succeeds");
    let static_samples = measure_profiled_prepares(
        &mut static_renderer,
        &mut static_scene,
        &assets,
        sample_count,
    );
    let deformed_samples = measure_profiled_prepares(
        &mut deformed_renderer,
        &mut deformed_scene,
        &assets,
        sample_count,
    );
    let release_scale = triangle_count == 33_334 && sample_count >= BENCHMARK_SAMPLE_COUNT;

    serde_json::json!({
        "id": "tangent-generation-static-deformed",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "triangle_count": triangle_count,
            "vertex_count": triangle_count.saturating_mul(3),
            "release_triangle_count": 33_334,
            "release_vertex_count": 100_002,
            "tangent_source": "generated MikkTSpace tangents from positions, normals, and UV0",
            "deformation": "one POSITION morph target materialized at weight 1.0",
        },
        "distributions": {
            "static_tangent_ms": duration_distribution_json(&static_samples.duration_ms),
            "deformed_tangent_ms": duration_distribution_json(&deformed_samples.duration_ms),
        },
        "allocations": {
            "static": allocation_measurement_json(
                &static_samples.allocation_counts,
                &static_samples.allocation_bytes,
            ),
            "deformed": allocation_measurement_json(
                &deformed_samples.allocation_counts,
                &deformed_samples.allocation_bytes,
            ),
        },
        "counters": {
            "static_cold": prepare_work_metrics_json(static_cold),
            "static": prepare_work_metrics_json(static_samples.metrics),
            "deformed": prepare_work_metrics_json(deformed_samples.metrics),
        },
        "provenance": performance_environment_metadata("cpu-assets-tangent-generation"),
    })
}

fn tangent_benchmark_geometry(triangle_count: usize, deformed: bool) -> GeometryDesc {
    let vertex_count = triangle_count
        .checked_mul(3)
        .expect("tangent benchmark vertex count fits usize");
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(vertex_count);
    let mut tex_coords0 = Vec::with_capacity(vertex_count);
    for triangle in 0..triangle_count {
        let base =
            u32::try_from(triangle.saturating_mul(3)).expect("tangent benchmark index fits u32");
        vertices.extend([
            GeometryVertex {
                position: Vec3::new(-0.4, -0.4, 0.0),
                normal: Vec3::Z,
            },
            GeometryVertex {
                position: Vec3::new(0.4, -0.4, 0.0),
                normal: Vec3::Z,
            },
            GeometryVertex {
                position: Vec3::new(0.0, 0.4, 0.0),
                normal: Vec3::Z,
            },
        ]);
        indices.extend([base, base + 1, base + 2]);
        tex_coords0.extend([[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]);
    }
    let geometry = GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        vertices,
        indices,
        vec![Color::WHITE; vertex_count],
        tex_coords0,
    )
    .expect("tangent benchmark geometry validates");
    if deformed {
        geometry
            .with_morph_targets(vec![GeometryMorphTarget::new(vec![
                Vec3::new(
                    0.01, 0.0, 0.0
                );
                vertex_count
            ])])
            .expect("tangent benchmark morph target validates")
    } else {
        geometry
    }
}

fn tangent_benchmark_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    deformed: bool,
) -> Scene {
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("tangent benchmark mesh inserts");
    if deformed {
        scene
            .set_morph_weights(mesh, [1.0])
            .expect("tangent benchmark morph weight sets");
    }
    scene
}

struct ProfiledPrepareSamples {
    duration_ms: Vec<f64>,
    allocation_counts: Vec<u64>,
    allocation_bytes: Vec<u64>,
    metrics: scena::PrepareWorkMetrics,
}

fn measure_profiled_prepares<F>(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets<F>,
    sample_count: usize,
) -> ProfiledPrepareSamples {
    renderer
        .prepare_with_assets_profiled(scene, assets)
        .expect("profiled prepare warmup succeeds");
    let mut duration_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_metrics = None;
    for _ in 0..sample_count {
        start_allocation_counting();
        let start = Instant::now();
        let result = renderer.prepare_with_assets_profiled(scene, assets);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let metrics = result.expect("profiled tangent prepare succeeds");
        if let Some(expected) = expected_metrics {
            assert_eq!(
                metrics, expected,
                "profiled prepare counters must be deterministic"
            );
        } else {
            expected_metrics = Some(metrics);
        }
        duration_ms.push(elapsed_ms);
        allocation_counts.push(allocation_count());
        allocated_bytes.push(allocation_bytes());
    }
    ProfiledPrepareSamples {
        duration_ms,
        allocation_counts,
        allocation_bytes: allocated_bytes,
        metrics: expected_metrics.expect("profiled prepare records metrics"),
    }
}

/// Same sampling contract as `measure_profiled_prepares`, but each sample runs
/// on a renderer that has never seen this scene.
///
/// The renderer caches shadow visibility across prepares whose light and
/// occluder signatures are unchanged, which is what makes the camera-behavior
/// loop affordable. A warm prepare therefore casts **no** shadow rays, so
/// sampling a shadow workload on one reused renderer measures cache hits and
/// reports `shadow_rays: 0` - numbers that would be published as shadow
/// intersection cost while describing the opposite. Scaling is a cold-path
/// property, so it is measured on the cold path.
fn measure_cold_profiled_prepares<F>(
    scene: &mut Scene,
    assets: &Assets<F>,
    sample_count: usize,
) -> ProfiledPrepareSamples
where
    F: scena::AssetFetcher,
{
    // Warm the caches that live in `Assets` rather than in the renderer -
    // generated tangents, most of all - on a throwaway renderer. They are shared
    // by every sample, so leaving them cold makes only the first sample pay for
    // them and the counters differ between otherwise identical samples. Only the
    // renderer-owned shadow visibility cache is meant to be cold here.
    {
        let mut warmup = Renderer::headless(1, 1).expect("cold shadow warmup renderer creates");
        warmup
            .prepare_with_assets_profiled(scene, assets)
            .expect("cold shadow warmup prepare succeeds");
    }
    let mut duration_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_metrics = None;
    for _ in 0..sample_count {
        let mut renderer = Renderer::headless(1, 1).expect("cold shadow renderer creates");
        start_allocation_counting();
        let start = Instant::now();
        let result = renderer.prepare_with_assets_profiled(scene, assets);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let metrics = result.expect("cold profiled prepare succeeds");
        if let Some(expected) = expected_metrics {
            assert_eq!(
                metrics, expected,
                "cold profiled prepare counters must be deterministic"
            );
        } else {
            expected_metrics = Some(metrics);
        }
        duration_ms.push(elapsed_ms);
        allocation_counts.push(allocation_count());
        allocated_bytes.push(allocation_bytes());
    }
    ProfiledPrepareSamples {
        duration_ms,
        allocation_counts,
        allocation_bytes: allocated_bytes,
        metrics: expected_metrics.expect("cold profiled prepare records metrics"),
    }
}

fn prepare_work_metrics_json(metrics: scena::PrepareWorkMetrics) -> serde_json::Value {
    serde_json::json!({
        "prepared_triangle_count": metrics.prepared_triangle_count,
        "prepared_model_vertex_buffer_count": metrics.prepared_model_vertex_buffer_count,
        "prepared_model_vertex_bytes": metrics.prepared_model_vertex_bytes,
        "prepared_unique_draw_transforms": metrics.prepared_unique_draw_transforms,
        "prepared_draw_transform_bytes": metrics.prepared_draw_transform_bytes,
        "prepared_triangle_reference_bytes": metrics.prepared_triangle_reference_bytes,
        "prepared_list_copy_bytes": metrics.prepared_list_copy_bytes,
        "asset_storage_lock_acquisitions": metrics.asset_storage_lock_acquisitions,
        "generated_tangent_calls": metrics.generated_tangent_calls,
        "generated_tangent_triangles": metrics.generated_tangent_triangles,
        "generated_tangent_vertices": metrics.generated_tangent_vertices,
        "generated_tangent_cache_hits": metrics.generated_tangent_cache_hits,
        "generated_tangent_cache_misses": metrics.generated_tangent_cache_misses,
        "tangent_input_transform_bytes": metrics.tangent_input_transform_bytes,
        "tangent_output_bytes": metrics.tangent_output_bytes,
        "deformed_vertex_bytes_materialized": metrics.deformed_vertex_bytes_materialized,
        "shadow_rays": metrics.shadow_rays,
        "shadow_visibility_cache_hits": metrics.shadow_visibility_cache_hits,
        "shadow_visibility_cache_misses": metrics.shadow_visibility_cache_misses,
        "bvh_node_bounds_tests": metrics.bvh_node_bounds_tests,
        "ray_triangle_intersection_tests": metrics.ray_triangle_intersection_tests,
        "area_light_samples": metrics.area_light_samples,
        "cpu_bake_subdivided_triangles": metrics.cpu_bake_subdivided_triangles,
        "cpu_bake_shaded_vertices": metrics.cpu_bake_shaded_vertices,
        "texture_samples": metrics.texture_samples,
        "cpu_bake_corner_bytes_copied": metrics.cpu_bake_corner_bytes_copied,
        "gpu_buffer_creations": metrics.gpu_buffer_creations,
        "gpu_texture_creations": metrics.gpu_texture_creations,
        "gpu_pipeline_creations": metrics.gpu_pipeline_creations,
        "gpu_bind_group_creations": metrics.gpu_bind_group_creations,
        "gpu_shader_module_creations": metrics.gpu_shader_module_creations,
        "gpu_triangle_shader_cache_hits": metrics.gpu_triangle_shader_cache_hits,
        "gpu_triangle_shader_cache_misses": metrics.gpu_triangle_shader_cache_misses,
        "gpu_nonblocking_polls": metrics.gpu_nonblocking_polls,
        "gpu_blocking_polls": metrics.gpu_blocking_polls,
        "draw_uniform_unique_values": metrics.draw_uniform_unique_values,
        "draw_uniform_lookup_probes": metrics.draw_uniform_lookup_probes,
        "draw_uniform_bytes_copied": metrics.draw_uniform_bytes_copied,
        "bytes_cloned_or_copied": metrics.bytes_cloned_or_copied(),
    })
}

fn benchmark_profiled_shadow_workload(
    triangle_counts: &[usize],
    sample_count: usize,
) -> serde_json::Value {
    assert!(!triangle_counts.is_empty());
    assert!(triangle_counts.iter().all(|count| *count > 0));
    assert!(sample_count > 0);
    let mut scales = Vec::with_capacity(triangle_counts.len());
    for &triangle_count in triangle_counts {
        let assets = Assets::new();
        let geometry = assets.create_geometry(tangent_benchmark_geometry(triangle_count, false));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut directional_scene =
            shadow_benchmark_scene(geometry, material, ShadowBenchmarkLight::Directional);
        let mut area_scene = shadow_benchmark_scene(geometry, material, ShadowBenchmarkLight::Area);
        let directional =
            measure_cold_profiled_prepares(&mut directional_scene, &assets, sample_count);
        let area = measure_cold_profiled_prepares(&mut area_scene, &assets, sample_count);
        scales.push(serde_json::json!({
            "triangle_count": triangle_count,
            "vertex_count": triangle_count.saturating_mul(3),
            "directional": {
                "prepare_ms": duration_distribution_json(&directional.duration_ms),
                "allocations": allocation_measurement_json(
                    &directional.allocation_counts,
                    &directional.allocation_bytes,
                ),
                "counters": prepare_work_metrics_json(directional.metrics),
            },
            "area": {
                "prepare_ms": duration_distribution_json(&area.duration_ms),
                "allocations": allocation_measurement_json(
                    &area.allocation_counts,
                    &area.allocation_bytes,
                ),
                "counters": prepare_work_metrics_json(area.metrics),
            },
        }));
    }
    let release_scale = triangle_counts == [1, 8, 32] && sample_count >= BENCHMARK_SAMPLE_COUNT;
    serde_json::json!({
        "id": "shadow-scaling-directional-area",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "release_triangle_counts": [1, 8, 32],
            "area_samples_per_shaded_vertex": 16,
            "geometry": "indexed UV-mapped triangles that also form the deterministic occluder set",
        },
        "scales": scales,
        "provenance": performance_environment_metadata("headless-cpu-shadow-scaling"),
    })
}

#[derive(Clone, Copy)]
enum ShadowBenchmarkLight {
    Directional,
    Area,
}

fn shadow_benchmark_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    light: ShadowBenchmarkLight,
) -> Scene {
    let mut scene = tangent_benchmark_scene(geometry, material, false);
    match light {
        ShadowBenchmarkLight::Directional => {
            scene
                .directional_light(DirectionalLight::default().with_shadows(true))
                .transform(Transform::default().rotate_x_deg(-35.0).rotate_y_deg(20.0))
                .add()
                .expect("directional benchmark light inserts");
        }
        ShadowBenchmarkLight::Area => {
            scene
                .area_light(AreaLight::softbox())
                .transform(Transform::at(Vec3::new(0.0, 1.5, 1.0)))
                .add()
                .expect("area benchmark light inserts");
        }
    }
    scene
}

fn benchmark_profiled_cpu_texture_bake_workload(sample_count: usize) -> serde_json::Value {
    assert!(sample_count > 0);
    let assets = Assets::new();
    let geometry = assets.create_geometry(tangent_benchmark_geometry(1, false));
    let albedo = pollster::block_on(assets.load_texture(
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        TextureColorSpace::Srgb,
    ))
    .expect("CPU texture-bake inline PNG loads");
    let nonqualifying_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut nonqualifying_scene = texture_bake_benchmark_scene(geometry, nonqualifying_material);
    let mut nonqualifying_renderer =
        Renderer::headless(256, 256).expect("nonqualifying CPU texture-bake renderer creates");
    let nonqualifying = measure_profiled_prepares(
        &mut nonqualifying_renderer,
        &mut nonqualifying_scene,
        &assets,
        sample_count,
    );
    let mut texture_roles = serde_json::Map::new();
    for (role, material) in cpu_texture_bake_role_materials(albedo) {
        let material = assets.create_material(material);
        let mut scene = texture_bake_benchmark_scene(geometry, material);
        let mut renderer =
            Renderer::headless(256, 256).expect("texture-role CPU bake renderer creates");
        let measured = measure_profiled_prepares(&mut renderer, &mut scene, &assets, sample_count);
        texture_roles.insert(
            role.to_string(),
            serde_json::json!({
                "bake_ms": duration_distribution_json(&measured.duration_ms),
                "allocations": allocation_measurement_json(
                    &measured.allocation_counts,
                    &measured.allocation_bytes,
                ),
                "counters": prepare_work_metrics_json(measured.metrics),
            }),
        );
    }
    let qualifying = texture_roles
        .get("base_color")
        .expect("base-color role is measured");
    let release_scale = sample_count >= BENCHMARK_SAMPLE_COUNT;

    serde_json::json!({
        "id": "cpu-texture-bake-qualifying-nonqualifying",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "source_triangle_count": 1,
            "adaptive_subdivision_hard_cap": 48,
            "nonqualifying_subdivisions": 1,
            "qualifying_texture": "decoded 1x1 sRGB RGBA8 inline PNG",
            "target_px": [256, 256],
            "texture_role_count": texture_roles.len(),
        },
        "distributions": {
            "qualifying_bake_ms": qualifying["bake_ms"].clone(),
            "nonqualifying_bake_ms": duration_distribution_json(&nonqualifying.duration_ms),
        },
        "allocations": {
            "qualifying": qualifying["allocations"].clone(),
            "nonqualifying": allocation_measurement_json(
                &nonqualifying.allocation_counts,
                &nonqualifying.allocation_bytes,
            ),
        },
        "counters": {
            "qualifying": qualifying["counters"].clone(),
            "nonqualifying": prepare_work_metrics_json(nonqualifying.metrics),
        },
        "texture_roles": texture_roles,
        "provenance": performance_environment_metadata("headless-cpu-texture-bake"),
    })
}

fn texture_bake_benchmark_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
) -> Scene {
    let mut scene = tangent_benchmark_scene(geometry, material, false);
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("texture-bake benchmark camera inserts");
    scene
        .set_active_camera(camera)
        .expect("texture-bake benchmark camera activates");
    scene
}

fn cpu_texture_bake_role_materials(
    texture: scena::TextureHandle,
) -> Vec<(&'static str, MaterialDesc)> {
    let pbr = || MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.2, 0.5);
    vec![
        ("base_color", pbr().with_base_color_texture(texture)),
        ("normal", pbr().with_normal_texture(texture)),
        (
            "metallic_roughness",
            pbr().with_metallic_roughness_texture(texture),
        ),
        ("occlusion", pbr().with_occlusion_texture(texture)),
        ("emissive", pbr().with_emissive_texture(texture)),
        ("clearcoat", pbr().with_clearcoat_texture(texture)),
        (
            "clearcoat_roughness",
            pbr().with_clearcoat_roughness_texture(texture),
        ),
        (
            "clearcoat_normal",
            pbr().with_clearcoat_normal_texture(texture),
        ),
        ("sheen_color", pbr().with_sheen_color_texture(texture)),
        (
            "sheen_roughness",
            pbr().with_sheen_roughness_texture(texture),
        ),
        ("anisotropy", pbr().with_anisotropy_texture(texture)),
        ("iridescence", pbr().with_iridescence_texture(texture)),
        (
            "iridescence_thickness",
            pbr().with_iridescence_thickness_texture(texture),
        ),
        (
            "transmission",
            pbr()
                .with_transmission_factor(1.0)
                .with_transmission_texture(texture),
        ),
        (
            "thickness",
            pbr()
                .with_transmission_factor(1.0)
                .with_thickness_factor(1.0)
                .with_thickness_texture(texture),
        ),
    ]
}

fn benchmark_profiled_one_node_transform_workload(sample_count: usize) -> serde_json::Value {
    assert!(sample_count > 0);
    let (mut cpu_scene, cpu_node, cpu_camera) = profiled_transform_scene();
    let mut cpu_renderer = Renderer::headless(32, 32).expect("CPU transform renderer creates");
    cpu_renderer
        .prepare(&mut cpu_scene)
        .expect("CPU transform warm prepare succeeds");
    cpu_renderer
        .render(&cpu_scene, cpu_camera)
        .expect("CPU transform warm render succeeds");
    let cpu = measure_profiled_transform_updates(
        &mut cpu_renderer,
        &mut cpu_scene,
        cpu_node,
        cpu_camera,
        sample_count,
    );

    let gpu = match Renderer::headless_gpu(32, 32) {
        Ok(mut renderer) => {
            let (mut scene, node, camera) = profiled_transform_scene();
            let provenance = performance_environment_metadata_with_renderer(
                "native-gpu-one-node-transform",
                &renderer,
            );
            renderer
                .prepare(&mut scene)
                .expect("GPU transform warm prepare succeeds");
            renderer
                .render(&scene, camera)
                .expect("GPU transform warm render succeeds");
            let samples = measure_profiled_transform_updates(
                &mut renderer,
                &mut scene,
                node,
                camera,
                sample_count,
            );
            let mut result = profiled_prepare_render_samples_json(&samples);
            result["provenance"] = provenance;
            result
        }
        Err(error) => serde_json::json!({
            "status": "hardware-unavailable",
            "release_evidence": false,
            "reason": error.to_string(),
        }),
    };
    let release_scale = sample_count >= BENCHMARK_SAMPLE_COUNT
        && gpu.get("status").and_then(serde_json::Value::as_str) != Some("hardware-unavailable");
    serde_json::json!({
        "id": "one-node-transform-prepare-render",
        "status": if release_scale { "measured-headless-cpu-and-gpu" } else { "contract-scale-or-hardware-partial" },
        "release_evidence": release_scale,
        "reason": if release_scale {
            "optimized CPU and native GPU prepare-plus-render distributions are measured; the PF00 summary separately binds complete real-GPU backend proof"
        } else {
            "the native GPU row or release-scale sample count is unavailable"
        },
        "fixture": {
            "width": 32,
            "height": 32,
            "mutated_nodes_per_sample": 1,
            "sample_policy": "warm renderer, alternate one node transform, then measure prepare and render separately",
        },
        "cpu": profiled_prepare_render_samples_json(&cpu),
        "headless_gpu": gpu,
        "provenance": performance_environment_metadata("one-node-transform-cpu-headless-gpu"),
    })
}

fn profiled_transform_scene() -> (Scene, scena::NodeKey, scena::CameraKey) {
    let mut scene = Scene::new();
    let node = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("profiled transform node inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("profiled transform camera inserts");
    scene
        .set_active_camera(camera)
        .expect("profiled transform active camera sets");
    (scene, node, camera)
}

struct ProfiledPrepareRenderSamples {
    prepare_ms: Vec<f64>,
    render_ms: Vec<f64>,
    combined_ms: Vec<f64>,
    allocation_counts: Vec<u64>,
    allocation_bytes: Vec<u64>,
    prepare_metrics: scena::PrepareWorkMetrics,
    render_metrics: scena::RenderWorkMetrics,
}

fn measure_profiled_transform_updates(
    renderer: &mut Renderer,
    scene: &mut Scene,
    node: scena::NodeKey,
    camera: scena::CameraKey,
    sample_count: usize,
) -> ProfiledPrepareRenderSamples {
    let mut prepare_ms = Vec::with_capacity(sample_count);
    let mut render_ms = Vec::with_capacity(sample_count);
    let mut combined_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_prepare = None;
    let mut expected_render = None;
    for sample in 0..sample_count {
        scene
            .set_transform(
                node,
                Transform::at(Vec3::new(
                    if sample % 2 == 0 { 0.01 } else { -0.01 },
                    0.0,
                    0.0,
                )),
            )
            .expect("profiled transform mutates");
        start_allocation_counting();
        let combined_start = Instant::now();
        let prepare_start = Instant::now();
        let prepare = renderer
            .prepare_profiled(scene)
            .expect("profiled transform prepare succeeds");
        let prepare_elapsed = prepare_start.elapsed().as_secs_f64() * 1000.0;
        let render_start = Instant::now();
        renderer
            .render(scene, camera)
            .expect("profiled transform render succeeds");
        let render_elapsed = render_start.elapsed().as_secs_f64() * 1000.0;
        let combined_elapsed = combined_start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let render = renderer.last_render_work_metrics();
        if let Some(expected) = expected_prepare {
            assert_eq!(
                prepare, expected,
                "transform prepare counters must be deterministic"
            );
        } else {
            expected_prepare = Some(prepare);
        }
        if let Some(expected) = expected_render {
            assert_eq!(
                render, expected,
                "transform render counters must be deterministic"
            );
        } else {
            expected_render = Some(render);
        }
        prepare_ms.push(prepare_elapsed);
        render_ms.push(render_elapsed);
        combined_ms.push(combined_elapsed);
        allocation_counts.push(allocation_count());
        allocated_bytes.push(allocation_bytes());
    }
    ProfiledPrepareRenderSamples {
        prepare_ms,
        render_ms,
        combined_ms,
        allocation_counts,
        allocation_bytes: allocated_bytes,
        prepare_metrics: expected_prepare.expect("transform prepare records metrics"),
        render_metrics: expected_render.expect("transform render records metrics"),
    }
}

fn profiled_prepare_render_samples_json(
    samples: &ProfiledPrepareRenderSamples,
) -> serde_json::Value {
    serde_json::json!({
        "status": "measured",
        "distributions": {
            "prepare_ms": duration_distribution_json(&samples.prepare_ms),
            "render_ms": duration_distribution_json(&samples.render_ms),
            "prepare_plus_render_ms": duration_distribution_json(&samples.combined_ms),
        },
        "allocations": allocation_measurement_json(
            &samples.allocation_counts,
            &samples.allocation_bytes,
        ),
        "counters": {
            "prepare": prepare_work_metrics_json(samples.prepare_metrics),
            "render": render_work_metrics_json(samples.render_metrics),
        },
    })
}

fn render_work_metrics_json(metrics: scena::RenderWorkMetrics) -> serde_json::Value {
    serde_json::json!({
        "prepared_primitive_list_clones": metrics.prepared_primitive_list_clones,
        "prepared_stroke_list_clones": metrics.prepared_stroke_list_clones,
        "prepared_label_list_clones": metrics.prepared_label_list_clones,
        "prepared_list_clone_bytes": metrics.prepared_list_clone_bytes,
        "readback_copies": metrics.readback_copies,
        "readback_bytes_copied": metrics.readback_bytes_copied,
        "map_requests": metrics.map_requests,
        "blocking_polls": metrics.blocking_polls,
        "blocking_waits": metrics.blocking_waits,
        "cpu_frame_copy_bytes": metrics.cpu_frame_copy_bytes,
        "gpu_buffer_creations": metrics.gpu_buffer_creations,
        "gpu_texture_creations": metrics.gpu_texture_creations,
        "gpu_pipeline_creations": metrics.gpu_pipeline_creations,
        "gpu_bind_group_creations": metrics.gpu_bind_group_creations,
        "gpu_shader_module_creations": metrics.gpu_shader_module_creations,
        "native_scene_color_passes": metrics.native_scene_color_passes,
        "gpu_queue_submissions": metrics.gpu_queue_submissions,
        "async_readback_submissions": metrics.async_readback_submissions,
        "peak_readbacks_in_flight": metrics.peak_readbacks_in_flight,
        "cpu_parallel_workers": metrics.cpu_parallel_workers,
        "cpu_raster_candidate_triangles": metrics.cpu_raster_candidate_triangles,
        "cpu_raster_full_rescan_triangles": metrics.cpu_raster_full_rescan_triangles,
        "cpu_raster_bin_storage_growth_bytes": metrics.cpu_raster_bin_storage_growth_bytes,
        "cpu_output_pixels_encoded": metrics.cpu_output_pixels_encoded,
        "cpu_primitive_flag_scan_items": metrics.cpu_primitive_flag_scan_items,
    })
}

fn benchmark_profiled_draw_uniform_workload(
    node_count: usize,
    sample_count: usize,
) -> serde_json::Value {
    assert!(node_count > 0);
    assert!(sample_count > 0);
    let (mut scene, _camera) = many_unique_transform_scene(node_count);
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return serde_json::json!({
            "id": "draw-uniform-indexing-many-unique-transforms",
            "status": "hardware-unavailable",
            "release_evidence": false,
            "reason": "no headless GPU adapter is available for the GPU prepare encoding path",
        });
    };
    let provenance =
        performance_environment_metadata_with_renderer("cpu-gpu-draw-uniform-encoding", &renderer);
    let mut duration_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_metrics: Option<scena::PrepareWorkMetrics> = None;
    for sample in 0..sample_count {
        renderer
            .handle_surface_event(SurfaceEvent::Resize {
                width: 16 + (sample % 2) as u32,
                height: 16,
            })
            .expect("draw-uniform benchmark target toggles");
        start_allocation_counting();
        let start = Instant::now();
        let result = renderer.prepare_profiled(&mut scene);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let metrics = result.expect("draw-uniform benchmark prepares");
        if let Some(expected) = expected_metrics {
            assert_eq!(
                metrics.draw_uniform_lookup_probes, expected.draw_uniform_lookup_probes,
                "draw-uniform lookup probes must be deterministic"
            );
        } else {
            expected_metrics = Some(metrics);
        }
        duration_ms.push(elapsed_ms);
        allocation_counts.push(allocation_count());
        allocated_bytes.push(allocation_bytes());
    }
    let metrics = expected_metrics.expect("draw-uniform benchmark records metrics");
    let release_scale = node_count == 512 && sample_count >= BENCHMARK_SAMPLE_COUNT;
    serde_json::json!({
        "id": "draw-uniform-indexing-many-unique-transforms",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "unique_transformed_nodes": node_count,
            "release_unique_transformed_nodes": 512,
            "triangles_per_node": 1,
            "target_toggle": "16x16 and 17x16 force full GPU resource preparation without timing renderer construction",
        },
        "distributions": {
            "prepare_ms": duration_distribution_json(&duration_ms),
        },
        "allocations": allocation_measurement_json(&allocation_counts, &allocated_bytes),
        "counters": prepare_work_metrics_json(metrics),
        "provenance": provenance,
    })
}

fn many_unique_transform_scene(node_count: usize) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    for index in 0..node_count {
        scene
            .add_renderable(
                scene.root(),
                vec![Primitive::unlit_triangle()],
                Transform::at(Vec3::new(index as f32 * 0.001, 0.0, 0.0)),
            )
            .expect("draw-uniform benchmark node inserts");
    }
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("draw-uniform benchmark camera inserts");
    scene
        .set_active_camera(camera)
        .expect("draw-uniform benchmark active camera sets");
    (scene, camera)
}

fn benchmark_profiled_environment_bake_workload(sample_count: usize) -> serde_json::Value {
    assert!(sample_count > 0);
    let source_path = root().join("tests/assets/environment/polyhaven/studio_small_08_1k.hdr");
    let source_bytes = fs::read(&source_path).expect("PF00 HDR fixture reads");
    let cold = EnvironmentDesc::from_equirectangular_hdr_bytes(
        source_path.to_string_lossy().into_owned(),
        &source_bytes,
    )
    .expect("PF00 HDR fixture decodes")
    .with_cubemap_resolution(8);
    let sidecar = scena::render::precompute_environment_sidecar(
        &cold,
        EnvironmentSidecarProfile::InteractiveWebGl2,
    )
    .expect("PF00 reference environment sidecar precomputes");
    let mut cold_ms = Vec::with_capacity(sample_count);
    let mut hit_ms = Vec::with_capacity(sample_count);
    let mut cold_allocations = Vec::with_capacity(sample_count);
    let mut cold_bytes = Vec::with_capacity(sample_count);
    let mut hit_allocations = Vec::with_capacity(sample_count);
    let mut hit_bytes = Vec::with_capacity(sample_count);
    let mut expected_metrics = None;
    for _ in 0..sample_count {
        start_allocation_counting();
        let start = Instant::now();
        let (baked, metrics) = scena::render::precompute_environment_sidecar_profiled(
            &cold,
            EnvironmentSidecarProfile::InteractiveWebGl2,
        )
        .expect("PF00 cold environment bake succeeds");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        assert_eq!(baked.source_sha256_hex(), sidecar.source_sha256_hex());
        if let Some(expected) = expected_metrics {
            assert_eq!(
                metrics, expected,
                "environment work counters are deterministic"
            );
        } else {
            expected_metrics = Some(metrics);
        }
        cold_ms.push(elapsed);
        cold_allocations.push(allocation_count());
        cold_bytes.push(allocation_bytes());

        start_allocation_counting();
        let start = Instant::now();
        let hit = cold
            .clone()
            .with_prefilter_sidecar(sidecar.clone())
            .expect("PF00 sidecar hit validates");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        assert_eq!(hit.source_sha256(), cold.source_sha256());
        hit_ms.push(elapsed);
        hit_allocations.push(allocation_count());
        hit_bytes.push(allocation_bytes());
    }
    let metrics = expected_metrics.expect("profiled environment bake records work");
    let release_scale = sample_count >= BENCHMARK_SAMPLE_COUNT;
    serde_json::json!({
        "id": "environment-bake-cold-sidecar-hit",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "fixture": {
            "source": source_path,
            "source_sha256": cold.source_sha256(),
            "cubemap_resolution": sidecar.cubemap_resolution(),
            "brdf_lut_size": sidecar.brdf_lut_size(),
            "profile": "interactive-webgl2",
            "sidecar_bytes": sidecar.to_bytes().len(),
        },
        "distributions": {
            "cold_bake_ms": duration_distribution_json(&cold_ms),
            "sidecar_hit_ms": duration_distribution_json(&hit_ms),
        },
        "allocations": {
            "cold": allocation_measurement_json(&cold_allocations, &cold_bytes),
            "sidecar_hit": allocation_measurement_json(&hit_allocations, &hit_bytes),
        },
        "counters": {
            "source_texture_samples": metrics.source_texture_samples,
            "brdf_integration_samples": metrics.brdf_integration_samples,
            "prefilter_output_texels": metrics.prefilter_output_texels,
            "brdf_lut_texels": metrics.brdf_lut_texels,
            "output_bytes_written": metrics.output_bytes_written,
            "parallel_workers": metrics.parallel_workers,
            "parallel_tasks": metrics.parallel_tasks,
            "sidecar_bytes": sidecar.to_bytes().len(),
            "bytes_cloned_or_copied": metrics.output_bytes_written,
        },
        "provenance": performance_environment_metadata("cpu-assets-environment-bake"),
    })
}

fn benchmark_profiled_native_capture_workload(sample_count: usize) -> serde_json::Value {
    assert!(sample_count > 0);
    let Ok(mut renderer) = Renderer::headless_gpu(32, 32) else {
        return serde_json::json!({
            "id": "native-present-capture-sync-async",
            "status": "hardware-unavailable",
            "release_evidence": false,
            "reason": "no native headless GPU adapter is available for readback-mode instrumentation",
        });
    };
    let provenance = performance_environment_metadata_with_renderer(
        "native-headless-gpu-readback-modes",
        &renderer,
    );
    let (mut scene, _node, camera) = profiled_transform_scene();
    renderer
        .prepare(&mut scene)
        .expect("PF00 capture-mode scene prepares");
    renderer
        .render_with_readback_mode(&scene, camera, scena::RenderReadbackMode::Synchronous)
        .expect("PF00 capture-mode warm readback succeeds");

    let mut present_ms = Vec::with_capacity(sample_count);
    let mut synchronous_ms = Vec::with_capacity(sample_count);
    let mut asynchronous_ms = Vec::with_capacity(sample_count);
    let mut present_allocations = Vec::with_capacity(sample_count);
    let mut present_bytes = Vec::with_capacity(sample_count);
    let mut synchronous_allocations = Vec::with_capacity(sample_count);
    let mut synchronous_bytes = Vec::with_capacity(sample_count);
    let mut asynchronous_allocations = Vec::with_capacity(sample_count);
    let mut asynchronous_bytes = Vec::with_capacity(sample_count);
    let mut present_metrics = None;
    let mut synchronous_metrics = None;
    let mut asynchronous_metrics = None;
    for _ in 0..sample_count {
        start_allocation_counting();
        let start = Instant::now();
        renderer
            .render_with_readback_mode(&scene, camera, scena::RenderReadbackMode::PresentOnly)
            .expect("PF00 present-only render succeeds");
        present_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        stop_allocation_counting();
        present_allocations.push(allocation_count());
        present_bytes.push(allocation_bytes());
        let metrics = renderer.last_render_work_metrics();
        assert_eq!(metrics.readback_copies, 0);
        assert_eq!(metrics.blocking_polls, 0);
        assert_eq!(metrics.blocking_waits, 0);
        if let Some(expected) = present_metrics {
            assert_eq!(metrics, expected);
        } else {
            present_metrics = Some(metrics);
        }

        start_allocation_counting();
        let start = Instant::now();
        renderer
            .render_with_readback_mode(&scene, camera, scena::RenderReadbackMode::Synchronous)
            .expect("PF00 synchronous readback render succeeds");
        synchronous_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        stop_allocation_counting();
        synchronous_allocations.push(allocation_count());
        synchronous_bytes.push(allocation_bytes());
        let metrics = renderer.last_render_work_metrics();
        assert_eq!(metrics.readback_copies, 1);
        assert_eq!(metrics.blocking_polls, 1);
        assert_eq!(metrics.blocking_waits, 1);
        if let Some(expected) = synchronous_metrics {
            assert_eq!(metrics, expected);
        } else {
            synchronous_metrics = Some(metrics);
        }

        start_allocation_counting();
        let start = Instant::now();
        let frames = renderer
            .render_batch_with_async_readback(&scene, &[camera, camera])
            .expect("PF00 double-buffered asynchronous batch succeeds");
        asynchronous_ms.push(start.elapsed().as_secs_f64() * 500.0);
        stop_allocation_counting();
        assert_eq!(frames.len(), 2);
        asynchronous_allocations.push(allocation_count());
        asynchronous_bytes.push(allocation_bytes());
        let metrics = renderer.last_render_work_metrics();
        assert_eq!(metrics.readback_copies, 2);
        assert_eq!(metrics.async_readback_submissions, 2);
        assert_eq!(metrics.peak_readbacks_in_flight, 2);
        if let Some(expected) = asynchronous_metrics {
            assert_eq!(metrics, expected);
        } else {
            asynchronous_metrics = Some(metrics);
        }
    }

    let release_scale = sample_count >= BENCHMARK_SAMPLE_COUNT;
    serde_json::json!({
        "id": "native-present-capture-sync-async",
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "release_evidence": release_scale,
        "reason": "native present-only, synchronous readback, and ordered two-buffer asynchronous readback distributions are measured; attached-surface zero-readback correctness is bound separately by the PF00 hardware summary",
        "fixture": {
            "width": 32,
            "height": 32,
            "backend": "HeadlessGpu",
            "sample_policy": "one synchronous warm frame followed by paired present-only and synchronous renders",
        },
        "distributions": {
            "present_only_ms": duration_distribution_json(&present_ms),
            "synchronous_capture_ms": duration_distribution_json(&synchronous_ms),
            "asynchronous_capture_ms": duration_distribution_json(&asynchronous_ms),
        },
        "allocations": {
            "present_only": allocation_measurement_json(&present_allocations, &present_bytes),
            "synchronous_capture": allocation_measurement_json(
                &synchronous_allocations,
                &synchronous_bytes,
            ),
            "asynchronous_batch": allocation_measurement_json(
                &asynchronous_allocations,
                &asynchronous_bytes,
            ),
        },
        "counters": {
            "present_only": render_work_metrics_json(
                present_metrics.expect("PF00 present-only metrics record"),
            ),
            "synchronous_capture": render_work_metrics_json(
                synchronous_metrics.expect("PF00 synchronous metrics record"),
            ),
            "asynchronous_batch": render_work_metrics_json(
                asynchronous_metrics.expect("PF00 asynchronous metrics record"),
            ),
        },
        "provenance": provenance,
    })
}

#[derive(Clone, Copy)]
enum Pf00OutputSetting {
    Baseline,
    Fxaa,
    Msaa4,
    Msaa8,
    Bloom,
    Ssao,
    Ssr,
    DepthOfField,
}

const PF00_OUTPUT_SETTINGS: [Pf00OutputSetting; 8] = [
    Pf00OutputSetting::Baseline,
    Pf00OutputSetting::Fxaa,
    Pf00OutputSetting::Msaa4,
    Pf00OutputSetting::Msaa8,
    Pf00OutputSetting::Bloom,
    Pf00OutputSetting::Ssao,
    Pf00OutputSetting::Ssr,
    Pf00OutputSetting::DepthOfField,
];

impl Pf00OutputSetting {
    const fn id(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Fxaa => "fxaa",
            Self::Msaa4 => "msaa4",
            Self::Msaa8 => "msaa8",
            Self::Bloom => "bloom",
            Self::Ssao => "ssao",
            Self::Ssr => "ssr",
            Self::DepthOfField => "depth_of_field",
        }
    }

    fn configure(self, renderer: &mut Renderer, enabled: bool) {
        match self {
            // Baseline is AA-off; FXAA is the deliberately different state
            // used between samples so every timed prepare follows a mutation.
            Self::Baseline => renderer.set_anti_aliasing(if enabled {
                AntiAliasing::None
            } else {
                AntiAliasing::Fxaa
            }),
            Self::Fxaa => renderer.set_anti_aliasing(if enabled {
                AntiAliasing::Fxaa
            } else {
                AntiAliasing::None
            }),
            Self::Msaa4 => renderer.set_anti_aliasing(if enabled {
                AntiAliasing::Msaa4
            } else {
                AntiAliasing::None
            }),
            Self::Msaa8 => renderer.set_anti_aliasing(if enabled {
                AntiAliasing::Msaa8
            } else {
                AntiAliasing::None
            }),
            Self::Bloom => renderer.set_bloom(enabled.then_some(PostBloomConfig::subtle())),
            Self::Ssao => renderer.set_screen_space_ambient_occlusion(
                enabled.then_some(ScreenSpaceAmbientOcclusionConfig::subtle()),
            ),
            Self::Ssr => renderer.set_screen_space_reflections(
                enabled.then_some(ScreenSpaceReflectionConfig::default()),
            ),
            Self::DepthOfField => {
                renderer.set_depth_of_field(enabled.then_some(DepthOfFieldConfig::new(2.5, 1.2, 6)))
            }
        }
    }
}

fn benchmark_profiled_gpu_output_settings_workload(sample_count: usize) -> serde_json::Value {
    assert!(sample_count > 0);
    let Ok(mut renderer) = Renderer::headless_gpu(32, 32) else {
        return serde_json::json!({
            "id": "gpu-first-render-output-settings",
            "status": "hardware-unavailable",
            "release_evidence": false,
            "reason": "no native headless GPU adapter is available for output-prepare instrumentation",
        });
    };
    let provenance = performance_environment_metadata_with_renderer(
        "native-headless-gpu-output-prepare",
        &renderer,
    );
    let (mut scene, _node, camera) = profiled_transform_scene();
    let settings = PF00_OUTPUT_SETTINGS
        .into_iter()
        .map(|setting| {
            benchmark_profiled_gpu_output_setting(
                &mut renderer,
                &mut scene,
                camera,
                setting,
                sample_count,
            )
        })
        .collect::<Vec<_>>();
    let release_scale = sample_count >= BENCHMARK_SAMPLE_COUNT
        && settings
            .iter()
            .all(|setting| setting["status"] == "measured");
    serde_json::json!({
        "id": "gpu-first-render-output-settings",
        "status": if release_scale { "measured" } else { "contract-scale-or-unsupported" },
        "release_evidence": release_scale,
        "reason": "all supported output settings are measured on native GPU; the PF00 summary separately binds the complete native-surface, WebGPU, and WebGL2 rendered proof",
        "settings": settings,
        "provenance": provenance,
    })
}

fn benchmark_profiled_gpu_output_setting(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: scena::CameraKey,
    setting: Pf00OutputSetting,
    sample_count: usize,
) -> serde_json::Value {
    let mut prepare_ms = Vec::with_capacity(sample_count);
    let mut render_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_prepare = None;
    let mut expected_render = None;

    for _ in 0..sample_count {
        setting.configure(renderer, false);
        if let Err(error) = renderer.prepare(scene) {
            return serde_json::json!({
                "id": setting.id(),
                "status": "unsupported",
                "reason": format!("disabled-state prepare failed before measurement: {error}"),
            });
        }
        setting.configure(renderer, true);

        start_allocation_counting();
        let prepare_start = Instant::now();
        let prepare_metrics = match renderer.prepare_profiled(scene) {
            Ok(metrics) => metrics,
            Err(error) => {
                stop_allocation_counting();
                setting.configure(renderer, false);
                return serde_json::json!({
                    "id": setting.id(),
                    "status": "unsupported",
                    "reason": format!("output-setting prepare failed: {error}"),
                });
            }
        };
        let elapsed_prepare_ms = prepare_start.elapsed().as_secs_f64() * 1000.0;
        let render_start = Instant::now();
        if let Err(error) = renderer.render_with_readback_mode(
            scene,
            camera,
            scena::RenderReadbackMode::PresentOnly,
        ) {
            stop_allocation_counting();
            setting.configure(renderer, false);
            return serde_json::json!({
                "id": setting.id(),
                "status": "unsupported",
                "reason": format!("first prepared render failed: {error}"),
            });
        }
        let elapsed_render_ms = render_start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let render_metrics = renderer.last_render_work_metrics();

        if let Some(expected) = expected_prepare {
            assert_eq!(
                prepare_metrics,
                expected,
                "{} prepare counters",
                setting.id()
            );
        } else {
            expected_prepare = Some(prepare_metrics);
        }
        if let Some(expected) = expected_render {
            assert_eq!(render_metrics, expected, "{} render counters", setting.id());
        } else {
            expected_render = Some(render_metrics);
        }
        prepare_ms.push(elapsed_prepare_ms);
        render_ms.push(elapsed_render_ms);
        allocation_counts.push(allocation_count());
        allocated_bytes.push(allocation_bytes());
    }
    setting.configure(renderer, false);

    let release_scale = sample_count >= BENCHMARK_SAMPLE_COUNT;
    serde_json::json!({
        "id": setting.id(),
        "status": if release_scale { "measured" } else { "contract-scale-measured" },
        "distributions": {
            "prepare_output_ms": duration_distribution_json(&prepare_ms),
            "first_prepared_render_ms": duration_distribution_json(&render_ms),
        },
        "allocations": allocation_measurement_json(&allocation_counts, &allocated_bytes),
        "counters": {
            "prepare": prepare_work_metrics_json(
                expected_prepare.expect("output-setting prepare metrics record"),
            ),
            "render": render_work_metrics_json(
                expected_render.expect("output-setting render metrics record"),
            ),
        },
    })
}

fn overlapping_pick_geometry(triangle_count: usize, deformed: bool) -> GeometryDesc {
    let vertex_count = triangle_count
        .checked_mul(3)
        .expect("pick benchmark vertex count fits usize");
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(vertex_count);
    for triangle in 0..triangle_count {
        let base =
            u32::try_from(triangle.saturating_mul(3)).expect("pick benchmark index fits u32");
        vertices.extend([
            GeometryVertex {
                position: Vec3::new(-0.4, -0.4, 0.0),
                normal: Vec3::Z,
            },
            GeometryVertex {
                position: Vec3::new(0.4, -0.4, 0.0),
                normal: Vec3::Z,
            },
            GeometryVertex {
                position: Vec3::new(0.0, 0.4, 0.0),
                normal: Vec3::Z,
            },
        ]);
        indices.extend([base, base + 1, base + 2]);
    }
    let geometry = GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
        .expect("pick benchmark geometry validates");
    if deformed {
        geometry
            .with_morph_targets(vec![GeometryMorphTarget::new(vec![
                Vec3::new(
                    0.01, 0.0, 0.0
                );
                vertex_count
            ])])
            .expect("pick benchmark morph target validates")
    } else {
        geometry
    }
}

fn profiled_pick_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    deformed: bool,
) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("pick benchmark mesh inserts");
    if deformed {
        scene
            .set_morph_weights(mesh, [1.0])
            .expect("pick benchmark morph weight sets");
    }
    let camera = scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera {
                left: -1.0,
                right: 1.0,
                bottom: -1.0,
                top: 1.0,
                near: 0.01,
                far: 10.0,
            },
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("pick benchmark camera inserts");
    (scene, camera)
}

struct ProfiledPickSamples {
    duration_ms: Vec<f64>,
    allocation_counts: Vec<u64>,
    allocation_bytes: Vec<u64>,
    metrics: scena::PickingMetrics,
}

fn measure_profiled_picks<F>(
    scene: &Scene,
    assets: &Assets<F>,
    camera: scena::CameraKey,
    cursor: CursorPosition,
    viewport: Viewport,
    sample_count: usize,
) -> ProfiledPickSamples {
    let (warm_hit, _) = scene
        .pick_with_assets_profiled(camera, cursor, viewport, assets)
        .expect("profiled pick warmup succeeds");
    assert!(warm_hit.is_some(), "profiled benchmark warmup ray must hit");
    let mut duration_ms = Vec::with_capacity(sample_count);
    let mut allocation_counts = Vec::with_capacity(sample_count);
    let mut allocated_bytes = Vec::with_capacity(sample_count);
    let mut expected_metrics = None;
    for _ in 0..sample_count {
        start_allocation_counting();
        let start = Instant::now();
        let result = scene.pick_with_assets_profiled(camera, cursor, viewport, assets);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        stop_allocation_counting();
        let count = allocation_count();
        let bytes = allocation_bytes();
        let (hit, metrics) = result.expect("profiled pick succeeds");
        assert!(hit.is_some(), "profiled benchmark ray must hit the fixture");
        if let Some(expected) = expected_metrics {
            assert_eq!(
                metrics, expected,
                "profiled pick counters must be deterministic"
            );
        } else {
            expected_metrics = Some(metrics);
        }
        duration_ms.push(elapsed_ms);
        allocation_counts.push(count);
        allocated_bytes.push(bytes);
    }
    ProfiledPickSamples {
        duration_ms,
        allocation_counts,
        allocation_bytes: allocated_bytes,
        metrics: expected_metrics.expect("profiled pick records metrics"),
    }
}

fn duration_distribution_json(samples: &[f64]) -> serde_json::Value {
    let distribution = benchmark_distribution(samples);
    serde_json::json!({
        "sample_count": distribution.sample_count,
        "min_ms": distribution.min_frame_ms,
        "p50_ms": distribution.p50_frame_ms,
        "p95_ms": distribution.p95_frame_ms,
        "max_ms": distribution.max_frame_ms,
        "population_stddev_ms": distribution.stddev_frame_ms,
    })
}

fn allocation_measurement_json(counts: &[u64], bytes: &[u64]) -> serde_json::Value {
    let count_distribution = allocation_distribution(counts);
    let byte_distribution = allocation_byte_distribution(bytes);
    serde_json::json!({
        "sample_count": counts.len(),
        "p95_allocation_count": count_distribution.p95_allocations_per_frame,
        "max_allocation_count": count_distribution.max_allocations_per_frame,
        "p50_allocated_bytes": byte_distribution.p50_allocated_bytes_per_frame,
        "p95_allocated_bytes": byte_distribution.p95_allocated_bytes_per_frame,
        "max_allocated_bytes": byte_distribution.max_allocated_bytes_per_frame,
    })
}

fn picking_metrics_json(metrics: scena::PickingMetrics) -> serde_json::Value {
    serde_json::json!({
        "mesh_nodes_considered": metrics.mesh_nodes_considered,
        "instance_sets_considered": metrics.instance_sets_considered,
        "mesh_bounds_tests": metrics.mesh_bounds_tests,
        "mesh_bounds_rejections": metrics.mesh_bounds_rejections,
        "bvh_node_bounds_tests": metrics.bvh_node_bounds_tests,
        "static_bvh_cache_hits": metrics.static_bvh_cache_hits,
        "static_bvh_cache_misses": metrics.static_bvh_cache_misses,
        "deformed_bvh_builds": metrics.deformed_bvh_builds,
        "triangles_considered": metrics.triangles_considered,
        "triangle_bounds_tests": metrics.triangle_bounds_tests,
        "ray_triangle_intersection_tests": metrics.ray_triangle_intersection_tests,
        "deformed_vertices_materialized": metrics.deformed_vertices_materialized,
        "deformed_vertex_bytes_materialized": metrics.deformed_vertex_bytes_materialized,
    })
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
    benchmark_scene_with_renderer_setup(input, scene, assets, camera, |_| {})
}

fn benchmark_scene_with_renderer_setup(
    input: BenchmarkSceneInput<'_>,
    scene: &mut Scene,
    assets: Option<&Assets>,
    camera: scena::CameraKey,
    configure_renderer: impl FnOnce(&mut Renderer),
) -> serde_json::Value {
    assert!(
        input.sample_count > 0,
        "benchmark sample count must be nonzero"
    );
    let mut renderer = Renderer::headless(input.width, input.height).expect("renderer builds");
    configure_renderer(&mut renderer);
    let camera_node = scene
        .camera_node(camera)
        .expect("benchmark camera node resolves");
    let camera_transform = scene
        .node(camera_node)
        .expect("benchmark camera node exists")
        .transform();
    let mut prepare_samples = Vec::with_capacity(input.sample_count);
    for sample_index in 0..input.sample_count {
        let mut transform = camera_transform;
        transform.translation.x += if sample_index % 2 == 0 { 0.000_1 } else { 0.0 };
        scene
            .set_transform(camera_node, transform)
            .expect("benchmark camera transform mutates");
        let start = Instant::now();
        if let Some(assets) = assets {
            renderer
                .prepare_with_assets(scene, assets)
                .expect("asset scene prepares");
        } else {
            renderer.prepare(scene).expect("scene prepares");
        }
        prepare_samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let start = Instant::now();
    let warmup = renderer.render(scene, camera).expect("warm scene render");
    let warmup_frame_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut samples = Vec::with_capacity(input.sample_count);
    let mut allocation_samples = Vec::with_capacity(input.sample_count);
    let mut allocation_byte_samples = Vec::with_capacity(input.sample_count);
    let mut outcome = warmup;
    for _ in 0..input.sample_count {
        let start = Instant::now();
        start_allocation_counting();
        let next = renderer.render(scene, camera);
        stop_allocation_counting();
        let allocation_count = allocation_count();
        let allocation_bytes = allocation_bytes();
        outcome = next.expect("scene renders");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        allocation_samples.push(allocation_count);
        allocation_byte_samples.push(allocation_bytes);
    }
    benchmark_row(BenchmarkRowInput {
        scene: input.name,
        backend: renderer.capabilities().backend,
        samples: &samples,
        allocation_samples: &allocation_samples,
        allocation_byte_samples: &allocation_byte_samples,
        prepare_samples: &prepare_samples,
        draw_calls: outcome.draw_calls,
        skipped: outcome.skipped,
        fixture: BenchmarkFixture {
            width: input.width,
            height: input.height,
            source: input.fixture_source,
            sample_count_policy: input.sample_count_policy,
        },
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
    allocation_samples: &'a [u64],
    allocation_byte_samples: &'a [u64],
    prepare_samples: &'a [f64],
    draw_calls: u64,
    skipped: bool,
    fixture: BenchmarkFixture<'a>,
    warmup_frame_ms: Option<f64>,
}

fn benchmark_row(input: BenchmarkRowInput<'_>) -> serde_json::Value {
    let distribution = benchmark_distribution(input.samples);
    let allocation_distribution = allocation_distribution(input.allocation_samples);
    let allocation_byte_distribution = allocation_byte_distribution(input.allocation_byte_samples);
    let prepare_distribution = benchmark_distribution(input.prepare_samples);
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
        "p95_allocations_per_frame": allocation_distribution.p95_allocations_per_frame,
        "max_allocations_per_frame": allocation_distribution.max_allocations_per_frame,
        "p50_allocated_bytes_per_frame": allocation_byte_distribution.p50_allocated_bytes_per_frame,
        "p95_allocated_bytes_per_frame": allocation_byte_distribution.p95_allocated_bytes_per_frame,
        "max_allocated_bytes_per_frame": allocation_byte_distribution.max_allocated_bytes_per_frame,
        "prepare_sample_count": prepare_distribution.sample_count,
        "p50_prepare_ms": prepare_distribution.p50_frame_ms,
        "p95_prepare_ms": prepare_distribution.p95_frame_ms,
        "max_prepare_ms": prepare_distribution.max_frame_ms,
        "prepare_ms": prepare_distribution.p50_frame_ms,
        "prepare_sample_policy": "one camera-node transform mutation before each prepare; first sample includes cold preparation",
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

struct AllocationByteDistribution {
    p50_allocated_bytes_per_frame: u64,
    p95_allocated_bytes_per_frame: u64,
    max_allocated_bytes_per_frame: u64,
}

fn allocation_byte_distribution(samples: &[u64]) -> AllocationByteDistribution {
    assert!(
        !samples.is_empty(),
        "allocation-byte distribution requires at least one sample"
    );
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    AllocationByteDistribution {
        p50_allocated_bytes_per_frame: percentile_nearest_rank_u64(&sorted, 0.50),
        p95_allocated_bytes_per_frame: percentile_nearest_rank_u64(&sorted, 0.95),
        max_allocated_bytes_per_frame: sorted[sorted.len() - 1],
    }
}

struct AllocationDistribution {
    p95_allocations_per_frame: u64,
    max_allocations_per_frame: u64,
}

fn allocation_distribution(samples: &[u64]) -> AllocationDistribution {
    assert!(
        !samples.is_empty(),
        "allocation distribution requires at least one sample"
    );
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    AllocationDistribution {
        p95_allocations_per_frame: percentile_nearest_rank_u64(&sorted, 0.95),
        max_allocations_per_frame: sorted[sorted.len() - 1],
    }
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

fn percentile_nearest_rank_u64(sorted_samples: &[u64], percentile: f64) -> u64 {
    debug_assert!(!sorted_samples.is_empty());
    let rank = (sorted_samples.len() as f64 * percentile).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    sorted_samples[index]
}

fn start_allocation_counting() {
    ALLOCATION_COUNT.set(0);
    ALLOCATION_BYTES.set(0);
    ALLOCATION_SIZES.with(|sizes| *sizes.borrow_mut() = [0; 64]);
    COUNT_ALLOCATIONS.with(|counting| counting.set(true));
}

fn stop_allocation_counting() {
    COUNT_ALLOCATIONS.with(|counting| counting.set(false));
}

fn allocation_count() -> u64 {
    ALLOCATION_COUNT.get() as u64
}

fn allocation_bytes() -> u64 {
    ALLOCATION_BYTES.get() as u64
}

fn record_allocation_size(size: usize) {
    let index = ALLOCATION_COUNT.get().saturating_sub(1).min(63);
    ALLOCATION_SIZES.with(|sizes| sizes.borrow_mut()[index] = size);
}

fn allocation_size_trace() -> Vec<usize> {
    let count = ALLOCATION_COUNT.get().min(64);
    ALLOCATION_SIZES.with(|sizes| sizes.borrow()[..count].to_vec())
}

fn capability_matrix_row(
    lane: &str,
    measured_current_lane: &RenderedArtifact,
    measured_headless_cpu: &RenderedArtifact,
    browser_results: &[serde_json::Value],
    wasm_size_artifact: Option<&serde_json::Value>,
) -> serde_json::Value {
    if lane == current_lane() {
        lane_capability_from_artifact(lane, measured_current_lane)
    } else if lane == HEADLESS_CPU_LANE {
        lane_capability_from_artifact(lane, measured_headless_cpu)
    } else if lane == "linux-webgl2-chromium" {
        browser_capability_from_probe(lane, "WebGl2", browser_results)
            .unwrap_or_else(|| missing_lane_capability(lane))
    } else if lane == "linux-webgpu-chromium" {
        browser_capability_from_probe(lane, "WebGpu", browser_results)
            .unwrap_or_else(|| missing_lane_capability(lane))
    } else if lane == "wasm32-unknown-unknown" {
        wasm_capability_from_artifact(lane, wasm_size_artifact)
            .unwrap_or_else(|| missing_lane_capability(lane))
    } else {
        missing_lane_capability(lane)
    }
}

fn material_preset_capability_rows() -> Vec<serde_json::Value> {
    ROUND_E_MATERIAL_PRESETS
        .iter()
        .flat_map(|preset| {
            ROUND_E_MATERIAL_LANES.iter().map(move |lane| {
                let status = material_preset_lane_status(preset, lane);
                serde_json::json!({
                    "preset": preset,
                    "lane": lane,
                    "status": status,
                    "measurement_source": material_preset_lane_source(&status),
                    "public_demo_required": material_preset_lane_required_for_public_demo(preset, lane),
                    "capability_contract": "real-world-material-preset",
                })
            })
        })
        .collect()
}

fn material_preset_lane_status(preset: &str, lane: &str) -> String {
    if lane == "cpu-reference" {
        return "measured".to_string();
    }
    if lane == "webgl2-desktop-chromium" && cloudflare_material_preset_passes(preset) {
        return "measured".to_string();
    }
    "proof-gap".to_string()
}

fn material_preset_lane_source(status: &str) -> &'static str {
    if status == "measured" {
        "round-e-material-lane-artifact"
    } else {
        "missing-material-lane-artifact"
    }
}

fn material_preset_lane_required_for_public_demo(preset: &str, lane: &str) -> bool {
    matches!(
        lane,
        "webgl2-desktop-chromium" | "ios-safari" | "android-chrome"
    ) && matches!(
        preset,
        "chrome" | "brushed_steel" | "clearcoat_plastic" | "clear_glass"
    )
}

fn cloudflare_material_preset_passes(preset: &str) -> bool {
    let path = root().join("target/gate-artifacts/round-e-cloudflare-material-proof.json");
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    value.get("status").and_then(serde_json::Value::as_str) == Some("pass")
        && value
            .get("errors")
            .and_then(serde_json::Value::as_array)
            .is_some_and(Vec::is_empty)
        && value
            .pointer(&format!("/per_material/{preset}/reference_delta_gate"))
            .and_then(serde_json::Value::as_str)
            == Some("hard")
        && value
            .pointer(&format!("/per_material/{preset}/passed_reference_delta"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
}

fn read_browser_probe_results() -> Vec<serde_json::Value> {
    let path = root().join("target/gate-artifacts/m6-rust-wasm-renderer-probe.json");
    if !path.is_file() {
        return Vec::new();
    }
    let text = fs::read_to_string(&path).expect("m6 browser probe JSON reads");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("m6 browser probe JSON parses");
    value
        .get("results")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn read_wasm_size_artifact() -> Option<serde_json::Value> {
    let path = root().join("target/gate-artifacts/m9-wasm-size.json");
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(&path).expect("m9 wasm-size JSON reads");
    Some(serde_json::from_str(&text).expect("m9 wasm-size JSON parses"))
}

fn browser_capability_from_probe(
    lane: &str,
    backend: &str,
    results: &[serde_json::Value],
) -> Option<serde_json::Value> {
    let result = results.iter().find(|result| {
        result
            .get("backend")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.eq_ignore_ascii_case(backend))
            && result.get("status").and_then(serde_json::Value::as_str) == Some("passed")
            && browser_nonblack_pixels(result) > 0
    })?;
    Some(serde_json::json!({
        "lane": lane,
        "status": "measured",
        "measurement_source": "browser-probe-runtime",
        "commit": current_commit_label(),
        "commit_sha": current_commit_label(),
        "timestamp_unix_seconds": current_timestamp_unix_seconds(),
        "backend": result.get("backend").cloned().unwrap_or(serde_json::Value::Null),
        "adapter": {
            "available": result
                .get("gpu_device")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            "runtime": "browser-canvas",
        },
        "host_gpu_available": result
            .get("gpu_device")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        "host_gpu_error": serde_json::Value::Null,
        "capabilities": result
            .get("capabilities")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "diagnostics": result
            .get("diagnostics")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "workflow": result.get("workflow").cloned().unwrap_or(serde_json::Value::Null),
        "surface_attached": result
            .get("surface_attached")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "pixel_statistics": browser_pixel_statistics(result)
            .unwrap_or(serde_json::Value::Null),
        "canvas_output_color_space": result
            .get("canvas_output_color_space")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    }))
}

fn browser_probe_has_passed_backend(results: &[serde_json::Value], backend: &str) -> bool {
    results.iter().any(|result| {
        result
            .get("backend")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.eq_ignore_ascii_case(backend))
            && result.get("status").and_then(serde_json::Value::as_str) == Some("passed")
            && browser_nonblack_pixels(result) > 0
    })
}

fn browser_nonblack_pixels(result: &serde_json::Value) -> u64 {
    browser_pixel_statistics(result)
        .and_then(|pixels| pixels.get("nonblack").cloned())
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn browser_pixel_statistics(result: &serde_json::Value) -> Option<serde_json::Value> {
    result
        .get("renderer_readback")
        .and_then(|readback| readback.get("pixel_statistics"))
        .cloned()
        .or_else(|| result.get("pixels").cloned())
}

fn wasm_capability_from_artifact(
    lane: &str,
    artifact: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let artifact = artifact?;
    (artifact.get("status").and_then(serde_json::Value::as_str) == Some("passed")).then(|| {
        serde_json::json!({
            "lane": lane,
            "status": "measured",
            "measurement_source": "wasm-size-gate-runtime",
            "commit": current_commit_label(),
            "commit_sha": current_commit_label(),
            "timestamp_unix_seconds": current_timestamp_unix_seconds(),
            "capabilities": {
                "wasm_bundle": artifact,
            },
            "diagnostics": [],
        })
    })
}

fn lane_capability_from_artifact(lane: &str, artifact: &RenderedArtifact) -> serde_json::Value {
    let mut row = lane_capability(lane, artifact.capabilities, "lane-renderer-runtime");
    row["status"] = serde_json::json!("measured");
    row["adapter"] = adapter_metadata(artifact.adapter.as_ref());
    row["host_gpu_available"] = serde_json::json!(artifact.host_gpu_available);
    row["host_gpu_error"] = serde_json::json!(artifact.host_gpu_error);
    row["commit"] = serde_json::json!(current_commit_label());
    row["commit_sha"] = serde_json::json!(current_commit_label());
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
        "commit_sha": current_commit_label(),
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
        "order_independent_transparency": { "state": format!("{:?}", capabilities.order_independent_transparency) },
        "physical_glass_transmission": { "state": format!("{:?}", capabilities.physical_glass_transmission) },
        "wide_gamut_output": { "state": format!("{:?}", capabilities.wide_gamut_output) },
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
        "subject_visible_mask": { "state": format!("{:?}", capabilities.subject_visible_mask) },
        "auto_exposure_metering": {
            "average": { "state": format!("{:?}", capabilities.auto_exposure_metering_average) },
            "center_weighted": { "state": format!("{:?}", capabilities.auto_exposure_metering_center_weighted) },
            "highlight_weighted": { "state": format!("{:?}", capabilities.auto_exposure_metering_highlight_weighted) },
            "subject": { "state": format!("{:?}", capabilities.auto_exposure_metering_subject) },
            "spot": { "state": format!("{:?}", capabilities.auto_exposure_metering_spot) },
        },
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
    let mut value = value.clone();
    let object = value.as_object_mut().expect("release JSON is an object");
    assert!(
        object
            .get("schema")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|schema| !schema.trim().is_empty()),
        "release JSON producer must declare its typed schema: {}",
        path.display()
    );
    object
        .entry("producer".to_string())
        .or_insert_with(|| serde_json::json!("cargo test --test m9_platform_release"));
    object
        .entry("commit_sha".to_string())
        .or_insert_with(|| serde_json::json!(current_commit_label()));
    object
        .entry("timestamp_unix_seconds".to_string())
        .or_insert_with(|| serde_json::json!(current_timestamp_unix_seconds()));
    object
        .entry("source_checksums".to_string())
        .or_insert_with(|| {
            serde_json::json!([
                release_source_checksum("Cargo.lock"),
                release_source_checksum("tests/m9_platform_release.rs")
            ])
        });
    let body = serde_json::to_string_pretty(&value).expect("json serializes");
    fs::write(path, format!("{body}\n")).expect("json writes");
}

fn release_source_checksum(relative: &str) -> serde_json::Value {
    let bytes = fs::read(root().join(relative)).expect("release provenance source reads");
    let digest = Sha256::digest(bytes);
    serde_json::json!({
        "path": relative,
        "sha256": digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    })
}

#[test]
fn m9_release_json_writer_attaches_source_provenance() {
    let path = platform_dir().join("writer-provenance-contract.json");
    fs::create_dir_all(path.parent().expect("writer fixture parent"))
        .expect("writer fixture directory");
    write_json(
        &path,
        &serde_json::json!({"schema":"scena.m9.writer_fixture.v1","status":"passed"}),
    );
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("writer provenance fixture reads"))
            .expect("writer provenance fixture parses");
    assert_eq!(value["producer"], "cargo test --test m9_platform_release");
    assert!(value["commit_sha"].as_str().is_some());
    assert!(value["timestamp_unix_seconds"].as_u64().is_some());
    assert!(
        value["source_checksums"]
            .as_array()
            .is_some_and(|entries| entries.len() >= 2)
    );
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
