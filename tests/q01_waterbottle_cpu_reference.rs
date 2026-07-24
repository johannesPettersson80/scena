//! Q01 default-lane WaterBottle proof.
//!
//! This test always invokes the deterministic CPU renderer at 256x256, compares
//! the live RGBA8/sRGB/top-row-first output to a committed CPU reference, and
//! proves the same oracle rejects rendered material and camera corruptions.
#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;
use std::io::BufWriter;

use scena::{
    Assets, CameraKey, Color, MaterialDesc, NodeKey, Renderer, Scene, Tonemapper, Transform, Vec3,
};

const SIZE: u32 = 256;
const WATERBOTTLE_PATH: &str = "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf";
const REFERENCE_PATH: &str = "tests/assets/gltf/khronos/WaterBottle/reference_cpu_256.png";
const REFERENCE_SHA256: &str = "922cc35e0c6420d2b3f8e533891291a9d4f9396697ae366f0b93de3c15973da4";
const ARTIFACT_DIR: &str = "target/gate-artifacts/q01-waterbottle-cpu";
const LIVE_PNG: &str = "target/gate-artifacts/q01-waterbottle-cpu/live.png";
const FLAT_CHROME_PNG: &str =
    "target/gate-artifacts/q01-waterbottle-cpu/known_bad_flattened_chrome.png";
const WRONG_MATERIAL_PNG: &str =
    "target/gate-artifacts/q01-waterbottle-cpu/known_bad_wrong_material.png";
const WRONG_CAMERA_PNG: &str =
    "target/gate-artifacts/q01-waterbottle-cpu/known_bad_wrong_camera.png";
const RESULT_JSON: &str = "target/gate-artifacts/q01-waterbottle-cpu/result.json";
const WATERBOTTLE_GLTF_SHA256: &str =
    "0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547";
const RGB_CHEBYSHEV_TOLERANCE: u8 = 4;
const MIN_WITHIN_TOLERANCE_FRACTION: f64 = 0.995;
const MAX_RGB_RMSE: f64 = 2.0;

#[test]
fn q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders() {
    std::fs::create_dir_all(ARTIFACT_DIR).expect("Q01 artifact directory creates");
    let mut fixture = build_waterbottle_scene();
    let mut renderer = configured_cpu_renderer();

    let live = render_current(&mut renderer, &mut fixture.scene, &fixture.assets);
    write_png(&live, LIVE_PNG);

    let repeat = render_baseline_repeat();
    assert_eq!(
        live, repeat,
        "independent in-process WaterBottle renders must be byte-identical before the committed reference is consulted"
    );

    let reference = read_reference_png();
    assert_eq!(sha256_hex(&reference.encoded), REFERENCE_SHA256);
    assert_eq!((reference.width, reference.height), (SIZE, SIZE));
    let live_metrics = compare_rgba8(&live, &reference.rgba);
    assert!(
        live_metrics.passes(),
        "live CPU WaterBottle must match the committed reference: {live_metrics:#?}"
    );

    let flat_chrome_frame = flattened_chrome_mutation(&live);
    write_png(&flat_chrome_frame, FLAT_CHROME_PNG);
    let flat_chrome_metrics = compare_rgba8(&flat_chrome_frame, &reference.rgba);
    assert!(
        !flat_chrome_metrics.passes(),
        "flattened chrome must be rejected by the live reference oracle: {flat_chrome_metrics:#?}"
    );

    let wrong_material_frame = render_wrong_material_scene();
    write_png(&wrong_material_frame, WRONG_MATERIAL_PNG);
    let wrong_material_metrics = compare_rgba8(&wrong_material_frame, &reference.rgba);
    assert!(
        !wrong_material_metrics.passes(),
        "wrong material must be rejected by the live reference oracle: {wrong_material_metrics:#?}"
    );

    let wrong_camera_frame = render_wrong_camera_scene();
    write_png(&wrong_camera_frame, WRONG_CAMERA_PNG);
    let wrong_camera_metrics = compare_rgba8(&wrong_camera_frame, &reference.rgba);
    assert!(
        !wrong_camera_metrics.passes(),
        "wrong camera must be rejected by the live reference oracle: {wrong_camera_metrics:#?}"
    );

    write_result(
        &live,
        &repeat,
        &reference.encoded,
        &live_metrics,
        &flat_chrome_metrics,
        &wrong_material_metrics,
        &wrong_camera_metrics,
    );
}

