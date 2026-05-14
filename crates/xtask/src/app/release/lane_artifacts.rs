use crate::app::prelude::*;

pub(crate) fn run_release_lane_artifact(lane: &str) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("RELEASE-LANE-ROOT", message)])?;
    let artifact = release_lane_artifact(&root, lane)
        .map_err(|message| vec![Finding::new("RELEASE-LANE", message)])?;
    let artifact_dir = root.join("target/gate-artifacts/release-lanes");
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        return Err(vec![Finding::new(
            "RELEASE-LANE",
            format!("failed to create {}: {error}", artifact_dir.display()),
        )]);
    }
    let artifact_path = artifact_dir.join(format!("{lane}.json"));
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| vec![Finding::new("RELEASE-LANE", error.to_string())])?;
    if let Err(error) = fs::write(&artifact_path, format!("{body}\n")) {
        return Err(vec![Finding::new(
            "RELEASE-LANE",
            format!("failed to write {}: {error}", artifact_path.display()),
        )]);
    }
    println!("{}", artifact_path.display());
    Ok(())
}

pub(crate) fn release_lane_artifact(root: &Path, lane: &str) -> Result<serde_json::Value, String> {
    let (os, backend) = match lane {
        "linux-native-vulkan" => ("ubuntu-24.04", "NativeSurface"),
        "headless-cpu" => ("ubuntu-24.04", "Headless"),
        "linux-webgl2-chromium" => ("ubuntu-24.04", "WebGl2"),
        "linux-webgpu-chromium" => ("ubuntu-24.04", "WebGpu"),
        "macos-metal" => ("macos-15", "Metal"),
        "windows-dx12" => ("windows-2025", "Dx12"),
        "wasm32-unknown-unknown" => ("ubuntu-24.04", "Wasm"),
        _ => return Err(format!("unknown release lane '{lane}'")),
    };
    let required_artifacts = release_lane_required_artifacts(lane);
    let evidence = required_artifacts
        .iter()
        .map(|rel| release_lane_evidence(root, rel))
        .collect::<Result<Vec<_>, _>>()?;
    let commands = release_lane_expected_commands(lane);
    let command_records = release_lane_command_records(root, lane, &commands, &evidence)?;
    let content_ok = release_lane_content_ok(root, lane)?;
    let commands_ok = release_lane_command_records_pass(&command_records);
    let status = if evidence
        .iter()
        .all(|entry| entry["exists"].as_bool().unwrap_or(false))
        && content_ok
        && commands_ok
    {
        "passed"
    } else {
        "incomplete"
    };
    Ok(json!({
        "schema": "scena.release_lane.v1",
        "lane": lane,
        "os": os,
        "backend": backend,
        "rustc": "1.93.1",
        "generated_at_unix_seconds": current_unix_seconds(),
        "commit": release_artifact_commit_label(root),
        "status": status,
        "required_artifacts": evidence,
        "content_ok": content_ok,
        "commands_ok": commands_ok,
        "commands": commands,
        "command_records": command_records,
        "note": "Lane status is passed only when the required local gate artifacts exist, are checksummed, and native GPU rendered-output proof is not CPU fallback. CI may populate command duration and failure-log fields through the same command_records schema."
    }))
}

