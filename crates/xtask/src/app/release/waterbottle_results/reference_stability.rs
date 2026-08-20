use crate::app::prelude::*;

pub(crate) fn validate_q11_reference_stability_result(
    value: &Value,
    expected_os: &str,
    expected_arch: &str,
) -> Result<(), String> {
    for (field, expected) in [
        ("schema", "scena.q11.reference_stability.v1"),
        ("status", "passed"),
        (
            "test_name",
            "q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison",
        ),
        ("os", expected_os),
        ("arch", expected_arch),
        ("backend", "Headless"),
        ("adapter", "software-rasterizer"),
        (
            "comparison_order",
            "independent-render-before-committed-reference",
        ),
    ] {
        if value.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "Q11 reference-stability result {field} must be {expected:?}"
            ));
        }
    }
    if value.get("release_evidence").and_then(Value::as_bool) != Some(true)
        || value.get("repeat_count").and_then(Value::as_u64) != Some(2)
        || value.get("width").and_then(Value::as_u64) != Some(256)
        || value.get("height").and_then(Value::as_u64) != Some(256)
        || value
            .get("timestamp_unix_seconds")
            .and_then(Value::as_u64)
            .is_none()
        || value.pointer("/reference/sha256").and_then(Value::as_str)
            != Some("8bbaa66e23a3dea4a9efcb53f9157226b666cd77bd020dea30cd89277d3037b5")
        || value
            .pointer("/source_asset/sha256")
            .and_then(Value::as_str)
            != Some("0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547")
    {
        return Err("Q11 reference-stability provenance or dimensions are incomplete".to_string());
    }
    if value.get("byte_identical").and_then(Value::as_bool) != Some(true) {
        return Err("Q11 reference-stability renders are not byte-identical".to_string());
    }
    let commit = value
        .get("commit_sha")
        .and_then(Value::as_str)
        .unwrap_or("");
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Q11 reference-stability result must bind an exact 40-hex commit".to_string());
    }
    let hashes = value
        .get("rgba8_sha256")
        .and_then(Value::as_array)
        .ok_or_else(|| "Q11 reference-stability hashes must be an array".to_string())?;
    if hashes.len() != 2
        || hashes[0] != hashes[1]
        || hashes.iter().any(|hash| {
            hash.as_str().is_none_or(|hash| {
                hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        })
    {
        return Err("Q11 reference-stability renders are not byte-identical".to_string());
    }
    let metrics = value
        .get("metric_distribution")
        .and_then(Value::as_array)
        .ok_or_else(|| "Q11 metric distribution must be an array".to_string())?;
    if metrics.len() != 2
        || metrics.iter().any(|metric| {
            metric.get("passed").and_then(Value::as_bool) != Some(true)
                || metric
                    .get("within_tolerance_fraction")
                    .and_then(Value::as_f64)
                    .is_none_or(|fraction| fraction < 0.995)
                || metric
                    .get("rgb_rmse")
                    .and_then(Value::as_f64)
                    .is_none_or(|rmse| rmse > 2.0)
                || metric.get("alpha_mismatch_pixels").and_then(Value::as_u64) != Some(0)
        })
    {
        return Err("Q11 metric distribution exceeds the approved fixed oracle".to_string());
    }
    Ok(())
}

pub(crate) fn validate_waterbottle_mutation_provenance(
    mutation: &Value,
    name: &str,
) -> Result<(), String> {
    let (kind, stage, render_count, required_coverage): (&str, &str, u64, &[&str]) = match name {
        "flattened_chrome" => ("post-hoc-pixel", "output-rgba8", 0, &["oracle-evaluator"]),
        "wrong_material" => (
            "rendered-scene",
            "scene-mesh-material-before-prepare",
            1,
            &[
                "gltf-import",
                "texture-resources-loaded",
                "scene-material-override",
                "cpu-material-resolution",
                "prepare",
                "render",
                "pbr-neutral-tonemap",
                "srgb8-output",
            ],
        ),
        "wrong_camera" => (
            "rendered-scene",
            "active-camera-transform-before-prepare",
            1,
            &[
                "gltf-import",
                "texture-resources-loaded",
                "active-camera",
                "prepare",
                "render",
                "pbr-neutral-tonemap",
                "srgb8-output",
            ],
        ),
        _ => return Err(format!("unknown WaterBottle mutation {name}")),
    };
    let coverage = mutation
        .get("pipeline_coverage")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("WaterBottle mutation {name} has no pipeline_coverage"))?;
    if mutation.get("mutation_kind").and_then(Value::as_str) != Some(kind)
        || mutation.get("mutation_stage").and_then(Value::as_str) != Some(stage)
        || mutation.get("render_count").and_then(Value::as_u64) != Some(render_count)
        || required_coverage.iter().any(|required| {
            !coverage
                .iter()
                .any(|entry| entry.as_str() == Some(required))
        })
    {
        return Err(format!(
            "WaterBottle mutation {name} does not prove the required {kind} pipeline execution"
        ));
    }
    Ok(())
}
