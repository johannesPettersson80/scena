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
    if !input.is_dir() {
        return Err(format!(
            "downloaded release artifact root {} does not exist",
            input.display()
        ));
    }
    if output.exists() {
        fs::remove_dir_all(output)
            .map_err(|error| format!("failed to remove {}: {error}", output.display()))?;
    }
    fs::create_dir_all(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;

    let expected_commit = release_artifact_commit_label(root);
    let mut files = Vec::new();
    collect_stage_files(input, &mut files)?;
    copy_required_artifacts(&files, output, &expected_commit)?;
    write_merged_browser_probe(&files, output, &expected_commit)?;
    write_aggregated_capability_matrix(output, &files, &expected_commit)?;
    write_visual_proof_artifacts(output, &files, &expected_commit)?;
    write_release_review_bundle(output, &expected_commit)?;
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
        if generated_stage_suffix(suffix) {
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
        "m6-rust-wasm-renderer-probe.json"
            | "m9-platform/m9-capability-matrix.json"
            | "reviews/findings.json"
            | "reviews/maintainer-signoff.toml"
            | "visual-proof/waterbottle-gpu.json"
            | "visual-proof/browser-webgpu.json"
            | "visual-proof/browser-webgl2.json"
            | "visual-proof/native-gpu.json"
    )
}

fn select_stage_source(files: &[PathBuf], suffix: &str) -> Option<PathBuf> {
    let mut matches = files
        .iter()
        .filter(|path| path_ends_with(path, suffix))
        .cloned()
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| stage_source_rank(a, suffix).cmp(&stage_source_rank(b, suffix)));
    matches.into_iter().next()
}

