use crate::app::prelude::*;

pub(crate) fn finalize_waterbottle_gpu_result(root: &Path) -> Result<(), String> {
    let result_path = root.join("target/gate-artifacts/m8-real-asset/waterbottle_gpu_result.json");
    let png_path = root.join("target/gate-artifacts/m8-real-asset/waterbottle_gpu.png");
    let command_path = root.join("target/gate-artifacts/release-lanes/macos-metal.commands.jsonl");
    let log_path = root.join("target/gate-artifacts/release-lanes/macos-metal.log");
    for path in [&result_path, &png_path, &command_path, &log_path] {
        if !path.is_file() {
            return Err(format!(
                "WaterBottle GPU release finalization is missing {}",
                path.display()
            ));
        }
    }
    let command_text = fs::read_to_string(&command_path)
        .map_err(|error| format!("failed to read {}: {error}", command_path.display()))?;
    let expected_command = "cargo test --test m8_real_asset_proof \
                            m8_real_asset_waterbottle_gpu_headline -- --exact";
    let mut exact_command_passed = false;
    for (index, line) in command_text.lines().enumerate() {
        let record = serde_json::from_str::<Value>(line).map_err(|error| {
            format!(
                "failed to parse {} line {}: {error}",
                command_path.display(),
                index + 1
            )
        })?;
        if record.get("command").and_then(Value::as_str) == Some(expected_command) {
            let duration_is_numeric = record.get("duration_ms").and_then(Value::as_u64).is_some();
            let expected_log = record.get("failure_log_path").and_then(Value::as_str)
                == Some("target/gate-artifacts/release-lanes/macos-metal.log");
            exact_command_passed = record.get("status").and_then(Value::as_str) == Some("passed")
                && duration_is_numeric
                && expected_log;
        }
    }
    if !exact_command_passed {
        return Err(format!(
            "WaterBottle GPU finalization requires the exact passed command record \
             {expected_command:?} with measured duration and canonical command log"
        ));
    }
    let log = fs::read_to_string(&log_path)
        .map_err(|error| format!("failed to read {}: {error}", log_path.display()))?;
    for marker in [
        "running 1 test",
        "test m8_real_asset_waterbottle_gpu_headline ... ok",
        "test result: ok. 1 passed",
    ] {
        if !log.contains(marker) {
            return Err(format!(
                "WaterBottle GPU command log does not prove the exact test passed: missing \
                 {marker:?}"
            ));
        }
    }
    let text = fs::read_to_string(&result_path)
        .map_err(|error| format!("failed to read {}: {error}", result_path.display()))?;
    let mut value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("failed to parse {}: {error}", result_path.display()))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "WaterBottle GPU result must be a JSON object".to_string())?;
    let png_sha = sha256_hex(&png_path).map_err(|error| error.to_string())?;
    if object.get("png_sha256").and_then(Value::as_str) != Some(&png_sha) {
        return Err("WaterBottle GPU result PNG hash does not match the produced PNG".to_string());
    }
    let command_sha = sha256_hex(&command_path).map_err(|error| error.to_string())?;
    let log_sha = sha256_hex(&log_path).map_err(|error| error.to_string())?;
    object.insert("command_record_sha256".to_string(), json!(command_sha));
    object.insert("rust_test_output_observed".to_string(), json!(true));
    object.insert(
        "source_checksums".to_string(),
        json!([
            {"path":"m8-real-asset/waterbottle_gpu.png", "sha256":png_sha},
            {"path":"release-lanes/macos-metal.commands.jsonl", "sha256":command_sha},
            {"path":"release-lanes/macos-metal.log", "sha256":log_sha}
        ]),
    );
    let body = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    fs::write(&result_path, format!("{body}\n"))
        .map_err(|error| format!("failed to write {}: {error}", result_path.display()))
}