pub(crate) fn release_lane_content_ok(root: &Path, lane: &str) -> Result<bool, String> {
    if lane == "headless-cpu" {
        let path = root.join("target/gate-artifacts/m9-platform/headless-cpu/rendered-output.json");
        if !path.is_file() {
            return Ok(false);
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let value = serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        return Ok(headless_cpu_render_proof_passes(&value));
    }
    if matches!(lane, "linux-webgl2-chromium" | "linux-webgpu-chromium") {
        let path = root.join("target/gate-artifacts/m6-rust-wasm-renderer-probe.json");
        if !path.is_file() {
            return Ok(false);
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let value = serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        return Ok(browser_probe_release_proof_passes(&value, lane));
    }
    if !matches!(lane, "macos-metal" | "windows-dx12") {
        return Ok(true);
    }
    let path = root.join(format!(
        "target/gate-artifacts/m9-platform/{lane}/rendered-output.json"
    ));
    if !path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(native_gpu_render_proof_passes(&value))
}

fn browser_probe_release_proof_passes(value: &serde_json::Value, lane: &str) -> bool {
    let expected_backend = match lane {
        "linux-webgl2-chromium" => "webgl2",
        "linux-webgpu-chromium" => "webgpu",
        _ => return false,
    };
    value.get("gate").and_then(serde_json::Value::as_str) == Some("m6-rust-wasm-renderer-probe")
        && value.get("status").and_then(serde_json::Value::as_str) == Some("passed")
        && value
            .get("results")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|results| {
                results.iter().any(|result| {
                    result
                        .get("backend")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|backend| {
                            backend.to_ascii_lowercase().replace("webgl2", "webgl2")
                                == expected_backend
                        })
                        && result.get("status").and_then(serde_json::Value::as_str)
                            == Some("passed")
                        && result
                            .get("pixels")
                            .and_then(|pixels| pixels.get("nonblack"))
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0)
                            > 0
                })
            })
}

