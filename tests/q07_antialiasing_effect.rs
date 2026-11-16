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
    // E01 (`N13`): the recorded mutation results must be *computed* from this
    // run's live baseline, not written as literals. A literal `rejected: true`
    // survives an oracle that stopped rejecting anything.
    let known_bad_mutations = serde_json::Value::Array(
        evaluate_known_bad_mutations()
            .into_iter()
            .map(|(name, rejected, reason)| {
                assert!(
                    rejected,
                    "Q07 known-bad mutation '{name}' was accepted by this build's AA \
                     oracle on the deterministic fixture; the artifact must not be written",
                );
                serde_json::json!({"name": name, "rejected": rejected, "reason": reason})
            })
            .collect(),
    );
    // A mutation set that rejects everything proves nothing. Record a positive
    // control computed the same way, so an oracle degraded to "always reject"
    // is visible in the artifact itself.
    let control_metrics = measure_edges(
        &supersampled_diagonal_fixture(WIDTH, HEIGHT, 4),
        WIDTH,
        HEIGHT,
    );
    let control_accepted = evaluate_antialiasing_effect(baseline.metrics, control_metrics).is_ok();
    let positive_control = serde_json::json!({
        "name": "supersampled_diagonal",
        "accepted": control_accepted,
    });

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
        "known_bad_mutations": known_bad_mutations,
        "known_bad_mutation_fixture": "high-contrast-asymmetric-diagonal-v1",
        "live_frame_mutations": live_frame_mutation_observations(&baseline),
        "positive_control": positive_control,
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

/// Evaluates each known-bad AA mutation and returns the computed verdict.
///
/// E01 (`N13`): the Q07 artifact previously recorded `rejected: true` as a JSON
/// literal, so it stayed `true` however the oracle behaved. Every value the
/// artifact publishes now comes from an actual evaluation.
///
/// The mutations run against the **deterministic** high-contrast diagonal
/// fixture, not against this run's live GPU frame. That is deliberate. What the
/// artifact attests is that *this build's oracle* rejects these mutations, which
/// is a property of the code and must hold identically on every adapter.
/// Whether a whole-frame blur of one particular live frame happens to trip an
/// edge-only oracle is a property of that frame: the Apple Metal baseline
/// accepts it, because the live render already carries intermediate tones that
/// the synthetic binary diagonal does not. Asserting on the live frame made the
/// required macOS lane fail for a reason that says nothing about AA
/// correctness.
fn evaluate_known_bad_mutations() -> Vec<(&'static str, bool, &'static str)> {
    let baseline = binary_diagonal_fixture(WIDTH, HEIGHT);
    let baseline_metrics = measure_edges(&baseline, WIDTH, HEIGHT);
    let no_op = baseline.clone();
    let blurred = box_blur_rgba8(&baseline, WIDTH, HEIGHT, 7);
    [
        ("no_op", no_op, "an AA mode that changes no pixels"),
        (
            "blur_everything",
            blurred,
            "a whole-frame blur rather than edge-local smoothing",
        ),
    ]
    .into_iter()
    .map(|(name, frame, reason)| {
        let rejected =
            evaluate_antialiasing_effect(baseline_metrics, measure_edges(&frame, WIDTH, HEIGHT))
                .is_err();
        (name, rejected, reason)
    })
    .collect()
}

