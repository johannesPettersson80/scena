#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use scena::{Primitive, RenderReadbackMode, Renderer, Scene, SurfaceEvent, Transform};

#[test]
fn full_gpu_reprepare_reuses_the_device_triangle_shader_module() {
    let mut renderer = Renderer::headless_gpu(32, 32)
        .expect("P01 focused proof requires the remote builder GPU adapter");
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("first P01 triangle inserts");
    let camera = scene.add_default_camera().expect("P01 camera inserts");

    let first = renderer
        .prepare_profiled(&mut scene)
        .expect("first full GPU prepare succeeds");
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("first P01 render succeeds");
    let first_pixels = renderer.frame_rgba8().to_vec();

    // Adding an identical coplanar primitive forces a structural full prepare
    // while preserving the rendered image. The device-owned shader module must
    // survive the prepared-resource rebuild and feed every compatible pipeline.
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("second P01 triangle inserts");
    let second = renderer
        .prepare_profiled(&mut scene)
        .expect("second full GPU prepare succeeds");
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("second P01 render succeeds");

    assert_eq!(
        first_pixels,
        renderer.frame_rgba8(),
        "sharing the shader module must preserve rendered output"
    );
    assert!(
        first.gpu_shader_module_creations > second.gpu_shader_module_creations,
        "the warm full prepare must compile fewer shader modules: cold={first:?}, warm={second:?}"
    );
    assert_eq!(first.gpu_triangle_shader_cache_hits, 0);
    assert_eq!(first.gpu_triangle_shader_cache_misses, 1);
    assert_eq!(second.gpu_triangle_shader_cache_hits, 1);
    assert_eq!(second.gpu_triangle_shader_cache_misses, 0);

    if std::env::var_os("SCENA_RUN_CONTROLLED_P01_BENCHMARK").is_some() {
        let adapter = renderer
            .gpu_adapter_report()
            .expect("P01 benchmark records its adapter");
        drop(renderer);
        let sample_count = 5;
        let cold_ms = cold_prepare_samples(sample_count);
        let warm_ms = warm_reprepare_samples(sample_count);
        let cold_p95_ms = percentile_95(&cold_ms);
        let warm_p95_ms = percentile_95(&warm_ms);
        let improvement_percent = ((cold_p95_ms - warm_p95_ms) / cold_p95_ms) * 100.0;
        let hardware_adapter = matches!(
            adapter.device_type.as_str(),
            "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu"
        );
        let status = if !hardware_adapter {
            "inconclusive-software-adapter"
        } else if improvement_percent >= 10.0 {
            "passed"
        } else {
            "failed"
        };
        let artifact = serde_json::json!({
            "schema": "scena.p01.shader_module_cache.v1",
            "status": status,
            "commit_sha": current_commit_label(),
            "timestamp_unix_seconds": current_timestamp_unix_seconds(),
            "command": std::env::var("SCENA_HARDWARE_PROOF_COMMAND").unwrap_or_else(|_| {
                "cargo test --test p01_shader_module_cache full_gpu_reprepare_reuses_the_device_triangle_shader_module -- --exact --nocapture --test-threads=1".to_owned()
            }),
            "adapter": adapter,
            "hardware_adapter": hardware_adapter,
            "sample_count": sample_count,
            "cold_full_prepare_ms": cold_ms,
            "warm_full_prepare_ms": warm_ms,
            "cold_p95_ms": cold_p95_ms,
            "warm_p95_ms": warm_p95_ms,
            "p95_improvement_percent": improvement_percent,
            "minimum_material_improvement_percent": 10.0,
            "cold_shader_module_creations": first.gpu_shader_module_creations,
            "warm_shader_module_creations": second.gpu_shader_module_creations,
            "cold_triangle_shader_cache_misses": first.gpu_triangle_shader_cache_misses,
            "warm_triangle_shader_cache_hits": second.gpu_triangle_shader_cache_hits,
            "method": "cold samples use a fresh device cache; warm samples force full resource rebuilds on one device while retaining the shader module",
        });
        let path = Path::new("target/gate-artifacts/p01-shader-module-cache.json");
        fs::create_dir_all(path.parent().expect("P01 artifact has a parent"))
            .expect("P01 artifact directory creates");
        fs::write(
            path,
            serde_json::to_vec_pretty(&artifact).expect("P01 artifact serializes"),
        )
        .expect("P01 artifact writes");
        if hardware_adapter {
            assert!(
                improvement_percent >= 10.0,
                "controlled P01 hardware p95 improvement must be material: {artifact:#}"
            );
        }
    }
}

fn current_commit_label() -> String {
    std::env::var("SCENA_RELEASE_COMMIT")
        .or_else(|_| std::env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local-checkout".to_owned())
}

fn current_timestamp_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn cold_prepare_samples(sample_count: usize) -> Vec<f64> {
    (0..sample_count)
        .map(|_| {
            let (mut renderer, mut scene) = benchmark_fixture(32);
            let start = Instant::now();
            let metrics = renderer
                .prepare_profiled(&mut scene)
                .expect("cold P01 prepare succeeds");
            let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
            assert_eq!(metrics.gpu_triangle_shader_cache_hits, 0);
            assert_eq!(metrics.gpu_triangle_shader_cache_misses, 1);
            elapsed_ms
        })
        .collect()
}

fn warm_reprepare_samples(sample_count: usize) -> Vec<f64> {
    let (mut renderer, mut scene) = benchmark_fixture(32);
    renderer
        .prepare(&mut scene)
        .expect("P01 benchmark warm-up prepare succeeds");
    (0..sample_count)
        .map(|sample| {
            let width = if sample % 2 == 0 { 33 } else { 32 };
            renderer
                .handle_surface_event(SurfaceEvent::Resize { width, height: 32 })
                .expect("P01 benchmark target resize succeeds");
            let start = Instant::now();
            let metrics = renderer
                .prepare_profiled(&mut scene)
                .expect("warm P01 full prepare succeeds");
            let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
            assert_eq!(metrics.gpu_triangle_shader_cache_hits, 1);
            assert_eq!(metrics.gpu_triangle_shader_cache_misses, 0);
            elapsed_ms
        })
        .collect()
}

fn benchmark_fixture(width: u32) -> (Renderer, Scene) {
    let renderer =
        Renderer::headless_gpu(width, 32).expect("controlled P01 benchmark requires a GPU adapter");
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("P01 benchmark triangle inserts");
    scene
        .add_default_camera()
        .expect("P01 benchmark camera inserts");
    (renderer, scene)
}

fn percentile_95(samples: &[f64]) -> f64 {
    assert!(!samples.is_empty());
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[index]
}