#[test]
fn q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison() {
    let first = render_baseline_repeat();
    let second = render_baseline_repeat();
    assert_eq!(
        first, second,
        "independent in-process WaterBottle renders must be byte-identical before the committed reference is consulted"
    );

    let reference = read_reference_png();
    assert_eq!(sha256_hex(&reference.encoded), REFERENCE_SHA256);
    let first_metrics = compare_rgba8(&first, &reference.rgba);
    let second_metrics = compare_rgba8(&second, &reference.rgba);
    if let Ok(candidate_dir) = std::env::var("SCENA_Q11_REFERENCE_CANDIDATE_DIR") {
        write_reference_candidate(
            &candidate_dir,
            &first,
            &reference.rgba,
            &reference.encoded,
            &first_metrics,
        );
    }
    assert!(
        first_metrics.passes(),
        "first Q11 render failed: {first_metrics:#?}"
    );
    assert!(
        second_metrics.passes(),
        "second Q11 render failed: {second_metrics:#?}"
    );
    write_q11_result(&first, &second, &first_metrics, &second_metrics);
    let result_path = format!(
        "target/gate-artifacts/q11-reference-stability/{}-{}.json",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let result: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).expect("Q11 result reads"))
            .expect("Q11 result parses");
    let source_checksums = result["source_checksums"]
        .as_array()
        .expect("Q11 result binds source checksums");
    assert_eq!(
        source_checksums,
        &[
            serde_json::json!({
                "path": WATERBOTTLE_PATH,
                "sha256": WATERBOTTLE_GLTF_SHA256,
            }),
            serde_json::json!({
                "path": REFERENCE_PATH,
                "sha256": REFERENCE_SHA256,
            }),
        ],
        "Q11 provenance must bind the source asset and committed reference"
    );
}

struct WaterBottleScene {
    assets: Assets,
    scene: Scene,
    mesh: NodeKey,
    camera: CameraKey,
}

fn build_waterbottle_scene() -> WaterBottleScene {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(WATERBOTTLE_PATH)).expect("WaterBottle loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("WaterBottle instantiates");
    let bounds = import
        .bounds_world(&scene)
        .expect("WaterBottle has world bounds");
    let centre = Vec3::new(
        (bounds.min.x + bounds.max.x) * 0.5,
        (bounds.min.y + bounds.max.y) * 0.5,
        (bounds.min.z + bounds.max.z) * 0.5,
    );
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(Vec3::new(centre.x + 0.12, centre.y + 0.05, centre.z + 0.25))
                .rotate_y_deg(25.0)
                .rotate_x_deg(-10.0),
        )
        .expect("WaterBottle camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_studio_lighting()
        .expect("Q01 deterministic studio lights insert");
    assert!(
        !import.roots().is_empty(),
        "WaterBottle imports scene roots"
    );
    let mesh = import
        .node("WaterBottle")
        .expect("WaterBottle imported mesh remains addressable");
    WaterBottleScene {
        assets,
        scene,
        mesh,
        camera,
    }
}

fn configured_cpu_renderer() -> Renderer {
    let mut renderer = Renderer::headless(SIZE, SIZE).expect("Q01 CPU renderer builds");
    renderer.set_background_color(Color::from_srgb_u8(216, 196, 170));
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
    renderer.set_exposure_ev(0.0);
    renderer
}

fn render_baseline_repeat() -> Vec<u8> {
    let mut fixture = build_waterbottle_scene();
    let mut renderer = configured_cpu_renderer();
    render_current(&mut renderer, &mut fixture.scene, &fixture.assets)
}