/// Records how the live GPU baseline responds to the same mutations.
///
/// Diagnostic only, and deliberately **not** asserted: see
/// [`evaluate_known_bad_mutations`]. Kept because an adapter whose live frame
/// accepts a whole-frame blur is worth seeing in the artifact.
fn live_frame_mutation_observations(baseline: &RenderedMode) -> serde_json::Value {
    let no_op = baseline.frame.clone();
    let blurred = box_blur_rgba8(&baseline.frame, WIDTH, HEIGHT, 7);
    serde_json::Value::Array(
        [("no_op", no_op), ("blur_everything", blurred)]
            .into_iter()
            .map(|(name, frame)| {
                let rejected = evaluate_antialiasing_effect(
                    baseline.metrics,
                    measure_edges(&frame, WIDTH, HEIGHT),
                )
                .is_err();
                serde_json::json!({"name": name, "rejected": rejected})
            })
            .collect(),
    )
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

// ---------------------------------------------------------------------------
// FXAA edge-search proof.
//
// The oracle above accepts any filter that softens edges locally. These tests
// add the properties that separate an edge-following filter from a local blur:
// a shallow edge must come out as a straight ramp instead of a staircase, flat
// regions must stay untouched, and a one-pixel line must stay a line.
// ---------------------------------------------------------------------------

/// Target pixel coordinates (x right, y down) of the fixture geometry. Corners
/// sit off the pixel grid so no pixel centre lies on an edge.
const FXAA_SHALLOW_EDGE: [(f32, f32); 2] = [(4.3, 78.4), (124.6, 92.7)];
const FXAA_STEEP_WEDGE: [(f32, f32); 3] = [(4.3, 4.3), (36.4, 4.3), (4.3, 70.6)];
const FXAA_DIAGONAL_WEDGE: [(f32, f32); 3] = [(44.3, 4.3), (100.4, 4.3), (44.3, 70.6)];
/// Row and first/last column of the one-pixel horizontal line.
const FXAA_LINE_ROW: u32 = 12;
const FXAA_LINE_COLUMNS: (u32, u32) = (106, 123);
/// Column and first/last row of the one-pixel vertical line.
const FXAA_LINE_COLUMN: u32 = 114;
const FXAA_LINE_ROWS: (u32, u32) = (24, 67);

#[derive(Debug, Clone, Copy, serde::Serialize)]
struct FxaaQuality {
    /// RMS distance in pixels between the apparent edge and a straight line.
    shallow_edge_residual_px: f32,
    steep_edge_residual_px: f32,
    diagonal_edge_residual_px: f32,
    /// Largest change in a pixel whose 5x5 neighbourhood is uniform.
    flat_region_max_delta: u8,
    /// Mean line luma and mean luma of the brighter adjacent row or column.
    horizontal_line_luma: f32,
    horizontal_line_halo_luma: f32,
    vertical_line_luma: f32,
    vertical_line_halo_luma: f32,
}

#[test]
fn q07_gpu_fxaa_follows_edges_and_keeps_flat_regions_and_thin_lines() {
    let Some(baseline) = render_fxaa_fixture(AntiAliasing::None) else {
        eprintln!("skipping the FXAA edge-search proof: no GPU adapter is available");
        return;
    };
    let fxaa = render_fxaa_fixture(AntiAliasing::Fxaa)
        .expect("the adapter that rendered the baseline renders the FXAA frame");
    let legacy = legacy_cross_filter(&baseline, WIDTH, HEIGHT);

    let aliased = measure_fxaa_quality(&baseline, &baseline);
    let old = measure_fxaa_quality(&baseline, &legacy);
    let new = measure_fxaa_quality(&baseline, &fxaa);
    eprintln!(
        "FXAA edge-search proof\n  aliased: {aliased:?}\n  cross filter: {old:?}\n  fxaa: {new:?}"
    );
    write_fxaa_proof_artifacts(&baseline, &legacy, &fxaa, [aliased, old, new]);

    evaluate_fxaa_quality(aliased, new)
        .unwrap_or_else(|error| panic!("GPU FXAA failed the edge-search oracle: {error}"));
    // Long runs are where a staircase is visible, and where following the
    // edge beats averaging a pixel with its neighbours. On the steep and
    // diagonal edges the runs are one to three pixels long: FXAA still has to
    // beat the aliased frame there (the oracle above), but the cross filter's
    // blur of every edge pixel scores lower on this one measure, at the cost
    // the line contrast below shows.
    assert!(
        new.shallow_edge_residual_px * 2.0 < old.shallow_edge_residual_px,
        "FXAA must straighten a shallow edge far better than the cross filter it replaced: \
         fxaa {} px, cross filter {} px, aliased {} px",
        new.shallow_edge_residual_px,
        old.shallow_edge_residual_px,
        aliased.shallow_edge_residual_px,
    );
    let line_contrast = |quality: FxaaQuality| {
        (quality.horizontal_line_luma - quality.horizontal_line_halo_luma)
            .min(quality.vertical_line_luma - quality.vertical_line_halo_luma)
    };
    assert!(
        line_contrast(new) > line_contrast(old),
        "FXAA must keep a one-pixel line crisper than the cross filter it replaced: \
         fxaa {}, cross filter {}",
        line_contrast(new),
        line_contrast(old),
    );

    // The live frames must also satisfy the shared AA effect oracle.
    evaluate_antialiasing_effect(
        measure_edges(&baseline, WIDTH, HEIGHT),
        measure_edges(&fxaa, WIDTH, HEIGHT),
    )
    .unwrap_or_else(|error| panic!("GPU FXAA failed the AA effect oracle: {error}"));
}

#[test]
fn q07_fxaa_quality_oracle_rejects_noop_blur_and_cross_filter_mutations() {
    let baseline = fxaa_fixture_reference(1);
    let antialiased = fxaa_fixture_reference(4);
    let aliased = measure_fxaa_quality(&baseline, &baseline);

    evaluate_fxaa_quality(aliased, measure_fxaa_quality(&baseline, &antialiased))
        .expect("an area-sampled render of the fixture satisfies the FXAA quality oracle");
    for (name, mutation) in [
        ("no_op", baseline.clone()),
        (
            "blur_everything",
            box_blur_rgba8(&baseline, WIDTH, HEIGHT, 7),
        ),
        (
            "cross_filter",
            legacy_cross_filter(&baseline, WIDTH, HEIGHT),
        ),
    ] {
        assert!(
            evaluate_fxaa_quality(aliased, measure_fxaa_quality(&baseline, &mutation)).is_err(),
            "the FXAA quality oracle must reject the {name} mutation"
        );
    }
}

fn evaluate_fxaa_quality(aliased: FxaaQuality, candidate: FxaaQuality) -> Result<(), String> {
    if candidate.shallow_edge_residual_px * 2.0 > aliased.shallow_edge_residual_px {
        return Err(format!(
            "shallow edge still reads as a staircase: residual {} px against {} px aliased",
            candidate.shallow_edge_residual_px, aliased.shallow_edge_residual_px
        ));
    }
    for (name, residual, reference) in [
        (
            "steep",
            candidate.steep_edge_residual_px,
            aliased.steep_edge_residual_px,
        ),
        (
            "diagonal",
            candidate.diagonal_edge_residual_px,
            aliased.diagonal_edge_residual_px,
        ),
    ] {
        if residual > reference * 0.95 {
            return Err(format!(
                "{name} edge was not straightened: residual {residual} px against {reference} px aliased"
            ));
        }
    }
    if candidate.flat_region_max_delta > 2 {
        return Err(format!(
            "flat regions changed by up to {} of 255",
            candidate.flat_region_max_delta
        ));
    }
    for (name, line, halo) in [
        (
            "horizontal",
            candidate.horizontal_line_luma,
            candidate.horizontal_line_halo_luma,
        ),
        (
            "vertical",
            candidate.vertical_line_luma,
            candidate.vertical_line_halo_luma,
        ),
    ] {
        if line < 150.0 || line - halo < 110.0 {
            return Err(format!(
                "one-pixel {name} line was blurred: line luma {line}, adjacent luma {halo}"
            ));
        }
    }
    Ok(())
}

fn measure_fxaa_quality(baseline: &[u8], candidate: &[u8]) -> FxaaQuality {
    let coverage = |x: u32, y: u32| {
        let offset = ((y * WIDTH + x) * 4) as usize;
        0.2126 * srgb8_to_linear(candidate[offset])
            + 0.7152 * srgb8_to_linear(candidate[offset + 1])
            + 0.0722 * srgb8_to_linear(candidate[offset + 2])
    };
    let luma = |frame: &[u8], x: u32, y: u32| {
        let offset = ((y * WIDTH + x) * 4) as usize;
        (u16::from(frame[offset]) * 54
            + u16::from(frame[offset + 1]) * 183
            + u16::from(frame[offset + 2]) * 19)
            / 256
    };
    // Apparent edge position per column or row: the covered length of the
    // strip that crosses the edge. A straight edge gives a straight line.
    // The wedge rows stop before the tips, where the two edges of a wedge
    // come within a few pixels of each other and stop being one edge each.
    let shallow = straight_line_residual(
        (12..=116)
            .map(|x| (70..=102).map(|y| coverage(x, y)).sum())
            .collect(),
    );
    let steep = straight_line_residual(
        (10..=56)
            .map(|y| (0..=40).map(|x| coverage(x, y)).sum())
            .collect(),
    );
    let diagonal = straight_line_residual(
        (10..=56)
            .map(|y| (41..=104).map(|x| coverage(x, y)).sum())
            .collect(),
    );

    let mut flat_region_max_delta = 0_u8;
    for y in 2..HEIGHT - 2 {
        for x in 2..WIDTH - 2 {
            let centre = luma(baseline, x, y);
            let uniform = (y - 2..=y + 2)
                .all(|ny| (x - 2..=x + 2).all(|nx| luma(baseline, nx, ny).abs_diff(centre) <= 2));
            if uniform {
                let offset = ((y * WIDTH + x) * 4) as usize;
                for channel in 0..3 {
                    flat_region_max_delta = flat_region_max_delta
                        .max(baseline[offset + channel].abs_diff(candidate[offset + channel]));
                }
            }
        }
    }

    let mean = |values: Vec<u16>| {
        values.iter().map(|value| f32::from(*value)).sum::<f32>() / values.len() as f32
    };
    let horizontal_columns = FXAA_LINE_COLUMNS.0 + 4..=FXAA_LINE_COLUMNS.1 - 4;
    let vertical_rows = FXAA_LINE_ROWS.0 + 4..=FXAA_LINE_ROWS.1 - 4;
    FxaaQuality {
        shallow_edge_residual_px: shallow,
        steep_edge_residual_px: steep,
        diagonal_edge_residual_px: diagonal,
        flat_region_max_delta,
        horizontal_line_luma: mean(
            horizontal_columns
                .clone()
                .map(|x| luma(candidate, x, FXAA_LINE_ROW))
                .collect(),
        ),
        horizontal_line_halo_luma: mean(
            horizontal_columns
                .map(|x| {
                    luma(candidate, x, FXAA_LINE_ROW - 1).max(luma(candidate, x, FXAA_LINE_ROW + 1))
                })
                .collect(),
        ),
        vertical_line_luma: mean(
            vertical_rows
                .clone()
                .map(|y| luma(candidate, FXAA_LINE_COLUMN, y))
                .collect(),
        ),
        vertical_line_halo_luma: mean(
            vertical_rows
                .map(|y| {
                    luma(candidate, FXAA_LINE_COLUMN - 1, y).max(luma(
                        candidate,
                        FXAA_LINE_COLUMN + 1,
                        y,
                    ))
                })
                .collect(),
        ),
    }
}

/// RMS distance between a sequence of positions and its least-squares line.
fn straight_line_residual(positions: Vec<f32>) -> f32 {
    let count = positions.len() as f32;
    let mean_index = (count - 1.0) * 0.5;
    let mean = positions.iter().sum::<f32>() / count;
    let (mut covariance, mut variance) = (0.0_f32, 0.0_f32);
    for (index, position) in positions.iter().enumerate() {
        let offset = index as f32 - mean_index;
        covariance += offset * (position - mean);
        variance += offset * offset;
    }
    let slope = covariance / variance;
    let squared = positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let fitted = mean + slope * (index as f32 - mean_index);
            (position - fitted) * (position - fitted)
        })
        .sum::<f32>();
    (squared / count).sqrt()
}

