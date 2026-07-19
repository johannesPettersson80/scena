use crate::app::prelude::*;

const RELEASE_LANES: &[&str] = &[
    "linux-native-vulkan",
    "headless-cpu",
    "linux-webgl2-chromium",
    "linux-webgpu-chromium",
    "wasm32-unknown-unknown",
    "macos-metal",
    "windows-dx12",
];

pub(crate) fn run_stage_release_artifacts(input: &str, output: &str) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("RELEASE-STAGE", message)])?;
    let input = resolve_stage_path(&root, input);
    let output = resolve_stage_path(&root, output);
    stage_release_artifacts(&root, &input, &output)
        .map_err(|message| vec![Finding::new("RELEASE-STAGE", message)])?;
    println!("{}", output.display());
    Ok(())
}

fn resolve_stage_path(root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

pub(crate) fn stage_release_artifacts(
    root: &Path,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    let expected_commit = release_artifact_commit_label(root);
    stage_release_artifacts_for_commit(input, output, &expected_commit)
}

pub(crate) fn stage_release_artifacts_for_commit(
    input: &Path,
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    validate_release_commit_label(expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-SOURCE-COMMIT", error))?;
    if !input.is_dir() {
        return Err(coded_stage_error(
            "RELEASE-SOURCE-ROOT",
            format!(
                "downloaded release artifact root {} does not exist",
                input.display()
            ),
        ));
    }
    if output.exists() {
        fs::remove_dir_all(output)
            .map_err(|error| format!("failed to remove {}: {error}", output.display()))?;
    }
    fs::create_dir_all(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;

    let mut files = Vec::new();
    collect_stage_files(input, &mut files)
        .map_err(|error| coded_stage_error("RELEASE-SOURCE-ROOT", error))?;
    copy_required_artifacts(&files, output, expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-SOURCE-EVIDENCE", error))?;
    write_merged_browser_probe(&files, output, expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-BROWSER-PROOF", error))?;
    write_aggregated_capability_matrix(output, &files, expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-CAPABILITY-AGGREGATION", error))?;
    super::stage_visual_proofs::write_visual_proof_artifacts(output, &files, expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-VISUAL-PROOF", error))?;
    super::stage_reviews::copy_and_validate_required_reviews(&files, output, expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-REVIEWS", error))?;
    write_staging_metadata(output, expected_commit)
        .map_err(|error| coded_stage_error("RELEASE-STAGING-METADATA", error))?;
    Ok(())
}

fn coded_stage_error(code: &str, error: impl AsRef<str>) -> String {
    let error = error.as_ref();
    if error.starts_with("RELEASE-") {
        error.to_string()
    } else {
        format!("{code}: {error}")
    }
}

fn validate_release_commit_label(commit: &str) -> Result<(), String> {
    if commit == "local-checkout" {
        return Err(
            "release staging rejects local-checkout provenance; provide an exact source commit"
                .to_string(),
        );
    }
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "release staging requires an exact 40-hex source commit, got {commit:?}"
        ));
    }
    Ok(())
}

fn collect_stage_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_stage_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}

fn copy_required_artifacts(
    files: &[PathBuf],
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    for suffix in REQUIRED_RELEASE_ARTIFACT_SUFFIXES {
        if generated_stage_suffix(suffix) || suffix.starts_with("reviews/") {
            continue;
        }
        let Some(source) = select_stage_source(files, suffix) else {
            return Err(format!(
                "downloaded release artifacts are missing required source {suffix}"
            ));
        };
        copy_stage_file(&source, &output.join(suffix), suffix, expected_commit)?;
    }
    Ok(())
}

fn generated_stage_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "staging-metadata.json"
            | "m6-rust-wasm-renderer-probe.json"
            | "m9-platform/m9-capability-matrix.json"
            | "visual-proof/waterbottle-gpu.json"
            | "visual-proof/waterbottle-cpu.json"
            | "visual-proof/browser-webgpu.json"
            | "visual-proof/browser-webgl2.json"
            | "visual-proof/native-gpu.json"
    )
}

fn write_staging_metadata(output: &Path, expected_commit: &str) -> Result<(), String> {
    let staged_at_unix_seconds = current_unix_seconds();
    let metadata = json!({
        "schema": "scena.release.staging.v1",
        "status": "passed",
        "producer": "cargo run -p xtask -- stage-release-artifacts",
        "source_commit_sha": expected_commit,
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": staged_at_unix_seconds,
        "staged_at": utc_rfc3339_from_unix(staged_at_unix_seconds),
        "staging_checkout": expected_commit,
        "staging_tool": "scena-xtask",
        "staging_tool_version": env!("CARGO_PKG_VERSION"),
    });
    write_stage_json(&output.join("staging-metadata.json"), &metadata)
}

pub(super) fn select_stage_source(files: &[PathBuf], suffix: &str) -> Option<PathBuf> {
    let mut matches = files
        .iter()
        .filter(|path| path_ends_with(path, suffix))
        .cloned()
        .collect::<Vec<_>>();
    matches.sort_by_key(|path| stage_source_rank(path, suffix));
    matches.into_iter().next()
}

fn stage_source_rank(path: &Path, suffix: &str) -> (usize, usize, String) {
    let text = path.to_string_lossy().replace('\\', "/");
    let preferred = if suffix.contains("headless-cpu")
        || suffix.starts_with("q01-waterbottle-cpu/")
        || suffix == "m9-platform/m9-benchmarks.json"
        || suffix == "m9-platform/m9-benchmarks-feature-matrix.json"
    {
        stage_source_matches_lane(&text, "linux-native-vulkan") as usize
    } else if suffix.contains("macos-metal") {
        stage_source_matches_lane(&text, "macos-metal") as usize
    } else if suffix.contains("windows-dx12") {
        stage_source_matches_lane(&text, "windows-dx12") as usize
    } else if suffix.contains("linux-native-vulkan") {
        stage_source_matches_lane(&text, "linux-native-vulkan") as usize
    } else if suffix.contains("linux-webgpu-chromium") {
        stage_source_matches_lane(&text, "linux-webgpu-chromium") as usize
    } else if suffix.contains("linux-webgl2-chromium") {
        stage_source_matches_lane(&text, "linux-webgl2-chromium") as usize
    } else if suffix.contains("wasm32-unknown-unknown") {
        stage_source_matches_lane(&text, "wasm32-unknown-unknown") as usize
    } else {
        0
    };
    (usize::MAX - preferred, text.len(), text)
}

fn stage_source_matches_lane(path: &str, lane: &str) -> bool {
    let ci_artifact = match lane {
        "linux-native-vulkan" => "linux-native-vulkan-gate-artifacts",
        "linux-webgpu-chromium" => "linux-browser-webgpu-gate-artifacts",
        "linux-webgl2-chromium" => "linux-browser-webgl2-gate-artifacts",
        "wasm32-unknown-unknown" => "wasm32-package",
        "macos-metal" => "macos-metal-gate-artifacts",
        "windows-dx12" => "windows-dx12-gate-artifacts",
        _ => return false,
    };
    let release_artifact = format!("release-{lane}");
    path.split('/')
        .any(|component| component == release_artifact || component == ci_artifact)
}

fn copy_stage_file(
    source: &Path,
    target: &Path,
    suffix: &str,
    expected_commit: &str,
) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if suffix.starts_with("reviews/") {
        fs::copy(source, target).map_err(|error| {
            format!(
                "failed to copy independent review evidence {} to {}: {error}",
                source.display(),
                target.display()
            )
        })?;
        Ok(())
    } else if source.extension().and_then(OsStr::to_str) == Some("json") {
        let text = fs::read_to_string(source)
            .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
        let value = serde_json::from_str::<Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", source.display()))?;
        super::stage_provenance::validate_release_json_metadata(&value, suffix, expected_commit)?;
        fs::copy(source, target).map_err(|error| {
            format!(
                "failed to copy provenance-bearing release artifact {} to {}: {error}",
                source.display(),
                target.display()
            )
        })?;
        Ok(())
    } else {
        fs::copy(source, target).map_err(|error| {
            format!(
                "failed to copy {} to {}: {error}",
                source.display(),
                target.display()
            )
        })?;
        Ok(())
    }
}

fn write_merged_browser_probe(
    files: &[PathBuf],
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    let probes = browser_probe_values(files)?;
    let results = probes
        .iter()
        .filter_map(|(_, value, _)| {
            value
                .get("release_results")
                .or_else(|| value.get("results"))
                .and_then(Value::as_array)
        })
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    for backend in ["webgl2", "webgpu"] {
        validate_browser_backend_result(&results, backend)?;
    }
    let source_checksums = super::stage_provenance::checksum_entries(
        probes
            .iter()
            .map(|(path, _, label)| (path.as_path(), label.as_str())),
    )?;
    let artifact = json!({
        "schema": "scena.m6.rust_wasm_renderer_probe.aggregate.v1",
        "gate": "m6-rust-wasm-renderer-probe",
        "status": "passed",
        "renderer": "scena Rust/WASM",
        "producer": "cargo run -p xtask -- stage-release-artifacts",
        "evidence_phase": "staging-aggregation",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "source_checksums": source_checksums,
        "results": results,
    });
    write_stage_json(&output.join("m6-rust-wasm-renderer-probe.json"), &artifact)
}

pub(super) fn browser_probe_values(
    files: &[PathBuf],
) -> Result<Vec<(PathBuf, Value, String)>, String> {
    let mut probes = Vec::new();
    for path in files
        .iter()
        .filter(|path| path_ends_with(path, "m6-rust-wasm-renderer-probe.json"))
    {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let value = serde_json::from_str::<Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        probes.push((
            path.clone(),
            value,
            path.to_string_lossy().replace('\\', "/"),
        ));
    }
    if probes.is_empty() {
        return Err("downloaded release artifacts contain no browser probe JSON".to_string());
    }
    Ok(probes)
}

pub(super) fn browser_release_results(files: &[PathBuf]) -> Result<Vec<Value>, String> {
    Ok(browser_probe_values(files)?
        .into_iter()
        .filter_map(|(_, value, _)| {
            value
                .get("release_results")
                .or_else(|| value.get("results"))
                .and_then(Value::as_array)
                .cloned()
        })
        .flatten()
        .collect())
}

pub(crate) fn validate_browser_backend_result(
    results: &[Value],
    backend: &str,
) -> Result<Value, String> {
    let matches = results
        .iter()
        .filter(|result| {
            result
                .get("backend")
                .and_then(Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(backend))
        })
        .collect::<Vec<_>>();
    let result = match matches.as_slice() {
        [] => {
            return Err(format!(
                "browser release probe is missing backend {backend}"
            ));
        }
        [result] => *result,
        _ => {
            return Err(format!(
                "browser release probe has {} ambiguous results for backend {backend}",
                matches.len()
            ));
        }
    };
    if result.get("status").and_then(Value::as_str) != Some("passed") {
        return Err(format!(
            "browser release probe backend {backend} did not pass"
        ));
    }
    let readback = result.get("renderer_readback").ok_or_else(|| {
        format!(
            "browser release probe backend {backend} is missing renderer_readback; \
             renderer-owned-gpu-copy is required"
        )
    })?;
    if readback.get("source").and_then(Value::as_str) != Some("renderer-owned-gpu-copy") {
        return Err(format!(
            "browser release probe backend {backend} must use \
             renderer_readback.source=renderer-owned-gpu-copy"
        ));
    }
    let width = readback.get("width").and_then(Value::as_u64).unwrap_or(0);
    let height = readback.get("height").and_then(Value::as_u64).unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(format!(
            "browser release probe backend {backend} renderer-owned readback must record positive \
             width and height"
        ));
    }
    if browser_nonblack_pixels(result) == 0 {
        return Err(format!(
            "browser release probe backend {backend} renderer-owned readback has zero nonblack \
             pixels"
        ));
    }
    let checksum = readback
        .get("rgba8_fnv1a64")
        .and_then(Value::as_str)
        .unwrap_or("");
    if checksum.len() != 16
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || checksum.bytes().all(|byte| byte == b'0')
    {
        return Err(format!(
            "browser release probe backend {backend} renderer-owned readback has an invalid or \
             zero rgba8_fnv1a64 checksum"
        ));
    }
    if backend.eq_ignore_ascii_case("webgl2") {
        super::stage_browser_parity::validate_cpu_webgl2_parity(result, readback)?;
    }
    Ok(result.clone())
}

