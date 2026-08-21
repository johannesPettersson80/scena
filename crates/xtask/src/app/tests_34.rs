use crate::app::prelude::*;

#[test]
fn pf00_performance_truth_rejects_fabricated_and_unregistered_evidence() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf00-performance-truth");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in [
        ".github/workflows",
        "docs/specs",
        "src/render/prepare",
        "tests/fixtures",
        "tests",
    ] {
        fs::create_dir_all(fixture_root.join(directory)).expect("PF00 fixture directory");
    }
    fs::write(
        fixture_root.join("tests/m5_release.rs"),
        "json!({ \"allocation_bytes\": 0 })\n",
    )
    .expect("weak M5 benchmark fixture writes");
    fs::write(
        fixture_root.join("tests/m9_platform_release.rs"),
        "ALLOCATION_COUNT p95_frame_ms prepare_ms regression_threshold_percent\n",
    )
    .expect("weak M9 benchmark fixture writes");
    fs::write(
        fixture_root.join("tests/fixtures/m9-baselines.json"),
        serde_json::to_string(&json!({
            "schema": "scena.m9.benchmark_baselines.v1",
            "rows": [{"scene": "weak", "allowed_regression_percent": 100.0}]
        }))
        .expect("weak baseline serializes"),
    )
    .expect("weak baseline writes");
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            "cargo test --release --test m9_platform_release\n",
        )
        .expect("weak workflow writes");
    }
    fs::write(
        fixture_root.join(".github/workflows/hardware-gpu.yml"),
        "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]\n",
    )
    .expect("weak hardware workflow writes");

    let mut findings = Vec::new();
    check_pf00_performance_truth_contracts(&fixture_root, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "PF00-PERFORMANCE-TRUTH"),
        "fabricated bytes, 100% baselines, missing distributions, and unregistered rows must fail: {findings:?}"
    );

    fs::write(
        fixture_root.join("tests/m5_release.rs"),
        "json!({ \"allocation_bytes\": { \"status\": \"unavailable\", \"reason\": \"not instrumented\" } })\n",
    )
    .expect("strong M5 benchmark fixture writes");
    fs::write(
        fixture_root.join("tests/m9_platform_release.rs"),
        "ALLOCATION_BYTES p50_allocated_bytes_per_frame p95_allocated_bytes_per_frame max_allocated_bytes_per_frame prepare_sample_count p50_prepare_ms p95_prepare_ms max_prepare_ms performance_environment SCENA_BENCHMARK_PROFILE SCENA_RUN_PF00_BENCHMARK SCENA_REAGGREGATE_PF00 sidecar_cache_state distribution-only pick_with_assets_profiled benchmark_profiled_native_capture_workload RenderReadbackMode::PresentOnly RenderReadbackMode::Synchronous blocking_waits gpu_bind_group_creations async_readback_submissions peak_readbacks_in_flight asset_storage_lock_acquisitions precompute_environment_sidecar_profiled source_texture_samples brdf_integration_samples benchmark_profiled_gpu_output_settings_workload first_prepared_render_ms PF00_REQUIRED_HARDWARE_ARTIFACTS validate_pf00_complete_hardware_summary load_pf00_complete_hardware_proof performance_environment_metadata_with_renderer pf00_release_evidence_ready classify_pf00_workload_provenance validate_pf00_existing_measurement_artifact fn m9_pf00_representative_performance_artifact() {}\n",
    )
    .expect("strong M9 benchmark fixture writes");
    fs::write(
        fixture_root.join("tests/pf10_cpu_occlusion.rs"),
        "SCENA_RUN_PF10_OCCLUSION_BENCHMARK sample_scene_pair cpu-occlusion-prepass-benefit pf10_release_evidence_ready measurement_evidence release_provenance source_checksums performance_environment\nfn pf10_cpu_occlusion_benchmark_artifact() {}\n",
    )
    .expect("strong PF10 benchmark fixture writes");
    fs::write(
        fixture_root.join("src/assets.rs"),
        "storage_lock_acquisitions: Arc<AtomicU64>\npub fn storage_lock_acquisitions() {}\nfetch_add(1, Ordering::Relaxed)\n",
    )
    .expect("strong asset-lock telemetry fixture writes");
    fs::write(
        fixture_root.join("src/render/prepare/environment_baker.rs"),
        "pub struct EnvironmentBakeMetrics { source_texture_samples: u64, brdf_integration_samples: u64, output_bytes_written: u64 }\nfn bake_environment_ibl_profiled() {}\n",
    )
    .expect("strong environment profiling fixture writes");
    fs::write(
        fixture_root.join("tests/fixtures/m9-baselines.json"),
        serde_json::to_string(&json!({
            "schema": "scena.m9.benchmark_baselines.v1",
            "rows": [{
                "scene": "strong",
                "allowed_regression_percent": 5.0,
                "p95_prepare_ms": 1.0,
                "max_allocated_bytes_per_frame": 1024
            }]
        }))
        .expect("strong baseline serializes"),
    )
    .expect("strong baseline writes");
    let workload_ids = [
        "one-node-transform-prepare-render",
        "shadow-scaling-directional-area",
        "pick-100k-triangle-deformed-undeformed",
        "cpu-texture-bake-qualifying-nonqualifying",
        "tangent-generation-static-deformed",
        "animation-many-channels-keyframes-weights",
        "environment-bake-cold-sidecar-hit",
        "native-present-capture-sync-async",
        "gpu-first-render-output-settings",
        "draw-uniform-indexing-many-unique-transforms",
        "cpu-occlusion-prepass-benefit",
    ];
    let workloads = workload_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "producer": if *id == "cpu-occlusion-prepass-benefit" {
                    "tests/pf10_cpu_occlusion.rs::pf10_cpu_occlusion_benchmark_artifact"
                } else {
                    "tests/m9_platform_release.rs::m9_pf00_representative_performance_artifact"
                },
                "artifact": "target/gate-artifacts/pf00/test.json",
                "artifact_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "status": "measured-nonrelease",
                "measurement_evidence": true,
                "release_evidence": false,
                "required_distributions": ["p50", "p95"],
                "required_counters": ["allocation_bytes"]
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        fixture_root.join("docs/specs/performance-evidence.json"),
        serde_json::to_string(&json!({
            "current_measurement_bundle": {
                "artifact": "target/gate-artifacts/pf00/performance-evidence.json",
                "artifact_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "status": "measured",
                "measurement_evidence": true,
                "hardware_evidence": true,
                "release_evidence": false
            },
            "workloads": workloads
        }))
            .expect("strong evidence registry serializes"),
    )
    .expect("strong evidence registry writes");
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            "SCENA_RUN_DEDICATED_4K_BENCHMARK\nSCENA_BENCHMARK_PROFILE: perf-test\ncargo test --profile perf-test --test m9_platform_release\n",
        )
        .expect("strong workflow writes");
    }
    fs::write(
        fixture_root.join(".github/workflows/hardware-gpu.yml"),
        "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]\nif: inputs.run_performance\nSCENA_RUN_PF00_BENCHMARK: \"1\"\nSCENA_BENCHMARK_PROFILE: perf-test\nSCENA_BENCHMARK_COMMAND: exact\nm9_pf00_representative_performance_artifact\nwindows_complete_hardware_proof_validation.js\nwindows-complete-hardware-proof/proof-summary.json\n",
    )
    .expect("strong hardware workflow writes");
    findings.clear();
    check_pf00_performance_truth_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let registry = fixture_root.join("docs/specs/performance-evidence.json");
    let registry_text = fs::read_to_string(&registry).expect("performance registry reads");
    let mut registry_value: Value =
        serde_json::from_str(&registry_text).expect("performance registry parses");
    registry_value["workloads"][0]["release_evidence"] = json!(true);
    fs::write(
        &registry,
        serde_json::to_string(&registry_value).expect("mutated registry serializes"),
    )
    .expect("mutated registry writes");
    findings.clear();
    check_pf00_performance_truth_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF00-PERFORMANCE-TRUTH" && finding.message.contains("release evidence")
        }),
        "a workload cannot claim release evidence when the current bundle is nonrelease: {findings:?}"
    );
    fs::write(&registry, registry_text).expect("performance registry restores");

    fs::write(
        fixture_root.join("src/assets.rs"),
        "storage_lock_acquisitions: Arc<AtomicU64>\npub fn storage_lock_acquisitions() {}\n",
    )
    .expect("mutated asset-lock telemetry fixture writes");
    findings.clear();
    check_pf00_performance_truth_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF00-PERFORMANCE-TRUTH"
                && finding.message.contains("fetch_add(1, Ordering::Relaxed)")
        }),
        "removing the actual lock-acquisition increment must fail: {findings:?}"
    );

    fs::write(
        fixture_root.join("src/assets.rs"),
        "storage_lock_acquisitions: Arc<AtomicU64>\npub fn storage_lock_acquisitions() {}\nfetch_add(1, Ordering::Relaxed)\n",
    )
    .expect("asset-lock fixture restores");
    let hardware = fixture_root.join(".github/workflows/hardware-gpu.yml");
    let source = fs::read_to_string(&hardware).expect("hardware workflow fixture reads");
    fs::write(
        &hardware,
        source.replace(
            "m9_pf00_representative_performance_artifact",
            "removed_pf00_hardware_producer",
        ),
    )
    .expect("PF00 hardware producer mutation writes");
    findings.clear();
    check_pf00_performance_truth_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF00-PERFORMANCE-TRUTH"
                && finding
                    .message
                    .contains("m9_pf00_representative_performance_artifact")
        }),
        "removing the hardware PF00 producer must fail: {findings:?}"
    );
}

