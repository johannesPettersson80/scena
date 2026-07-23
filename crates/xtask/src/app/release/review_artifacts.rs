use crate::app::prelude::*;

pub(crate) const REQUIRED_RELEASE_ARTIFACT_SUFFIXES: &[&str] = &[
    "staging-metadata.json",
    "ci-provenance.json",
    "release-lanes/linux-native-vulkan.json",
    "release-lanes/headless-cpu.json",
    "release-lanes/linux-webgl2-chromium.json",
    "release-lanes/linux-webgpu-chromium.json",
    "release-lanes/wasm32-unknown-unknown.json",
    "release-lanes/macos-metal.json",
    "release-lanes/windows-dx12.json",
    "m5-benchmarks.json",
    "m5-public-api-freeze.json",
    "examples-visual/camera_framing_frame_bounds.json",
    "examples-visual/camera_framing_frame_bounds.ppm",
    "examples-visual/cad_plate_drawing_import.ppm",
    "m6-rust-wasm-renderer-probe.json",
    "m9-wasm-size.json",
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/m9-benchmarks.json",
    "m9-platform/m9-benchmarks-4k.json",
    "m9-platform/m9-benchmarks-feature-matrix.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/linux-native-vulkan/surface-context-loss.json",
    "m9-platform/linux-native-vulkan/default-scene.ppm",
    "m9-platform/linux-native-vulkan/static-gltf.ppm",
    "m9-platform/linux-native-vulkan/pbr-directional-red.ppm",
    "m9-platform/linux-native-vulkan/pbr-point-green.ppm",
    "m9-platform/linux-native-vulkan/pbr-spot-blue.ppm",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "m9-platform/headless-cpu/default-scene.ppm",
    "m9-platform/headless-cpu/static-gltf.ppm",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/macos-metal/surface-context-loss.json",
    "m9-platform/macos-metal/default-scene.ppm",
    "m9-platform/macos-metal/static-gltf.ppm",
    "m9-platform/macos-metal/pbr-directional-red.ppm",
    "m9-platform/macos-metal/pbr-point-green.ppm",
    "m9-platform/macos-metal/pbr-spot-blue.ppm",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
    "m9-platform/windows-dx12/surface-context-loss.json",
    "m9-platform/windows-dx12/default-scene.ppm",
    "m9-platform/windows-dx12/static-gltf.ppm",
    "m9-platform/windows-dx12/pbr-directional-red.ppm",
    "m9-platform/windows-dx12/pbr-point-green.ppm",
    "m9-platform/windows-dx12/pbr-spot-blue.ppm",
    "m8-real-asset/waterbottle_gpu.png",
    "m8-real-asset/waterbottle_diff.png",
    "m8-real-asset/waterbottle_gpu_result.json",
    "q07-antialiasing-effect/result.json",
    "q07-antialiasing-effect/none.ppm",
    "q07-antialiasing-effect/fxaa.ppm",
    "q07-antialiasing-effect/msaa4.ppm",
    "q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json",
    "q08-required-parity/close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json",
    "q08-required-parity/dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json",
    "q08-required-parity/z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json",
    "q08-required-parity/core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json",
    "q08-required-parity/pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json",
    "q01-waterbottle-cpu/live.png",
    "q01-waterbottle-cpu/known_bad_flattened_chrome.png",
    "q01-waterbottle-cpu/known_bad_wrong_material.png",
    "q01-waterbottle-cpu/known_bad_wrong_camera.png",
    "q01-waterbottle-cpu/result.json",
    "q11-reference-stability/linux-x86_64.json",
    "q11-reference-stability/macos-aarch64.json",
    "q11-reference-stability/windows-x86_64.json",
    "round-e-cpu-material-proof/live-frame.png",
    "round-e-cpu-material-proof/live-cpu-frame.json",
    "round-e-cpu-material-proof.json",
    "round-e-cloudflare-material-proof.json",
    "round-e-cloudflare-material-proof/canvas.png",
    "round-e-cloudflare-material-proof/matte.png",
    "round-e-cloudflare-material-proof/plastic.png",
    "round-e-cloudflare-material-proof/metal.png",
    "round-e-cloudflare-material-proof/rough_metal.png",
    "round-e-cloudflare-material-proof/chrome.png",
    "round-e-cloudflare-material-proof/brushed_steel.png",
    "round-e-cloudflare-material-proof/clearcoat_plastic.png",
    "round-e-cloudflare-material-proof/satin.png",
    "round-e-cloudflare-material-proof/leather.png",
    "round-e-cloudflare-material-proof/clear_glass.png",
    "round-e-cloudflare-material-proof/frosted_glass.png",
    "round-e-cloudflare-material-proof/rubber.png",
    "round-e-webgpu-material-proof/live-frame.png",
    "round-e-webgpu-material-proof/result.json",
    "c09-gpu-resource-lifecycle/required-result.json",
    "release-lanes/headless-cpu.commands.jsonl",
    "release-lanes/headless-cpu.log",
    "visual-proof/waterbottle-gpu.json",
    "visual-proof/waterbottle-cpu.json",
    "visual-proof/browser-webgpu.json",
    "visual-proof/browser-webgl2.json",
    "visual-proof/native-gpu.json",
];

