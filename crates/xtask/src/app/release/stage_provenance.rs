use crate::app::prelude::*;

pub(super) fn validate_release_json_metadata(
    value: &Value,
    suffix: &str,
    expected_commit: &str,
) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err(format!(
            "release artifact {suffix} must be a JSON object with source provenance"
        ));
    };
    if object
        .get("schema")
        .and_then(Value::as_str)
        .is_none_or(|schema| schema.trim().is_empty())
    {
        return Err(format!(
            "release artifact {suffix} is missing required non-blank schema provenance"
        ));
    }
    let producer = object
        .get("producer")
        .or_else(|| object.get("producing_command"))
        .or_else(|| object.get("test_name"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    if producer.is_none() {
        return Err(format!(
            "release artifact {suffix} is missing required producer command or test provenance"
        ));
    }
    let source_checksums = object
        .get("source_checksums")
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
        .ok_or_else(|| {
            format!(
                "release artifact {suffix} is missing required non-empty source_checksums \
                 provenance"
            )
        })?;
    for (index, checksum) in source_checksums.iter().enumerate() {
        let path_present = checksum
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
        let sha = checksum.get("sha256").and_then(Value::as_str).unwrap_or("");
        if !path_present
            || sha.len() != 64
            || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
            || sha.bytes().all(|byte| byte == b'0')
        {
            return Err(format!(
                "release artifact {suffix} source_checksums[{index}] must contain a non-blank \
                 path and nonzero 64-hex sha256"
            ));
        }
    }
    let recorded = object
        .get("commit_sha")
        .or_else(|| object.get("source_commit_sha"))
        .or_else(|| object.get("commit"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let Some(recorded) = recorded else {
        return Err(format!(
            "release artifact {suffix} is missing required commit provenance"
        ));
    };
    if recorded != expected_commit {
        return Err(format!(
            "release artifact {suffix} was generated for commit {recorded}, expected {expected_commit}"
        ));
    }

    let Some(timestamp) = object.get("timestamp_unix_seconds").and_then(Value::as_u64) else {
        return Err(format!(
            "release artifact {suffix} is missing required numeric timestamp_unix_seconds \
             provenance"
        ));
    };
    let now = current_unix_seconds();
    if timestamp > now.saturating_add(RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS) {
        return Err(format!(
            "release artifact {suffix} timestamp {timestamp} is too far in the future relative \
             to {now}"
        ));
    }
    if now.saturating_sub(timestamp) > RELEASE_ARTIFACT_MAX_AGE_SECONDS {
        return Err(format!(
            "release artifact {suffix} timestamp {timestamp} is stale relative to {now}"
        ));
    }
    if matches!(suffix, "m5-benchmarks.json" | "m5-public-api-freeze.json") {
        validate_m5_release_provenance(value, suffix)?;
    }
    Ok(())
}

fn validate_m5_release_provenance(value: &Value, suffix: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .expect("release JSON object was checked before M5 provenance");
    for field in ["toolchain", "profile", "producing_command"] {
        if object
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|entry| entry.trim().is_empty())
        {
            return Err(format!(
                "release artifact {suffix} is missing required non-blank {field} provenance"
            ));
        }
    }
    if object
        .get("sample_count")
        .and_then(Value::as_u64)
        .is_none_or(|count| count == 0)
    {
        return Err(format!(
            "release artifact {suffix} is missing required positive numeric sample_count provenance"
        ));
    }
    let payload_sha256 = object
        .get("payload_sha256")
        .and_then(Value::as_str)
        .unwrap_or("");
    if payload_sha256.len() != 64
        || !payload_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || payload_sha256.bytes().all(|byte| byte == b'0')
    {
        return Err(format!(
            "release artifact {suffix} is missing required nonzero 64-hex payload_sha256 provenance"
        ));
    }
    let expected = release_payload_sha256(value)?;
    if payload_sha256 != expected {
        return Err(format!(
            "release artifact {suffix} payload_sha256 does not match its content: recorded {payload_sha256}, expected {expected}"
        ));
    }
    Ok(())
}

fn release_payload_sha256(value: &Value) -> Result<String, String> {
    let mut payload = value.clone();
    payload
        .as_object_mut()
        .ok_or_else(|| "release payload must be a JSON object".to_string())?
        .remove("payload_sha256");
    let bytes = serde_json::to_vec(&payload)
        .map_err(|error| format!("failed to serialize release payload for hashing: {error}"))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn checksum_entries<'a>(
    sources: impl IntoIterator<Item = (&'a Path, &'a str)>,
) -> Result<Vec<Value>, String> {
    sources
        .into_iter()
        .map(|(path, label)| {
            Ok(json!({
                "path": label,
                "sha256": sha256_hex(path).map_err(|error| {
                    format!("failed to hash staging source {}: {error}", path.display())
                })?,
            }))
        })
        .collect()
}