fn render_wrong_material_scene() -> Vec<u8> {
    let mut fixture = build_waterbottle_scene();
    let wrong_material = fixture
        .assets
        .create_material(MaterialDesc::unlit(Color::MAGENTA));
    fixture
        .scene
        .set_mesh_material(fixture.mesh, wrong_material)
        .expect("wrong-material scene mutation targets the imported mesh");
    let mut renderer = configured_cpu_renderer();
    render_current(&mut renderer, &mut fixture.scene, &fixture.assets)
}

fn render_wrong_camera_scene() -> Vec<u8> {
    let mut fixture = build_waterbottle_scene();
    let camera_node = fixture
        .scene
        .camera_node(fixture.camera)
        .expect("active WaterBottle camera owns a scene node");
    fixture
        .scene
        .set_transform(camera_node, Transform::at(Vec3::new(50.0, 50.0, 50.0)))
        .expect("wrong-camera scene mutation updates the active camera node");
    let mut renderer = configured_cpu_renderer();
    render_current(&mut renderer, &mut fixture.scene, &fixture.assets)
}

fn render_current(renderer: &mut Renderer, scene: &mut Scene, assets: &Assets) -> Vec<u8> {
    renderer
        .prepare_with_assets(scene, assets)
        .expect("Q01 WaterBottle prepares");
    renderer
        .render_active(scene)
        .expect("Q01 WaterBottle renders");
    renderer.frame_rgba8().to_vec()
}

fn flattened_chrome_mutation(source: &[u8]) -> Vec<u8> {
    mutate_foreground(source, |pixel| {
        let luma =
            (u16::from(pixel[0]) * 54 + u16::from(pixel[1]) * 183 + u16::from(pixel[2]) * 19) / 256;
        let flattened = (luma / 2 + 48).min(255) as u8;
        [flattened, flattened, flattened, pixel[3]]
    })
}

fn mutate_foreground(source: &[u8], mutation: impl Fn([u8; 4]) -> [u8; 4]) -> Vec<u8> {
    let background = [source[0], source[1], source[2]];
    let mut output = source.to_vec();
    for pixel in output.chunks_exact_mut(4) {
        let distance = pixel[0]
            .abs_diff(background[0])
            .max(pixel[1].abs_diff(background[1]))
            .max(pixel[2].abs_diff(background[2]));
        if distance > 6 {
            pixel.copy_from_slice(&mutation([pixel[0], pixel[1], pixel[2], pixel[3]]));
        }
    }
    output
}

#[derive(Debug)]
struct ReferenceMetrics {
    within_tolerance_pixels: usize,
    total_pixels: usize,
    within_tolerance_fraction: f64,
    rgb_rmse: f64,
    max_rgb_chebyshev: u8,
    alpha_mismatch_pixels: usize,
}

impl ReferenceMetrics {
    fn passes(&self) -> bool {
        self.total_pixels == (SIZE as usize) * (SIZE as usize)
            && self.within_tolerance_fraction >= MIN_WITHIN_TOLERANCE_FRACTION
            && self.rgb_rmse <= MAX_RGB_RMSE
            && self.alpha_mismatch_pixels == 0
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "within_tolerance_pixels": self.within_tolerance_pixels,
            "total_pixels": self.total_pixels,
            "within_tolerance_fraction": self.within_tolerance_fraction,
            "min_within_tolerance_fraction": MIN_WITHIN_TOLERANCE_FRACTION,
            "rgb_rmse": self.rgb_rmse,
            "max_rgb_rmse": MAX_RGB_RMSE,
            "max_rgb_chebyshev": self.max_rgb_chebyshev,
            "rgb_chebyshev_tolerance": RGB_CHEBYSHEV_TOLERANCE,
            "alpha_mismatch_pixels": self.alpha_mismatch_pixels,
            "passed": self.passes(),
        })
    }
}