pub(crate) const REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES: &[&str] = &[
    "staging-metadata.json",
    "m6-rust-wasm-renderer-probe.json",
    "m9-platform/m9-capability-matrix.json",
    "q01-waterbottle-cpu/result.json",
    "q11-reference-stability/linux-x86_64.json",
    "q11-reference-stability/macos-aarch64.json",
    "q11-reference-stability/windows-x86_64.json",
    "round-e-cpu-material-proof.json",
    "round-e-cloudflare-material-proof.json",
    "round-e-webgpu-material-proof/result.json",
    "c09-gpu-resource-lifecycle/required-result.json",
    "q07-antialiasing-effect/result.json",
    "q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json",
    "q08-required-parity/close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json",
    "q08-required-parity/dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json",
    "q08-required-parity/z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json",
    "q08-required-parity/core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json",
    "q08-required-parity/pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json",
];

pub(crate) const RELEASE_LANE_ARTIFACT_SUFFIXES: &[&str] = &[
    "release-lanes/linux-native-vulkan.json",
    "release-lanes/headless-cpu.json",
    "release-lanes/linux-webgl2-chromium.json",
    "release-lanes/linux-webgpu-chromium.json",
    "release-lanes/wasm32-unknown-unknown.json",
    "release-lanes/macos-metal.json",
    "release-lanes/windows-dx12.json",
];

pub(crate) const REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/windows-dx12/rendered-output.json",
];

pub(crate) const REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES: &[&str] = &[
    "staging-metadata.json",
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "q01-waterbottle-cpu/result.json",
    "q11-reference-stability/linux-x86_64.json",
    "q11-reference-stability/macos-aarch64.json",
    "q11-reference-stability/windows-x86_64.json",
    "round-e-cpu-material-proof/live-cpu-frame.json",
    "round-e-cpu-material-proof.json",
    "round-e-cloudflare-material-proof.json",
    "round-e-webgpu-material-proof/result.json",
    "c09-gpu-resource-lifecycle/required-result.json",
    "q07-antialiasing-effect/result.json",
    "q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json",
    "q08-required-parity/close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json",
    "q08-required-parity/dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json",
    "q08-required-parity/z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json",
    "q08-required-parity/core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json",
    "q08-required-parity/pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
];
pub(crate) const REQUIRED_JSON_COMMIT_ARTIFACT_SUFFIXES: &[&str] = &[
    "staging-metadata.json",
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "q11-reference-stability/linux-x86_64.json",
    "q11-reference-stability/macos-aarch64.json",
    "q11-reference-stability/windows-x86_64.json",
    "round-e-cpu-material-proof/live-cpu-frame.json",
    "round-e-cpu-material-proof.json",
    "round-e-cloudflare-material-proof.json",
    "round-e-webgpu-material-proof/result.json",
    "c09-gpu-resource-lifecycle/required-result.json",
    "q07-antialiasing-effect/result.json",
    "q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json",
    "q08-required-parity/close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json",
    "q08-required-parity/dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json",
    "q08-required-parity/z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json",
    "q08-required-parity/core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json",
    "q08-required-parity/pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
    "visual-proof/waterbottle-gpu.json",
    "visual-proof/waterbottle-cpu.json",
    "visual-proof/browser-webgpu.json",
    "visual-proof/browser-webgl2.json",
    "visual-proof/native-gpu.json",
];

pub(crate) const REQUIRED_NON_CONSTANT_PPM_ARTIFACT_SUFFIXES: &[&str] = &[
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
    "q07-antialiasing-effect/none.ppm",
    "q07-antialiasing-effect/fxaa.ppm",
    "q07-antialiasing-effect/msaa4.ppm",
];

pub(crate) const REQUIRED_MEASURED_CAPABILITY_ARTIFACT_SUFFIXES: &[&str] =
    &["m9-platform/m9-capability-matrix.json"];