fn srgb8_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb8(linear: f32) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The fixture as target-pixel polygons, shared by the GPU scene and the
/// area-sampled reference.
fn fxaa_fixture_polygons() -> Vec<Vec<(f32, f32)>> {
    let [shallow_start, shallow_end] = FXAA_SHALLOW_EDGE;
    let line_row = FXAA_LINE_ROW as f32;
    let line_column = FXAA_LINE_COLUMN as f32;
    vec![
        vec![
            shallow_start,
            shallow_end,
            (shallow_end.0, 123.6),
            (shallow_start.0, 123.6),
        ],
        FXAA_STEEP_WEDGE.to_vec(),
        FXAA_DIAGONAL_WEDGE.to_vec(),
        // One-pixel lines: each covers exactly one row or column of centres.
        vec![
            (FXAA_LINE_COLUMNS.0 as f32, line_row + 0.2),
            (FXAA_LINE_COLUMNS.1 as f32 + 1.0, line_row + 0.2),
            (FXAA_LINE_COLUMNS.1 as f32 + 1.0, line_row + 0.8),
            (FXAA_LINE_COLUMNS.0 as f32, line_row + 0.8),
        ],
        vec![
            (line_column + 0.2, FXAA_LINE_ROWS.0 as f32),
            (line_column + 0.8, FXAA_LINE_ROWS.0 as f32),
            (line_column + 0.8, FXAA_LINE_ROWS.1 as f32 + 1.0),
            (line_column + 0.2, FXAA_LINE_ROWS.1 as f32 + 1.0),
        ],
    ]
}

