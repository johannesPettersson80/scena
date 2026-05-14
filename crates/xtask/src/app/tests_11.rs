use crate::app::prelude::*;

#[test]
pub(crate) fn stage_release_artifacts_generates_canonical_release_evidence() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-readiness-test/stage-input");
    let output_root = root.join("target/xtask-release-readiness-test/stage-output");
    let _ = fs::remove_dir_all(&fixture_root);
    let _ = fs::remove_dir_all(&output_root);
    fs::create_dir_all(&fixture_root).expect("fixture root");

    write_stage_test_json(
        &fixture_root.join("release-webgl2/m6-rust-wasm-renderer-probe.json"),
        &browser_probe_fixture("webgl2"),
    );
    write_stage_test_json(
        &fixture_root.join("release-webgpu/m6-rust-wasm-renderer-probe.json"),
        &browser_probe_fixture("webgpu"),
    );
    write_stage_test_json(
        &fixture_root.join("release-wasm/m9-wasm-size.json"),
        &json!({"schema":"scena.m9.wasm_size.v1","status":"passed"}),
    );
    for lane in [
        "linux-native-vulkan",
        "headless-cpu",
        "macos-metal",
        "windows-dx12",
    ] {
        let lane_dir = fixture_root.join(format!("release-{lane}/m9-platform/{lane}"));
        fs::create_dir_all(&lane_dir).expect("lane dir");
        write_stage_test_json(
            &lane_dir.join("capabilities.json"),
            &json!({
                "schema": "scena.capabilities.v1",
                "lane": lane,
                "backend": lane,
                "adapter": { "available": lane != "headless-cpu" },
                "features": {},
                "diagnostics": [],
                "timestamp_unix_seconds": current_unix_seconds()
            }),
        );
        write_stage_test_json(
            &lane_dir.join("rendered-output.json"),
            &native_render_fixture(lane != "headless-cpu"),
        );
    }
    let waterbottle = fixture_root.join("release-macos-metal/m8-real-asset/waterbottle_gpu.png");
    fs::create_dir_all(waterbottle.parent().expect("waterbottle parent")).expect("waterbottle dir");
    fs::write(
        &waterbottle,
        [&[0x89, b'P', b'N', b'G'][..], &[1u8; 2048][..]].concat(),
    )
    .expect("waterbottle fixture");
    for suffix in [
        "release-lanes/linux-native-vulkan.json",
        "release-lanes/headless-cpu.json",
        "release-lanes/linux-webgl2-chromium.json",
        "release-lanes/linux-webgpu-chromium.json",
        "release-lanes/wasm32-unknown-unknown.json",
        "release-lanes/macos-metal.json",
        "release-lanes/windows-dx12.json",
        "m5-benchmarks.json",
        "m5-public-api-freeze.json",
        "m9-platform/m9-benchmarks.json",
        "m9-platform/m9-benchmarks-4k.json",
    ] {
        write_stage_test_json(
            &fixture_root.join(format!("release-linux-native-vulkan/{suffix}")),
            &json!({
                "status": "passed",
                "command_records": [{
                    "command": "fixture",
                    "status": "passed",
                    "duration_ms": 1,
                    "failure_log_path": "fixture.log",
                    "artifact_checksums": [{"path":"fixture","sha256":"x","bytes":1}]
                }],
                "baseline_comparison": {"status":"passed"},
                "rows": []
            }),
        );
    }
    for suffix in [
        "examples-visual/cad_plate_drawing_import.ppm",
        "m9-platform/linux-native-vulkan/default-scene.ppm",
        "m9-platform/linux-native-vulkan/static-gltf.ppm",
        "m9-platform/linux-native-vulkan/pbr-directional-red.ppm",
        "m9-platform/linux-native-vulkan/pbr-point-green.ppm",
        "m9-platform/linux-native-vulkan/pbr-spot-blue.ppm",
        "m9-platform/headless-cpu/default-scene.ppm",
        "m9-platform/headless-cpu/static-gltf.ppm",
        "m9-platform/macos-metal/default-scene.ppm",
        "m9-platform/macos-metal/static-gltf.ppm",
        "m9-platform/macos-metal/pbr-directional-red.ppm",
        "m9-platform/macos-metal/pbr-point-green.ppm",
        "m9-platform/macos-metal/pbr-spot-blue.ppm",
        "m9-platform/windows-dx12/default-scene.ppm",
        "m9-platform/windows-dx12/static-gltf.ppm",
        "m9-platform/windows-dx12/pbr-directional-red.ppm",
        "m9-platform/windows-dx12/pbr-point-green.ppm",
        "m9-platform/windows-dx12/pbr-spot-blue.ppm",
    ] {
        write_stage_test_ppm(&fixture_root.join(format!("release-linux-native-vulkan/{suffix}")));
    }
    for suffix in [
        "m9-platform/linux-native-vulkan/surface-context-loss.json",
        "m9-platform/macos-metal/surface-context-loss.json",
        "m9-platform/windows-dx12/surface-context-loss.json",
    ] {
        write_stage_test_json(
            &fixture_root.join(format!("release-linux-native-vulkan/{suffix}")),
            &json!({"schema":"fixture","status":"passed"}),
        );
    }

    stage_release_artifacts(&root, &fixture_root, &output_root).expect("stage succeeds");

    assert!(output_root.join("reviews/findings.json").is_file());
    assert!(
        output_root
            .join("reviews/maintainer-signoff.toml")
            .is_file()
    );
    assert!(
        output_root
            .join("visual-proof/browser-webgpu.json")
            .is_file()
    );
    assert!(
        output_root
            .join("visual-proof/waterbottle-gpu.json")
            .is_file()
    );
    let matrix_text = fs::read_to_string(output_root.join("m9-platform/m9-capability-matrix.json"))
        .expect("matrix reads");
    assert!(matrix_text.contains("\"status\": \"passed\""));
    assert!(!matrix_text.contains("missing-lane-artifact"));
}