pub(crate) const REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/m9-benchmarks.json",
    "m9-platform/m9-benchmarks-4k.json",
    "m9-platform/m9-benchmarks-feature-matrix.json",
];
pub(crate) const REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/windows-dx12/rendered-output.json",
];
pub(crate) const REQUIRED_VISUAL_PROOF_ARTIFACT_SUFFIXES: &[&str] = &[
    "visual-proof/waterbottle-gpu.json",
    "visual-proof/waterbottle-cpu.json",
    "visual-proof/browser-webgpu.json",
    "visual-proof/browser-webgl2.json",
    "visual-proof/native-gpu.json",
];
pub(crate) const MIN_BENCHMARK_SAMPLE_COUNT: u64 = 100;

pub(crate) const RELEASE_ARTIFACT_MAX_AGE_SECONDS: u64 = 24 * 60 * 60;
pub(crate) const RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS: u64 = 60 * 60;

pub(crate) fn require_json_status_passed(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not read downloaded artifact {}", path.display()),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded artifact {} is not valid JSON", path.display()),
        ));
        return;
    };
    if value.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded release artifact {suffix} does not have status 'passed'"),
        ));
    }
}

pub(crate) fn required_gpu_resource_lifecycle_proof_passes(value: &Value) -> bool {
    if value.get("schema").and_then(Value::as_str)
        != Some("scena.q04.required_gpu_resource_lifecycle.v1")
        || value.get("status").and_then(Value::as_str) != Some("passed")
        || value.get("proof_class").and_then(Value::as_str) != Some("physical-hardware-required")
        || value.get("complete_lifecycle").and_then(Value::as_bool) != Some(true)
        || value
            .get("assertions_executed")
            .and_then(Value::as_u64)
            .is_none_or(|count| count < 10)
    {
        return false;
    }

    let Some(adapter) = value.get("adapter").and_then(Value::as_object) else {
        return false;
    };
    if !matches!(
        adapter.get("device_type").and_then(Value::as_str),
        Some("DiscreteGpu" | "IntegratedGpu" | "VirtualGpu")
    ) {
        return false;
    }
    let identity = ["name", "device_type", "driver", "driver_info"]
        .iter()
        .filter_map(|key| adapter.get(*key).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if [
        "swiftshader",
        "llvmpipe",
        "lavapipe",
        "software rasterizer",
        "microsoft basic render",
    ]
    .iter()
    .any(|marker| identity.contains(marker))
    {
        return false;
    }

    let Some(baseline) = lifecycle_resource_shape(value.get("baseline")) else {
        return false;
    };
    let Some(prepared) = lifecycle_resource_shape(value.get("prepared")) else {
        return false;
    };
    let Some(released) = lifecycle_resource_shape(value.get("released")) else {
        return false;
    };
    let baseline_total = baseline.iter().sum::<u64>();
    let prepared_total = prepared.iter().sum::<u64>();
    let baseline_pending = lifecycle_u64(value.get("baseline"), "pending_destructions");
    let released_pending = lifecycle_u64(value.get("released"), "pending_destructions");
    prepared_total > baseline_total
        && released == baseline
        && released_pending.is_some_and(|count| count > 0)
        && value.get("poll_status").and_then(Value::as_str) == Some("Confirmed")
        && value.get("poll_pending_before").and_then(Value::as_u64) == released_pending
        && value
            .get("poll_destroyed_resources")
            .and_then(Value::as_u64)
            == released_pending
        && value.get("poll_pending_after").and_then(Value::as_u64) == baseline_pending
}

fn lifecycle_resource_shape(value: Option<&Value>) -> Option<[u64; 6]> {
    let value = value?;
    Some([
        value.get("buffers")?.as_u64()?,
        value.get("gpu_textures")?.as_u64()?,
        value.get("render_targets")?.as_u64()?,
        value.get("pipelines")?.as_u64()?,
        value.get("bind_groups")?.as_u64()?,
        value.get("shader_modules")?.as_u64()?,
    ])
}

fn lifecycle_u64(value: Option<&Value>, field: &str) -> Option<u64> {
    value?.get(field)?.as_u64()
}

pub(crate) fn require_gpu_resource_lifecycle_proof(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    let value = fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if value
        .as_ref()
        .is_none_or(|value| !required_gpu_resource_lifecycle_proof_passes(value))
    {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release artifact {suffix} is not a complete physical-hardware GPU resource-lifecycle proof"
            ),
        ));
    }
}
