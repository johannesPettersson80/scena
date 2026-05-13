use crate::app::prelude::*;

pub(crate) fn check_required_visual_proof_artifacts(
    artifact_root: &Path,
    findings: &mut Vec<Finding>,
) {
    if !artifact_root.is_dir() {
        return;
    }
    for suffix in REQUIRED_VISUAL_PROOF_ARTIFACT_SUFFIXES {
        require_visual_proof_artifact_file(&artifact_root.join(suffix), suffix, findings);
    }
}

pub(crate) fn require_visual_proof_artifact_file(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "missing visual proof artifact {suffix} at {}",
                path.display()
            ),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} is not valid JSON"),
        ));
        return;
    };
    if value.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} does not have status 'passed'"),
        ));
    }
    if value
        .get("preview_only")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} is preview-only and cannot count as release evidence"),
        ));
    }
    if value
        .get("rust_test_command")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && !value
            .get("rust_test_output_observed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} recorded a Rust test command without Rust test summary output"),
        ));
    }
    if value
        .get("skip_marker_observed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} observed a skip marker; skipped visual proof is not release evidence"),
        ));
    }
}

pub(crate) fn reject_stale_json_timestamp(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(timestamp) = value
        .get("timestamp_unix_seconds")
        .and_then(serde_json::Value::as_u64)
    else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release artifact {suffix} is missing timestamp_unix_seconds; stale artifacts cannot be detected"
            ),
        ));
        return;
    };
    let now = current_unix_seconds();
    if timestamp + RELEASE_ARTIFACT_MAX_AGE_SECONDS < now {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release artifact {suffix} is stale; regenerate it in the current release run"
            ),
        ));
    }
    if timestamp > now + RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release artifact {suffix} has a future timestamp; regenerate it with a valid clock"
            ),
        ));
    }
}

pub(crate) fn reject_stale_json_commit(
    path: &Path,
    suffix: &str,
    expected_commit: &str,
    findings: &mut Vec<Finding>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(commit) = value
        .get("commit_sha")
        .or_else(|| value.get("source_commit_sha"))
        .and_then(serde_json::Value::as_str)
    else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release artifact {suffix} is missing commit_sha; stale commit artifacts cannot be detected"
            ),
        ));
        return;
    };
    if expected_commit != "local-checkout" && commit != expected_commit {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release artifact {suffix} was generated for commit {commit}, expected {expected_commit}"
            ),
        ));
    }
}

pub(crate) fn reject_constant_ppm_artifact(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    let Some(pixel_bytes) = ppm_pixel_payload(&bytes) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded screenshot artifact {suffix} is not a valid P6 PPM"),
        ));
        return;
    };
    if ppm_payload_is_constant_rgb(pixel_bytes) {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded screenshot artifact {suffix} is constant-color; visual proof must contain inspectable rendered content"
            ),
        ));
    }
}

pub(crate) fn reject_unmeasured_capability_matrix_rows(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    if json_contains_string_value(&value, "measurement_source", "factory-contract") {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded capability artifact {suffix} contains factory-contract rows; final release requires measured per-lane probes"
            ),
        ));
    }
    if json_contains_string_value(&value, "measurement_source", "missing-lane-artifact") {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded capability artifact {suffix} contains missing-lane-artifact rows; final release requires measured per-lane probes"
            ),
        ));
    }
}

pub(crate) fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub(crate) fn ppm_pixel_payload(bytes: &[u8]) -> Option<&[u8]> {
    let mut index = 0;
    let mut token_count = 0;
    let mut first_token = None;

    while token_count < 4 {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        if bytes[index] == b'#' {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        let start = index;
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if token_count == 0 {
            first_token = Some(&bytes[start..index]);
        }
        token_count += 1;
    }

    if first_token != Some(&b"P6"[..]) {
        return None;
    }
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    Some(&bytes[index..])
}

pub(crate) fn ppm_payload_is_constant_rgb(pixel_bytes: &[u8]) -> bool {
    if pixel_bytes.len() < 6 || !pixel_bytes.len().is_multiple_of(3) {
        return true;
    }
    let first = &pixel_bytes[..3];
    pixel_bytes.chunks_exact(3).all(|pixel| pixel == first)
}

pub(crate) fn json_contains_string_value(
    value: &serde_json::Value,
    key: &str,
    expected: &str,
) -> bool {
    match value {
        serde_json::Value::Object(object) => object.iter().any(|(field, child)| {
            (field == key && child.as_str() == Some(expected))
                || json_contains_string_value(child, key, expected)
        }),
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_contains_string_value(child, key, expected)),
        _ => false,
    }
}

pub(crate) fn require_benchmark_baseline_comparison_file(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "could not read downloaded benchmark artifact {}",
                path.display()
            ),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded benchmark artifact {suffix} is not valid JSON"),
        ));
        return;
    };
    require_benchmark_baseline_comparison(&value, suffix, findings);
}

pub(crate) fn require_benchmark_baseline_comparison(
    value: &serde_json::Value,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    let Some(summary) = value.get("baseline_comparison") else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded benchmark artifact {suffix} is missing stored baseline comparison"),
        ));
        return;
    };

    let status = summary
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<missing>");
    let baseline_path_present = summary
        .get("baseline_path")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|path| !path.is_empty());
    let baseline_hash_present = summary
        .get("baseline_sha256")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|hash| !hash.is_empty());
    let metric_is_p95 =
        summary.get("metric").and_then(serde_json::Value::as_str) == Some("p95_frame_ms");
    if status != "passed" || !baseline_path_present || !baseline_hash_present || !metric_is_p95 {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded benchmark artifact {suffix} has failed or incomplete stored baseline comparison"
            ),
        ));
    }

    let Some(rows) = value.get("rows").and_then(serde_json::Value::as_array) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded benchmark artifact {suffix} has no benchmark rows"),
        ));
        return;
    };

    for row in rows {
        let row_name = row
            .get("scene")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unnamed>");
        if row.get("status").and_then(serde_json::Value::as_str)
            == Some("deferred-to-dedicated-performance-lane")
        {
            if row
                .get("baseline_comparison")
                .and_then(|comparison| comparison.get("status"))
                .and_then(serde_json::Value::as_str)
                != Some("deferred")
            {
                findings.push(Finding::new(
                    "RELEASE-READY-ARTIFACTS",
                    format!(
                        "deferred benchmark row {row_name} in {suffix} must record a deferred baseline comparison"
                    ),
                ));
            }
            continue;
        }

        if row
            .get("sample_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < MIN_BENCHMARK_SAMPLE_COUNT
        {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "benchmark row {row_name} in {suffix} has fewer than {MIN_BENCHMARK_SAMPLE_COUNT} samples"
                ),
            ));
        }

        let Some(comparison) = row.get("baseline_comparison") else {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "benchmark row {row_name} in {suffix} is missing stored baseline comparison"
                ),
            ));
            continue;
        };
        if comparison.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "benchmark regression for row {row_name} in {suffix}; p95_frame_ms exceeds the stored baseline allowance"
                ),
            ));
        }
    }
}

pub(crate) fn require_rendered_output_screenshot_metadata_file(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not read rendered-output artifact {}", path.display()),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("rendered-output artifact {suffix} is not valid JSON"),
        ));
        return;
    };
    require_rendered_output_screenshot_metadata(&value, suffix, findings);
}