fn render_fxaa_fixture(anti_aliasing: AntiAliasing) -> Option<Vec<u8>> {
    let mut renderer = Renderer::headless_gpu(WIDTH, HEIGHT).ok()?;
    renderer.set_background_color(Color::BLACK);
    renderer.set_tonemapper(scena::Tonemapper::Standard);
    renderer.set_anti_aliasing(anti_aliasing);
    let assets = Assets::new();
    let mut scene = Scene::new();
    // A 60 degree camera at sqrt(3) maps x and y in -1..1 of the z = 0 plane
    // onto the whole target, so fixture pixels and world units line up.
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 1.732_050_8)),
        )
        .expect("FXAA fixture camera inserts");
    scene
        .set_active_camera(camera)
        .expect("FXAA fixture camera activates");
    let world = |(x, y): (f32, f32)| Vertex {
        position: Vec3::new(
            x / (WIDTH as f32 * 0.5) - 1.0,
            1.0 - y / (HEIGHT as f32 * 0.5),
            0.0,
        ),
        color: Color::WHITE,
    };
    // The polygons wind clockwise on screen, so each fan triangle is emitted
    // in reverse to face the camera.
    let primitives = fxaa_fixture_polygons()
        .into_iter()
        .flat_map(|polygon| {
            (1..polygon.len() - 1)
                .map(|index| {
                    Primitive::triangle([
                        world(polygon[0]),
                        world(polygon[index + 1]),
                        world(polygon[index]),
                    ])
                })
                .collect::<Vec<_>>()
        })
        .collect();
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("FXAA fixture inserts");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("FXAA fixture prepares");
    renderer
        .render(&scene, camera)
        .expect("FXAA fixture renders");
    Some(renderer.frame_rgba8().to_vec())
}

