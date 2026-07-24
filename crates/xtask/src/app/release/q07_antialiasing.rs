use crate::app::prelude::*;

pub(super) fn validate_q07_antialiasing_result(
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    let path = output.join("q07-antialiasing-effect/result.json");
    let value: Value = serde_json::from_str(
        &fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    for (field, expected) in [
        ("schema", "scena.q07.antialiasing_effect.v1"),
        ("status", "passed"),
        ("commit_sha", expected_commit),
        ("fixture", "high-contrast-asymmetric-diagonal-v1"),
    ] {
        if value.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "Q07 antialiasing result {field} must be {expected:?}"
            ));
        }
    }
    if value.get("release_evidence").and_then(Value::as_bool) != Some(true) {
        return Err("Q07 antialiasing result must be release evidence".to_string());
    }
    let adapter = value
        .get("adapter")
        .ok_or_else(|| "Q07 antialiasing result is missing adapter evidence".to_string())?;
    let device_type = adapter
        .get("device_type")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !matches!(device_type, "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu") {
        return Err(format!(
            "Q07 antialiasing adapter is not hardware: {device_type:?}"
        ));
    }
    let baseline = value
        .pointer("/baseline/metrics")
        .ok_or_else(|| "Q07 antialiasing result is missing baseline metrics".to_string())?;
    for mode in ["fxaa", "msaa4"] {
        let result = value
            .pointer(&format!("/modes/{mode}"))
            .ok_or_else(|| format!("Q07 antialiasing result is missing {mode}"))?;
        if result.get("status").and_then(Value::as_str) != Some("passed")
            || !effect_passes(baseline, result.get("metrics").unwrap_or(&Value::Null))
        {
            return Err(format!(
                "Q07 antialiasing {mode} lacks the required measured pixel effect"
            ));
        }
    }
    let msaa8 = value
        .pointer("/modes/msaa8")
        .ok_or_else(|| "Q07 antialiasing result is missing MSAA8 status".to_string())?;
    let msaa8_valid = match msaa8.get("status").and_then(Value::as_str) {
        Some("passed") => effect_passes(baseline, msaa8.get("metrics").unwrap_or(&Value::Null)),
        Some("degraded") => {
            msaa8.get("reason_code").and_then(Value::as_str) == Some("UNSUPPORTED_SAMPLE_COUNT")
                && msaa8.get("requested").and_then(Value::as_u64) == Some(8)
                && msaa8.get("maximum").and_then(Value::as_u64).is_some()
        }
        _ => false,
    };
    if !msaa8_valid {
        return Err(
            "Q07 MSAA8 must pass the effect oracle or record explicit sample-count degradation"
                .to_string(),
        );
    }
    for mutation in ["no_op", "blur_everything"] {
        let rejected = value
            .get("known_bad_mutations")
            .and_then(Value::as_array)
            .is_some_and(|mutations| {
                mutations.iter().any(|entry| {
                    entry.get("name").and_then(Value::as_str) == Some(mutation)
                        && entry.get("rejected").and_then(Value::as_bool) == Some(true)
                })
            });
        if !rejected {
            return Err(format!("Q07 antialiasing result did not reject {mutation}"));
        }
    }
    let checksums = value
        .get("source_checksums")
        .and_then(Value::as_array)
        .ok_or_else(|| "Q07 antialiasing result is missing source checksums".to_string())?;
    let mut required_frames = vec![
        "q07-antialiasing-effect/none.ppm",
        "q07-antialiasing-effect/fxaa.ppm",
        "q07-antialiasing-effect/msaa4.ppm",
    ];
    if msaa8.get("status").and_then(Value::as_str) == Some("passed") {
        required_frames.push("q07-antialiasing-effect/msaa8.ppm");
    }
    for relative in required_frames {
        let actual = sha256_hex(&output.join(relative)).map_err(|error| error.to_string())?;
        let bound = checksums.iter().any(|entry| {
            entry.get("path").and_then(Value::as_str) == Some(relative)
                && entry.get("sha256").and_then(Value::as_str) == Some(&actual)
        });
        if !bound {
            return Err(format!(
                "Q07 antialiasing result does not bind staged frame {relative}"
            ));
        }
    }
    Ok(())
}

fn effect_passes(baseline: &Value, candidate: &Value) -> bool {
    let base_intermediate = metric(baseline, "intermediate_luma_pixels");
    let base_hard = metric(baseline, "hard_transition_count");
    let base_energy = metric(baseline, "squared_edge_energy");
    let base_range = metric(baseline, "luma_range");
    let candidate_intermediate = metric(candidate, "intermediate_luma_pixels");
    let candidate_hard = metric(candidate, "hard_transition_count");
    let candidate_energy = metric(candidate, "squared_edge_energy");
    let candidate_range = metric(candidate, "luma_range");
    let maximum_intermediate = base_intermediate
        .saturating_add(base_hard.saturating_mul(6))
        .max(base_intermediate.saturating_add(20));
    candidate_intermediate >= base_intermediate.saturating_add(20)
        && candidate_intermediate <= maximum_intermediate
        && candidate_hard < base_hard
        && candidate_energy.saturating_mul(100) < base_energy.saturating_mul(98)
        && candidate_range.saturating_mul(10) >= base_range.saturating_mul(9)
}

fn metric(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(u64::MAX)
}