#[test]
fn pf03_pf05_hot_path_doctor_rejects_prepared_list_cloning() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf03-pf05-hot-path");
    let _ = fs::remove_dir_all(&fixture_root);
    let sources = [
        "src/render/cpu_render.rs",
        "src/render/state.rs",
        "src/render/prepare/types/geometry_storage.rs",
        "src/render/prepare/types.rs",
        "src/render/prepare_lifecycle.rs",
        "src/render/gpu/resource_encoding.rs",
        "src/geometry.rs",
        "tests/m9_platform_release.rs",
        "src/assets.rs",
        "src/assets/store.rs",
        "src/assets/snapshot_tests.rs",
        "src/render/prepare/materials.rs",
        "src/render/prepare/primitives.rs",
        "src/render/phase5_tests.rs",
        "src/material/color.rs",
        "src/scene/resolved_cache.rs",
        "src/scene/transforms.rs",
        "tests/pf03_pf05_hot_path_contracts.rs",
    ];
    for relative in sources {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("hot-path fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("hot-path source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf03_pf05_hot_path_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let cpu_render = fixture_root.join("src/render/cpu_render.rs");
    let mut source = fs::read_to_string(&cpu_render).expect("CPU render fixture reads");
    source.push_str("\n// mutation: prepared.primitives.clone()\n");
    fs::write(cpu_render, source).expect("CPU render mutation writes");
    findings.clear();
    check_pf03_pf05_hot_path_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF03-PF05-HOT-PATH-CONTRACTS"
                && finding.message.contains("prepared.primitives.clone()")
        }),
        "restoring wholesale prepared-list cloning must fail: {findings:?}"
    );

    let prepared_types = fixture_root.join("src/render/prepare/types.rs");
    let source = fs::read_to_string(&prepared_types).expect("prepared types fixture reads");
    fs::write(
        &prepared_types,
        source.replace(
            "draw_transform: Arc<PreparedDrawTransform>",
            "draw_transform: Box<PreparedDrawTransform>",
        ),
    )
    .expect("per-triangle transform ownership mutation writes");
    findings.clear();
    check_pf03_pf05_hot_path_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF03-PF05-HOT-PATH-CONTRACTS"
                && finding
                    .message
                    .contains("draw_transform: Arc<PreparedDrawTransform>")
        }),
        "restoring per-triangle transform ownership must fail: {findings:?}"
    );
}