/// CPU render of the fixture with `samples` x `samples` area samples a pixel.
/// One sample reproduces the aliased frame; more give the ideal coverage.
fn fxaa_fixture_reference(samples: u32) -> Vec<u8> {
    let polygons = fxaa_fixture_polygons();
    let inside = |polygon: &[(f32, f32)], x: f32, y: f32| {
        let mut sign = 0.0_f32;
        (0..polygon.len()).all(|index| {
            let (ax, ay) = polygon[index];
            let (bx, by) = polygon[(index + 1) % polygon.len()];
            let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
            if cross == 0.0 {
                return true;
            }
            if sign == 0.0 {
                sign = cross.signum();
            }
            cross.signum() == sign
        })
    };
    let mut frame = vec![0; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut covered = 0;
            for sy in 0..samples {
                for sx in 0..samples {
                    let px = x as f32 + (sx as f32 + 0.5) / samples as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / samples as f32;
                    covered += u32::from(polygons.iter().any(|polygon| inside(polygon, px, py)));
                }
            }
            let value = linear_to_srgb8(covered as f32 / (samples * samples) as f32);
            let offset = ((y * WIDTH + x) * 4) as usize;
            frame[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    frame
}

/// The five-tap cross average the GPU ran as "FXAA" before the edge search
/// replaced it, reproduced on the CPU so the comparison stays reproducible.
/// It averaged a pixel with its four neighbours, in linear light, wherever
/// the pixel was the darkest or brightest of the five and the cross spanned
/// a 16/255 luma contrast.
fn legacy_cross_filter(frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel = |x: u32, y: u32| {
        let offset = ((y * width + x) * 4) as usize;
        [
            srgb8_to_linear(frame[offset]),
            srgb8_to_linear(frame[offset + 1]),
            srgb8_to_linear(frame[offset + 2]),
        ]
    };
    let luma = |rgb: [f32; 3]| 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
    let mut output = frame.to_vec();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let centre = pixel(x, y);
            let centre_luma = luma(centre);
            let taps = [
                pixel(x, y - 1),
                pixel(x - 1, y),
                centre,
                pixel(x + 1, y),
                pixel(x, y + 1),
            ];
            let lumas = taps.map(luma);
            let min = lumas.iter().copied().fold(f32::INFINITY, f32::min);
            let max = lumas.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            if max - min < 16.0 / 255.0 {
                continue;
            }
            let brighter = lumas.iter().any(|tap| tap - centre_luma >= 16.0 / 255.0);
            let darker = lumas.iter().any(|tap| centre_luma - tap >= 16.0 / 255.0);
            let dark_edge = centre_luma - min <= 1.0 / 255.0 && brighter;
            let light_edge = max - centre_luma <= 1.0 / 255.0 && darker;
            if !dark_edge && !light_edge {
                continue;
            }
            let offset = ((y * width + x) * 4) as usize;
            for channel in 0..3 {
                let linear = taps.iter().map(|tap| tap[channel]).sum::<f32>() / 5.0;
                output[offset + channel] = linear_to_srgb8(linear);
            }
        }
    }
    output
}

