use crate::app::prelude::*;

pub(crate) fn browser_probe_release_proof_passes(value: &Value, lane: &str) -> bool {
    browser_probe_release_proof_passes_for_class(value, lane, "hardware-release")
}

pub(crate) fn browser_probe_release_proof_passes_for_class(
    value: &Value,
    lane: &str,
    evidence_class: &str,
) -> bool {
    let expected_backend = match lane {
        "linux-webgl2-chromium" => "webgl2",
        "linux-webgpu-chromium" => "webgpu",
        _ => return false,
    };
    let base = value.get("gate").and_then(Value::as_str) == Some("m6-rust-wasm-renderer-probe")
        && value.get("status").and_then(Value::as_str) == Some("passed")
        && value
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| {
                results.iter().any(|result| {
                    result
                        .get("backend")
                        .and_then(Value::as_str)
                        .is_some_and(|backend| backend.eq_ignore_ascii_case(expected_backend))
                        && result.get("status").and_then(Value::as_str) == Some("passed")
                        && result
                            .get("pixels")
                            .and_then(|pixels| pixels.get("nonblack"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                })
            });
    if !base {
        return false;
    }
    match lane {
        "linux-webgl2-chromium" => true,
        "linux-webgpu-chromium" => match evidence_class {
            "software-conformance" => browser_gpu_conformance_passes(value, expected_backend),
            "hardware-release" => required_browser_gpu_parity_passes(value, expected_backend),
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn release_lane_evidence_class(lane: &str) -> &'static str {
    if lane != "linux-webgpu-chromium" {
        return "not-applicable";
    }
    match env::var("SCENA_GPU_EVIDENCE_CLASS").as_deref() {
        Ok("software-conformance") => "software-conformance",
        Ok("hardware-release") => "hardware-release",
        _ => "hardware-release",
    }
}

pub(crate) fn browser_gpu_conformance_passes(value: &Value, expected_backend: &str) -> bool {
    value.get("gate").and_then(Value::as_str) == Some("m6-rust-wasm-renderer-probe")
        && value.get("status").and_then(Value::as_str) == Some("passed")
        && value
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| {
                results.iter().any(|result| {
                    result.get("workflow").and_then(Value::as_str) == Some("triangle")
                        && result
                            .get("backend")
                            .and_then(Value::as_str)
                            .is_some_and(|backend| backend.eq_ignore_ascii_case(expected_backend))
                        && result.get("status").and_then(Value::as_str) == Some("passed")
                        && result.get("gpu_device").and_then(Value::as_bool) == Some(true)
                        && result
                            .get("draw_calls")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                        && result
                            .get("gpu_submissions")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                        && result
                            .get("renderer_readback")
                            .and_then(|readback| readback.get("source"))
                            .and_then(Value::as_str)
                            == Some("renderer-owned-gpu-copy")
                        && result
                            .get("renderer_readback")
                            .and_then(|readback| readback.get("pixel_statistics"))
                            .and_then(|pixels| pixels.get("nonblack"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                        && result
                            .get("pixels")
                            .and_then(|pixels| pixels.get("nonblack"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                })
            })
}

pub(crate) fn required_browser_gpu_parity_passes(value: &Value, expected_backend: &str) -> bool {
    let parity = value.get("required_parity");
    if parity
        .and_then(|value| value.get("enabled"))
        .and_then(Value::as_bool)
        != Some(true)
        || parity
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            != Some("passed")
    {
        return false;
    }
    value
        .get("results")
        .and_then(Value::as_array)
        .is_some_and(|results| {
            results.iter().any(|result| {
                let adapter = result.get("adapter");
                result.get("workflow").and_then(Value::as_str) == Some("triangle")
                    && result
                        .get("backend")
                        .and_then(Value::as_str)
                        .is_some_and(|backend| backend.eq_ignore_ascii_case(expected_backend))
                    && result.get("status").and_then(Value::as_str) == Some("passed")
                    && result.get("gpu_device").and_then(Value::as_bool) == Some(true)
                    && result
                        .get("draw_calls")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        > 0
                    && result
                        .get("gpu_submissions")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        > 0
                    && result
                        .get("renderer_readback")
                        .and_then(|readback| readback.get("source"))
                        .and_then(Value::as_str)
                        == Some("renderer-owned-gpu-copy")
                    && result
                        .get("renderer_readback")
                        .and_then(|readback| readback.get("pixel_statistics"))
                        .and_then(|pixels| pixels.get("nonblack"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        > 0
                    && adapter_is_hardware(adapter)
            })
        })
}

fn adapter_is_hardware(adapter: Option<&Value>) -> bool {
    let Some(adapter) = adapter else {
        return false;
    };
    let device_type = adapter
        .get("device_type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !matches!(
        device_type.to_ascii_lowercase().as_str(),
        "discretegpu" | "integratedgpu" | "virtualgpu"
    ) {
        return false;
    }
    let identity = ["name", "driver", "driver_info"]
        .into_iter()
        .filter_map(|field| adapter.get(field).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    ![
        "swiftshader",
        "llvmpipe",
        "lavapipe",
        "software rasterizer",
        "microsoft basic render",
    ]
    .into_iter()
    .any(|marker| identity.contains(marker))
}