pub(crate) fn finalize_waterbottle_cpu_result(root: &Path) -> Result<(), String> {
    let artifact_root = root.join("target/gate-artifacts");
    let result_path = artifact_root.join("q01-waterbottle-cpu/result.json");
    let command_path = artifact_root.join("release-lanes/headless-cpu.commands.jsonl");
    let log_path = artifact_root.join("release-lanes/headless-cpu.log");
    let png_paths = [
        "q01-waterbottle-cpu/live.png",
        "q01-waterbottle-cpu/known_bad_flattened_chrome.png",
        "q01-waterbottle-cpu/known_bad_wrong_material.png",
        "q01-waterbottle-cpu/known_bad_wrong_camera.png",
    ];
    for path in [&result_path, &command_path, &log_path] {
        if !path.is_file() {
            return Err(format!(
                "WaterBottle CPU release finalization is missing {}",
                path.display()
            ));
        }
    }
    for relative in png_paths {
        let path = artifact_root.join(relative);
        if !path.is_file() {
            return Err(format!(
                "WaterBottle CPU release finalization is missing {}",
                path.display()
            ));
        }
    }

    let expected_command = "cargo test --test q01_waterbottle_cpu_reference \
                            q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact";
    let command_text = fs::read_to_string(&command_path)
        .map_err(|error| format!("failed to read {}: {error}", command_path.display()))?;
    let mut exact_command_passed = false;
    for (index, line) in command_text.lines().enumerate() {
        let record = serde_json::from_str::<Value>(line).map_err(|error| {
            format!(
                "failed to parse {} line {}: {error}",
                command_path.display(),
                index + 1
            )
        })?;
        if record.get("command").and_then(Value::as_str) == Some(expected_command) {
            exact_command_passed = record.get("status").and_then(Value::as_str) == Some("passed")
                && record.get("duration_ms").and_then(Value::as_u64).is_some()
                && record.get("failure_log_path").and_then(Value::as_str)
                    == Some("target/gate-artifacts/release-lanes/headless-cpu.log");
        }
    }
    if !exact_command_passed {
        return Err(format!(
            "WaterBottle CPU finalization requires the exact passed command record \
             {expected_command:?} with measured duration and canonical command log"
        ));
    }
    let log = fs::read_to_string(&log_path)
        .map_err(|error| format!("failed to read {}: {error}", log_path.display()))?;
    for marker in [
        "running 1 test",
        "test q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders ... ok",
        "test result: ok. 1 passed",
    ] {
        if !log.contains(marker) {
            return Err(format!(
                "WaterBottle CPU command log does not prove the exact test passed: missing \
                 {marker:?}"
            ));
        }
    }

    let text = fs::read_to_string(&result_path)
        .map_err(|error| format!("failed to read {}: {error}", result_path.display()))?;
    let mut value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("failed to parse {}: {error}", result_path.display()))?;
    validate_waterbottle_cpu_result(&artifact_root, &value)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "WaterBottle CPU result must be a JSON object".to_string())?;
    let command_sha = sha256_hex(&command_path).map_err(|error| error.to_string())?;
    let log_sha = sha256_hex(&log_path).map_err(|error| error.to_string())?;
    object.insert("command_record_sha256".to_string(), json!(command_sha));
    object.insert("rust_test_output_observed".to_string(), json!(true));
    let checksums = object
        .entry("source_checksums")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| "WaterBottle CPU source_checksums must be an array".to_string())?;
    checksums.retain(|entry| {
        !matches!(
            entry.get("path").and_then(Value::as_str),
            Some("release-lanes/headless-cpu.commands.jsonl" | "release-lanes/headless-cpu.log")
        )
    });
    checksums.push(json!({
        "path":"release-lanes/headless-cpu.commands.jsonl",
        "sha256":command_sha
    }));
    checksums.push(json!({
        "path":"release-lanes/headless-cpu.log",
        "sha256":log_sha
    }));
    let body = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    fs::write(&result_path, format!("{body}\n"))
        .map_err(|error| format!("failed to write {}: {error}", result_path.display()))
}

pub(crate) fn validate_waterbottle_cpu_result(
    artifact_root: &Path,
    value: &Value,
) -> Result<(), String> {
    for (field, expected) in [
        ("schema", "scena.q01.waterbottle_cpu_reference.v1"),
        ("status", "passed"),
        (
            "test_name",
            "q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders",
        ),
        ("backend", "Headless"),
        ("adapter", "software-rasterizer"),
        ("color_type", "rgba8"),
        ("color_space", "srgb-output"),
        ("row_orientation", "top-to-bottom"),
        ("alpha_contract", "opaque"),
    ] {
        if value.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "WaterBottle CPU result {field} must be {expected:?}"
            ));
        }
    }
    if value.get("release_evidence").and_then(Value::as_bool) != Some(true)
        || value
            .get("metrics")
            .and_then(|metrics| metrics.get("passed"))
            .and_then(Value::as_bool)
            != Some(true)
        || value
            .get("timestamp_unix_seconds")
            .and_then(Value::as_u64)
            .is_none()
        || value.get("width").and_then(Value::as_u64) != Some(256)
        || value.get("height").and_then(Value::as_u64) != Some(256)
    {
        return Err(
            "WaterBottle CPU result must carry passed release evidence, timestamp, and 256x256 dimensions"
                .to_string(),
        );
    }
    let commit = value
        .get("commit_sha")
        .and_then(Value::as_str)
        .unwrap_or("");
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("WaterBottle CPU result must bind an exact 40-hex commit".to_string());
    }
    let live_path = artifact_root.join("q01-waterbottle-cpu/live.png");
    let live_sha = sha256_hex(&live_path).map_err(|error| error.to_string())?;
    if value.get("live_png_sha256").and_then(Value::as_str) != Some(&live_sha) {
        return Err(
            "WaterBottle CPU result live PNG hash does not match the produced PNG".to_string(),
        );
    }
    let mutations = value
        .get("mutations")
        .and_then(Value::as_array)
        .ok_or_else(|| "WaterBottle CPU result mutations must be an array".to_string())?;
    for (name, relative) in [
        (
            "flattened_chrome",
            "q01-waterbottle-cpu/known_bad_flattened_chrome.png",
        ),
        (
            "wrong_material",
            "q01-waterbottle-cpu/known_bad_wrong_material.png",
        ),
        (
            "wrong_camera",
            "q01-waterbottle-cpu/known_bad_wrong_camera.png",
        ),
    ] {
        let mutation = mutations
            .iter()
            .find(|mutation| mutation.get("name").and_then(Value::as_str) == Some(name))
            .ok_or_else(|| format!("WaterBottle CPU result is missing mutation {name}"))?;
        let actual_sha =
            sha256_hex(&artifact_root.join(relative)).map_err(|error| error.to_string())?;
        if mutation.get("path").and_then(Value::as_str) != Some(relative)
            || mutation.get("sha256").and_then(Value::as_str) != Some(&actual_sha)
            || mutation.get("oracle_rejected").and_then(Value::as_bool) != Some(true)
            || mutation.pointer("/metrics/passed").and_then(Value::as_bool) != Some(false)
        {
            return Err(format!(
                "WaterBottle CPU mutation {name} must bind its PNG and be rejected by the oracle"
            ));
        }
    }
    Ok(())
}
