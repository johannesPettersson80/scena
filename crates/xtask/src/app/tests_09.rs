use crate::app::prelude::*;

#[test]
pub(crate) fn release_readiness_accepts_benchmark_artifact_with_passed_baseline_comparison() {
    let artifact = json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": "linux-native-vulkan",
        "baseline_comparison": {
            "status": "passed",
            "baseline_path": "docs/benchmarks/m9-baselines.json",
            "baseline_sha256": "fnv1a64:0000000000000001",
            "metric": "p95_frame_ms"
        },
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "sample_count": 100,
                "p95_frame_ms": 10.2,
                "baseline_comparison": {
                    "status": "passed",
                    "baseline_p95_frame_ms": 10.0,
                    "allowed_regression_percent": 5.0,
                    "regression_percent": 2.0
                }
            },
            {
                "scene": "headless-4k",
                "status": "deferred-to-dedicated-performance-lane",
                "sample_count": 0,
                "baseline_comparison": {
                    "status": "deferred"
                }
            }
        ]
    });
    let mut findings = Vec::new();

    require_benchmark_baseline_comparison(
        &artifact,
        "m9-platform/m9-benchmarks.json",
        &mut findings,
    );

    assert_eq!(
        findings,
        Vec::new(),
        "passed stored-baseline benchmark comparison should not block release readiness"
    );
}

#[test]
pub(crate) fn release_readiness_rejects_rendered_output_without_screenshot_metadata() {
    let artifact = json!({
        "schema": "scena.m9.platform_render.v1",
        "default_scene": {
            "backend": "Headless",
            "screenshot": "target/default.ppm",
            "width": 96,
            "height": 64
        },
        "static_gltf": {
            "backend": "Headless",
            "screenshot": "target/static.ppm",
            "width": 96,
            "height": 64,
            "asset_provenance": { "path": "tests/assets/gltf/non_ndc_camera_scene.gltf" }
        }
    });
    let mut findings = Vec::new();

    require_rendered_output_screenshot_metadata(
        &artifact,
        "m9-platform/headless-cpu/rendered-output.json",
        &mut findings,
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("screenshot metadata")),
        "release readiness must reject rendered-output artifacts without full screenshot metadata: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_accepts_rendered_output_with_screenshot_metadata() {
    let screenshot = json!({
        "backend": "Headless",
        "adapter": { "available": false },
        "renderer_settings": { "width": 96, "height": 64 },
        "color_management": { "output_encoding": "srgb8-after-aces" },
        "tolerance": { "policy": "native-rendered-output-smoke" },
        "screenshot": "target/default.ppm",
        "width": 96,
        "height": 64
    });
    let artifact = json!({
        "schema": "scena.m9.platform_render.v1",
        "default_scene": screenshot,
        "static_gltf": {
            "backend": "Headless",
            "adapter": { "available": false },
            "renderer_settings": { "width": 96, "height": 64 },
            "color_management": { "output_encoding": "srgb8-after-aces" },
            "tolerance": { "policy": "native-rendered-output-smoke" },
            "screenshot": "target/static.ppm",
            "width": 96,
            "height": 64,
            "asset_provenance": {
                "path": "tests/assets/gltf/non_ndc_camera_scene.gltf",
                "hash": "fnv1a64:0000000000000001"
            }
        },
        "pbr_lights": {
            "lights": [
                {
                    "light_type": "directional",
                    "backend": "Headless",
                    "adapter": { "available": false },
                    "renderer_settings": { "width": 96, "height": 64 },
                    "color_management": { "output_encoding": "srgb8-after-aces" },
                    "tolerance": { "policy": "native-rendered-output-smoke" },
                    "screenshot": "target/light.ppm",
                    "width": 96,
                    "height": 64
                }
            ]
        }
    });
    let mut findings = Vec::new();

    require_rendered_output_screenshot_metadata(
        &artifact,
        "m9-platform/headless-cpu/rendered-output.json",
        &mut findings,
    );

    assert_eq!(
        findings,
        Vec::new(),
        "complete rendered-output screenshot metadata should satisfy release readiness"
    );
}

#[test]
pub(crate) fn release_readiness_rejects_cpu_fallback_native_render_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/cpu-fallback-render");
    let lane_dir = artifact_root.join("m9-platform/macos-metal");
    fs::create_dir_all(&lane_dir).expect("lane artifact dir");
    fs::write(
        lane_dir.join("rendered-output.json"),
        r#"{
            "schema": "scena.m9.platform_render.v1",
            "gpu_proof": false,
            "host_gpu_available": false,
            "fallback_policy": "cpu fallback is diagnostic only and never satisfies GPU rendered-output claims",
            "static_gltf": {
                "proof_class": "cpu-fallback-camera-framed-non-ndc",
                "production_claim": false,
                "gpu_proof": false
            }
        }"#,
    )
    .expect("fallback rendered-output write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| { finding.message.contains("does not prove GPU output") }),
        "release readiness must reject native GPU artifacts that are CPU fallback only: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_native_render_artifact_without_pbr_light_proof() {
    let artifact = json!({
        "schema": "scena.m9.platform_render.v1",
        "gpu_proof": true,
        "host_gpu_available": true,
        "static_gltf": {
            "proof_class": "camera-framed-non-ndc",
            "production_claim": true,
            "gpu_proof": true
        }
    });

    assert!(
        !native_gpu_render_proof_passes(&artifact),
        "native release proof must include PBR punctual-light rendered-output evidence"
    );
}