pub(super) fn browser_nonblack_pixels(result: &Value) -> u64 {
    result
        .get("renderer_readback")
        .and_then(|readback| readback.get("pixel_statistics"))
        .and_then(|pixels| pixels.get("nonblack"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn write_aggregated_capability_matrix(
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
    let source_checksums = super::stage_provenance::checksum_entries(
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
        "capabilities": {
            "wasm_bundle": value,
        },
    }))
}

pub(super) fn write_stage_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, format!("{body}\n")).map_err(|error| {
        format!(
            "failed to write staged release artifact {}: {error}",
            path.display()
        )
    })
}

pub(crate) fn utc_rfc3339_from_unix(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::select_stage_source;
    use std::path::PathBuf;

    #[test]
    fn q01_stage_source_prefers_the_finalized_headless_cpu_producer() {
        let suffix = "q01-waterbottle-cpu/result.json";
        for (layout, linux_root, macos_root, windows_root) in [
            (
                "premerge",
                "linux-native-vulkan-gate-artifacts",
                "macos-metal-gate-artifacts",
                "windows-dx12-gate-artifacts",
            ),
            (
                "release",
                "release-linux-native-vulkan",
                "release-macos-metal",
                "release-windows-dx12",
            ),
        ] {
            let expected = PathBuf::from(format!("{linux_root}/{suffix}"));
            let files = vec![
                PathBuf::from(format!("{macos_root}/{suffix}")),
                PathBuf::from(format!("{windows_root}/{suffix}")),
                expected.clone(),
            ];

            assert_eq!(
                select_stage_source(&files, suffix),
                Some(expected),
                "{layout} staging must select the finalized Linux headless-CPU result",
            );
        }
    }
}