pub(crate) fn release_lane_required_artifacts(lane: &str) -> Vec<String> {
    match lane {
        "headless-cpu" => [
            "target/gate-artifacts/m9-platform/headless-cpu/rendered-output.json".to_string(),
            "target/gate-artifacts/m9-platform/headless-cpu/capabilities.json".to_string(),
            "target/gate-artifacts/m9-platform/headless-cpu/default-scene.ppm".to_string(),
            "target/gate-artifacts/m9-platform/headless-cpu/static-gltf.ppm".to_string(),
            "target/gate-artifacts/m9-platform/m9-benchmarks.json".to_string(),
        ]
        .into_iter()
        .collect(),
        "linux-native-vulkan" | "macos-metal" | "windows-dx12" => [
            format!("target/gate-artifacts/m9-platform/{lane}/rendered-output.json"),
            format!("target/gate-artifacts/m9-platform/{lane}/capabilities.json"),
            format!("target/gate-artifacts/m9-platform/{lane}/surface-context-loss.json"),
            format!("target/gate-artifacts/m9-platform/{lane}/default-scene.ppm"),
            format!("target/gate-artifacts/m9-platform/{lane}/static-gltf.ppm"),
            format!("target/gate-artifacts/m9-platform/{lane}/pbr-directional-red.ppm"),
            format!("target/gate-artifacts/m9-platform/{lane}/pbr-point-green.ppm"),
            format!("target/gate-artifacts/m9-platform/{lane}/pbr-spot-blue.ppm"),
            "target/gate-artifacts/m9-platform/m9-benchmarks.json".to_string(),
        ]
        .into_iter()
        .collect(),
        "linux-webgl2-chromium" | "linux-webgpu-chromium" => {
            vec!["target/gate-artifacts/m6-rust-wasm-renderer-probe.json".to_string()]
        }
        "wasm32-unknown-unknown" => {
            vec!["target/gate-artifacts/m9-wasm-size.json".to_string()]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn release_lane_expected_commands(lane: &str) -> Vec<&'static str> {
    match lane {
        "headless-cpu" => vec!["cargo test --test m9_platform_release"],
        "linux-native-vulkan" | "macos-metal" | "windows-dx12" => vec![
            "cargo test --test m9_platform_release",
            "cargo check --examples",
        ],
        "linux-webgl2-chromium" | "linux-webgpu-chromium" => vec![
            "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe",
            "npm run browser:m6",
        ],
        "wasm32-unknown-unknown" => vec![
            "wasm-pack build --release --target web --out-dir target/m9-browser-pkg . --features browser-probe",
            "npm run wasm:size",
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn release_lane_evidence(root: &Path, rel: &str) -> Result<serde_json::Value, String> {
    let path = root.join(rel);
    if !path.exists() {
        return Ok(json!({
            "path": rel,
            "exists": false,
        }));
    }
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    Ok(json!({
        "path": rel,
        "exists": true,
        "bytes": metadata.len(),
        "sha256": sha256_hex(&path).map_err(|error| error.to_string())?,
    }))
}

pub(crate) fn release_lane_command_records(
    root: &Path,
    lane: &str,
    commands: &[&str],
    evidence: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, String> {
    let measured = release_lane_measured_command_records(root, lane)?;
    let artifact_checksums = evidence
        .iter()
        .filter(|entry| entry["exists"].as_bool() == Some(true))
        .filter_map(|entry| {
            Some(json!({
                "path": entry.get("path")?.clone(),
                "bytes": entry.get("bytes")?.clone(),
                "sha256": entry.get("sha256")?.clone(),
            }))
        })
        .collect::<Vec<_>>();
    let evidence_status = if artifact_checksums.len() == evidence.len() {
        "artifact-evidence-present"
    } else {
        "pending-artifact-evidence"
    };
    Ok(commands
        .iter()
        .map(|command| {
            let concrete_command = command.replace("<lane>", lane);
            let measured_record = measured
                .get(*command)
                .or_else(|| measured.get(concrete_command.as_str()));
            if let Some(measured_record) = measured_record {
                let mut record = json!({
                    "command": measured_record
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or(command),
                    "status": measured_record
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or(evidence_status),
                    "duration_ms": measured_record
                        .get("duration_ms")
                        .cloned()
                        .unwrap_or(Value::Null),
                    "duration_source": measured_record
                        .get("duration_source")
                        .and_then(Value::as_str)
                        .unwrap_or("ci-wrapper"),
                    "failure_log_path": measured_record
                        .get("failure_log_path")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("target/gate-artifacts/release-lanes/{lane}.log")),
                    "artifact_checksums": artifact_checksums,
                    "measurement_source": measured_record
                        .get("measurement_source")
                        .and_then(Value::as_str)
                        .unwrap_or("target/gate-artifacts/release-lanes/<lane>.commands.jsonl"),
                });
                copy_optional_json_field(measured_record, &mut record, "failure_log_sha256");
                copy_optional_json_field(measured_record, &mut record, "started_at_unix_seconds");
                copy_optional_json_field(measured_record, &mut record, "finished_at_unix_seconds");
                return record;
            }
            json!({
                "command": command,
                "status": evidence_status,
                "duration_ms": null,
                "duration_source": "ci-step-summary-or-wrapper",
                "failure_log_path": format!("target/gate-artifacts/release-lanes/{lane}.log"),
                "artifact_checksums": artifact_checksums,
            })
        })
        .collect())
}

pub(crate) fn release_lane_measured_command_records(
    root: &Path,
    lane: &str,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let rel = format!("target/gate-artifacts/release-lanes/{lane}.commands.jsonl");
    let path = root.join(&rel);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut records = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut value = serde_json::from_str::<serde_json::Value>(line).map_err(|error| {
            format!(
                "failed to parse command record {} line {}: {error}",
                path.display(),
                index + 1
            )
        })?;
        let command = value
            .get("command")
            .and_then(Value::as_str)
            .filter(|command| !command.trim().is_empty())
            .ok_or_else(|| {
                format!(
                    "command record {} line {} is missing command",
                    path.display(),
                    index + 1
                )
            })?
            .to_string();
        if value.get("duration_ms").is_some_and(|duration| {
            !duration.is_null() && duration.as_u64().is_none() && duration.as_f64().is_none()
        }) {
            return Err(format!(
                "command record {} line {} has non-numeric duration_ms",
                path.display(),
                index + 1
            ));
        }
        if value.get("measurement_source").is_none() {
            value["measurement_source"] = json!(rel);
        }
        records.insert(command, value);
    }
    Ok(records)
}

pub(crate) fn release_lane_command_records_pass(records: &[serde_json::Value]) -> bool {
    records.iter().all(|record| {
        !matches!(
            record.get("status").and_then(Value::as_str),
            Some("failed" | "failure" | "cancelled" | "timed-out" | "timed_out")
        )
    })
}

pub(crate) fn copy_optional_json_field(
    source: &serde_json::Value,
    target: &mut serde_json::Value,
    key: &str,
) {
    if let Some(value) = source.get(key) {
        target[key] = value.clone();
    }
}

pub(crate) fn release_artifact_commit_label(root: &Path) -> String {
    env::var("GITHUB_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "local-checkout".to_string())
        })
}

pub(crate) fn run_claim_audit() -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("CLAIM-AUDIT-ROOT", message)])?;
    let artifact = match build_claim_audit(&root) {
        Ok(artifact) => artifact,
        Err(error) => return Err(vec![Finding::new("CLAIM-AUDIT", error)]),
    };
    let artifact_dir = root.join("target/gate-artifacts");
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        return Err(vec![Finding::new(
            "CLAIM-AUDIT",
            format!("failed to create target/gate-artifacts: {error}"),
        )]);
    }
    let artifact_path = artifact_dir.join("m10-claim-audit.json");
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| vec![Finding::new("CLAIM-AUDIT", error.to_string())])?;
    if let Err(error) = fs::write(&artifact_path, format!("{body}\n")) {
        return Err(vec![Finding::new(
            "CLAIM-AUDIT",
            format!("failed to write {}: {error}", artifact_path.display()),
        )]);
    }
    println!("{}", artifact_path.display());
    Ok(())
}

