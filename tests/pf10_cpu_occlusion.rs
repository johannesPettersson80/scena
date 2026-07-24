#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use scena::{Color, Primitive, Renderer, Scene, Transform, Vec3, Vertex};
use sha2::{Digest, Sha256};

const BENCHMARK_SAMPLE_COUNT: usize = 100;

fn pf10_release_evidence_ready(measurement_complete: bool, commit: &str) -> bool {
    measurement_complete
        && commit.len() == 40
        && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        && commit.bytes().any(|byte| byte != b'0')
}

#[test]
fn pf10_release_evidence_requires_exact_source_commit() {
    assert!(pf10_release_evidence_ready(
        true,
        "0123456789abcdef0123456789abcdef01234567"
    ));
    assert!(!pf10_release_evidence_ready(true, "local-checkout"));
    assert!(!pf10_release_evidence_ready(
        true,
        "0000000000000000000000000000000000000000"
    ));
    assert!(!pf10_release_evidence_ready(
        false,
        "0123456789abcdef0123456789abcdef01234567"
    ));
}

#[test]
fn pf10_cpu_occlusion_can_be_disabled_without_changing_scene_output() {
    for size in [96, 512] {
        let (mut scene, camera) = dense_occlusion_scene(64);

        let mut enabled = Renderer::headless(size, size).expect("enabled renderer builds");
        enabled.set_cpu_occlusion_culling(true);
        enabled
            .prepare(&mut scene)
            .expect("enabled prepare succeeds");
        let enabled_outcome = enabled
            .render(&scene, camera)
            .expect("enabled render succeeds");
        let enabled_frame = enabled.frame_rgba8().to_vec();

        let mut disabled = Renderer::headless(size, size).expect("disabled renderer builds");
        disabled.set_cpu_occlusion_culling(false);
        disabled
            .prepare(&mut scene)
            .expect("disabled prepare succeeds");
        let disabled_outcome = disabled
            .render(&scene, camera)
            .expect("disabled render succeeds");

        assert!(enabled.cpu_occlusion_culling());
        assert!(!disabled.cpu_occlusion_culling());
        assert!(enabled.stats().culled_objects > 0);
        assert_eq!(disabled.stats().culled_objects, 0);
        assert!(enabled_outcome.draw_calls < disabled_outcome.draw_calls);
        assert_eq!(
            enabled_frame,
            disabled.frame_rgba8(),
            "occlusion enabled/disabled output must match at {size}x{size}",
        );
    }
}

#[test]
fn pf10_cpu_occlusion_benchmark_artifact() {
    let artifact_dir = PathBuf::from("target/gate-artifacts/pf10");
    fs::create_dir_all(&artifact_dir).expect("PF10 artifact directory creates");
    let artifact_path = artifact_dir.join("cpu-occlusion-prepass-benefit.json");
    if std::env::var_os("SCENA_RUN_PF10_OCCLUSION_BENCHMARK").is_none() {
        write_pf10_artifact(
            &artifact_path,
            &serde_json::json!({
                "schema": "scena.performance_workload.v1",
                "id": "cpu-occlusion-prepass-benefit",
                "status": "required",
                "measurement_evidence": false,
                "release_evidence": false,
                "reason": "SCENA_RUN_PF10_OCCLUSION_BENCHMARK is not set",
                "run_hint": "SCENA_RUN_PF10_OCCLUSION_BENCHMARK=1 cargo test --profile perf-test --test pf10_cpu_occlusion pf10_cpu_occlusion_benchmark_artifact -- --nocapture",
            }),
        )
        .expect("required artifact writes");
        return;
    }

    let rows = [
        benchmark_scene("dense-32-below-threshold", dense_occlusion_scene(32)),
        benchmark_scene("dense-128", dense_occlusion_scene(128)),
        benchmark_scene("sparse-128", sparse_scene(128)),
    ];
    let commit = current_commit_label();
    let measurement_evidence = rows.iter().all(|row| {
        row["disabled"]["sample_count"] == BENCHMARK_SAMPLE_COUNT as u64
            && row["enabled"]["sample_count"] == BENCHMARK_SAMPLE_COUNT as u64
    });
    let release_evidence = pf10_release_evidence_ready(measurement_evidence, &commit);
    let artifact = serde_json::json!({
        "schema": "scena.performance_workload.v1",
        "id": "cpu-occlusion-prepass-benefit",
        "status": "measured",
        "measurement_evidence": measurement_evidence,
        "release_evidence": release_evidence,
        "release_provenance": if release_evidence {
            serde_json::json!({"status": "exact-commit", "commit_sha": commit.clone()})
        } else {
            serde_json::json!({
                "status": "unavailable",
                "reason": "the measured comparison is not bound to one exact source commit",
            })
        },
        "sample_count": BENCHMARK_SAMPLE_COUNT,
        "warmup_pairs": 10,
        "performance_environment": pf10_performance_environment(&commit),
        "rows": rows,
        "policy": {
            "gpu": "disabled",
            "cpu_min_primitives": 64,
            "decision": "enabled by default only for CPU scenes with at least 64 primitives and projected tile overlap; explicit opt-out remains available",
            "comparison": "enabled and disabled samples are interleaved on the same builder, source snapshot, profile, and fixture",
            "claim": "the artifact reports the observed distributions; it does not promise an absolute speedup on other hosts",
        },
    });
    write_pf10_artifact(&artifact_path, &artifact).expect("measured artifact writes");
    println!("{}", serde_json::to_string_pretty(&artifact).unwrap());
}

