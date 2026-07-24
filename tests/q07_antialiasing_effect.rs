#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;

use scena::{
    AntiAliasing, Assets, Color, PrepareError, Primitive, Renderer, Scene, Transform, Vec3, Vertex,
};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;
const ARTIFACT_DIR: &str = "target/gate-artifacts/q07-antialiasing-effect";

#[derive(Debug, Clone, Copy, serde::Serialize)]
struct EdgeMetrics {
    intermediate_luma_pixels: u64,
    hard_transition_count: u64,
    squared_edge_energy: u64,
    luma_range: u8,
}

#[derive(Debug)]
struct RenderedMode {
    name: &'static str,
    frame: Vec<u8>,
    metrics: EdgeMetrics,
    adapter: serde_json::Value,
}

#[test]
fn q07_effect_oracle_rejects_noop_and_blur_everything_mutations() {
    let baseline = binary_diagonal_fixture(WIDTH, HEIGHT);
    let antialiased = supersampled_diagonal_fixture(WIDTH, HEIGHT, 4);
    let no_op = baseline.clone();
    let blurred = box_blur_rgba8(&baseline, WIDTH, HEIGHT, 7);

    let baseline_metrics = measure_edges(&baseline, WIDTH, HEIGHT);
    assert!(
        evaluate_antialiasing_effect(baseline_metrics, measure_edges(&antialiased, WIDTH, HEIGHT),)
            .is_ok(),
        "the deterministic sampled diagonal must satisfy the AA effect oracle"
    );
    assert!(
        evaluate_antialiasing_effect(baseline_metrics, measure_edges(&no_op, WIDTH, HEIGHT),)
            .is_err(),
        "a no-op AA implementation must fail"
    );
    assert!(
        evaluate_antialiasing_effect(baseline_metrics, measure_edges(&blurred, WIDTH, HEIGHT),)
            .is_err(),
        "blurring the whole frame must not satisfy an edge-only AA oracle"
    );
}

#[test]
fn q07_effect_oracle_allows_edge_growth_on_an_intermediate_tone_baseline() {
    let baseline = EdgeMetrics {
        intermediate_luma_pixels: 4_554,
        hard_transition_count: 762,
        squared_edge_energy: 1_000_000,
        luma_range: 200,
    };
    let fxaa = EdgeMetrics {
        intermediate_luma_pixels: 4_831,
        hard_transition_count: 500,
        squared_edge_energy: 700_000,
        luma_range: 190,
    };

    assert!(
        evaluate_antialiasing_effect(baseline, fxaa).is_ok(),
        "the Windows DX12 observation must retain the baseline intermediate-tone fill \
         while allowing bounded edge-local FXAA growth"
    );
}

#[test]
fn q07_release_commit_prefers_valid_explicit_then_github_provenance() {
    const EXPLICIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const GITHUB: &str = "89abcdef0123456789abcdef0123456789abcdef";
    assert_eq!(
        select_release_commit(Some(EXPLICIT.to_string()), Some(GITHUB.to_string())),
        EXPLICIT
    );
    assert_eq!(
        select_release_commit(None, Some(GITHUB.to_string())),
        GITHUB
    );
    assert_eq!(
        select_release_commit(Some("invalid".to_string()), Some(GITHUB.to_string())),
        GITHUB
    );
    assert_eq!(select_release_commit(None, None), "local-checkout");
}