#[test]
fn pf10_hot_path_doctor_rejects_linear_keyframe_scanning() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf10-hot-path");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/animation/sampling.rs",
        "src/render/prepare/types.rs",
        "src/render/prepare/types/geometry_storage.rs",
        "src/render/gpu/vertices.rs",
        "src/render/gpu/instancing.rs",
        "src/scene/import/animation_bindings.rs",
        "src/scene/import/source_node_index.rs",
        "src/assets/gltf/textures.rs",
        "src/geometry.rs",
        "src/assets/gltf/meshes.rs",
        "src/render/prepare/primitives.rs",
        "src/scene_host/wasm.rs",
        "src/scene_host/wasm/support.rs",
        "src/render/culling.rs",
        "src/render/settings.rs",
        "src/render.rs",
        "src/render/cpu_render.rs",
        "tests/pf10_cpu_occlusion.rs",
        "tests/browser/scene_host_browser_proof.js",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("PF10 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("PF10 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf10_hot_path_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let sampling = fixture_root.join("src/animation/sampling.rs");
    let mut source = fs::read_to_string(&sampling).expect("sampling fixture reads");
    source.push_str("\n// mutation: for index in 0..times.len().saturating_sub(1) {}\n");
    fs::write(&sampling, source).expect("sampling mutation writes");
    findings.clear();
    check_pf10_hot_path_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF10-MEASURED-HOT-PATH-WASTE"
                && finding.message.contains("for index in 0..times.len()")
        }),
        "restoring linear keyframe scanning must fail: {findings:?}"
    );

    fs::copy(root.join("src/animation/sampling.rs"), &sampling).expect("sampling fixture restores");
    let culling = fixture_root.join("src/render/culling.rs");
    let source = fs::read_to_string(&culling).expect("culling fixture reads");
    fs::write(
        &culling,
        source.replace(
            "has_projected_tile_overlap",
            "removed_projected_tile_overlap",
        ),
    )
    .expect("culling mutation writes");
    findings.clear();
    check_pf10_hot_path_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF10-MEASURED-HOT-PATH-WASTE"
                && finding.message.contains("has_projected_tile_overlap")
        }),
        "removing the sparse-scene overlap gate must fail: {findings:?}"
    );
}