fn pf10_performance_environment(commit: &str) -> serde_json::Value {
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
        "profile": profile,
        "optimized": optimized,
        "toolchain": command_output("rustc", &["-Vv"]),
        "cpu": cpu_model(),
        "gpu": {"status": "not-applicable", "reason": "PF10 measures the CPU renderer"},
        "driver": {"status": "not-applicable", "reason": "PF10 measures the CPU renderer"},
        "sample_count": BENCHMARK_SAMPLE_COUNT,
        "warmup_pairs": 10,
        "commit": commit,
        "command": command,
        "sidecar_cache_state": "not-applicable",
        "confidence": {
            "status": "distribution-only",
            "reason": "no parametric confidence interval is claimed; compare the complete interleaved same-host distributions",
        },
        "distribution": {
            "percentile_method": "nearest-rank",
            "reported": ["sample_count", "min_ms", "p50_ms", "p95_ms", "max_ms", "population_stddev_ms"],
        },
    })
}

fn write_pf10_artifact(path: &Path, artifact: &serde_json::Value) -> Result<(), String> {
    let mut artifact = artifact.clone();
    let object = artifact
        .as_object_mut()
        .ok_or_else(|| "PF10 artifact must be an object".to_owned())?;
    object.insert(
        "producer".to_owned(),
        serde_json::json!("cargo test --test pf10_cpu_occlusion"),
    );
    object.insert(
        "commit_sha".to_owned(),
        serde_json::json!(current_commit_label()),
    );
    object.insert(
        "timestamp_unix_seconds".to_owned(),
        serde_json::json!(current_timestamp_unix_seconds()),
    );
    object.insert(
        "source_checksums".to_owned(),
        serde_json::json!([
            source_checksum("Cargo.lock")?,
            source_checksum("tests/pf10_cpu_occlusion.rs")?,
        ]),
    );
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| format!("PF10 artifact serialization failed: {error}"))?;
    fs::write(path, format!("{body}\n"))
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn source_checksum(relative: &str) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let bytes =
        fs::read(&path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    Ok(serde_json::json!({
        "path": relative,
        "sha256": Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    }))
}

fn current_commit_label() -> String {
    std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local-checkout".to_owned())
}

fn current_timestamp_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|output| !output.is_empty())
        .unwrap_or_else(|| "unavailable".to_owned())
}

fn cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.split_once(':')
                    .filter(|(key, _)| key.trim() == "model name")
                    .map(|(_, value)| value.trim().to_owned())
            })
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_owned())
}

fn benchmark_scene(id: &str, (mut scene, camera): (Scene, scena::CameraKey)) -> serde_json::Value {
    let (disabled, enabled) = sample_scene_pair(&mut scene, camera);
    serde_json::json!({
        "id": id,
        "disabled": distribution(&disabled.duration_ms),
        "enabled": distribution(&enabled.duration_ms),
        "disabled_draw_calls": disabled.draw_calls,
        "enabled_draw_calls": enabled.draw_calls,
        "enabled_culled_objects": enabled.culled_objects,
        "p50_change_percent": percent_change(
            percentile(&disabled.duration_ms, 50),
            percentile(&enabled.duration_ms, 50),
        ),
        "p95_change_percent": percent_change(
            percentile(&disabled.duration_ms, 95),
            percentile(&enabled.duration_ms, 95),
        ),
    })
}

