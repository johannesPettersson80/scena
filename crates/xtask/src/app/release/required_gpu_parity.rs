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
    browser_evidence_classification_matches(value, "software-conformance")
        && value.get("gate").and_then(Value::as_str) == Some("m6-rust-wasm-renderer-probe")
        && value.get("status").and_then(Value::as_str) == Some("passed")
        && required_pixel_parity_evaluation_passes(value.get("required_parity"), expected_backend)
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
                        && renderer_parity_source_matches(result, expected_backend)
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
    if !browser_evidence_classification_matches(value, "hardware-release") {
        return false;
    }
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
    if !required_pixel_parity_evaluation_passes(parity, expected_backend) {
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
                    && renderer_parity_source_matches(result, expected_backend)
                    && adapter_is_hardware(adapter)
            })
        })
}

fn browser_evidence_classification_matches(value: &Value, evidence_class: &str) -> bool {
    let scope = value.get("parity_scope").and_then(Value::as_array);
    match evidence_class {
        "software-conformance" => {
            value.get("proof_class").and_then(Value::as_str)
                == Some("renderer-conformance-with-diagnostic-webgpu-pixel-diff")
                && value.get("release_evidence").and_then(Value::as_bool) == Some(false)
                && value.get("parity_claim").and_then(Value::as_str) == Some("diagnostic-only")
                && scope.is_some_and(|scope| {
                    scope.len() == 1
                        && scope[0].as_str() == Some("webgpu:m6-identical-unlit-triangle-v1")
                })
        }
        "hardware-release" => {
            value.get("proof_class").and_then(Value::as_str)
                == Some("renderer-smoke-with-required-webgpu-full-frame-parity")
                && value.get("release_evidence").and_then(Value::as_bool) == Some(true)
                && value.get("parity_claim").and_then(Value::as_str)
                    == Some("full-frame-reference-diff")
                && scope.is_some_and(|scope| {
                    scope.len() == 1
                        && scope[0].as_str() == Some("webgpu:m6-identical-unlit-triangle-v1")
                })
        }
        _ => false,
    }
}