fn compare_rgba8(live: &[u8], reference: &[u8]) -> ReferenceMetrics {
    assert_eq!(live.len(), reference.len(), "RGBA dimensions must match");
    let mut within_tolerance_pixels = 0usize;
    let mut squared_error = 0u64;
    let mut max_rgb_chebyshev = 0u8;
    let mut alpha_mismatch_pixels = 0usize;
    for (live, reference) in live.chunks_exact(4).zip(reference.chunks_exact(4)) {
        let dr = live[0].abs_diff(reference[0]);
        let dg = live[1].abs_diff(reference[1]);
        let db = live[2].abs_diff(reference[2]);
        let distance = dr.max(dg).max(db);
        within_tolerance_pixels += usize::from(distance <= RGB_CHEBYSHEV_TOLERANCE);
        max_rgb_chebyshev = max_rgb_chebyshev.max(distance);
        squared_error += u64::from(dr) * u64::from(dr)
            + u64::from(dg) * u64::from(dg)
            + u64::from(db) * u64::from(db);
        alpha_mismatch_pixels += usize::from(live[3] != reference[3]);
    }
    let total_pixels = live.len() / 4;
    let rgb_rmse = (squared_error as f64 / (total_pixels * 3) as f64).sqrt();
    ReferenceMetrics {
        within_tolerance_pixels,
        total_pixels,
        within_tolerance_fraction: within_tolerance_pixels as f64 / total_pixels as f64,
        rgb_rmse,
        max_rgb_chebyshev,
        alpha_mismatch_pixels,
    }
}

struct ReferenceImage {
    encoded: Vec<u8>,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

fn read_reference_png() -> ReferenceImage {
    let encoded = std::fs::read(REFERENCE_PATH).expect("committed Q01 CPU reference reads");
    let decoder = png::Decoder::new(std::io::Cursor::new(&encoded));
    let mut reader = decoder.read_info().expect("Q01 reference header reads");
    assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    assert_eq!(reader.info().bit_depth, png::BitDepth::Eight);
    let mut rgba = vec![
        0;
        reader
            .output_buffer_size()
            .expect("Q01 reference output size is known")
    ];
    let info = reader
        .next_frame(&mut rgba)
        .expect("Q01 reference payload reads");
    rgba.truncate(info.buffer_size());
    ReferenceImage {
        encoded,
        rgba,
        width: info.width,
        height: info.height,
    }
}

fn write_result(
    live: &[u8],
    repeat: &[u8],
    reference_png: &[u8],
    live_metrics: &ReferenceMetrics,
    flat_chrome_metrics: &ReferenceMetrics,
    wrong_material_metrics: &ReferenceMetrics,
    wrong_camera_metrics: &ReferenceMetrics,
) {
    let generated_at = current_unix_seconds();
    let artifact = serde_json::json!({
        "schema": "scena.q01.waterbottle_cpu_reference.v1",
        "status": "passed",
        "release_evidence": true,
        "test_name": "q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders",
        "producer": "cargo test --test q01_waterbottle_cpu_reference q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact",
        "commit_sha": current_release_commit(),
        "timestamp_unix_seconds": generated_at,
        "backend": "Headless",
        "adapter": "software-rasterizer",
        "width": SIZE,
        "height": SIZE,
        "color_type": "rgba8",
        "color_space": "srgb-output",
        "row_orientation": "top-to-bottom",
        "alpha_contract": "opaque",
        "live_png_path": "q01-waterbottle-cpu/live.png",
        "live_png_sha256": sha256_hex_file(LIVE_PNG),
        "reference_path": "tests/assets/gltf/khronos/WaterBottle/reference_cpu_256.png",
        "reference_sha256": sha256_hex(reference_png),
        "determinism": {
            "comparison_order": "independent-render-before-committed-reference",
            "repeat_count": 2,
            "byte_identical": live == repeat,
            "rgba8_sha256": [sha256_hex(live), sha256_hex(repeat)],
        },
        "metrics": live_metrics.json(),
        "mutations": [
            mutation_json(
                "flattened_chrome",
                FLAT_CHROME_PNG,
                flat_chrome_metrics,
                "post-hoc-pixel",
                "output-rgba8",
                &["oracle-evaluator"]
            ),
            mutation_json(
                "wrong_material",
                WRONG_MATERIAL_PNG,
                wrong_material_metrics,
                "rendered-scene",
                "scene-mesh-material-before-prepare",
                &["gltf-import", "texture-resources-loaded", "scene-material-override", "cpu-material-resolution", "prepare", "render", "pbr-neutral-tonemap", "srgb8-output"]
            ),
            mutation_json(
                "wrong_camera",
                WRONG_CAMERA_PNG,
                wrong_camera_metrics,
                "rendered-scene",
                "active-camera-transform-before-prepare",
                &["gltf-import", "texture-resources-loaded", "active-camera", "prepare", "render", "pbr-neutral-tonemap", "srgb8-output"]
            ),
        ],
        "rust_test_output_observed": false,
        "command_record_path": "release-lanes/headless-cpu.commands.jsonl",
        "source_checksums": [
            {"path":"q01-waterbottle-cpu/live.png", "sha256":sha256_hex_file(LIVE_PNG)},
            {"path":"tests/assets/gltf/khronos/WaterBottle/reference_cpu_256.png", "sha256":sha256_hex(reference_png)},
            {"path":"q01-waterbottle-cpu/known_bad_flattened_chrome.png", "sha256":sha256_hex_file(FLAT_CHROME_PNG)},
            {"path":"q01-waterbottle-cpu/known_bad_wrong_material.png", "sha256":sha256_hex_file(WRONG_MATERIAL_PNG)},
            {"path":"q01-waterbottle-cpu/known_bad_wrong_camera.png", "sha256":sha256_hex_file(WRONG_CAMERA_PNG)},
        ]
    });
    std::fs::write(
        RESULT_JSON,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&artifact).expect("Q01 result serializes")
        ),
    )
    .expect("Q01 result writes");
    assert_eq!(live.len(), (SIZE * SIZE * 4) as usize);
}

