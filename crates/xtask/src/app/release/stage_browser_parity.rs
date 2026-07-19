use crate::app::prelude::*;

const PARITY_SCHEMA: &str = "scena.m6.cpu_webgl2_parity.v1";

pub(super) fn validate_cpu_webgl2_parity(
    result: &Value,
    renderer_readback: &Value,
) -> Result<(), String> {
    let parity = result
        .get("parity")
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            "browser release probe backend webgl2 is missing required CPU/WebGL2 parity evidence"
                .to_string()
        })?;
    if parity.get("schema").and_then(Value::as_str) != Some(PARITY_SCHEMA) {
        return Err(format!(
            "browser release probe backend webgl2 CPU/WebGL2 parity must use schema {PARITY_SCHEMA}"
        ));
    }
    if parity.get("status").and_then(Value::as_str) != Some("passed") {
        return Err(
            "browser release probe backend webgl2 CPU/WebGL2 parity did not pass".to_string(),
        );
    }
    if !parity
        .get("failure_codes")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Err(
            "browser release probe backend webgl2 CPU/WebGL2 parity failure_codes must be empty"
                .to_string(),
        );
    }

    let normalization = parity.get("normalization").ok_or_else(|| {
        "browser release probe backend webgl2 CPU/WebGL2 parity is missing normalization"
            .to_string()
    })?;
    for (field, expected) in [
        ("row_origin", "top-left"),
        ("transfer", "srgb8"),
        ("alpha", "straight-opaque"),
        ("dimensions", "exact"),
        ("comparison_channels", "rgb"),
        ("ssim_domain", "srgb8-luma"),
    ] {
        if normalization.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "browser release probe backend webgl2 CPU/WebGL2 parity normalization.{field} must be {expected}"
            ));
        }
    }
    let width = positive_u64(normalization, "width", "normalization")?;
    let height = positive_u64(normalization, "height", "normalization")?;
    let cpu = validate_frame(
        parity,
        "cpu_frame",
        "renderer-owned-cpu-frame",
        width,
        height,
    )?;
    let gpu = validate_frame(
        parity,
        "gpu_frame",
        "renderer-owned-gpu-copy",
        width,
        height,
    )?;

    if renderer_readback.get("width").and_then(Value::as_u64) != Some(width)
        || renderer_readback.get("height").and_then(Value::as_u64) != Some(height)
        || renderer_readback
            .get("rgba8_fnv1a64")
            .and_then(Value::as_str)
            != gpu.get("rgba8_fnv1a64").and_then(Value::as_str)
    {
        return Err(
            "browser release probe backend webgl2 parity GPU frame must match the renderer-owned headline readback dimensions and checksum"
                .to_string(),
        );
    }
    if cpu.get("rgba8_fnv1a64").and_then(Value::as_str)
        == gpu.get("rgba8_fnv1a64").and_then(Value::as_str)
    {
        return Err(
            "browser release probe backend webgl2 CPU/WebGL2 parity must retain distinct measured frame checksums"
                .to_string(),
        );
    }

    validate_metrics(parity, width, height)?;
    validate_known_bad_mutation(parity)?;
    Ok(())
}

fn validate_frame<'a>(
    parity: &'a Value,
    field: &str,
    expected_source: &str,
    width: u64,
    height: u64,
) -> Result<&'a Value, String> {
    let frame = parity
        .get(field)
        .ok_or_else(|| format!("browser release probe backend webgl2 parity is missing {field}"))?;
    if frame.get("source").and_then(Value::as_str) != Some(expected_source) {
        return Err(format!(
            "browser release probe backend webgl2 parity {field}.source must be {expected_source}"
        ));
    }
    if frame.get("width").and_then(Value::as_u64) != Some(width)
        || frame.get("height").and_then(Value::as_u64) != Some(height)
    {
        return Err(
            "browser release probe backend webgl2 parity CPU/GPU frames must have matching dimensions"
                .to_string(),
        );
    }
    if frame.get("alpha_deviations").and_then(Value::as_u64) != Some(0) {
        return Err(format!(
            "browser release probe backend webgl2 parity {field} must record zero alpha deviations"
        ));
    }
    let checksum = frame
        .get("rgba8_fnv1a64")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !valid_fnv1a64(checksum) {
        return Err(format!(
            "browser release probe backend webgl2 parity {field} has an invalid RGBA8 checksum"
        ));
    }
    let encoded = frame
        .get("rgba8_base64")
        .and_then(Value::as_str)
        .unwrap_or("");
    let byte_count = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "CPU/WebGL2 parity dimensions overflow RGBA8 byte count".to_string())?;
    let expected_base64_len = byte_count
        .div_ceil(3)
        .checked_mul(4)
        .ok_or_else(|| "CPU/WebGL2 parity dimensions overflow base64 length".to_string())?;
    if encoded.is_empty()
        || u64::try_from(encoded.len()).ok() != Some(expected_base64_len)
        || !encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    {
        return Err(format!(
            "browser release probe backend webgl2 parity {field} must retain a complete RGBA8 frame input"
        ));
    }
    Ok(frame)
}