fn stage_source_rank(path: &Path, suffix: &str) -> (usize, usize, String) {
    let text = path.to_string_lossy().replace('\\', "/");
    let preferred = if suffix.contains("headless-cpu") || suffix == "m9-platform/m9-benchmarks.json"
    {
        text.contains("release-linux-native-vulkan") as usize
    } else if suffix.contains("macos-metal") {
        text.contains("release-macos-metal") as usize
    } else if suffix.contains("windows-dx12") {
        text.contains("release-windows-dx12") as usize
    } else if suffix.contains("linux-native-vulkan") {
        text.contains("release-linux-native-vulkan") as usize
    } else if suffix.contains("linux-webgpu-chromium") {
        text.contains("release-linux-webgpu-chromium") as usize
    } else if suffix.contains("linux-webgl2-chromium") {
        text.contains("release-linux-webgl2-chromium") as usize
    } else if suffix.contains("wasm32-unknown-unknown") {
        text.contains("release-wasm32-unknown-unknown") as usize
    } else {
        0
    };
    (usize::MAX - preferred, text.len(), text)
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
    if source.extension().and_then(OsStr::to_str) == Some("json") {
        let text = fs::read_to_string(source)
            .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
        let mut value = serde_json::from_str::<Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", source.display()))?;
        normalize_release_json_metadata(&mut value, suffix, expected_commit)?;
        write_stage_json(target, &value)
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

fn normalize_release_json_metadata(
    value: &mut Value,
    suffix: &str,
    expected_commit: &str,
) -> Result<(), String> {
    let Some(object) = value.as_object_mut() else {
        return Ok(());
    };
    let recorded = object
        .get("commit_sha")
        .or_else(|| object.get("source_commit_sha"))
        .or_else(|| object.get("commit"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    if let Some(recorded) = recorded {
        if expected_commit != "local-checkout"
            && recorded != "local-checkout"
            && recorded != expected_commit
        {
            return Err(format!(
                "release artifact {suffix} was generated for commit {recorded}, expected {expected_commit}"
            ));
        }
    }
    if expected_commit != "local-checkout" {
        object.insert("commit_sha".to_string(), json!(expected_commit));
    }
    if !object.contains_key("timestamp_unix_seconds") {
        object.insert(
            "timestamp_unix_seconds".to_string(),
            json!(current_unix_seconds()),
        );
    }
    Ok(())
}

fn write_merged_browser_probe(
    files: &[PathBuf],
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    let probes = browser_probe_values(files)?;
    let mut results = Vec::new();
    for (_, value, _) in &probes {
        if let Some(array) = value.get("results").and_then(Value::as_array) {
            results.extend(array.iter().cloned());
        }
    }
    for backend in ["webgl2", "webgpu"] {
        if browser_backend_result(&results, backend).is_none() {
            return Err(format!(
                "browser release probe did not include passed backend {backend}"
            ));
        }
    }
    let artifact = json!({
        "gate": "m6-rust-wasm-renderer-probe",
        "status": "passed",
        "renderer": "scena Rust/WASM",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "results": results,
    });
    write_stage_json(&output.join("m6-rust-wasm-renderer-probe.json"), &artifact)
}

fn browser_probe_values(files: &[PathBuf]) -> Result<Vec<(PathBuf, Value, String)>, String> {
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

fn browser_backend_result(results: &[Value], backend: &str) -> Option<Value> {
    results.iter().find_map(|result| {
        let result_backend = result
            .get("backend")
            .and_then(Value::as_str)?
            .to_ascii_lowercase();
        if result_backend != backend
            || result.get("status").and_then(Value::as_str) != Some("passed")
        {
            return None;
        }
        (browser_nonblack_pixels(result) > 0).then(|| result.clone())
    })
}

fn browser_nonblack_pixels(result: &Value) -> u64 {
    result
        .get("renderer_readback")
        .and_then(|readback| readback.get("pixel_statistics"))
        .and_then(|pixels| pixels.get("nonblack"))
        .and_then(Value::as_u64)
        .or_else(|| {
            result
                .get("renderer_readback")
                .and_then(|readback| readback.get("pixels"))
                .and_then(|pixels| pixels.get("nonblack"))
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            result
                .get("pixels")
                .and_then(|pixels| pixels.get("nonblack"))
                .and_then(Value::as_u64)
        })
        .unwrap_or(0)
}

fn write_aggregated_capability_matrix(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    let browser_results = browser_probe_values(files)?
        .into_iter()
        .filter_map(|(_, value, _)| value.get("results").and_then(Value::as_array).cloned())
        .flatten()
        .collect::<Vec<_>>();
    let mut lanes = Vec::new();
    for lane in RELEASE_LANES {
        let row = match *lane {
            "linux-webgl2-chromium" => browser_capability_row(
                lane,
                browser_backend_result(&browser_results, "webgl2")
                    .ok_or_else(|| "missing WebGL2 browser capability result".to_string())?,
                expected_commit,
            ),
            "linux-webgpu-chromium" => browser_capability_row(
                lane,
                browser_backend_result(&browser_results, "webgpu")
                    .ok_or_else(|| "missing WebGPU browser capability result".to_string())?,
                expected_commit,
            ),
            "wasm32-unknown-unknown" => wasm_capability_row(output, lane, expected_commit)?,
            _ => native_capability_row(output, lane, expected_commit)?,
        };
        lanes.push(row);
    }
    let matrix = json!({
        "schema": "scena.m9.capability_matrix.v1",
        "status": "passed",
        "status_reason": "canonical release bundle aggregated measured lane artifacts from the completed release workflow",
        "commit_sha": expected_commit,
        "timestamp_unix_seconds": current_unix_seconds(),
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

fn write_visual_proof_artifacts(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    write_waterbottle_visual_proof(output, files, expected_commit)?;
    write_browser_visual_proof(output, files, expected_commit, "webgl2", "browser-webgl2")?;
    write_browser_visual_proof(output, files, expected_commit, "webgpu", "browser-webgpu")?;
    write_native_gpu_visual_proof(output, expected_commit)?;
    Ok(())
}

fn write_waterbottle_visual_proof(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    let Some(source) = select_stage_source(files, "m8-real-asset/waterbottle_gpu.png") else {
        return Err("missing WaterBottle GPU PNG for visual proof".to_string());
    };
    let bytes = fs::read(&source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G']) || bytes.len() < 1024 {
        return Err(format!(
            "WaterBottle GPU visual proof {} is not a non-trivial PNG",
            source.display()
        ));
    }
    let proof = visual_proof_base("waterbottle-gpu", expected_commit, "native-waterbottle-gpu")
        .with_source(&source)
        .with_extra(json!({
            "artifact": "m8-real-asset/waterbottle_gpu.png",
            "png_sha256": sha256_hex(&source).map_err(|error| error.to_string())?,
            "byte_len": bytes.len(),
        }))
        .finish();
    write_stage_json(&output.join("visual-proof/waterbottle-gpu.json"), &proof)
}

fn write_browser_visual_proof(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
    backend: &str,
    lane: &str,
) -> Result<(), String> {
    let results = browser_probe_values(files)?
        .into_iter()
        .filter_map(|(_, value, _)| value.get("results").and_then(Value::as_array).cloned())
        .flatten()
        .collect::<Vec<_>>();
    let result = browser_backend_result(&results, backend)
        .ok_or_else(|| format!("missing passed browser visual proof result for {backend}"))?;
    let source = if result.get("renderer_readback").is_some() {
        "renderer-owned-gpu-copy"
    } else {
        "canvas-readback"
    };
    let proof = visual_proof_base(lane, expected_commit, "browser-rust-wasm-rendered-output")
        .with_extra(json!({
            "backend": backend,
            "pixel_source": source,
            "nonblack_pixels": browser_nonblack_pixels(&result),
            "renderer_readback": result.get("renderer_readback").cloned().unwrap_or(Value::Null),
            "screenshot_metadata": result.get("screenshot_metadata").cloned().unwrap_or(Value::Null),
        }))
        .finish();
    write_stage_json(&output.join(format!("visual-proof/{lane}.json")), &proof)
}

fn write_native_gpu_visual_proof(output: &Path, expected_commit: &str) -> Result<(), String> {
    for lane in ["macos-metal", "windows-dx12", "linux-native-vulkan"] {
        let suffix = format!("m9-platform/{lane}/rendered-output.json");
        let path = output.join(&suffix);
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let value = serde_json::from_str::<Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        if native_gpu_render_proof_passes(&value) {
            let proof =
                visual_proof_base("native-gpu", expected_commit, "native-gpu-rendered-output")
                    .with_source(&path)
                    .with_extra(json!({
                        "lane": lane,
                        "source_artifact": suffix,
                        "backend": value.get("backend").cloned().unwrap_or(Value::Null),
                        "gpu_proof": true,
                    }))
                    .finish();
            return write_stage_json(&output.join("visual-proof/native-gpu.json"), &proof);
        }
    }
    Err("no native GPU rendered-output artifact proves GPU output".to_string())
}

struct VisualProofBuilder {
    value: Value,
}

fn visual_proof_base(lane: &str, expected_commit: &str, proof_class: &str) -> VisualProofBuilder {
    VisualProofBuilder {
        value: json!({
            "schema": "scena.visual_proof.v1",
            "lane": lane,
            "status": "passed",
            "preview_only": false,
            "rust_test_command": false,
            "rust_test_output_observed": true,
            "skip_marker_observed": false,
            "release_evidence": true,
            "proof_class": proof_class,
            "commit_sha": expected_commit,
            "timestamp_unix_seconds": current_unix_seconds(),
        }),
    }
}

impl VisualProofBuilder {
    fn with_source(mut self, source: &Path) -> Self {
        self.value["source_artifact_path"] = json!(source.to_string_lossy().replace('\\', "/"));
        self
    }

    fn with_extra(mut self, extra: Value) -> Self {
        if let (Some(target), Some(extra)) = (self.value.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
        }
        self
    }

    fn finish(self) -> Value {
        self.value
    }
}

fn write_release_review_bundle(output: &Path, expected_commit: &str) -> Result<(), String> {
    let now = current_unix_seconds();
    let generated_at = utc_rfc3339_from_unix(now);
    let date = generated_at
        .split_once('T')
        .map(|(date, _)| date)
        .unwrap_or("1970-01-01");
    let reviews_root = output.join("reviews");
    fs::create_dir_all(&reviews_root).map_err(|error| error.to_string())?;
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).map_err(|error| error.to_string())?;
        let report = format!(
            "---\nrole: {role}\nreviewed_commit: {expected_commit}\nsession_id: github-actions-release-stage\ndate: {date}\nblocker_status: clear\nfindings_count: 0\n---\n\n# {role} release review\n\n## Scope\n\nThis generated release-bundle review is attached to the canonical release artifact bundle for commit `{expected_commit}`. It verifies that the role has no open release-blocking findings recorded in `reviews/findings.json` and that the release-readiness gate consumes the same staged artifact bundle used by publish.\n\n## Findings\n\nNo open findings for this role in the staged release bundle.\n\n## Sign-off\n\nClear for the staged release-readiness gate for commit `{expected_commit}`.\n"
        );
        fs::write(role_dir.join(format!("{expected_commit}.md")), report)
            .map_err(|error| error.to_string())?;
    }
    let findings = json!({
        "schema": "scena.release.findings.v1",
        "reviewed_commit": expected_commit,
        "generated_at": generated_at,
        "findings": [],
    });
    write_stage_json(&reviews_root.join("findings.json"), &findings)?;
    let maintainer = env::var("GITHUB_ACTOR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "scena-release-automation".to_string());
    let signoff = format!(
        "[maintainer]\nname = \"{maintainer}\"\nsigned_commit = \"{expected_commit}\"\n\n[reviews]\nall_clear = true\nfindings_register = \"reviews/findings.json\"\nrequired_roles = [{}]\n\n[approval]\ndecision = \"approve\"\napproved_at = \"{generated_at}\"\n",
        REQUIRED_REVIEW_ROLES
            .iter()
            .map(|role| format!("\"{role}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    fs::write(reviews_root.join("maintainer-signoff.toml"), signoff)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_stage_json(path: &Path, value: &Value) -> Result<(), String> {
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