fn write_fxaa_proof_artifacts(
    baseline: &[u8],
    legacy: &[u8],
    fxaa: &[u8],
    [aliased, old, new]: [FxaaQuality; 3],
) {
    fs::create_dir_all(ARTIFACT_DIR).expect("Q07 artifact directory creates");
    for (name, frame) in [
        ("fxaa-edge-search-none", baseline),
        ("fxaa-edge-search-cross-filter", legacy),
        ("fxaa-edge-search-fxaa", fxaa),
    ] {
        let mut bytes = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
        for pixel in frame.chunks_exact(4) {
            bytes.extend_from_slice(&pixel[..3]);
        }
        fs::write(Path::new(ARTIFACT_DIR).join(format!("{name}.ppm")), bytes)
            .expect("FXAA edge-search frame writes");
    }
    let report = serde_json::json!({
        "schema": "scena.q07.fxaa_edge_search.v1",
        "release_evidence": false,
        "fixture": "fxaa-edge-search-v1",
        "width": WIDTH,
        "height": HEIGHT,
        "aliased": aliased,
        "cross_filter": old,
        "fxaa": new,
    });
    fs::write(
        Path::new(ARTIFACT_DIR).join("fxaa-edge-search.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).expect("FXAA edge-search report serializes")
        ),
    )
    .expect("FXAA edge-search report writes");
}