#[test]
fn pf07_pf08_doctor_rejects_unbounded_cpu_texture_bake() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf07-pf08-cpu-prepare");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/geometry.rs",
        "src/render/prepare/tangents.rs",
        "src/render/prepare/primitives.rs",
        "src/render/prepare/cpu_bake.rs",
        "tests/m9_platform_release.rs",
        "tests/pf08_texture_bake_parity.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("PF07/PF08 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("PF07/PF08 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf07_pf08_cpu_prepare_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let cpu_bake = fixture_root.join("src/render/prepare/cpu_bake.rs");
    let original_source = fs::read_to_string(&cpu_bake).expect("CPU bake fixture reads");
    let mut source = original_source.clone();
    source.push_str("\n// mutation: return vec![corners];\n");
    fs::write(&cpu_bake, source).expect("CPU bake mutation writes");
    findings.clear();
    check_pf07_pf08_cpu_prepare_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF07-PF08-BOUNDED-CPU-PREPARE"
                && finding.message.contains("return vec![corners]")
        }),
        "restoring factor-one allocation and fixed expansion must fail: {findings:?}"
    );

    let mut source = original_source.clone();
    source.push_str("\n// mutation: subdivided_cpu_corners(corners, 48\n");
    fs::write(&cpu_bake, source).expect("fixed CPU bake mutation writes");
    findings.clear();
    check_pf07_pf08_cpu_prepare_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF07-PF08-BOUNDED-CPU-PREPARE"
                && finding
                    .message
                    .contains("subdivided_cpu_corners(corners, 48")
        }),
        "restoring a fixed-48 subdivision call must fail: {findings:?}"
    );

    fs::write(&cpu_bake, original_source).expect("CPU bake fixture restores");
    let rendered_proof = fixture_root.join("tests/pf08_texture_bake_parity.rs");
    let source = fs::read_to_string(&rendered_proof).expect("PF08 proof fixture reads");
    fs::write(
        &rendered_proof,
        source.replace(
            "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu",
            "removed_pf08_rendered_parity_contract",
        ),
    )
    .expect("PF08 proof mutation writes");
    findings.clear();
    check_pf07_pf08_cpu_prepare_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF07-PF08-BOUNDED-CPU-PREPARE"
                && finding.message.contains(
                    "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu",
                )
        }),
        "removing PF08 seam/perspective/material CPU-GPU proof must fail: {findings:?}"
    );
}
