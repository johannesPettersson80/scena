//! Q01 default-lane WaterBottle proof.
//!
//! This test always invokes the deterministic CPU renderer at 256x256, compares
//! the live RGBA8/sRGB/top-row-first output to a committed CPU reference, and
//! proves the same oracle rejects rendered material and camera corruptions.
#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;
use std::io::BufWriter;

use scena::{Assets, Color, Renderer, Scene, Tonemapper, Transform, Vec3};

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
const RGB_CHEBYSHEV_TOLERANCE: u8 = 4;
const MIN_WITHIN_TOLERANCE_FRACTION: f64 = 0.995;
const MAX_RGB_RMSE: f64 = 2.0;

#[test]
fn q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders() {
    std::fs::create_dir_all(ARTIFACT_DIR).expect("Q01 artifact directory creates");
    let (assets, mut scene) = build_waterbottle_scene();

    let mut renderer = Renderer::headless(SIZE, SIZE).expect("Q01 CPU renderer builds");
    renderer.set_background_color(Color::from_srgb_u8(216, 196, 170));
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
    renderer.set_exposure_ev(0.0);

    let live = render_current(&mut renderer, &mut scene, &assets);
    write_png(&live, LIVE_PNG);

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

    let wrong_material_frame = wrong_material_mutation(&live);
    write_png(&wrong_material_frame, WRONG_MATERIAL_PNG);
    let wrong_material_metrics = compare_rgba8(&wrong_material_frame, &reference.rgba);
    assert!(
        !wrong_material_metrics.passes(),
        "wrong material must be rejected by the live reference oracle: {wrong_material_metrics:#?}"
    );

    let wrong_camera_frame = wrong_camera_mutation(&live);
    write_png(&wrong_camera_frame, WRONG_CAMERA_PNG);
    let wrong_camera_metrics = compare_rgba8(&wrong_camera_frame, &reference.rgba);
    assert!(
        !wrong_camera_metrics.passes(),
        "wrong camera must be rejected by the live reference oracle: {wrong_camera_metrics:#?}"
    );

    write_result(
        &live,
        &reference.encoded,
        &live_metrics,
        &flat_chrome_metrics,
        &wrong_material_metrics,
        &wrong_camera_metrics,
    );
}

fn build_waterbottle_scene() -> (Assets, Scene) {
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
    (assets, scene)
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

fn wrong_material_mutation(source: &[u8]) -> Vec<u8> {
    mutate_foreground(source, |pixel| [230, 24, 180, pixel[3]])
}

fn wrong_camera_mutation(source: &[u8]) -> Vec<u8> {
    let background = [source[0], source[1], source[2], source[3]];
    background.repeat((SIZE * SIZE) as usize)
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
        "metrics": live_metrics.json(),
        "mutations": [
            mutation_json("flattened_chrome", FLAT_CHROME_PNG, flat_chrome_metrics),
            mutation_json("wrong_material", WRONG_MATERIAL_PNG, wrong_material_metrics),
            mutation_json("wrong_camera", WRONG_CAMERA_PNG, wrong_camera_metrics),
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

fn mutation_json(name: &str, path: &str, metrics: &ReferenceMetrics) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "path": path.strip_prefix("target/gate-artifacts/").unwrap_or(path),
        "sha256": sha256_hex_file(path),
        "oracle_rejected": !metrics.passes(),
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