struct Samples {
    duration_ms: Vec<f64>,
    draw_calls: u64,
    culled_objects: u64,
}

fn sample_scene_pair(scene: &mut Scene, camera: scena::CameraKey) -> (Samples, Samples) {
    for index in 0..10 {
        let order = if index % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for enabled in order {
            let _ = run_sample(scene, camera, enabled);
        }
    }

    let mut disabled = Samples {
        duration_ms: Vec::with_capacity(BENCHMARK_SAMPLE_COUNT),
        draw_calls: 0,
        culled_objects: 0,
    };
    let mut enabled = Samples {
        duration_ms: Vec::with_capacity(BENCHMARK_SAMPLE_COUNT),
        draw_calls: 0,
        culled_objects: 0,
    };
    for index in 0..BENCHMARK_SAMPLE_COUNT {
        let order = if index % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for is_enabled in order {
            let (duration_ms, draw_calls, culled_objects) = run_sample(scene, camera, is_enabled);
            let samples = if is_enabled {
                &mut enabled
            } else {
                &mut disabled
            };
            samples.duration_ms.push(duration_ms);
            if samples.duration_ms.len() == 1 {
                samples.draw_calls = draw_calls;
                samples.culled_objects = culled_objects;
            } else {
                assert_eq!(samples.draw_calls, draw_calls);
                assert_eq!(samples.culled_objects, culled_objects);
            }
        }
    }
    (disabled, enabled)
}

fn run_sample(scene: &mut Scene, camera: scena::CameraKey, enabled: bool) -> (f64, u64, u64) {
    let mut renderer = Renderer::headless(128, 128).expect("benchmark renderer builds");
    renderer.set_cpu_occlusion_culling(enabled);
    let started = Instant::now();
    renderer.prepare(scene).expect("benchmark prepare succeeds");
    let outcome = renderer
        .render(scene, camera)
        .expect("benchmark render succeeds");
    (
        started.elapsed().as_secs_f64() * 1000.0,
        outcome.draw_calls,
        renderer.stats().culled_objects,
    )
}

fn distribution(samples: &[f64]) -> serde_json::Value {
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|sample| {
            let delta = sample - mean;
            delta * delta
        })
        .sum::<f64>()
        / samples.len() as f64;
    serde_json::json!({
        "sample_count": samples.len(),
        "min_ms": samples.iter().copied().fold(f64::INFINITY, f64::min),
        "p50_ms": percentile(samples, 50),
        "p95_ms": percentile(samples, 95),
        "max_ms": samples.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "population_stddev_ms": variance.sqrt(),
    })
}

fn percentile(samples: &[f64], percentile: usize) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = percentile.saturating_mul(sorted.len()).div_ceil(100);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn percent_change(before: f64, after: f64) -> f64 {
    (after - before) / before * 100.0
}

fn dense_occlusion_scene(primitive_count: usize) -> (Scene, scena::CameraKey) {
    let primitives = (0..primitive_count)
        .map(|index| {
            triangle(
                [
                    Vec3::new(-0.75, -0.75, -(index as f32) * 0.002),
                    Vec3::new(0.75, -0.75, -(index as f32) * 0.002),
                    Vec3::new(0.0, 0.75, -(index as f32) * 0.002),
                ],
                Color::WHITE,
            )
        })
        .collect();
    scene_with_primitives(primitives)
}

fn sparse_scene(primitive_count: usize) -> (Scene, scena::CameraKey) {
    let columns = 16;
    let rows = primitive_count.div_ceil(columns);
    let primitives = (0..primitive_count)
        .map(|index| {
            let column = index % columns;
            let row = index / columns;
            let x = -0.9 + (column as f32 + 0.5) * 1.8 / columns as f32;
            let y = -0.9 + (row as f32 + 0.5) * 1.8 / rows as f32;
            let radius = (1.6 / columns as f32).min(1.6 / rows as f32) * 0.35;
            triangle(
                [
                    Vec3::new(x - radius, y - radius, 0.0),
                    Vec3::new(x + radius, y - radius, 0.0),
                    Vec3::new(x, y + radius, 0.0),
                ],
                Color::WHITE,
            )
        })
        .collect();
    scene_with_primitives(primitives)
}

fn scene_with_primitives(primitives: Vec<Primitive>) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("benchmark renderable inserts");
    (scene, camera)
}

fn triangle(points: [Vec3; 3], color: Color) -> Primitive {
    Primitive::triangle(points.map(|position| Vertex { position, color }))
}
