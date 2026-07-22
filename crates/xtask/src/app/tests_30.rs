use crate::app::prelude::*;

#[test]
fn q06_required_webgpu_artifact_rejects_unavailable_zero_and_software_results() {
    let unavailable = json!({
        "gate": "m6-rust-wasm-renderer-probe",
        "status": "unavailable",
        "required_parity": {"enabled": true, "status": "failed"},
        "results": [{"backend": "WebGpu", "status": "unavailable", "error": "NoAdapter"}]
    });
    assert!(!required_browser_gpu_parity_passes(&unavailable, "webgpu"));

    let zero_output = required_webgpu_fixture("DiscreteGpu", "NVIDIA RTX", 0, 0);
    assert!(!required_browser_gpu_parity_passes(&zero_output, "webgpu"));

    let software = required_webgpu_fixture("Cpu", "Google SwiftShader", 1, 64);
    assert!(!required_browser_gpu_parity_passes(&software, "webgpu"));

    let unproven = required_webgpu_fixture("Other", "unknown adapter", 1, 64);
    assert!(!required_browser_gpu_parity_passes(&unproven, "webgpu"));

    let hardware = required_webgpu_fixture("DiscreteGpu", "NVIDIA RTX", 1, 64);
    assert!(required_browser_gpu_parity_passes(&hardware, "webgpu"));

    let mut smoke_only = hardware.clone();
    smoke_only["required_parity"]
        .as_object_mut()
        .expect("required parity is an object")
        .remove("evaluations");
    assert!(
        !required_browser_gpu_parity_passes(&smoke_only, "webgpu"),
        "hardware identity plus nonblack pixels is not image parity"
    );

    let mut wrong_pixels = hardware.clone();
    wrong_pixels["required_parity"]["evaluations"][0]["pixel_parity"]["status"] = json!("failed");
    assert!(
        !required_browser_gpu_parity_passes(&wrong_pixels, "webgpu"),
        "a materially wrong image must fail required hardware parity"
    );
}

#[test]
fn q06_hosted_webgpu_conformance_accepts_rendered_software_output_without_claiming_hardware() {
    let software = required_webgpu_fixture("Cpu", "Google SwiftShader", 1, 64);
    assert!(browser_gpu_conformance_passes(&software, "webgpu"));
    assert!(browser_probe_release_proof_passes_for_class(
        &software,
        "linux-webgpu-chromium",
        "software-conformance",
    ));
    assert!(!browser_probe_release_proof_passes(
        &software,
        "linux-webgpu-chromium",
    ));
    assert!(!required_browser_gpu_parity_passes(&software, "webgpu"));

    let mut smoke_only = software.clone();
    smoke_only["required_parity"]
        .as_object_mut()
        .expect("required parity is an object")
        .remove("evaluations");
    assert!(
        !browser_gpu_conformance_passes(&smoke_only, "webgpu"),
        "software conformance must still compare pixels"
    );

    let zero_output = required_webgpu_fixture("Cpu", "Google SwiftShader", 0, 0);
    assert!(!browser_gpu_conformance_passes(&zero_output, "webgpu"));
}

#[test]
fn q06_linux_native_lane_content_rejects_cpu_fallback() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-regressions/q06-linux-cpu-fallback");
    let lane_dir = fixture_root.join("target/gate-artifacts/m9-platform/linux-native-vulkan");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&lane_dir).expect("Q06 native fixture directory");
    fs::write(
        lane_dir.join("rendered-output.json"),
        serde_json::to_string_pretty(&json!({
            "schema": "scena.m9.platform_render.v1",
            "lane": "linux-native-vulkan",
            "backend": "Headless",
            "host_gpu_available": false,
            "gpu_proof": false,
            "static_gltf": {
                "proof_class": "cpu-fallback-camera-framed-non-ndc",
                "production_claim": false,
                "gpu_proof": false
            },
            "pbr_lights": {
                "proof_class": "native-pbr-punctual-light",
                "production_claim": false,
                "gpu_proof": false,
                "lights": []
            }
        }))
        .expect("Q06 native fixture serializes"),
    )
    .expect("Q06 native fixture writes");

    assert!(
        !release_lane_content_ok(&fixture_root, "linux-native-vulkan")
            .expect("Q06 native lane validation runs"),
        "required Linux native content must reject host_gpu_available=false CPU fallback"
    );
}