#[test]
pub(crate) fn release_readiness_accepts_native_render_artifact_with_pbr_light_proof() {
    let artifact = json!({
        "schema": "scena.m9.platform_render.v1",
        "gpu_proof": true,
        "host_gpu_available": true,
        "static_gltf": {
            "proof_class": "camera-framed-non-ndc",
            "production_claim": true,
            "gpu_proof": true
        },
        "pbr_lights": {
            "proof_class": "native-pbr-punctual-light",
            "production_claim": true,
            "gpu_proof": true,
            "lights": [
                {
                    "light_type": "directional",
                    "production_claim": true,
                    "gpu_proof": true,
                    "color_assertion_passed": true,
                    "nonblack_pixels": 1200
                },
                {
                    "light_type": "point",
                    "production_claim": true,
                    "gpu_proof": true,
                    "color_assertion_passed": true,
                    "nonblack_pixels": 1200
                },
                {
                    "light_type": "spot",
                    "production_claim": true,
                    "gpu_proof": true,
                    "color_assertion_passed": true,
                    "nonblack_pixels": 1200
                }
            ]
        }
    });

    assert!(
        native_gpu_render_proof_passes(&artifact),
        "native release proof should pass when camera-framed glTF and all PBR punctual-light proofs pass"
    );
}

#[test]
pub(crate) fn release_lane_artifact_uses_required_file_evidence_not_command_recorded_status() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact =
        release_lane_artifact(&root, "linux-native-vulkan").expect("release lane artifact builds");

    assert_ne!(artifact["status"], "command-recorded");
    assert!(
        artifact["required_artifacts"]
            .as_array()
            .expect("required artifacts array")
            .iter()
            .any(|entry| entry["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("rendered-output.json"))),
        "release-lane artifact must name required proof files instead of only recording a command",
    );
}

#[test]
pub(crate) fn release_lane_artifact_consumes_measured_command_records() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-lane-command-record-test");
    let lane = "linux-native-vulkan";
    let command_dir = fixture_root.join("target/gate-artifacts/release-lanes");
    fs::create_dir_all(&command_dir).expect("command record dir");
    fs::write(
        command_dir.join(format!("{lane}.log")),
        b"focused lane command output\n",
    )
    .expect("failure log");
    let log_sha = sha256_hex(&command_dir.join(format!("{lane}.log"))).expect("log sha");
    fs::write(
        command_dir.join(format!("{lane}.commands.jsonl")),
        format!(
            r#"{{"command":"cargo test --test m9_platform_release","status":"passed","duration_ms":1234,"duration_source":"ci-wrapper","failure_log_path":"target/gate-artifacts/release-lanes/{lane}.log","failure_log_sha256":"{log_sha}"}}"#
        ),
    )
    .expect("command record jsonl");

    let artifact = release_lane_artifact(&fixture_root, lane)
        .expect("release lane artifact builds with measured command records");
    let records = artifact["command_records"]
        .as_array()
        .expect("command records");
    let measured = records
        .iter()
        .find(|record| record["command"] == "cargo test --test m9_platform_release")
        .expect("measured test command record");

    assert_eq!(measured["duration_ms"], 1234);
    assert_eq!(measured["duration_source"], "ci-wrapper");
    assert_eq!(measured["failure_log_sha256"], log_sha);
    assert_eq!(
        measured["measurement_source"],
        "target/gate-artifacts/release-lanes/linux-native-vulkan.commands.jsonl"
    );
}

#[test]
pub(crate) fn release_lane_artifact_status_requires_native_gpu_content_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-lane-content-test");
    let lane = "macos-metal";
    let lane_dir = fixture_root.join(format!("target/gate-artifacts/m9-platform/{lane}"));
    fs::create_dir_all(&lane_dir).expect("lane artifact dir");
    fs::write(
        lane_dir.join("rendered-output.json"),
        r#"{
            "schema": "scena.m9.platform_render.v1",
            "gpu_proof": false,
            "host_gpu_available": false,
            "static_gltf": {
                "proof_class": "cpu-fallback-camera-framed-non-ndc",
                "production_claim": false,
                "gpu_proof": false
            }
        }"#,
    )
    .expect("rendered output artifact");
    for file in [
        "capabilities.json",
        "surface-context-loss.json",
        "default-scene.ppm",
        "static-gltf.ppm",
    ] {
        fs::write(lane_dir.join(file), b"fixture").expect("lane file");
    }
    let platform_dir = fixture_root.join("target/gate-artifacts/m9-platform");
    fs::write(platform_dir.join("m9-benchmarks.json"), b"{}").expect("benchmarks");

    let artifact =
        release_lane_artifact(&fixture_root, lane).expect("release lane artifact builds");

    assert_eq!(artifact["content_ok"], false);
    assert_eq!(artifact["status"], "incomplete");
}