#[test]
pub(crate) fn stage_release_artifact_timestamp_format_is_rfc3339_utc() {
    assert_eq!(utc_rfc3339_from_unix(0), "1970-01-01T00:00:00Z");
    assert_eq!(utc_rfc3339_from_unix(1_688_212_096), "2023-07-01T11:48:16Z");
}

fn browser_probe_fixture(backend: &str) -> serde_json::Value {
    json!({
        "gate": "m6-rust-wasm-renderer-probe",
        "status": "passed",
        "results": [{
            "backend": backend,
            "status": "passed",
            "pixels": { "nonblack": 42 },
            "capabilities": { "backend": backend },
            "renderer_readback": {
                "source": "renderer-owned-gpu-copy",
                "pixel_statistics": { "nonblack": 42 },
                "rgba8_fnv1a64": "fnv1a64:0000000000000001"
            }
        }]
    })
}

fn write_stage_test_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).expect("fixture serializes")
        ),
    )
    .expect("json fixture");
}

fn write_stage_test_ppm(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("ppm parent");
    }
    fs::write(path, b"P6\n2 1\n255\n\x00\x00\x00\xff\x80\x40").expect("ppm fixture");
}

fn native_render_fixture(gpu: bool) -> serde_json::Value {
    json!({
        "schema": "scena.m9.platform_render.v1",
        "backend": if gpu { "Metal" } else { "Headless" },
        "host_gpu_available": gpu,
        "gpu_proof": gpu,
        "headless_cpu_proof": !gpu,
        "timestamp_unix_seconds": current_unix_seconds(),
        "default_scene": screenshot_metadata_fixture(),
        "static_gltf": {
            "production_claim": true,
            "gpu_proof": gpu,
            "proof_class": if gpu { "camera-framed-non-ndc" } else { "cpu-camera-framed-non-ndc" },
            "nonblack_pixels": 10,
            "asset_provenance": { "hash": "fnv1a64:0000000000000001" },
            "backend": "fixture",
            "adapter": {},
            "renderer_settings": {},
            "color_management": {},
            "tolerance": {},
            "screenshot": "fixture.ppm",
            "width": 2,
            "height": 1
        },
        "pbr_lights": {
            "proof_class": "native-pbr-punctual-light",
            "production_claim": gpu,
            "gpu_proof": gpu,
            "lights": [
                pbr_light_fixture("directional", gpu),
                pbr_light_fixture("point", gpu),
                pbr_light_fixture("spot", gpu)
            ]
        }
    })
}

fn screenshot_metadata_fixture() -> serde_json::Value {
    json!({
        "backend": "fixture",
        "adapter": {},
        "renderer_settings": {},
        "color_management": {},
        "tolerance": {},
        "screenshot": "fixture.ppm",
        "width": 2,
        "height": 1
    })
}

fn pbr_light_fixture(light_type: &str, gpu: bool) -> serde_json::Value {
    json!({
        "light_type": light_type,
        "gpu_proof": gpu,
        "production_claim": gpu,
        "color_assertion_passed": true,
        "nonblack_pixels": 10,
        "backend": "fixture",
        "adapter": {},
        "renderer_settings": {},
        "color_management": {},
        "tolerance": {},
        "screenshot": "fixture.ppm",
        "width": 2,
        "height": 1
    })
}