fn required_pixel_parity_evaluation_passes(
    required_parity: Option<&Value>,
    expected_backend: &str,
) -> bool {
    const MUTATIONS: [&str; 6] = [
        "wrong-colors",
        "geometry-shift",
        "missing-object",
        "vertical-flip",
        "linear-as-srgb",
        "stale-reference",
    ];
    required_parity
        .and_then(|parity| parity.get("evaluations"))
        .and_then(Value::as_array)
        .is_some_and(|evaluations| {
            evaluations.iter().any(|evaluation| {
                let pixel = evaluation.get("pixel_parity");
                let normalization = pixel.and_then(|value| value.get("normalization"));
                let thresholds = pixel.and_then(|value| value.get("thresholds"));
                let metrics = pixel.and_then(|value| value.get("metrics"));
                let mutations = pixel
                    .and_then(|value| value.get("mutations"))
                    .and_then(Value::as_array);
                evaluation
                    .get("backend")
                    .and_then(Value::as_str)
                    .is_some_and(|backend| backend.eq_ignore_ascii_case(expected_backend))
                    && evaluation
                        .get("status")
                        .and_then(Value::as_str)
                        .is_some_and(|status| matches!(status, "passed" | "diagnostic"))
                    && empty_array(evaluation.get("failure_codes"))
                    && pixel
                        .and_then(|value| value.get("status"))
                        .and_then(Value::as_str)
                        == Some("passed")
                    && empty_array(pixel.and_then(|value| value.get("failure_codes")))
                    && string_field(normalization, "row_origin") == Some("top-left")
                    && string_field(normalization, "transfer") == Some("srgb8")
                    && string_field(normalization, "alpha") == Some("straight-opaque")
                    && string_field(normalization, "dimensions") == Some("exact")
                    && string_field(normalization, "comparison_channels") == Some("rgb")
                    && number_field(thresholds, "rgb_chebyshev_tolerance")
                        .is_some_and(|value| value <= 4.0)
                    && number_field(thresholds, "within_tolerance_fraction_min")
                        .is_some_and(|value| value >= 0.995)
                    && number_field(thresholds, "rgb_rmse_max").is_some_and(|value| value <= 2.0)
                    && number_field(thresholds, "p99_5_channel_delta_max")
                        .is_some_and(|value| value <= 4.0)
                    && number_field(thresholds, "foreground_iou_min")
                        .is_some_and(|value| value >= 0.995)
                    && number_field(metrics, "compared_pixels").is_some_and(|value| value > 0.0)
                    && number_field(metrics, "within_tolerance_fraction")
                        .is_some_and(|value| value >= 0.995)
                    && number_field(metrics, "rgb_rmse").is_some_and(|value| value <= 2.0)
                    && number_field(metrics, "p99_5_channel_delta")
                        .is_some_and(|value| value <= 4.0)
                    && number_field(metrics, "foreground_iou").is_some_and(|value| value >= 0.995)
                    && pixel
                        .and_then(|value| value.get("mask"))
                        .and_then(|value| value.get("kind"))
                        .and_then(Value::as_str)
                        == Some("two-pixel-gradient-edge-exclusion")
                    && pixel
                        .and_then(|value| value.get("mask"))
                        .and_then(|value| value.get("source"))
                        .and_then(Value::as_str)
                        == Some("cpu-reference-gradient")
                    && pixel
                        .and_then(|value| value.get("mask"))
                        .and_then(|value| value.get("foreground_domain"))
                        .and_then(Value::as_str)
                        == Some("edge-excluded")
                    && pixel
                        .and_then(|value| value.get("worst_region"))
                        .and_then(|value| value.get("bbox"))
                        .and_then(Value::as_array)
                        .is_some_and(|bbox| bbox.len() == 4)
                    && pixel
                        .and_then(|value| value.get("diff_heatmap_rgba8_base64"))
                        .and_then(Value::as_str)
                        .is_some_and(|heatmap| !heatmap.is_empty())
                    && mutations.is_some_and(|mutations| {
                        mutations.len() == MUTATIONS.len()
                            && mutations.iter().zip(MUTATIONS).all(|(mutation, expected)| {
                                mutation.get("name").and_then(Value::as_str) == Some(expected)
                                    && mutation.get("rejected").and_then(Value::as_bool)
                                        == Some(true)
                                    && mutation
                                        .get("failure_codes")
                                        .and_then(Value::as_array)
                                        .is_some_and(|codes| !codes.is_empty())
                            })
                    })
            })
        })
}

fn renderer_parity_source_matches(result: &Value, expected_backend: &str) -> bool {
    let parity = result.get("parity");
    let readback = result.get("renderer_readback");
    let cpu = parity.and_then(|value| value.get("cpu_frame"));
    let gpu = parity.and_then(|value| value.get("gpu_frame"));
    parity
        .and_then(|value| value.get("schema"))
        .and_then(Value::as_str)
        == Some("scena.m6.cpu_webgpu_parity.v1")
        && parity
            .and_then(|value| value.get("backend"))
            .and_then(Value::as_str)
            .is_some_and(|backend| backend.eq_ignore_ascii_case(expected_backend))
        && string_field(cpu, "source") == Some("renderer-owned-cpu-frame")
        && string_field(gpu, "source") == Some("renderer-owned-gpu-copy")
        && number_field(gpu, "width") == number_field(readback, "width")
        && number_field(gpu, "height") == number_field(readback, "height")
        && string_field(gpu, "rgba8_fnv1a64") == string_field(readback, "rgba8_fnv1a64")
}

fn empty_array(value: Option<&Value>) -> bool {
    value.and_then(Value::as_array).is_some_and(Vec::is_empty)
}

fn string_field<'a>(value: Option<&'a Value>, field: &str) -> Option<&'a str> {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_str)
}

fn number_field(value: Option<&Value>, field: &str) -> Option<f64> {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_f64)
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