fn write_q11_result(
    first: &[u8],
    second: &[u8],
    first_metrics: &ReferenceMetrics,
    second_metrics: &ReferenceMetrics,
) {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let artifact_dir = "target/gate-artifacts/q11-reference-stability";
    std::fs::create_dir_all(artifact_dir).expect("Q11 artifact directory creates");
    let result_path = format!("{artifact_dir}/{os}-{arch}.json");
    let commit_sha = current_release_commit();
    let artifact = serde_json::json!({
        "schema": "scena.q11.reference_stability.v1",
        "status": "passed",
        "release_evidence": true,
        "test_name": "q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison",
        "producer": "cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
        "commit_sha": commit_sha,
        "timestamp_unix_seconds": current_unix_seconds(),
        "os": os,
        "arch": arch,
        "backend": "Headless",
        "adapter": "software-rasterizer",
        "width": SIZE,
        "height": SIZE,
        "comparison_order": "independent-render-before-committed-reference",
        "repeat_count": 2,
        "byte_identical": first == second,
        "rgba8_sha256": [sha256_hex(first), sha256_hex(second)],
        "metric_distribution": [first_metrics.json(), second_metrics.json()],
        "reference": {
            "path": REFERENCE_PATH,
            "sha256": REFERENCE_SHA256,
            "rgb_chebyshev_tolerance": RGB_CHEBYSHEV_TOLERANCE,
            "min_within_tolerance_fraction": MIN_WITHIN_TOLERANCE_FRACTION,
            "max_rgb_rmse": MAX_RGB_RMSE,
        },
        "source_asset": {
            "path": WATERBOTTLE_PATH,
            "sha256": WATERBOTTLE_GLTF_SHA256,
        },
        "source_checksums": [
            {
                "path": WATERBOTTLE_PATH,
                "sha256": WATERBOTTLE_GLTF_SHA256,
            },
            {
                "path": REFERENCE_PATH,
                "sha256": REFERENCE_SHA256,
            },
        ],
        "generator": {
            "crate_version": env!("CARGO_PKG_VERSION"),
            "rustc": "1.93.1",
            "profile": option_env!("PROFILE").unwrap_or("cargo-test"),
        }
    });
    std::fs::write(
        &result_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&artifact).expect("Q11 result serializes")
        ),
    )
    .expect("Q11 result writes");
}