#[test]
fn q07_required_native_antialiasing_modes_have_pixel_effect() {
    if std::env::var("SCENA_REQUIRE_AA_EFFECT_PROOF").as_deref() != Ok("1") {
        return;
    }

    let baseline = render_mode("none", AntiAliasing::None)
        .expect("required native baseline must render on a hardware GPU");
    assert_hardware_adapter(&baseline.adapter);
    let fxaa = render_mode("fxaa", AntiAliasing::Fxaa)
        .expect("required native FXAA mode must render on a hardware GPU");
    let msaa4 = render_mode("msaa4", AntiAliasing::Msaa4)
        .expect("required native MSAA4 mode must render on a hardware GPU");

    for candidate in [&fxaa, &msaa4] {
        evaluate_antialiasing_effect(baseline.metrics, candidate.metrics).unwrap_or_else(|error| {
            panic!("{} failed the AA pixel oracle: {error}", candidate.name)
        });
    }

    let msaa8 = match render_mode("msaa8", AntiAliasing::Msaa8) {
        Ok(mode) => {
            evaluate_antialiasing_effect(baseline.metrics, mode.metrics)
                .unwrap_or_else(|error| panic!("msaa8 failed the AA pixel oracle: {error}"));
            serde_json::json!({
                "status": "passed",
                "metrics": mode.metrics,
                "frame_path": write_frame(&mode),
            })
        }
        Err(PrepareError::UnsupportedSampleCount {
            requested,
            maximum,
            backend,
        }) => serde_json::json!({
            "status": "degraded",
            "reason_code": "UNSUPPORTED_SAMPLE_COUNT",
            "requested": requested,
            "maximum": maximum,
            "backend": format!("{backend:?}"),
        }),
        Err(error) => panic!("MSAA8 failed without an explicit capability result: {error:?}"),
    };

    let baseline_path = write_frame(&baseline);
    let fxaa_path = write_frame(&fxaa);
    let msaa4_path = write_frame(&msaa4);
    let mut source_paths = vec![
        "tests/q07_antialiasing_effect.rs".to_string(),
        baseline_path.clone(),
        fxaa_path.clone(),
        msaa4_path.clone(),
    ];
    if let Some(path) = msaa8.get("frame_path").and_then(serde_json::Value::as_str) {
        source_paths.push(path.to_string());
    }
    let source_checksums = source_paths
        .into_iter()
        .map(|relative| {
            let file = if relative.starts_with("q07-antialiasing-effect/") {
                Path::new("target/gate-artifacts").join(&relative)
            } else {
                Path::new(&relative).to_path_buf()
            };
            serde_json::json!({"path":relative, "sha256":sha256_file(&file)})
        })
        .collect::<Vec<_>>();
    fs::create_dir_all(ARTIFACT_DIR).expect("Q07 artifact directory creates");
    let artifact = serde_json::json!({
        "schema": "scena.q07.antialiasing_effect.v1",
        "status": "passed",
        "release_evidence": true,
        "producer": "cargo test --test q07_antialiasing_effect q07_required_native_antialiasing_modes_have_pixel_effect -- --exact",
        "commit_sha": release_commit(),
        "timestamp_unix_seconds": release_timestamp(),
        "fixture": "high-contrast-asymmetric-diagonal-v1",
        "width": WIDTH,
        "height": HEIGHT,
        "adapter": baseline.adapter,
        "baseline": {
            "mode": "none",
            "metrics": baseline.metrics,
            "frame_path": baseline_path,
        },
        "modes": {
            "fxaa": {"status":"passed", "metrics":fxaa.metrics, "frame_path":fxaa_path},
            "msaa4": {"status":"passed", "metrics":msaa4.metrics, "frame_path":msaa4_path},
            "msaa8": msaa8,
        },
        "known_bad_mutations": [
            {"name":"no_op", "rejected":true},
            {"name":"blur_everything", "rejected":true},
        ],
        "source_checksums": source_checksums,
    });
    fs::write(
        Path::new(ARTIFACT_DIR).join("result.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&artifact).expect("Q07 artifact serializes")
        ),
    )
    .expect("Q07 artifact writes");
}

fn render_mode(
    name: &'static str,
    anti_aliasing: AntiAliasing,
) -> Result<RenderedMode, PrepareError> {
    let mut renderer = Renderer::headless_gpu(WIDTH, HEIGHT)
        .unwrap_or_else(|error| panic!("required Q07 hardware renderer unavailable: {error:?}"));
    renderer.set_background_color(Color::BLACK);
    renderer.set_anti_aliasing(anti_aliasing);
    let (assets, mut scene, camera) = diagonal_scene();
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer
        .render(&scene, camera)
        .unwrap_or_else(|error| panic!("{name} render failed: {error:?}"));
    let frame = renderer.frame_rgba8().to_vec();
    let metrics = measure_edges(&frame, WIDTH, HEIGHT);
    let adapter =
        serde_json::to_value(renderer.gpu_adapter_report()).expect("GPU adapter report serializes");
    Ok(RenderedMode {
        name,
        frame,
        metrics,
        adapter,
    })
}