#[test]
pub(crate) fn release_lane_artifact_status_requires_browser_probe_passed_status() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-lane-browser-content-test");
    let artifact_dir = fixture_root.join("target/gate-artifacts");
    let command_dir = artifact_dir.join("release-lanes");
    fs::create_dir_all(&command_dir).expect("command record dir");
    fs::write(
        artifact_dir.join("m6-rust-wasm-renderer-probe.json"),
        r#"{
            "gate": "m6-rust-wasm-renderer-probe",
            "status": "unavailable",
            "results": [{
                "backend": "WebGpu",
                "status": "failed",
                "gpu_device": true,
                "pixels": { "nonblack": 0 }
            }]
        }"#,
    )
    .expect("browser probe artifact");
    fs::write(
        command_dir.join("linux-webgpu-chromium.commands.jsonl"),
        r#"{"command":"wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe","status":"passed","duration_ms":1}
{"command":"npm run browser:m6","status":"passed","duration_ms":1}
"#,
    )
    .expect("command record jsonl");

    let artifact = release_lane_artifact(&fixture_root, "linux-webgpu-chromium")
        .expect("release lane artifact builds");

    assert_eq!(artifact["content_ok"], false);
    assert_eq!(artifact["status"], "incomplete");
}

#[test]
pub(crate) fn release_lane_artifact_supports_separate_headless_cpu_proof_lane() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-headless-cpu-lane-test");
    let lane_dir = fixture_root.join("target/gate-artifacts/m9-platform/headless-cpu");
    fs::create_dir_all(&lane_dir).expect("headless lane artifact dir");
    fs::write(
        lane_dir.join("rendered-output.json"),
        r#"{
            "schema": "scena.m9.platform_render.v1",
            "lane": "headless-cpu",
            "backend": "Headless",
            "headless_cpu_proof": true,
            "static_gltf": {
                "proof_class": "cpu-camera-framed-non-ndc",
                "production_claim": true,
                "nonblack_pixels": 42
            }
        }"#,
    )
    .expect("headless rendered-output write");
    for file in ["capabilities.json", "default-scene.ppm", "static-gltf.ppm"] {
        fs::write(lane_dir.join(file), b"fixture").expect("headless lane file");
    }
    let platform_dir = fixture_root.join("target/gate-artifacts/m9-platform");
    fs::write(platform_dir.join("m9-benchmarks.json"), b"{}").expect("benchmarks");

    let artifact = release_lane_artifact(&fixture_root, "headless-cpu")
        .expect("headless release lane artifact builds");

    assert_eq!(artifact["status"], "passed");
    assert_eq!(artifact["content_ok"], true);
    assert!(
        artifact["required_artifacts"]
            .as_array()
            .expect("required artifacts array")
            .iter()
            .any(|entry| entry["path"]
                .as_str()
                .is_some_and(|path| path.contains("headless-cpu/rendered-output.json"))),
        "headless CPU lane must have its own rendered-output artifact",
    );
}

#[test]
pub(crate) fn m8_gltf_asset_matrix_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_gltf_asset_matrix_contract(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m8_gltf_asset_matrix_rejects_unhashed_sample_files() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-gltf-asset-matrix-hash-test");
    let matrix_dir = fixture_root.join("docs/assets");
    let sample_dir = fixture_root.join("tests/assets/gltf/khronos/Sample");
    fs::create_dir_all(&matrix_dir).expect("matrix dir");
    fs::create_dir_all(&sample_dir).expect("sample dir");
    fs::write(
        sample_dir.join("Sample.gltf"),
        br#"{"asset":{"version":"2.0"}}"#,
    )
    .expect("sample glTF");
    fs::write(
        matrix_dir.join("gltf-asset-matrix.md"),
        r#"# glTF Asset Matrix

This matrix catches fail with a structured error and silent fallback rows.

| Asset/Fixture | Source/License | Features | Expected Result | Expected Diagnostics | Rendered Output Reference | Evidence |
|---|---|---|---|---|---|---|
| Khronos `Sample` | Khronos sample / upstream sample license | mesh | pass | none expected | deferred structured non-visual proof | `tests/m8_assets_materials_ecosystem.rs` |
"#,
    )
    .expect("matrix write");
    fs::write(
        fixture_root.join("tests/assets/gltf/khronos/manifest.toml"),
        r#"[source]
repository = "https://github.com/KhronosGroup/glTF-Sample-Assets"
commit = "sample"
license_reference = "Upstream LICENSES directory in glTF-Sample-Assets"

[[asset]]
name = "Sample"
path = "Sample/Sample.gltf"
contract = "hash guard"
"#,
    )
    .expect("manifest write");
    let mut findings = Vec::new();

    check_gltf_asset_matrix_contract(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.message.contains(
            "must record a SHA-256 hash for tests/assets/gltf/khronos/Sample/Sample.gltf"
        )),
        "sample assets must not be accepted without source hashes: {findings:?}",
    );
}

#[test]
pub(crate) fn m8_assets_materials_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m8_assets_materials_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}
