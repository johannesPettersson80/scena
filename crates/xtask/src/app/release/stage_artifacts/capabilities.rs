use crate::app::prelude::*;

use super::RELEASE_LANES;
use super::{browser_release_results, validate_browser_backend_result, write_stage_json};

pub(super) fn write_aggregated_capability_matrix(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    let browser_results = browser_release_results(files)?;
    let mut lanes = Vec::new();
    for lane in RELEASE_LANES {
        let row = match *lane {
            "linux-webgl2-chromium" => browser_capability_row(
                lane,
                validate_browser_backend_result(&browser_results, "webgl2")?,
                expected_commit,
            ),
            "linux-webgpu-chromium" => browser_capability_row(
                lane,
                validate_browser_backend_result(&browser_results, "webgpu")?,
                expected_commit,
            ),
            "wasm32-unknown-unknown" => wasm_capability_row(output, lane, expected_commit)?,
            _ => native_capability_row(output, lane, expected_commit)?,
        };
        lanes.push(row);
    }
    let source_paths = [
        "m6-rust-wasm-renderer-probe.json",
        "m9-wasm-size.json",
        "m9-platform/linux-native-vulkan/capabilities.json",
        "m9-platform/headless-cpu/capabilities.json",
        "m9-platform/macos-metal/capabilities.json",
        "m9-platform/windows-dx12/capabilities.json",
    ]
    .into_iter()
    .map(|relative| (output.join(relative), relative.to_string()))
    .filter(|(path, _)| path.is_file())
    .collect::<Vec<_>>();
    let source_checksums = super::super::stage_provenance::checksum_entries(
        source_paths
            .iter()
            .map(|(path, label)| (path.as_path(), label.as_str())),
    )?;
    let matrix = json!({
        "schema": "scena.m9.capability_matrix.v1",
        "status": "passed",
        "status_reason": "canonical release bundle aggregated measured lane artifacts from the completed release workflow",
        "producer": "cargo run -p xtask -- stage-release-artifacts",
        "evidence_phase": "staging-aggregation",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "source_checksums": source_checksums,
        "lanes": lanes,
    });
    write_stage_json(
        &output.join("m9-platform/m9-capability-matrix.json"),
        &matrix,
    )
}

fn native_capability_row(
    output: &Path,
    lane: &str,
    expected_commit: &str,
) -> Result<Value, String> {
    let suffix = format!("m9-platform/{lane}/capabilities.json");
    let path = output.join(&suffix);
    let text =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {suffix}: {error}"))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("failed to parse {suffix}: {error}"))?;
    Ok(json!({
        "lane": lane,
        "status": "measured",
        "measurement_source": "lane-renderer-runtime",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "backend": value.get("backend").cloned().unwrap_or(Value::Null),
        "adapter": value.get("adapter").cloned().unwrap_or(Value::Null),
        "host_gpu_available": value
            .get("adapter")
            .and_then(|adapter| adapter.get("available"))
            .cloned()
            .unwrap_or(Value::Bool(false)),
        "capabilities": value.get("features").cloned().unwrap_or(Value::Null),
        "diagnostics": value.get("diagnostics").cloned().unwrap_or_else(|| json!([])),
    }))
}

fn browser_capability_row(lane: &str, result: Value, expected_commit: &str) -> Value {
    json!({
        "lane": lane,
        "status": "measured",
        "measurement_source": "browser-probe-runtime",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "backend": result.get("backend").cloned().unwrap_or(Value::Null),
        "capabilities": result.get("capabilities").cloned().unwrap_or(Value::Null),
        "pixel_statistics": result
            .get("renderer_readback")
            .and_then(|readback| readback.get("pixel_statistics"))
            .cloned()
            .or_else(|| result.get("pixels").cloned())
            .unwrap_or(Value::Null),
    })
}

fn wasm_capability_row(output: &Path, lane: &str, expected_commit: &str) -> Result<Value, String> {
    let path = output.join("m9-wasm-size.json");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read m9-wasm-size.json: {error}"))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("failed to parse m9-wasm-size.json: {error}"))?;
    Ok(json!({
        "lane": lane,
        "status": "measured",
        "measurement_source": "wasm-size-gate-runtime",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "capabilities": { "wasm_bundle": value },
    }))
}