pub(crate) fn run_release_readiness() -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("RELEASE-READY-ROOT", message)])?;
    let mut findings = Vec::new();
    check_release_readiness(&root, &mut findings);
    if findings.is_empty() {
        println!("scena release readiness: pass");
        Ok(())
    } else {
        Err(findings)
    }
}

pub(crate) fn check_release_readiness(root: &Path, findings: &mut Vec<Finding>) {
    run_docs_doctor(root, findings);
    run_architecture_doctor(root, findings);
    check_release_readiness_adr(root, findings);
    check_release_readiness_checklists(root, findings);
    check_release_readiness_artifact_env(root, findings);
}

pub(crate) fn check_release_readiness_adr(root: &Path, findings: &mut Vec<Finding>) {
    let rel = "docs/decisions/ADR-0005-local-release-candidate-deferrals.md";
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            "RELEASE-READY-M10",
            format!("could not read {rel}"),
        ));
        return;
    };
    if text.contains("Status: Accepted.")
        && text.contains("local release-candidate deferrals")
        && text.contains("blocking for public v1.0")
    {
        findings.push(Finding::new(
            "RELEASE-READY-M10",
            "ADR-0005 still records blocking local release-candidate deferrals",
        ));
    }
}

pub(crate) fn check_release_readiness_checklists(root: &Path, findings: &mut Vec<Finding>) {
    for rel in [
        "docs/checklists/m9-platform-ci-release-parity.md",
        "docs/checklists/m10-threejs-replacement-acceptance.md",
        "docs/checklists/threejs-replacement-index.md",
        "docs/checklists/state-of-art-threejs-replacement-plan.md",
    ] {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "RELEASE-READY-M10",
                format!("could not read {rel}"),
            ));
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- [ ]") {
                findings.push(Finding::new(
                    "RELEASE-READY-M10",
                    format!("{rel}:{} has open release gate: {trimmed}", index + 1),
                ));
            }
        }
    }
}

pub(crate) fn check_release_readiness_artifact_env(root: &Path, findings: &mut Vec<Finding>) {
    let Ok(configured_root) = env::var("SCENA_RELEASE_ARTIFACT_ROOT") else {
        return;
    };
    let configured_path = PathBuf::from(configured_root);
    let artifact_root = if configured_path.is_absolute() {
        configured_path
    } else {
        root.join(configured_path)
    };
    check_release_artifact_bundle(&artifact_root, findings);
}