fn validate_metrics(parity: &Value, width: u64, height: u64) -> Result<(), String> {
    let thresholds = parity.get("thresholds").ok_or_else(|| {
        "browser release probe backend webgl2 parity is missing fixed thresholds".to_string()
    })?;
    for (field, expected) in [
        ("rmse_max", 0.08),
        ("ssim_min", 0.93),
        ("mean_channel_delta_max", 6.0),
        ("foreground_iou_min", 0.90),
        ("foreground_region_rmse_max", 0.13),
    ] {
        if thresholds.get(field).and_then(Value::as_f64) != Some(expected) {
            return Err(format!(
                "browser release probe backend webgl2 parity threshold {field} drifted from {expected}"
            ));
        }
    }
    if thresholds
        .get("p95_channel_delta_max")
        .and_then(Value::as_u64)
        != Some(24)
        || thresholds
            .get("alpha_deviations_max")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "browser release probe backend webgl2 parity channel/alpha thresholds drifted"
                .to_string(),
        );
    }
    let metrics = parity.get("metrics").ok_or_else(|| {
        "browser release probe backend webgl2 parity is missing bounded metrics".to_string()
    })?;
    let bounded = [
        ("rmse", f64::NEG_INFINITY, 0.08),
        ("ssim", 0.93, f64::INFINITY),
        ("mean_channel_delta", f64::NEG_INFINITY, 6.0),
        ("foreground_iou", 0.90, f64::INFINITY),
        ("foreground_region_rmse", f64::NEG_INFINITY, 0.13),
    ];
    for (field, min, max) in bounded {
        let value = metrics.get(field).and_then(Value::as_f64).ok_or_else(|| {
            format!("browser release probe backend webgl2 parity metric {field} is missing")
        })?;
        if value < min || value > max {
            return Err(format!(
                "browser release probe backend webgl2 parity metric {field} is outside its bound"
            ));
        }
    }
    if metrics
        .get("p95_channel_delta")
        .and_then(Value::as_u64)
        .is_none_or(|value| value > 24)
        || metrics.get("compared_pixels").and_then(Value::as_u64) != width.checked_mul(height)
    {
        return Err(
            "browser release probe backend webgl2 parity channel metric or compared pixel count is invalid"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_known_bad_mutation(parity: &Value) -> Result<(), String> {
    let mutation = parity.get("known_bad_mutation").ok_or_else(|| {
        "browser release probe backend webgl2 parity is missing known-bad mutation evidence"
            .to_string()
    })?;
    if mutation.get("kind").and_then(Value::as_str) != Some("gpu-center-channel-perturbation")
        || mutation.get("rejected").and_then(Value::as_bool) != Some(true)
        || mutation
            .get("failure_codes")
            .and_then(Value::as_array)
            .is_none_or(|codes| codes.is_empty())
    {
        return Err(
            "browser release probe backend webgl2 parity known-bad mutation was not rejected"
                .to_string(),
        );
    }
    Ok(())
}

fn positive_u64(value: &Value, field: &str, context: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "browser release probe backend webgl2 CPU/WebGL2 parity {context}.{field} must be positive"
            )
        })
}

fn valid_fnv1a64(value: &str) -> bool {
    value.len() == 16
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}