pub(crate) fn required_webgpu_fixture(
    device_type: &str,
    adapter_name: &str,
    draw_calls: u64,
    nonblack: u64,
) -> Value {
    json!({
        "gate": "m6-rust-wasm-renderer-probe",
        "status": "passed",
        "required_parity": {
            "enabled": true,
            "status": "passed",
            "backends": ["webgpu"],
            "evaluations": [{
                "backend": "webgpu",
                "status": "passed",
                "failure_codes": [],
                "pixel_parity": {
                    "status": "passed",
                    "failure_codes": [],
                    "normalization": {
                        "row_origin": "top-left",
                        "transfer": "srgb8",
                        "alpha": "straight-opaque",
                        "dimensions": "exact",
                        "comparison_channels": "rgb"
                    },
                    "thresholds": {
                        "rgb_chebyshev_tolerance": 4,
                        "within_tolerance_fraction_min": 0.995,
                        "rgb_rmse_max": 2.0,
                        "p99_5_channel_delta_max": 4,
                        "foreground_iou_min": 0.995
                    },
                    "metrics": {
                        "compared_pixels": 4096,
                        "within_tolerance_fraction": 1.0,
                        "rgb_rmse": 0.0,
                        "p99_5_channel_delta": 0,
                        "foreground_iou": 1.0
                    },
                    "mask": {
                        "kind": "two-pixel-gradient-edge-exclusion",
                        "source": "cpu-reference-gradient",
                        "excluded_pixels": 128,
                        "foreground_domain": "edge-excluded"
                    },
                    "worst_region": {"bbox": [0, 0, 8, 8], "rgb_rmse": 0.0},
                    "diff_heatmap_rgba8_base64": "AAAA/w==",
                    "mutations": [
                        {"name": "wrong-colors", "rejected": true, "failure_codes": ["RGB_RMSE"]},
                        {"name": "geometry-shift", "rejected": true, "failure_codes": ["FOREGROUND_IOU"]},
                        {"name": "missing-object", "rejected": true, "failure_codes": ["FOREGROUND_IOU"]},
                        {"name": "vertical-flip", "rejected": true, "failure_codes": ["FOREGROUND_IOU"]},
                        {"name": "linear-as-srgb", "rejected": true, "failure_codes": ["RGB_RMSE"]},
                        {"name": "stale-reference", "rejected": true, "failure_codes": ["FOREGROUND_IOU"]}
                    ]
                }
            }]
        },
        "results": [{
            "schema": "scena.m6.browser_renderer_probe.v1",
            "backend": "WebGpu",
            "status": "passed",
            "workflow": "triangle",
            "gpu_device": true,
            "draw_calls": draw_calls,
            "gpu_submissions": draw_calls,
            "adapter": {"name": adapter_name, "device_type": device_type, "backend": "BrowserWebGpu"},
            "renderer_readback": {
                "source": "renderer-owned-gpu-copy",
                "width": 64,
                "height": 64,
                "rgba8_fnv1a64": "abcd",
                "pixel_statistics": {"nonblack": nonblack}
            },
            "parity": {
                "schema": "scena.m6.cpu_webgpu_parity.v1",
                "status": "passed",
                "backend": "WebGpu",
                "cpu_frame": {
                    "source": "renderer-owned-cpu-frame",
                    "width": 64,
                    "height": 64,
                    "rgba8_fnv1a64": "dcba"
                },
                "gpu_frame": {
                    "source": "renderer-owned-gpu-copy",
                    "width": 64,
                    "height": 64,
                    "rgba8_fnv1a64": "abcd"
                }
            },
            "pixels": {"nonblack": nonblack}
        }]
    })
}