fn diagonal_scene() -> (Assets, Scene, scena::CameraKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("Q07 camera inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.88, -0.82, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.91, -0.63, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(-0.37, 0.89, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("Q07 diagonal inserts");
    (assets, scene, camera)
}

fn evaluate_antialiasing_effect(
    baseline: EdgeMetrics,
    candidate: EdgeMetrics,
) -> Result<(), String> {
    let minimum_intermediate = baseline.intermediate_luma_pixels.saturating_add(20);
    let maximum_intermediate = baseline
        .intermediate_luma_pixels
        .saturating_add(baseline.hard_transition_count.saturating_mul(6))
        .max(minimum_intermediate);
    if candidate.intermediate_luma_pixels < minimum_intermediate {
        return Err(format!(
            "intermediate coverage {} is below required {}",
            candidate.intermediate_luma_pixels, minimum_intermediate
        ));
    }
    if candidate.intermediate_luma_pixels > maximum_intermediate {
        return Err(format!(
            "intermediate coverage {} exceeds edge-local maximum {}",
            candidate.intermediate_luma_pixels, maximum_intermediate
        ));
    }
    if candidate.hard_transition_count >= baseline.hard_transition_count {
        return Err(format!(
            "hard transitions did not decrease: baseline {} candidate {}",
            baseline.hard_transition_count, candidate.hard_transition_count
        ));
    }
    if candidate.squared_edge_energy * 100 >= baseline.squared_edge_energy * 98 {
        return Err(format!(
            "squared edge energy did not materially decrease: baseline {} candidate {}",
            baseline.squared_edge_energy, candidate.squared_edge_energy
        ));
    }
    if u16::from(candidate.luma_range) * 10 < u16::from(baseline.luma_range) * 9 {
        return Err(format!(
            "global contrast collapsed: baseline {} candidate {}",
            baseline.luma_range, candidate.luma_range
        ));
    }
    Ok(())
}

fn assert_hardware_adapter(adapter: &serde_json::Value) {
    let device_type = adapter
        .get("device_type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    assert!(
        matches!(device_type, "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu"),
        "Q07 release evidence requires a hardware adapter, got {adapter}"
    );
    let identity = ["name", "driver", "driver_info"]
        .into_iter()
        .filter_map(|field| adapter.get(field).and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    for marker in [
        "llvmpipe",
        "lavapipe",
        "swiftshader",
        "software",
        "basic render",
    ] {
        assert!(
            !identity.contains(marker),
            "Q07 release evidence rejects software adapter marker {marker:?}: {adapter}"
        );
    }
}

fn measure_edges(frame: &[u8], width: u32, height: u32) -> EdgeMetrics {
    assert_eq!(frame.len(), (width * height * 4) as usize);
    let luma = frame
        .chunks_exact(4)
        .map(|pixel| {
            ((u16::from(pixel[0]) * 54 + u16::from(pixel[1]) * 183 + u16::from(pixel[2]) * 19)
                / 256) as u8
        })
        .collect::<Vec<_>>();
    let mut hard_transition_count = 0u64;
    let mut squared_edge_energy = 0u64;
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            for other in [
                (x + 1 < width).then_some(index + 1),
                (y + 1 < height).then_some(index + width as usize),
            ]
            .into_iter()
            .flatten()
            {
                let delta = u64::from(luma[index].abs_diff(luma[other]));
                squared_edge_energy = squared_edge_energy.saturating_add(delta * delta);
                if delta >= 192 {
                    hard_transition_count += 1;
                }
            }
        }
    }
    let min_luma = luma.iter().copied().min().unwrap_or(0);
    let max_luma = luma.iter().copied().max().unwrap_or(0);
    EdgeMetrics {
        intermediate_luma_pixels: luma
            .iter()
            .filter(|value| **value > 8 && **value < 247)
            .count() as u64,
        hard_transition_count,
        squared_edge_energy,
        luma_range: max_luma.saturating_sub(min_luma),
    }
}

fn binary_diagonal_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut frame = vec![0; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let inside = (x as f32 + y as f32 * 0.73) < width as f32 * 0.92;
            let offset = ((y * width + x) * 4) as usize;
            let value = if inside { 255 } else { 0 };
            frame[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    frame
}

fn supersampled_diagonal_fixture(width: u32, height: u32, samples: u32) -> Vec<u8> {
    let mut frame = vec![0; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let mut covered = 0;
            for sy in 0..samples {
                for sx in 0..samples {
                    let px = x as f32 + (sx as f32 + 0.5) / samples as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / samples as f32;
                    covered += u32::from(px + py * 0.73 < width as f32 * 0.92);
                }
            }
            let value = (covered * 255 / (samples * samples)) as u8;
            let offset = ((y * width + x) * 4) as usize;
            frame[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    frame
}

fn box_blur_rgba8(source: &[u8], width: u32, height: u32, radius: u32) -> Vec<u8> {
    let mut output = vec![0; source.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0u32;
            let mut count = 0u32;
            for sample_y in y.saturating_sub(radius)..=(y + radius).min(height - 1) {
                for sample_x in x.saturating_sub(radius)..=(x + radius).min(width - 1) {
                    sum += u32::from(source[((sample_y * width + sample_x) * 4) as usize]);
                    count += 1;
                }
            }
            let value = (sum / count) as u8;
            let offset = ((y * width + x) * 4) as usize;
            output[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    output
}

fn write_frame(mode: &RenderedMode) -> String {
    fs::create_dir_all(ARTIFACT_DIR).expect("Q07 artifact directory creates");
    let relative = format!("q07-antialiasing-effect/{}.ppm", mode.name);
    let path = Path::new("target/gate-artifacts").join(&relative);
    let mut bytes = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    for pixel in mode.frame.chunks_exact(4) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    fs::write(path, bytes).expect("Q07 frame writes");
    relative
}

fn release_commit() -> String {
    select_release_commit(
        std::env::var("SCENA_RELEASE_COMMIT").ok(),
        std::env::var("GITHUB_SHA").ok(),
    )
}

fn select_release_commit(explicit: Option<String>, github: Option<String>) -> String {
    explicit
        .into_iter()
        .chain(github)
        .find(|value| value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .unwrap_or_else(|| "local-checkout".to_string())
}

fn release_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time follows Unix epoch")
        .as_secs()
}

fn sha256_file(path: &Path) -> String {
    use sha2::Digest as _;
    let bytes =
        fs::read(path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