fn write_reference_candidate(
    candidate_dir: &str,
    candidate: &[u8],
    reference: &[u8],
    reference_png: &[u8],
    metrics: &ReferenceMetrics,
) {
    let normalized = candidate_dir.replace('\\', "/");
    assert!(
        normalized.starts_with("target/reference-candidates/q01-waterbottle-")
            && !normalized.split('/').any(|part| part == ".."),
        "Q11 candidate output must be a task-specific target/reference-candidates/q01-waterbottle-* directory"
    );
    std::fs::create_dir_all(candidate_dir).expect("Q11 candidate directory creates");
    let candidate_path = format!("{candidate_dir}/candidate.png");
    let diff_path = format!("{candidate_dir}/diff-heatmap.png");
    write_png(candidate, &candidate_path);
    let diff = candidate
        .chunks_exact(4)
        .zip(reference.chunks_exact(4))
        .flat_map(|(candidate, reference)| {
            [
                candidate[0].abs_diff(reference[0]).saturating_mul(4),
                candidate[1].abs_diff(reference[1]).saturating_mul(4),
                candidate[2].abs_diff(reference[2]).saturating_mul(4),
                255,
            ]
        })
        .collect::<Vec<_>>();
    write_png(&diff, &diff_path);
    let manifest = serde_json::json!({
        "schema": "scena.q11.reference_candidate.v1",
        "status": "review-required",
        "release_evidence": false,
        "candidate_only": true,
        "approval": null,
        "generator_commit": current_release_commit(),
        "generator_version": env!("CARGO_PKG_VERSION"),
        "rustc": "1.93.1",
        "generated_at_unix_seconds": current_unix_seconds(),
        "command": "scripts/stage_q01_waterbottle_reference_candidate.sh",
        "source_asset": {"path": WATERBOTTLE_PATH, "sha256": WATERBOTTLE_GLTF_SHA256},
        "current_reference": {"path": REFERENCE_PATH, "sha256": sha256_hex(reference_png)},
        "external_anchor": {
            "path": "tests/assets/gltf/khronos/WaterBottle/reference_blender_cycles_512.png",
            "sha256": "17db39248ce1966ae60c3b85d09491ebfb7f654777dc2d150a64db4e938a6883",
            "required_for_approval": true,
        },
        "candidate": {"path": "candidate.png", "sha256": sha256_hex_file(&candidate_path)},
        "diff": {"path": "diff-heatmap.png", "sha256": sha256_hex_file(&diff_path)},
        "metrics_against_current_reference": metrics.json(),
        "tolerance_change_allowed": false,
        "promotion_requires": [
            "separate approval file not generated by this command",
            "named human reviewer",
            "candidate and before-reference SHA-256 bindings",
            "external-anchor review",
            "before/after diff review",
            "three-architecture stability evidence after promotion"
        ]
    });
    std::fs::write(
        format!("{candidate_dir}/candidate.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest).expect("Q11 candidate manifest serializes")
        ),
    )
    .expect("Q11 candidate manifest writes");
}

fn mutation_json(
    name: &str,
    path: &str,
    metrics: &ReferenceMetrics,
    mutation_kind: &str,
    mutation_stage: &str,
    pipeline_coverage: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "path": path.strip_prefix("target/gate-artifacts/").unwrap_or(path),
        "sha256": sha256_hex_file(path),
        "oracle_rejected": !metrics.passes(),
        "mutation_kind": mutation_kind,
        "mutation_stage": mutation_stage,
        "render_count": u8::from(mutation_kind == "rendered-scene"),
        "pipeline_coverage": pipeline_coverage,
        "metrics": metrics.json(),
    })
}

fn write_png(rgba: &[u8], path: &str) {
    let file = File::create(path).expect("Q01 PNG creates");
    let mut encoder = png::Encoder::new(BufWriter::new(file), SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("Q01 PNG header writes");
    writer
        .write_image_data(rgba)
        .expect("Q01 PNG payload writes");
}

fn current_release_commit() -> String {
    std::env::var("GITHUB_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("SCENA_RELEASE_COMMIT").ok())
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_string())
        })
        .unwrap_or_else(|| "local-checkout".to_string())
}

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn sha256_hex_file(path: &str) -> String {
    let bytes = std::fs::read(path).expect("Q01 artifact reads for checksum");
    sha256_hex(&bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
