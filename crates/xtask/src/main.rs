use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn main() {
    let outcome = match parse_command(env::args().skip(1).collect()) {
        Ok(Command::Doctor(mode)) => run_doctor(mode),
        Ok(Command::ClaimAudit) => run_claim_audit(),
        Ok(Command::ReleaseLaneArtifact(lane)) => run_release_lane_artifact(&lane),
        Ok(Command::ReleaseReadiness) => run_release_readiness(),
        Ok(Command::Help) => {
            print_usage();
            Ok(())
        }
        Err(error) => Err(vec![Finding::new("CLI", error)]),
    };

    match outcome {
        Ok(()) => {}
        Err(findings) => {
            eprintln!("scena doctor failed with {} finding(s):", findings.len());
            for finding in findings {
                eprintln!("- {}: {}", finding.rule, finding.message);
            }
            process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorMode {
    Docs,
    Architecture,
    Full,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Doctor(DoctorMode),
    ClaimAudit,
    ReleaseLaneArtifact(String),
    ReleaseReadiness,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    rule: &'static str,
    message: String,
}

impl Finding {
    fn new(rule: &'static str, message: impl Into<String>) -> Self {
        let mut message = message.into();
        let reference = finding_reference(rule);
        if !message.contains(reference) {
            message.push_str("; see ");
            message.push_str(reference);
        }
        Self { rule, message }
    }
}

fn finding_reference(rule: &str) -> &'static str {
    if rule.starts_with("RELEASE") || rule.starts_with("CLAIM") || rule.starts_with("M10") {
        "docs/specs/release-gates.md"
    } else if rule.contains("STATE-OF-ART")
        || rule == "ARCH-RENDER-TRUTH"
        || rule == "ARCH-RENDER-WORLD-BAKE"
        || rule == "BINARY-ASSET-TRUTH-P9"
    {
        "docs/checklists/state-of-art-threejs-replacement-plan.md"
    } else if rule.contains("M8") || rule.contains("GLTF") || rule.contains("ASSETS") {
        "docs/specs/asset-gltf-contract.md"
    } else if rule.contains("M7") || rule.contains("ERGONOMICS") {
        "docs/specs/public-api.md"
    } else if rule.contains("VISUAL") || rule.contains("SCREENSHOT") {
        "docs/specs/visual-quality-contract.md"
    } else if rule.contains("PLATFORM") || rule.contains("BACKEND") || rule.contains("WEBGL") {
        "docs/specs/platform-capabilities.md"
    } else if rule.contains("PREPARE") || rule.contains("LIFECYCLE") {
        "docs/specs/render-lifecycle.md"
    } else {
        "docs/specs/doctor-contract.md"
    }
}

fn parse_command(args: Vec<String>) -> Result<Command, String> {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(Command::Help);
    }

    if args.first().map(String::as_str) == Some("claim-audit") {
        if args.len() == 1 {
            return Ok(Command::ClaimAudit);
        }
        return Err("claim-audit accepts no arguments".to_string());
    }

    if args.first().map(String::as_str) == Some("release-lane-artifact") {
        if args.len() == 2 {
            return Ok(Command::ReleaseLaneArtifact(args[1].clone()));
        }
        return Err("release-lane-artifact expects exactly one lane argument".to_string());
    }

    if args.first().map(String::as_str) == Some("release-readiness") {
        if args.len() == 1 {
            return Ok(Command::ReleaseReadiness);
        }
        return Err("release-readiness accepts no arguments".to_string());
    }

    if args.first().map(String::as_str) != Some("doctor") {
        return Err(format!(
            "unknown command '{}'; expected 'doctor', 'claim-audit', 'release-lane-artifact', or 'release-readiness'",
            args.first().map(String::as_str).unwrap_or("")
        ));
    }

    let mode = match args.get(1).map(String::as_str) {
        None | Some("--full") => DoctorMode::Full,
        Some("--docs") => DoctorMode::Docs,
        Some("--architecture") => DoctorMode::Architecture,
        Some("--help") | Some("-h") => return Ok(Command::Help),
        Some(other) => {
            return Err(format!(
                "unknown doctor mode '{other}'; expected --docs, --architecture, or --full"
            ));
        }
    };

    if args.len() > 2 {
        return Err("doctor accepts at most one mode flag".to_string());
    }

    Ok(Command::Doctor(mode))
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p xtask -- doctor --docs\n  cargo run -p xtask -- doctor --architecture\n  cargo run -p xtask -- doctor --full\n  cargo run -p xtask -- claim-audit\n  cargo run -p xtask -- release-lane-artifact <lane>\n  cargo run -p xtask -- release-readiness"
    );
}

fn run_release_lane_artifact(lane: &str) -> Result<(), Vec<Finding>> {
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

fn release_lane_artifact(root: &Path, lane: &str) -> Result<serde_json::Value, String> {
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

fn release_lane_content_ok(root: &Path, lane: &str) -> Result<bool, String> {
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
    if !matches!(lane, "linux-native-vulkan" | "macos-metal" | "windows-dx12") {
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

fn release_lane_required_artifacts(lane: &str) -> Vec<String> {
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

fn release_lane_expected_commands(lane: &str) -> Vec<&'static str> {
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

fn release_lane_evidence(root: &Path, rel: &str) -> Result<serde_json::Value, String> {
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

fn release_lane_command_records(
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

fn release_lane_measured_command_records(
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

fn release_lane_command_records_pass(records: &[serde_json::Value]) -> bool {
    records.iter().all(|record| {
        !matches!(
            record.get("status").and_then(Value::as_str),
            Some("failed" | "failure" | "cancelled" | "timed-out" | "timed_out")
        )
    })
}

fn copy_optional_json_field(source: &serde_json::Value, target: &mut serde_json::Value, key: &str) {
    if let Some(value) = source.get(key) {
        target[key] = value.clone();
    }
}

fn release_artifact_commit_label(root: &Path) -> String {
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

fn run_claim_audit() -> Result<(), Vec<Finding>> {
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

fn run_release_readiness() -> Result<(), Vec<Finding>> {
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

fn check_release_readiness(root: &Path, findings: &mut Vec<Finding>) {
    run_docs_doctor(root, findings);
    run_architecture_doctor(root, findings);
    check_release_readiness_adr(root, findings);
    check_release_readiness_checklists(root, findings);
    check_release_readiness_artifact_env(root, findings);
}

fn check_release_readiness_adr(root: &Path, findings: &mut Vec<Finding>) {
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

fn check_release_readiness_checklists(root: &Path, findings: &mut Vec<Finding>) {
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

fn check_release_readiness_artifact_env(root: &Path, findings: &mut Vec<Finding>) {
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

fn check_release_artifact_bundle(artifact_root: &Path, findings: &mut Vec<Finding>) {
    if !artifact_root.is_dir() {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("missing release artifact root {}", artifact_root.display()),
        ));
        return;
    }

    let mut files = Vec::new();
    if let Err(error) = collect_files_with_extensions(artifact_root, &["json", "ppm"], &mut files) {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not collect release artifacts: {error}"),
        ));
        return;
    }

    for suffix in REQUIRED_RELEASE_ARTIFACT_SUFFIXES {
        if !files.iter().any(|path| path_ends_with(path, suffix)) {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!("downloaded release artifacts are missing {suffix}"),
            ));
        }
    }

    for suffix in REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES {
        let matches = files
            .iter()
            .filter(|path| path_ends_with(path, suffix))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            continue;
        }
        for path in matches {
            require_json_status_passed(path, suffix, findings);
        }
    }

    for suffix in RELEASE_LANE_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_release_lane_artifact_file(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_native_gpu_render_proof(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_stale_json_timestamp(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_NON_CONSTANT_PPM_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_constant_ppm_artifact(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_MEASURED_CAPABILITY_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_unmeasured_capability_matrix_rows(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_benchmark_baseline_comparison_file(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_rendered_output_screenshot_metadata_file(path, suffix, findings);
        }
    }
}

const REQUIRED_RELEASE_ARTIFACT_SUFFIXES: &[&str] = &[
    "release-lanes/linux-native-vulkan.json",
    "release-lanes/headless-cpu.json",
    "release-lanes/linux-webgl2-chromium.json",
    "release-lanes/linux-webgpu-chromium.json",
    "release-lanes/wasm32-unknown-unknown.json",
    "release-lanes/macos-metal.json",
    "release-lanes/windows-dx12.json",
    "m6-rust-wasm-renderer-probe.json",
    "m9-wasm-size.json",
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/m9-benchmarks.json",
    "m9-platform/m9-benchmarks-4k.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/linux-native-vulkan/surface-context-loss.json",
    "m9-platform/linux-native-vulkan/default-scene.ppm",
    "m9-platform/linux-native-vulkan/static-gltf.ppm",
    "m9-platform/linux-native-vulkan/pbr-directional-red.ppm",
    "m9-platform/linux-native-vulkan/pbr-point-green.ppm",
    "m9-platform/linux-native-vulkan/pbr-spot-blue.ppm",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "m9-platform/headless-cpu/default-scene.ppm",
    "m9-platform/headless-cpu/static-gltf.ppm",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/macos-metal/surface-context-loss.json",
    "m9-platform/macos-metal/default-scene.ppm",
    "m9-platform/macos-metal/static-gltf.ppm",
    "m9-platform/macos-metal/pbr-directional-red.ppm",
    "m9-platform/macos-metal/pbr-point-green.ppm",
    "m9-platform/macos-metal/pbr-spot-blue.ppm",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
    "m9-platform/windows-dx12/surface-context-loss.json",
    "m9-platform/windows-dx12/default-scene.ppm",
    "m9-platform/windows-dx12/static-gltf.ppm",
    "m9-platform/windows-dx12/pbr-directional-red.ppm",
    "m9-platform/windows-dx12/pbr-point-green.ppm",
    "m9-platform/windows-dx12/pbr-spot-blue.ppm",
];

const REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES: &[&str] = &[
    "m6-rust-wasm-renderer-probe.json",
    "m9-platform/m9-capability-matrix.json",
];

const RELEASE_LANE_ARTIFACT_SUFFIXES: &[&str] = &[
    "release-lanes/linux-native-vulkan.json",
    "release-lanes/headless-cpu.json",
    "release-lanes/linux-webgl2-chromium.json",
    "release-lanes/linux-webgpu-chromium.json",
    "release-lanes/wasm32-unknown-unknown.json",
    "release-lanes/macos-metal.json",
    "release-lanes/windows-dx12.json",
];

const REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/windows-dx12/rendered-output.json",
];

const REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
];

const REQUIRED_NON_CONSTANT_PPM_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/default-scene.ppm",
    "m9-platform/linux-native-vulkan/static-gltf.ppm",
    "m9-platform/linux-native-vulkan/pbr-directional-red.ppm",
    "m9-platform/linux-native-vulkan/pbr-point-green.ppm",
    "m9-platform/linux-native-vulkan/pbr-spot-blue.ppm",
    "m9-platform/headless-cpu/default-scene.ppm",
    "m9-platform/headless-cpu/static-gltf.ppm",
    "m9-platform/macos-metal/default-scene.ppm",
    "m9-platform/macos-metal/static-gltf.ppm",
    "m9-platform/macos-metal/pbr-directional-red.ppm",
    "m9-platform/macos-metal/pbr-point-green.ppm",
    "m9-platform/macos-metal/pbr-spot-blue.ppm",
    "m9-platform/windows-dx12/default-scene.ppm",
    "m9-platform/windows-dx12/static-gltf.ppm",
    "m9-platform/windows-dx12/pbr-directional-red.ppm",
    "m9-platform/windows-dx12/pbr-point-green.ppm",
    "m9-platform/windows-dx12/pbr-spot-blue.ppm",
];

const REQUIRED_MEASURED_CAPABILITY_ARTIFACT_SUFFIXES: &[&str] =
    &["m9-platform/m9-capability-matrix.json"];

const REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/m9-benchmarks.json",
    "m9-platform/m9-benchmarks-4k.json",
];
const REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/windows-dx12/rendered-output.json",
];
const MIN_BENCHMARK_SAMPLE_COUNT: u64 = 100;

const RELEASE_ARTIFACT_MAX_AGE_SECONDS: u64 = 24 * 60 * 60;
const RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS: u64 = 60 * 60;

fn require_json_status_passed(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not read downloaded artifact {}", path.display()),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded artifact {} is not valid JSON", path.display()),
        ));
        return;
    };
    if value.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded release artifact {suffix} does not have status 'passed'"),
        ));
    }
}

fn reject_stale_json_timestamp(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
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

fn reject_constant_ppm_artifact(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
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

fn reject_unmeasured_capability_matrix_rows(
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

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn ppm_pixel_payload(bytes: &[u8]) -> Option<&[u8]> {
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

fn ppm_payload_is_constant_rgb(pixel_bytes: &[u8]) -> bool {
    if pixel_bytes.len() < 6 || pixel_bytes.len() % 3 != 0 {
        return true;
    }
    let first = &pixel_bytes[..3];
    pixel_bytes.chunks_exact(3).all(|pixel| pixel == first)
}

fn json_contains_string_value(value: &serde_json::Value, key: &str, expected: &str) -> bool {
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

fn require_benchmark_baseline_comparison_file(
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

fn require_benchmark_baseline_comparison(
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

fn require_rendered_output_screenshot_metadata_file(
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

fn require_rendered_output_screenshot_metadata(
    value: &serde_json::Value,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    for section in ["default_scene", "static_gltf"] {
        let Some(entry) = value.get(section) else {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "rendered-output artifact {suffix} is missing screenshot metadata section {section}"
                ),
            ));
            continue;
        };
        require_screenshot_metadata_entry(entry, suffix, section, findings);
    }

    if let Some(lights) = value
        .get("pbr_lights")
        .and_then(|pbr_lights| pbr_lights.get("lights"))
        .and_then(serde_json::Value::as_array)
    {
        for (index, light) in lights.iter().enumerate() {
            require_screenshot_metadata_entry(
                light,
                suffix,
                &format!("pbr_lights.lights[{index}]"),
                findings,
            );
        }
    }
}

fn require_screenshot_metadata_entry(
    entry: &serde_json::Value,
    suffix: &str,
    section: &str,
    findings: &mut Vec<Finding>,
) {
    for field in [
        "backend",
        "adapter",
        "renderer_settings",
        "color_management",
        "tolerance",
        "screenshot",
    ] {
        if entry.get(field).is_none() {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "rendered-output artifact {suffix} section {section} is missing screenshot metadata field {field}"
                ),
            ));
        }
    }
    if entry
        .get("width")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        == 0
        || entry
            .get("height")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            == 0
    {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "rendered-output artifact {suffix} section {section} has invalid screenshot dimensions"
            ),
        ));
    }
    if section == "static_gltf"
        && entry
            .get("asset_provenance")
            .and_then(|provenance| provenance.get("hash"))
            .and_then(serde_json::Value::as_str)
            .is_none_or(str::is_empty)
    {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "rendered-output artifact {suffix} section {section} is missing fixture source hash"
            ),
        ));
    }
}

fn require_release_lane_artifact_file(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not read release lane artifact {}", path.display()),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("release lane artifact {suffix} is not valid JSON"),
        ));
        return;
    };
    require_release_lane_artifact_evidence(&value, suffix, findings);
}

fn require_release_lane_artifact_evidence(
    value: &serde_json::Value,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    if value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status == "command-recorded")
    {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "downloaded release lane artifact {suffix} only records a command; regenerate it with file evidence"
            ),
        ));
        return;
    }
    if value.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded release lane artifact {suffix} does not have status 'passed'"),
        ));
    }
    let Some(records) = value
        .get("command_records")
        .and_then(serde_json::Value::as_array)
    else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("release lane artifact {suffix} is missing command_records"),
        ));
        return;
    };
    if records.is_empty() {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("release lane artifact {suffix} has no command_records"),
        ));
        return;
    }
    for (index, record) in records.iter().enumerate() {
        let command = record
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<missing command>");
        if command == "<missing command>" || command.trim().is_empty() {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!("release lane artifact {suffix} command record {index} is missing command"),
            ));
        }
        if record.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!("release lane artifact {suffix} command '{command}' did not pass"),
            ));
        }
        if !record
            .get("duration_ms")
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|duration| duration >= 0.0)
        {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "release lane artifact {suffix} command '{command}' is missing measured command duration"
                ),
            ));
        }
        if record
            .get("failure_log_path")
            .and_then(serde_json::Value::as_str)
            .is_none_or(str::is_empty)
        {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "release lane artifact {suffix} command '{command}' is missing failure_log_path"
                ),
            ));
        }
        if !record
            .get("artifact_checksums")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|checksums| !checksums.is_empty())
        {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!(
                    "release lane artifact {suffix} command '{command}' is missing artifact checksums"
                ),
            ));
        }
    }
}

fn require_native_gpu_render_proof(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "could not read native GPU rendered-output artifact {}",
                path.display()
            ),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("native GPU rendered-output artifact {suffix} is not valid JSON"),
        ));
        return;
    };
    if !native_gpu_render_proof_passes(&value) {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "native GPU rendered-output artifact {suffix} does not prove GPU output; CPU fallback artifacts cannot satisfy GPU release claims"
            ),
        ));
    }
}

fn native_gpu_render_proof_passes(value: &serde_json::Value) -> bool {
    value.get("gpu_proof").and_then(serde_json::Value::as_bool) == Some(true)
        && value
            .get("host_gpu_available")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && value
            .get("static_gltf")
            .and_then(|static_gltf| static_gltf.get("gpu_proof"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && value
            .get("static_gltf")
            .and_then(|static_gltf| static_gltf.get("production_claim"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && value
            .get("static_gltf")
            .and_then(|static_gltf| static_gltf.get("proof_class"))
            .and_then(serde_json::Value::as_str)
            == Some("camera-framed-non-ndc")
        && pbr_light_render_proof_passes(value)
}

fn pbr_light_render_proof_passes(value: &serde_json::Value) -> bool {
    let Some(pbr_lights) = value.get("pbr_lights") else {
        return false;
    };
    if pbr_lights
        .get("gpu_proof")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
        || pbr_lights
            .get("production_claim")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || pbr_lights
            .get("proof_class")
            .and_then(serde_json::Value::as_str)
            != Some("native-pbr-punctual-light")
    {
        return false;
    }
    let Some(lights) = pbr_lights
        .get("lights")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    ["directional", "point", "spot"]
        .into_iter()
        .all(|light_type| {
            lights.iter().any(|light| {
                light.get("light_type").and_then(serde_json::Value::as_str) == Some(light_type)
                    && light.get("gpu_proof").and_then(serde_json::Value::as_bool) == Some(true)
                    && light
                        .get("production_claim")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                    && light
                        .get("color_assertion_passed")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                    && light
                        .get("nonblack_pixels")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                        > 0
            })
        })
}

fn headless_cpu_render_proof_passes(value: &serde_json::Value) -> bool {
    value.get("backend").and_then(serde_json::Value::as_str) == Some("Headless")
        && value
            .get("headless_cpu_proof")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && value
            .get("static_gltf")
            .and_then(|static_gltf| static_gltf.get("production_claim"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && value
            .get("static_gltf")
            .and_then(|static_gltf| static_gltf.get("nonblack_pixels"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0
}

fn path_ends_with(path: &Path, suffix: &str) -> bool {
    path.to_string_lossy().replace('\\', "/").ends_with(suffix)
}

fn build_claim_audit(root: &Path) -> Result<serde_json::Value, String> {
    let mut files = Vec::new();
    for path in claim_audit_paths(root)? {
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let categories = claim_categories(&source);
        files.push(json!({
            "path": relative,
            "sha256": sha256_hex(&path).map_err(|error| error.to_string())?,
            "evidence_categories": categories,
            "evidence": categories
                .iter()
                .map(|category| json!({
                    "category": category,
                    "links": evidence_links_for_category(category),
                }))
                .collect::<Vec<_>>(),
        }));
    }
    Ok(json!({
        "schema": "scena.m10.claim_audit.v1",
        "status": "generated-with-evidence-index",
        "scope": [
            "repo-root markdown",
            "docs markdown",
            "examples rust"
        ],
        "files": files,
        "required_final_gates": [
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test",
            "cargo check --examples",
            "cargo run -p xtask -- doctor --full",
            "RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --all-features",
            "browser WebGPU/WebGL2 rendered-output proof",
            "native platform rendered-output proof",
            "clean cargo publish --dry-run",
            "cargo run -p xtask -- release-readiness",
            "external review reports",
            "named maintainer sign-off"
        ]
    }))
}

fn claim_audit_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for name in ["README.md", "CHANGELOG.md", "AGENTS.md"] {
        let path = root.join(name);
        if path.is_file() {
            paths.push(path);
        }
    }
    collect_files_with_extensions(&root.join("docs"), &["md"], &mut paths)?;
    collect_files_with_extensions(&root.join("examples"), &["rs"], &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_files_with_extensions(
    dir: &Path,
    extensions: &[&str],
    paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extensions(&path, extensions, paths)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            paths.push(path);
        }
    }
    Ok(())
}

fn claim_categories(source: &str) -> Vec<&'static str> {
    let mut categories = Vec::new();
    for (needle, category) in [
        ("Scene", "public-api"),
        ("Renderer", "public-api"),
        ("glTF", "assets-gltf"),
        ("WebGPU", "browser-platform"),
        ("WebGL2", "browser-platform"),
        ("Metal", "native-platform"),
        ("DX12", "native-platform"),
        ("screenshot", "visual-proof"),
        ("benchmark", "performance"),
        ("doctor", "doctor"),
        ("non-goal", "scope-non-goal"),
        ("physics", "scope-non-goal"),
        ("simulation", "scope-non-goal"),
        ("prepare", "render-lifecycle"),
        ("render", "render-lifecycle"),
    ] {
        if source.contains(needle) && !categories.contains(&category) {
            categories.push(category);
        }
    }
    categories
}

fn evidence_links_for_category(category: &str) -> Vec<&'static str> {
    match category {
        "public-api" => vec![
            "docs/specs/public-api.md",
            "tests/m0_foundation.rs",
            "tests/m5_release.rs",
        ],
        "assets-gltf" => vec![
            "docs/specs/asset-gltf-contract.md",
            "tests/m3a_app_features.rs",
            "tests/m3b_gltf_animation.rs",
            "tests/m8_assets_materials_ecosystem.rs",
        ],
        "browser-platform" => vec![
            "docs/checklists/m6-browser-renderer-parity.md",
            "tests/m6_browser_renderer_parity.rs",
            "tests/browser/m6_rust_wasm_renderer_probe.js",
        ],
        "native-platform" => vec![
            "docs/specs/platform-capabilities.md",
            "docs/decisions/ADR-0005-local-release-candidate-deferrals.md",
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
        ],
        "visual-proof" => vec![
            "docs/specs/visual-quality-contract.md",
            "tests/m1_visual_proof.rs",
            "tests/m2_visual_proof.rs",
            "tests/m3a_visual_proof.rs",
            "tests/m3b_visual_proof.rs",
        ],
        "performance" => vec![
            "docs/specs/release-gates.md",
            "tests/m4_performance_platform.rs",
            "tests/m5_release.rs",
        ],
        "doctor" => vec!["docs/specs/doctor-contract.md", "crates/xtask/src/main.rs"],
        "scope-non-goal" => vec![
            "docs/decisions/ADR-0001-renderer-not-engine.md",
            "docs/specs/module-boundaries.md",
            "AGENTS.md",
        ],
        "render-lifecycle" => vec![
            "docs/specs/render-lifecycle.md",
            "docs/decisions/ADR-0002-explicit-prepare-lifecycle.md",
            "tests/m0_foundation.rs",
        ],
        _ => Vec::new(),
    }
}

fn run_doctor(mode: DoctorMode) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("DOCTOR-ROOT", message)])?;
    let mut findings = Vec::new();

    match mode {
        DoctorMode::Docs => run_docs_doctor(&root, &mut findings),
        DoctorMode::Architecture => run_architecture_doctor(&root, &mut findings),
        DoctorMode::Full => {
            run_docs_doctor(&root, &mut findings);
            run_architecture_doctor(&root, &mut findings);
        }
    }

    if findings.is_empty() {
        println!("scena doctor: mode={mode:?} status=pass");
        Ok(())
    } else {
        Err(findings)
    }
}

fn repo_root() -> Result<PathBuf, String> {
    let mut dir = env::current_dir().map_err(|error| error.to_string())?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs/RFC-rust-3d-renderer.md").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find scena repo root".to_string());
        }
    }
}

fn run_docs_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "DOCS-REQUIRED", REQUIRED_DOCS);
    check_markdown_links(root, findings);
    check_for_stale_doc_terms(root, findings);
    check_required_doc_contracts(root, findings);
    check_default_environment_manifest(root, findings);
    check_visual_fixture_metadata(root, findings);
    check_m2_visual_fixture_metadata(root, findings);
    check_m1_browser_rendered_output(root, findings);
    check_m2_browser_rendered_output(root, findings);
    check_m6_browser_renderer_probe(root, findings);
    check_gltf_asset_matrix_contract(root, findings);
    check_m9_ci_release_lanes(root, findings);
    check_m10_claim_audit_contract(root, findings);
    check_state_of_art_checklist_links(root, findings);
}

fn run_architecture_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-REQUIRED", REQUIRED_SOURCE_MODULES);
    check_source_scope(root, findings);
    check_module_boundaries(root, findings);
    check_asset_api_contracts(root, findings);
    check_prepare_asset_contracts(root, findings);
    check_environment_lifecycle_contracts(root, findings);
    check_equirectangular_hdr_environment_contracts(root, findings);
    check_environment_ibl_prepare_contracts(root, findings);
    check_scene_light_contracts(root, findings);
    check_direct_light_shading_contracts(root, findings);
    check_directional_shadow_contracts(root, findings);
    check_shadow_map_contracts(root, findings);
    check_depth_prepass_contracts(root, findings);
    check_reversed_z_contracts(root, findings);
    check_webgl2_depth_contracts(root, findings);
    check_m2_leak_stats_contracts(root, findings);
    check_camera_depth_contracts(root, findings);
    check_origin_shift_contracts(root, findings);
    check_clipping_contracts(root, findings);
    check_m3a_scene_import_contracts(root, findings);
    check_m3b_animation_contracts(root, findings);
    check_m4_platform_contracts(root, findings);
    check_m5_release_contracts(root, findings);
    check_m7_ergonomics_contracts(root, findings);
    check_m8_assets_materials_contracts(root, findings);
    check_binary_render_asset_contracts(root, findings);
    check_render_alpha_contracts(root, findings);
    check_output_stage_contracts(root, findings);
    check_fxaa_output_contracts(root, findings);
    check_diagnostics_contracts(root, findings);
    check_renderer_stats_contracts(root, findings);
    check_renderer_truth_contracts(root, findings);
    check_render_world_bake_contracts(root, findings);
    check_solid_kiss(root, findings);
    check_backend_vocabulary(root, findings);
    check_unit_test_first_governance(root, findings);
    check_agent_validation(root, findings);
}

const REQUIRED_DOCS: &[&str] = &[
    "AGENTS.md",
    "README.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "docs/RFC-rust-3d-renderer.md",
    "docs/api/m5-public-api-baseline.txt",
    "docs/api/m5-semver-baseline.toml",
    "docs/agents/subagents.md",
    "docs/specs/public-api.md",
    "docs/specs/module-boundaries.md",
    "docs/specs/render-lifecycle.md",
    "docs/specs/asset-gltf-contract.md",
    "docs/specs/visual-quality-contract.md",
    "docs/specs/platform-capabilities.md",
    "docs/specs/release-gates.md",
    "docs/specs/doctor-contract.md",
    "docs/specs/release-reviews.md",
    "docs/checklists/acceptance-index.md",
    "docs/checklists/m0-foundation.md",
    "docs/checklists/m1-geometry-materials.md",
    "docs/checklists/m2-lighting-depth-clipping.md",
    "docs/checklists/m3a-app-features.md",
    "docs/checklists/m3b-gltf-animation.md",
    "docs/checklists/m4-performance-platform.md",
    "docs/checklists/m5-v1-release.md",
    "docs/decisions/ADR-0001-renderer-not-engine.md",
    "docs/decisions/ADR-0002-explicit-prepare-lifecycle.md",
    "docs/decisions/ADR-0003-gltf-primary-format.md",
    "docs/decisions/ADR-0004-visual-evidence-policy.md",
    ".codex/skills/scena-doctor/SKILL.md",
    ".codex/skills/scena-git-github/SKILL.md",
    ".codex/skills/scena-gltf-assets/SKILL.md",
    ".codex/skills/scena-release-hygiene/SKILL.md",
    ".codex/skills/scena-renderer-architecture/SKILL.md",
    ".codex/skills/scena-renderer-quality/SKILL.md",
    ".codex/skills/scena-rfc-governance/SKILL.md",
    ".claude/agents/scena-doctor-reviewer.md",
];

const REQUIRED_SOURCE_MODULES: &[&str] = &[
    "src/lib.rs",
    "src/scene.rs",
    "src/scene/camera.rs",
    "src/scene/connectors.rs",
    "src/scene/dirty.rs",
    "src/scene/inspection.rs",
    "src/scene/lights.rs",
    "src/scene/origin.rs",
    "src/scene/skinning.rs",
    "src/diagnostics/capabilities.rs",
    "src/assets.rs",
    "src/assets/environment.rs",
    "src/assets/load.rs",
    "src/assets/gltf/accessor/skin.rs",
    "src/assets/gltf/skins.rs",
    "src/assets/gltf/transform.rs",
    "src/geometry.rs",
    "src/geometry/bounds.rs",
    "src/geometry/primitive.rs",
    "src/geometry/skinning.rs",
    "src/geometry/static_batch.rs",
    "src/material.rs",
    "src/render.rs",
    "src/viewer.rs",
    "src/render/build.rs",
    "src/render/camera.rs",
    "src/render/culling.rs",
    "src/render/surface.rs",
    "src/render/gpu/build.rs",
    "src/render/gpu/depth.rs",
    "src/render/gpu/culling.rs",
    "src/render/gpu/shadow.rs",
    "src/render/gpu/vertices.rs",
    "src/render/prepare/strokes.rs",
    "src/animation.rs",
    "src/animation/sampling.rs",
    "src/controls.rs",
    "src/picking.rs",
    "src/diagnostics.rs",
    "src/platform.rs",
    "src/bin/scena-convert.rs",
];

const STALE_DOC_TERMS: &[&str] = &[
    "TBD",
    "TODO",
    "FIXME",
    "not final API",
    "complete working example",
    "Renderer::prepare(&mut self, &mut scene)",
    "Renderer::render(&mut self, &scene",
    "RenderError::BackendCapabilityMismatch",
    "MutationQueueFull",
    "HardwareTier::Low / Mid",
    "rotation_quat",
    "gpu_memory_mb",
    "frame_time_ms",
    "render_on_change_skips",
    "texture_count",
    "Assets owns all GPU",
    "Load error unless feature enabled",
    "load error unless feature enabled",
    "Scene::replace_import(import, new_scene_asset)",
    "instantiate(scene_asset)",
    "instantiate_with(scene_asset",
    "Color::from_rgb(",
];

const SOURCE_SCOPE_TERMS: &[&str] = &[
    "plc",
    "robotics",
    "robot",
    "physics",
    "simulation",
    "process semantics",
    "game engine",
    "game loop",
];

const MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE: usize = 500;

const CATCH_ALL_TYPE_NAMES: &[&str] = &["World", "Engine", "Manager", "Registry", "ServiceLocator"];

const CATCH_ALL_TYPE_SUFFIXES: &[&str] = &["Manager", "Engine"];

const ALLOWED_CONTEXT_TYPES: &[&str] = &[
    "InteractionContext",
    "RenderContext",
    "PrepareContext",
    "DiagnosticContext",
];

fn require_files(root: &Path, findings: &mut Vec<Finding>, rule: &'static str, paths: &[&str]) {
    for rel in paths {
        if !root.join(rel).is_file() {
            findings.push(Finding::new(rule, format!("missing required file {rel}")));
        }
    }
}

fn check_markdown_links(root: &Path, findings: &mut Vec<Finding>) {
    for rel in markdown_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "DOCS-LINKS",
                format!("could not read {}", rel.display()),
            ));
            continue;
        };

        for target in markdown_link_targets(&text) {
            if is_external_link(&target) || target.starts_with('#') {
                continue;
            }

            let without_fragment = target.split('#').next().unwrap_or_default();
            if without_fragment.is_empty() {
                continue;
            }

            let target_path = path
                .parent()
                .unwrap_or(root)
                .join(without_fragment.trim_matches(['<', '>']));
            if !target_path.exists() {
                findings.push(Finding::new(
                    "DOCS-LINKS",
                    format!("{} links to missing {}", rel.display(), target),
                ));
            }
        }
    }
}

fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("README.md"), PathBuf::from("AGENTS.md")];
    collect_markdown(&root.join("docs"), Path::new("docs"), &mut files);
    collect_markdown(
        &root.join(".codex/skills"),
        Path::new(".codex/skills"),
        &mut files,
    );
    files
}

fn collect_markdown(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_markdown(&path, &rel, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(rel);
        }
    }
}

fn markdown_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index] == b']' && bytes[index + 1] == b'(' {
            let start = index + 2;
            if let Some(end_offset) = text[start..].find(')') {
                let target = text[start..start + end_offset].trim();
                if !target.is_empty() {
                    targets.push(target.to_string());
                }
                index = start + end_offset + 1;
                continue;
            }
        }
        index += 1;
    }
    targets
}

fn is_external_link(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("app://")
}

fn check_for_stale_doc_terms(root: &Path, findings: &mut Vec<Finding>) {
    for rel in markdown_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for term in STALE_DOC_TERMS {
            if text.contains(term) {
                findings.push(Finding::new(
                    "DOCS-STALE-TERM",
                    format!("{} contains stale term '{}'", rel.display(), term),
                ));
            }
        }
    }
}

fn check_required_doc_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-PUBLIC-API",
        "docs/specs/public-api.md",
        &[
            "pub fn prepare(&mut self, scene: &mut Scene) -> Result<(), PrepareError>;",
            "pub struct RendererStats",
            "pub enum Error",
            "pub enum SurfaceEvent",
            "Color::from_linear_rgb",
            "`MaterialDesc` is an immutable descriptor value",
            "Texture slots store `TextureHandle` values only",
            "MaterialDesc::unlit(base_color);",
            "MaterialDesc::pbr_metallic_roughness(base_color, metallic, roughness);",
            "material.with_base_color_texture(texture);",
            "material.with_normal_texture(texture);",
            "material.with_metallic_roughness_texture(texture);",
            "material.with_occlusion_texture(texture);",
            "material.with_emissive_texture(texture);",
            "material.with_alpha_mode(alpha_mode);",
            "material.with_emissive(color);",
            "material.with_emissive_strength(strength);",
            "material.with_double_sided(true);",
            "MaterialDesc::line(base_color, width_px);",
            "MaterialDesc::wireframe(base_color, width_px);",
            "MaterialDesc::edge(base_color, width_px);",
            "material.with_stroke_width_px(width_px);",
            "material.with_edge_angle_threshold_degrees(angle_threshold_degrees);",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-LIFECYCLE",
        "docs/specs/render-lifecycle.md",
        &[
            "warning watermark is 1024",
            "Retain policy is global and prospective",
            "`mixer.seek()` while paused",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-GLTF",
        "docs/specs/asset-gltf-contract.md",
        &[
            "Coordinate conversion must preserve visible winding",
            "extras.scena.connectors[]",
            "LookupError::StaleImport",
            "cubic-spline quaternion output is normalized",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-VISUAL",
        "docs/specs/visual-quality-contract.md",
        &[
            "Rgba8UnormSrgb",
            "source SHA-256",
            "Screenshot determinism is scoped to a pinned backend profile",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-DOCTOR",
        "docs/specs/doctor-contract.md",
        &[
            "cargo run -p xtask -- doctor --docs",
            "cargo run -p xtask -- doctor --architecture",
            "cargo run -p xtask -- doctor --full",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-RELEASE-GATES",
        "docs/specs/release-gates.md",
        &["Doctor", "cargo run -p xtask -- doctor --full"],
    );
}

fn require_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(rule, format!("could not read {rel}")));
        return;
    };

    for needle in needles {
        if !text.contains(needle) {
            findings.push(Finding::new(
                rule,
                format!("{rel} is missing required contract text '{}'", needle),
            ));
        }
    }
}

fn check_source_scope(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for term in SOURCE_SCOPE_TERMS {
            if contains_scope_term(&lower, term) {
                findings.push(Finding::new(
                    "ARCH-SCOPE",
                    format!(
                        "{} contains renderer-forbidden term '{}'",
                        rel.display(),
                        term
                    ),
                ));
            }
        }
    }
}

fn check_module_boundaries(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-MODULES",
        "docs/specs/module-boundaries.md",
        &[
            "`scene`",
            "`assets`",
            "`geometry`",
            "`material`",
            "`render`",
            "`animation`",
            "`controls`",
            "`picking`",
            "`diagnostics`",
            "`platform`",
            "No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`",
        ],
    );

    forbid_contains(
        root,
        findings,
        "ARCH-PLATFORM",
        "src/platform.rs",
        &["wgpu::", "ForwardPass", "ShadowPass", "PostProcessPass"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSETS",
        "src/assets.rs",
        &["wgpu::", "RenderPass", "Surface"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER",
        "src/render.rs",
        &["fetch(", "load_scene", "load_texture", "gltf::"],
    );
    for rel in source_files(root)
        .into_iter()
        .filter(|path| path.starts_with("src/render"))
    {
        forbid_contains_path(
            root,
            findings,
            "ARCH-RENDER",
            &rel,
            &["fetch(", "load_scene", "load_texture", "gltf::"],
        );
    }
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/draw.rs",
        &[
            "create_shader_module",
            "create_render_pipeline",
            "create_buffer",
            "create_texture",
            "create_bind_group",
            "request_adapter",
            "request_device",
            "mapped_at_creation: true",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/webgl2.rs",
        &[
            "pub(super) fn prepare_canvas",
            "pub(super) fn prepare_canvas_vertices",
            "webgl2 resources were not prepared; call Renderer::prepare before render",
            "webgl2 vertex stream was not prepared; call Renderer::prepare after scene changes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu.rs",
        &["webgl2::prepare_canvas_vertices(", "material_slots"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/lifecycle.rs",
        &[
            "pub(in crate::render) fn clear_prepared_resources_for_context_recovery",
            "webgl2::clear_render_cache();",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/webgl2.rs",
        &["pub(super) fn clear_render_cache"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/surface.rs",
        &["gpu.clear_prepared_resources_for_context_recovery();"],
    );
}

fn check_asset_api_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/assets.rs",
        &[
            "pub async fn load_texture",
            "&self,",
            "color_space: TextureColorSpace",
            "Result<TextureHandle, AssetError>",
            "pub fn create_material(&self, material: impl Into<MaterialDesc>) -> MaterialHandle",
            "pub fn default_environment(&self) -> EnvironmentHandle",
            "pub async fn load_environment",
            "pub fn environment(&self, handle: EnvironmentHandle) -> Option<EnvironmentDesc>",
            "pub fn try_geometry",
            "pub fn try_material",
            "pub fn try_texture",
            "pub fn try_environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/assets/environment.rs",
        &[
            "pub struct EnvironmentDesc",
            "pub struct EnvironmentDerivative",
            "pub enum EnvironmentSourceKind",
            "pub enum WasmEnvironmentDelivery",
            "pub const fn source_kind(&self) -> EnvironmentSourceKind",
            "pub const fn source_dimensions(&self) -> Option<(u32, u32)>",
            "pub const fn is_equirectangular_hdr(&self) -> bool",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/diagnostics.rs",
        &[
            "pub enum AssetError",
            "UnsupportedRequiredExtension",
            "UnsupportedEnvironmentFormat",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/material.rs",
        &[
            "pub struct MaterialDesc",
            "pub const DEFAULT_STROKE_WIDTH_PX",
            "pub const DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES",
            "pub enum MaterialKind",
            "Unlit",
            "PbrMetallicRoughness",
            "Line",
            "Wireframe",
            "Edge",
            "pub const fn unlit",
            "pub const fn pbr_metallic_roughness",
            "pub const fn line",
            "pub const fn wireframe",
            "pub const fn edge",
            "pub const fn with_stroke_width_px",
            "pub const fn with_edge_angle_threshold_degrees",
            "pub const fn with_base_color_texture",
            "pub const fn with_normal_texture",
            "pub const fn with_metallic_roughness_texture",
            "pub const fn with_occlusion_texture",
            "pub const fn with_emissive_texture",
            "pub const fn with_alpha_mode",
            "pub const fn with_emissive(",
            "pub const fn with_emissive_strength",
            "pub const fn with_double_sided",
            "pub const fn kind(&self) -> MaterialKind",
            "pub const fn base_color(&self) -> Color",
            "pub const fn base_color_texture(&self) -> Option<TextureHandle>",
            "pub const fn normal_texture(&self) -> Option<TextureHandle>",
            "pub const fn metallic_roughness_texture(&self) -> Option<TextureHandle>",
            "pub const fn occlusion_texture(&self) -> Option<TextureHandle>",
            "pub const fn emissive_texture(&self) -> Option<TextureHandle>",
            "pub const fn alpha_mode(&self) -> AlphaMode",
            "pub const fn emissive(&self) -> Color",
            "pub const fn emissive_strength(&self) -> f32",
            "pub const fn metallic_factor(&self) -> f32",
            "pub const fn roughness_factor(&self) -> f32",
            "pub const fn double_sided(&self) -> bool",
            "pub const fn stroke_width_px(&self) -> Option<f32>",
            "pub const fn edge_angle_threshold_degrees(&self) -> Option<f32>",
            "metallic_factor: clamp_unit_or",
            "roughness_factor: clamp_unit_or",
            "cutoff: clamp_unit_or",
            "self.emissive_strength = non_negative_or",
            "DEFAULT_STROKE_WIDTH_PX",
            "DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES",
            "stroke_width_px: Some(positive_or",
            "Some(clamp_degrees_or",
        ],
    );
    check_material_desc_fields_private(root, findings);
    forbid_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/material.rs",
        &[
            "pub kind:",
            "pub base_color:",
            "pub base_color_texture:",
            "pub normal_texture:",
            "pub metallic_roughness_texture:",
            "pub occlusion_texture:",
            "pub emissive_texture:",
            "pub alpha_mode:",
            "pub emissive:",
            "pub emissive_strength:",
            "pub metallic_factor:",
            "pub roughness_factor:",
            "pub double_sided:",
            "pub struct MaterialTexture",
            "pub enum MaterialTexture",
            "pub type MaterialTexture",
            "pub trait MaterialTexture",
            "pub fn basic(",
            "pub const fn basic(",
            "Basic",
            "basic(",
            "Basic,",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/lib.rs",
        &["MaterialTexture", "Basic", "basic"],
    );
}

fn check_render_alpha_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum AlphaPipelineStatus",
            "LinearSourceOver",
            "BackendPassthrough",
            "pub alpha_pipeline: AlphaPipelineStatus",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/cpu.rs",
        &[
            "fn blend_source_over(source: Color, destination: Color) -> Color",
            "let blended = blend_source_over(color, cpu_frame.linear_frame[pixel_index])",
            "cpu_frame.linear_frame[pixel_index] = blended",
            "cpu_frame.output.encode_rgba8(blended)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render.rs",
        &[
            "linear_frame: Option<Vec<Color>>",
            "cpu::clear_cpu",
            "cpu::draw_primitive_cpu",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/build.rs",
        &["linear_frame: (!has_gpu).then(|| vec![Color::BLACK; target.pixel_len()])"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/gpu/pipeline.rs",
        &["blend: Some(wgpu::BlendState::ALPHA_BLENDING)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/prepare.rs",
        &["average_sort_depth", "camera_projection.camera_depth"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/prepare/types.rs",
        &["camera_projection: Option<&'lights CameraProjection>"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "tests/m1_geometry_materials.rs",
        &[
            "headless_alpha_blends_in_linear_before_output_encoding",
            "prepare_with_assets_sorts_blend_meshes_by_camera_space_depth",
            "headless_gpu_alpha_blends_sorted_asset_meshes_when_available",
            "AlphaPipelineStatus::LinearSourceOver",
        ],
    );
}

fn check_output_stage_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/output.rs",
        &[
            "fn aces_tonemap",
            "fn rrt_and_odt_fit",
            "ACES_INPUT_MATRIX",
            "ACES_OUTPUT_MATRIX",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/gpu/output.rs",
        &[
            "fn aces_tonemap(color: vec3<f32>) -> vec3<f32>",
            "fn rrt_and_odt_fit(value: f32) -> f32",
            "camera_position_exposure: vec4<f32>",
            "viewport_near_far: vec4<f32>",
            "color_management: vec4<f32>",
            "fn encode_output_uniform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/gpu/pipeline.rs",
        &[
            "GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb",
            "pass.set_bind_group(0, inputs.output_bind_group, &[])",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum OutputStageStatus",
            "output_stage: OutputStageStatus::AcesSrgb",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "tests/m1_geometry_materials.rs",
        &["headless_gpu_output_stage_applies_aces_srgb_for_pinned_white_fixture"],
    );
}

fn check_fxaa_output_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/diagnostics.rs",
        &["pub fxaa_passes: u64"],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/render.rs",
        &[
            "fxaa_scratch: Vec<u8>",
            "output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch)",
            "self.stats.fxaa_passes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/render/output.rs",
        &[
            "pub(super) fn apply_fxaa_rgba8",
            "luma_from_srgb8",
            "FXAA_LUMA_THRESHOLD",
            "fn aces_tonemap",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "fxaa_pass_runs_after_aces_without_second_tonemap",
            "stats.fxaa_passes",
            "[206, 206, 206, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "docs/specs/public-api.md",
        &["pub fxaa_passes: u64", "tonemapper again"],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["FXAA pass attached", "ARCH-FXAA-OUTPUT"],
    );
}

fn check_diagnostics_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/diagnostics/diagnostic.rs",
        &[
            "pub struct Diagnostic",
            "pub code: DiagnosticCode",
            "pub severity: DiagnosticSeverity",
            "pub message: String",
            "pub help: Option<String>",
            "pub enum DiagnosticCode",
            "InvalidCameraProjection",
            "ObjectsBehindCamera",
            "SceneOutsideCameraFrustum",
            "LargeScenePrecisionRisk",
            "DepthPrecisionRisk",
            "WebGl2DepthCompatibility",
            "pub enum DiagnosticSeverity",
            "pub fn warning",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/diagnostics.rs",
        &["pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticSeverity}"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/lib.rs",
        &["Diagnostic", "DiagnosticCode", "DiagnosticSeverity"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render.rs",
        &[
            "diagnostics: Vec<Diagnostic>",
            "self.diagnostics.clear()",
            "prepare::collect_precision_diagnostics(scene, self.target.backend)",
            "prepare::collect_camera_projection_diagnostics(scene)",
            "prepare::collect_camera_visibility_diagnostics",
            "prepare::collect_asset_camera_visibility_diagnostics",
            "pub fn diagnostics(&self) -> &[Diagnostic]",
            "pub fn diagnose_scene_with_assets",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render/prepare/diagnostics.rs",
        &[
            "pub(in crate::render) fn collect_precision_diagnostics",
            "pub(in crate::render) fn collect_camera_projection_diagnostics",
            "pub(in crate::render) fn collect_camera_visibility_diagnostics",
            "pub(in crate::render) fn collect_asset_camera_visibility_diagnostics",
            "LARGE_SCENE_TRANSLATION_WARNING: f32 = 10_000.0",
            "DEPTH_RANGE_RATIO_WARNING: f32 = 100_000.0",
            "DiagnosticCode::InvalidCameraProjection",
            "DiagnosticCode::LargeScenePrecisionRisk",
            "DiagnosticCode::DepthPrecisionRisk",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "scene.mesh_bounds_nodes()",
            "mesh bounds",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/scene.rs",
        &[
            "pub(crate) fn node_transforms",
            "pub(crate) fn camera_nodes",
            "pub(crate) fn mesh_bounds_nodes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "prepare_emits_structured_depth_precision_warnings",
            "DiagnosticCode::DepthPrecisionRisk",
            "DiagnosticCode::LargeScenePrecisionRisk",
            "DiagnosticSeverity::Warning",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "m7_diagnostics_report_invalid_camera_projection_before_empty_frame",
            "m7_diagnostics_report_camera_visibility_failures_before_empty_frame",
            "m7_diagnostics_report_import_bounds_outside_camera_frustum",
            "m7_diagnostics_with_assets_report_direct_mesh_bounds_outside_camera_frustum",
            "m7_frame_all_uses_imported_mesh_bounds_without_manual_bounds_math",
            "frame_node",
            "DiagnosticCode::InvalidCameraProjection",
            "DiagnosticCode::ObjectsBehindCamera",
            "DiagnosticCode::SceneOutsideCameraFrustum",
            "DiagnosticSeverity::Error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "docs/specs/public-api.md",
        &[
            "pub struct Diagnostic",
            "InvalidCameraProjection",
            "ObjectsBehindCamera",
            "SceneOutsideCameraFrustum",
            "LargeScenePrecisionRisk",
            "DepthPrecisionRisk",
            "far/near ratio greater than",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Large-scene precision diagnostics", "ARCH-DIAGNOSTICS"],
    );
}

fn check_renderer_stats_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/diagnostics.rs",
        &[
            "pub struct RendererStats",
            "pub buffers: u64",
            "pub textures: u64",
            "pub materials: u64",
            "pub render_targets: u64",
            "pub pipelines: u64",
            "pub bind_groups: u64",
            "pub shader_modules: u64",
            "pub environments: u64",
            "pub environment_cubemaps: u64",
            "pub environment_prefilter_passes: u64",
            "pub environment_brdf_luts: u64",
            "pub scene_imports: u64",
            "pub shadow_maps: u64",
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
            "pub fxaa_passes: u64",
            "pub live_logical_handles: u64",
            "pub pending_destructions: u64",
            "pub approximate_gpu_memory_bytes: Option<u64>",
            "pub gpu_frame_ms: Option<f32>",
            "pub directional_shadow_map_resolution: Option<u32>",
            "pub directional_shadow_pcf_kernel: Option<u8>",
            "pub struct DevicePoll",
            "pub destroyed_resources: u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) struct PreparedEnvironmentStats",
            "pub(in crate::render) struct PreparedDepthStats",
            "pub(in crate::render) fn collect_environment_prepare_stats",
            "pub(in crate::render) fn collect_depth_prepass_stats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare/resources.rs",
        &[
            "pub(in crate::render) struct PreparedLogicalResourceStats",
            "pub(in crate::render) fn collect_logical_resource_stats",
            "material.base_color_texture()",
            "live_logical_handles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/stats.rs",
        &[
            "pub(in crate::render) struct GpuResourceStats",
            "fn estimate_prepared_resource_stats",
            "approximate_gpu_memory_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu.rs",
        &[
            "mod lifecycle;",
            "pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/lifecycle.rs",
        &[
            "pub(in crate::render) fn pending_destructions(&self) -> u64",
            "pub(in crate::render) fn poll_device(&mut self) -> (u64, bool)",
            "pub(in crate::render) fn release_prepared_resources(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render.rs",
        &[
            "pub fn poll_device(&mut self) -> DevicePoll",
            "self.stats.live_logical_handles = logical_stats.live_logical_handles",
            "self.stats.shadow_maps = lighting_stats.shadow_maps",
            "self.stats.depth_prepass_passes = depth_stats.passes",
            "self.stats.depth_prepass_draws = depth_stats.draws",
            "self.stats.textures = logical_stats.textures",
            "self.stats.environment_cubemaps = environment_prepare_stats.cubemaps",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "tests/m1_geometry_materials.rs",
        &[
            "m1_cpu_resource_lifetime_counters_return_to_baseline",
            "m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare",
            "m1_headless_gpu_resource_counters_return_to_baseline_after_empty_reprepare",
            "poll.pending_destructions_before",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/stats.rs",
        &[
            "estimates_prepared_headless_gpu_resource_counters",
            "estimates_empty_headless_gpu_resource_counters_at_baseline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "docs/specs/public-api.md",
        &[
            "pub struct RendererStats",
            "pub struct DevicePoll",
            "shadow_maps",
            "depth_prepass_passes",
            "depth_prepass_draws",
            "fxaa_passes",
            "live_logical_handles",
            "pub buffers: u64",
            "pub target_height: u32",
            "logical `TextureHandle` values only",
        ],
    );
}

fn check_render_world_bake_contracts(root: &Path, findings: &mut Vec<Finding>) {
    // Per-draw model/normal uniforms: prepared primitives must carry world_from_model
    // metadata via prepared_primitive(...) instead of being orchestrated through the bare
    // transform_primitive(...) baker that drops the per-renderable transform on the floor.
    // transforms.rs, shadows.rs, diagnostics.rs, and tangents.rs still call transform_primitive
    // and transform_position internally for ray-cast, bounds, and tangent helpers — those
    // call sites operate on local copies that never reach the GPU upload path.
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare.rs",
        &["prepared_primitive(primitive, transform, origin_shift)"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare.rs",
        &["transform_primitive(primitive, transform, origin_shift)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare/transforms.rs",
        &[
            "pub(super) fn prepared_primitive",
            "pub(super) fn world_from_model_matrix",
            "pub(super) fn normal_from_model_matrix",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/geometry/primitive.rs",
        &[
            "pub(crate) fn with_world_from_model",
            "pub(crate) fn world_from_model",
            "pub(crate) fn normal_from_model",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/gpu/vertices.rs",
        &[
            "pub(super) draw_uniform_index: u32",
            "pub(super) struct DrawUniformValue",
            "pub(super) world_from_model: [f32; 16]",
            "pub(super) normal_from_model: [f32; 16]",
        ],
    );
}

fn check_renderer_truth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities.rs",
        &[
            "const fn forward_pbr_status",
            "const fn directional_shadow_status",
            "const fn punctual_shadow_status",
            "CapabilityStatus::Degraded",
            "DiagnosticCode::ForwardPbrDegraded",
            "DiagnosticCode::DirectionalShadowsDegraded",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "DiagnosticCode::BloomDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
            "DiagnosticCode::GpuCullingDisabled",
            "const fn postprocess_status",
            "fn gpu_frustum_culling_status",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/camera.rs",
        &[
            "pub(super) struct CameraProjection",
            "view_from_world_matrix",
            "world_from_view_matrix",
            "clip_from_view_matrix",
            "view_from_clip_matrix",
            "clip_from_world_matrix",
            "world_to_view",
            "ndc_x",
            "ndc_y",
            "depth: f32",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/cpu.rs",
        &[
            "CameraProjection",
            "camera.project(vertex.position)",
            "depth_frame: &'frame mut [f32]",
            "mix_depth",
            "depth > cpu_frame.depth_frame[pixel_index]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "cpu_depth_buffer_keeps_nearer_triangle_visible_when_submitted_first",
            "headless_gpu_depth_buffer_keeps_nearer_triangle_visible_when_available",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &[
            "clip_from_world: mat4x4<f32>",
            "camera_position_exposure: vec4<f32>",
            "viewport_near_far: vec4<f32>",
            "color_management: vec4<f32>",
            "world_from_model: mat4x4<f32>",
            "normal_from_model: mat4x4<f32>",
            "view_from_world: mat4x4<f32>",
            "clip_from_view: mat4x4<f32>",
            "struct LightingUniform",
            "directional_light_direction_intensity",
            "point_light_position_intensity",
            "spot_light_direction_cones",
            "pbr_light_contribution",
            "pbr_environment_lighting",
            "fresnel_schlick",
            "distribution_ggx",
            "geometry_smith",
            "environment_diffuse_intensity",
            "environment_specular_intensity",
            "OUTPUT_UNIFORM_BYTE_LEN: u64 = 528",
            "camera.clip_from_view * camera.view_from_world * world_position",
            "camera.normal_from_model * vec4<f32>(in.normal, 0.0)",
            "@location(2) normal: vec3<f32>",
            "@location(3) tex_coord0: vec2<f32>",
            "@location(4) tangent: vec4<f32>",
            "in.tangent.w",
            "let normal_texture_sample = textureSample(normal_texture",
            "normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal",
            "var base_color_sampler: sampler",
            "var base_color_texture: texture_2d<f32>",
            "var<uniform> material: MaterialUniform",
            "var normal_sampler: sampler",
            "var normal_texture: texture_2d<f32>",
            "var metallic_roughness_sampler: sampler",
            "var metallic_roughness_texture: texture_2d<f32>",
            "var occlusion_sampler: sampler",
            "var occlusion_texture: texture_2d<f32>",
            "var emissive_sampler: sampler",
            "var emissive_texture: texture_2d<f32>",
            "base_color_uv_offset_scale",
            "base_color_uv_rotation",
            "base_color_factor",
            "emissive_strength",
            "metallic_roughness_alpha",
            "base.a < material.metallic_roughness_alpha.z",
            "discard;",
            "textureSample(base_color_texture, base_color_sampler, transformed_uv)",
            "textureSample(normal_texture",
            "textureSample(metallic_roughness_texture",
            "textureSample(occlusion_texture",
            "textureSample(emissive_texture",
            "triangle_shader_uses_camera_projection_uniform",
            "triangle_shader_declares_material_texture_bindings",
            "triangle_shader_samples_all_material_texture_roles",
            "triangle_shader_discards_alpha_masked_fragments",
            "triangle_shader_consumes_gpu_punctual_light_uniforms",
            "triangle_shader_consumes_gpu_environment_light_uniforms",
            "triangle_shader_builds_tangent_space_normal_from_normal_map",
        ],
    );
    if let Ok(shader_source) = fs::read_to_string(root.join("src/render/gpu/output.rs")) {
        let shader_const = shader_source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(&shader_source);
        if shader_const.contains("let normal_sample = textureSample(normal_texture") {
            findings.push(Finding::new(
                "ARCH-RENDER-TRUTH",
                "src/render/gpu/output.rs redeclares normal_sample in WGSL; Chrome WebGPU rejects \
                 this and the browser canvas can go black while Rust-side render stats still pass",
            ));
        }
    }
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/geometry.rs",
        &[
            "pub(crate) struct PrimitiveVertexAttributes",
            "pub(crate) normal: Vec3",
            "pub(crate) tex_coord0: [f32; 2]",
            "attributes: [PrimitiveVertexAttributes; 3]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/geometry/primitive.rs",
        &[
            "triangle_with_attributes",
            "attributes: [PrimitiveVertexAttributes; 3]",
            "vertex_attributes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/prepare.rs",
        &["accumulate_vertex_tangents", "authored_vertex_tangents"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/prepare/tangents.rs",
        &[
            "accumulate_vertex_tangents",
            "authored_vertex_tangents",
            "triangle_tangent",
            "raw_triangle_tangent_frame",
            "TangentFrame",
            "handedness",
            "fallback_tangent",
            "accumulated_vertex_tangents_average_shared_triangle_contributions",
            "accumulated_vertex_tangents_preserve_mirrored_uv_handedness",
            "authored_vertex_tangents_preserve_handedness_and_orthogonalize",
            "generated_triangle_tangent_follows_texcoord_u_axis",
            "generated_triangle_tangent_falls_back_for_degenerate_uvs",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/vertices.rs",
        &[
            "PrimitiveDrawBatch",
            "encode_draw_batches",
            "primitive.render_material_slot()",
            "VERTEX_BYTE_LEN: usize = 17",
            "shader_location: 2",
            "shader_location: 3",
            "shader_location: 4",
            "shader_location: 5",
            "Float32x4",
            "primitive.vertex_attributes()",
            "attributes.normal.x",
            "attributes.tex_coord0[0]",
            "attributes.tangent_handedness",
            "attributes.tangent.x",
            "attributes.shadow_visibility",
            "gpu_vertex_stream_carries_normals_and_texcoord0",
            "gpu_draw_batches_preserve_prepared_material_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/materials.rs",
        &[
            "MaterialTextureResources",
            "MaterialTextureUpload",
            "MaterialUniformUpload",
            "MATERIAL_UNIFORM_BYTE_LEN",
            "from_base_color_texture",
            "from_normal_texture",
            "from_metallic_roughness_texture",
            "from_occlusion_texture",
            "from_emissive_texture",
            "from_linear_texture",
            "create_material_bind_group_layout",
            "create_material_resources",
            "material_texture_byte_len",
            "Vec<MaterialTextureResources>",
            "binding: 2",
            "NORMAL_BINDINGS",
            "METALLIC_ROUGHNESS_BINDINGS",
            "OCCLUSION_BINDINGS",
            "EMISSIVE_BINDINGS",
            "SamplerBindingType::Filtering",
            "TextureSampleType::Float { filterable: true }",
            "scena.material.base_color",
            "scena.material.normal",
            "scena.material.metallic_roughness",
            "scena.material.occlusion",
            "scena.material.emissive",
            "scena.material.fallback_base_color",
            "scena.material.fallback_bind_group",
            "texture_byte_len",
            "decoded_base_color_texture_becomes_backend_upload",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/material_uniform.rs",
        &[
            "MaterialUniformUpload",
            "MATERIAL_UNIFORM_BYTE_LEN",
            "from_material",
            "from_transform",
            "base_color_factor",
            "emissive_strength",
            "metallic_roughness_alpha",
            "material_uniform_upload_encodes_base_color_texture_transform",
            "material_uniform_upload_encodes_material_factors",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render.rs",
        &[
            "collect_backend_material_slots(scene, assets)",
            "backend_material_handles",
            "backend_sampled_base_color_textures",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/pipeline.rs",
        &[
            "RenderPassDepthStencilAttachment",
            "depth_stencil: depth_compare.map",
            "depth_write_enabled: Some(false)",
            "material_bind_group_layout",
            "material_resources",
            "pass.set_bind_group(1, &material.bind_group",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/depth.rs",
        &[
            "camera.clip_from_view * camera.view_from_world * camera.world_from_model",
            "pub(super) color_compare",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2.rs",
        &[
            "camera_uniforms",
            "bind_camera_uniforms",
            "WebGl2CameraUniformUpload",
            "bind_material_texture",
            "PrimitiveDrawBatch",
            "draw_batch_hash",
            "uniform4f",
            "uniform1i",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2_camera.rs",
        &[
            "world_from_model",
            "normal_from_model",
            "view_from_world",
            "clip_from_view",
            "clip_from_world",
            "camera_position_exposure",
            "viewport_near_far",
            "color_management",
            "bind_camera_uniforms",
            "uniform_matrix4fv_with_f32_array",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2_program.rs",
        &[
            "uniform mat4 world_from_model",
            "uniform mat4 normal_from_model",
            "uniform mat4 view_from_world",
            "uniform mat4 clip_from_view",
            "uniform mat4 clip_from_world",
            "uniform vec4 camera_position_exposure",
            "uniform vec4 viewport_near_far",
            "uniform vec4 color_management",
            "uniform vec4 directional_light_direction_intensity",
            "uniform vec4 point_light_position_intensity",
            "uniform vec4 spot_light_direction_cones",
            "uniform vec4 environment_diffuse_intensity",
            "uniform vec4 environment_specular_intensity",
            "pbrLightContribution",
            "pbrEnvironmentLighting",
            "fresnelSchlick",
            "distributionGgx",
            "geometrySmith",
            "uniform vec4 base_color_uv_offset_scale",
            "uniform vec4 base_color_uv_rotation",
            "uniform sampler2D base_color_texture",
            "in vec3 normal",
            "in vec4 tangent",
            "in vec2 tex_coord0",
            "in vec4 v_tangent",
            "in vec2 v_tex_coord0",
            "v_tangent.w",
            "normal_sample_tangent_space.x * world_tangent + normal_sample_tangent_space.y * bitangent + normal_sample_tangent_space.z * world_normal",
            "texture(base_color_texture, transformed_uv)",
            "base.a < metallic_roughness_alpha.z",
            "discard;",
            "webgl2_fragment_shader_discards_alpha_masked_fragments",
            "clip_from_view * view_from_world * world_position",
            "mat3(normal_from_model) * normal",
            "webgl2_vertex_shader_consumes_model_normal_view_and_projection_uniforms",
            "webgl2_fragment_shader_consumes_gpu_punctual_light_uniforms",
            "webgl2_fragment_shader_consumes_gpu_environment_light_uniforms",
            "webgl2_shader_builds_tangent_space_normal_from_normal_map",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2_lighting.rs",
        &[
            "WebGl2LightingUniformLocations",
            "query_lighting_uniform_locations",
            "bind_lighting_uniforms",
            "directional_light_direction_intensity",
            "point_light_position_intensity",
            "spot_light_direction_cones",
            "spot_light_cone_range",
            "environment_diffuse_intensity",
            "environment_specular_intensity",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2_materials.rs",
        &[
            "create_material_texture",
            "upload_material_texture_if_dirty",
            "MaterialTextureUpload::from_base_color_texture",
            "upload.sampler.wrap_s()",
            "upload.sampler.wrap_t()",
            "webgl2_wrap_mode",
            "webgl2_filter_mode",
            "webgl2_filter_uses_mipmaps",
            "tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &["out.position = vec4<f32>(in.position, 1.0);"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2.rs",
        &["gl_Position = vec4(position, 1.0);"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/webgl2_program.rs",
        &["gl_Position = vec4(position, 1.0);"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/depth.rs",
        &["return vec4<f32>(in.position, 1.0);"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/picking.rs",
        &[
            "struct Ray",
            "fn camera_ray",
            "ray_hits_bounds",
            "ray_triangle_intersection",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors.rs",
        &[
            "with_allowed_mate",
            "with_snap_tolerance",
            "with_clearance_hint",
            "with_roll_policy",
            "with_polarity",
            "with_metadata",
            "pub const fn metadata",
            "pub struct ConnectionLineOverlay",
            "pub const fn connection_line",
            "pub const fn resolved_parent",
            "fn reparent_for_connection",
            "pub fn from_anchor_frame",
            "pub fn add_connector",
            "pub fn connector_named",
            "pub fn validate_connections",
            "pub fn connect_by_key",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/scale.rs",
        &["fn preserve_source_scale", "rotate_vec3"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/metadata.rs",
        &[
            "pub struct ConnectorMetadata",
            "pub enum ConnectorRollPolicy",
            "pub enum ConnectorPolarity",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/error.rs",
        &[
            "pub enum ConnectionError",
            "StaleConnectorHandle",
            "AmbiguousConnector",
            "UnitMismatch",
            "CoordinateSystemMismatch",
            "FlippedConnection",
            "ConnectionWouldMoveLockedNode",
            "ConnectionWouldCreateCycle",
            "ConnectorHostNotPrepared",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/options.rs",
        &[
            "pub enum ConnectionAlignment",
            "pub enum ConnectionRoll",
            "pub enum ConnectionParenting",
            "ForwardToBack",
            "NormalToOpposite",
            "pub const fn with_alignment",
            "pub const fn preserve_roll",
            "pub fn choose_nearest_roll_degrees",
            "pub const fn with_explicit_roll_degrees",
            "pub const fn reparent_source_to_target_parent",
            "alignment_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/imports.rs",
        &[
            "pub fn connect_import_connectors",
            "ConnectorFrame::from_import_connector",
            "connector_lookup_error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/locks.rs",
        &[
            "pub fn lock_node_for_connections",
            "pub fn unlock_node_for_connections",
            "pub fn node_connections_locked",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/assets/gltf/transform.rs",
        &[
            "basis_rotation",
            "forward",
            "up",
            "quat_from_rotation_columns",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/assets/gltf/anchors.rs",
        &[
            "tags",
            "label",
            "source_units",
            "parse_source_units",
            "pub struct SceneAssetAnchor",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/anchors.rs",
        &[
            "pub struct AnchorFrame",
            "pub fn from_import_anchor",
            "pub fn add_anchor",
            "pub fn anchor_named",
            "placement_node",
            "MissingAnchor",
            "StaleAnchorHandle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/validation.rs",
        &[
            "fn is_valid_rotation",
            "validate_connector_live",
            "validate_connector_host_prepared",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "m7_camera_projection_renders_world_space_triangle_outside_ndc",
            "m7_moving_camera_changes_rendered_pixels_and_screen_position",
            "m7_rotating_camera_changes_rendered_pixels_and_recenters_target",
            "m7_perspective_and_orthographic_cameras_project_different_pixel_footprints",
            "m7_default_perspective_camera_uses_render_target_aspect_on_wide_viewports",
            "m7_device_pixel_ratio_resize_preserves_projection_aspect",
            "m7_cpu_and_headless_gpu_camera_projection_match_within_tolerance_when_available",
            "m7_picking_uses_camera_ray_for_world_space_triangle_outside_ndc",
            "m7_rotated_connector_connects_without_sideways_orientation_or_offset",
            "m7_forward_to_back_alignment_flips_source_without_manual_rotation",
            "m7_connector_mate_offset_is_applied_in_target_connector_space",
            "m7_connectors_reject_degenerate_connector_rotation",
            "m7_manual_source_unit_mismatch_returns_structured_error",
            "m7_manual_source_coordinate_mismatch_returns_structured_error",
            "m7_negative_determinant_connector_scale_returns_flipped_connection",
            "m7_negative_determinant_node_scale_returns_flipped_connection",
            "m7_locked_connection_source_fails_before_moving_node",
            "m7_gltf_anchor_and_connector_basis_fields_avoid_manual_quaternions",
            "m7_z_up_import_node_rotation_converts_before_connection_solving",
            "m7_explicit_roll_alignment_rotates_around_mated_connector_forward_axis",
            "m7_preserve_roll_alignment_keeps_source_roll_without_manual_matrix_math",
            "m7_choose_nearest_roll_alignment_snaps_source_roll_without_guessing",
            "m7_connection_reparenting_is_explicit_and_preserves_world_transform",
            "m7_connector_placement_preserves_fit_inside_scale_when_solving_position",
            "m7_connector_name_lookup_reports_ambiguity_with_typed_handles",
            "m7_validate_connections_returns_preview_without_mutating_scene",
            "connection_line",
            "m7_stale_import_connector_handle_after_hot_reload_is_detected",
            "m7_connector_placement_applies_source_units_before_solving",
            "m7_gltf_anchor_units_override_import_units_for_connection_solving",
            "m7_anchor_frame_registry_uses_typed_handles_and_metadata",
            "m7_import_anchor_tags_and_label_survive_anchor_frame_adapter",
            "m7_import_anchor_frame_preserves_source_metadata_for_connector_adapter",
            "m7_connector_frame_metadata_guides_compatibility_without_domain_logic",
            "m7_imported_gltf_connectors_have_kind_lookup_and_stale_errors",
            "m7_imported_gltf_connector_metadata_survives_frame_adapter",
            "m7_three_imported_objects_connect_into_assembly_without_raw_matrix_math",
            "m7_first_assembly_helper_connects_imported_connectors_by_name",
            "m7_imported_nested_connector_moves_import_root_without_breaking_child_local_transform",
            "m7_imported_animated_connector_keeps_import_local_animation_binding_after_connection",
            "ImportDiagnosticOverlayKind::Connector",
            "AnchorFrame::from_import_anchor",
            "ConnectorFrame::from_import_anchor",
            "ConnectorFrame::from_import_connector",
            "connect_import_connectors",
            "ConnectorFrame::from_anchor_frame",
            "NonUniformScaleConnectionRisk",
            "ConnectionAlignment::ForwardToBack",
            "with_explicit_roll_degrees",
            "choose_nearest_roll_degrees",
            "ConnectorRollPolicy::ChooseNearest",
            "ConnectorPolarity::Plug",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities.rs",
        &[
            "forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "directional_shadow_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "punctual_shadow_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "gpu_frustum_culling_status(backend: Backend) -> CapabilityStatus {\n    match backend {\n        Backend::Headless\n        | Backend::HeadlessGpu\n        | Backend::SurfaceDescriptor\n        | Backend::NativeSurface\n        | Backend::WebGpu\n        | Backend::WebGl2 => CapabilityStatus::Supported",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["forward_pbr: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["directional_shadows: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["point_shadows: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["spot_shadows: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "Cargo.toml",
        &["version = \"1.0.0\""],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "examples/glb_model_viewer.rs",
        &["minimal_scene.gltf"],
    );
}

fn check_prepare_asset_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render.rs",
        &[
            "pub fn prepare_with_assets",
            "prepare::collect_prepared_primitives",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare.rs",
        &[
            "fn collect_prepared_primitives",
            "PrepareError::AssetsRequired",
            "fn append_geometry_primitives",
            "TransparentPrimitive",
            "total_cmp",
            "fn average_sort_depth",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/materials.rs",
        &["fn material_pass", "validate_material_texture_handles"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/strokes.rs",
        &[
            "fn append_line_primitives",
            "fn append_wireframe_primitives",
            "fn append_edge_primitives",
            "struct EdgeCandidate",
            "fn append_line_segment",
            "fn screen_x_to_ndc",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/diagnostics.rs",
        &[
            "AssetsRequired",
            "GeometryNotFound",
            "MaterialNotFound",
            "TextureNotFound",
            "UnsupportedGeometryTopology",
            "UnsupportedMaterialKind",
            "UnsupportedAlphaMode",
            "UnsupportedModelNode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/scene.rs",
        &["pub(crate) fn mesh_nodes", "pub(crate) fn model_nodes"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "tests/m1_geometry_materials.rs",
        &[
            "prepare_with_assets_renders_scene_mesh_unlit_geometry",
            "prepare_without_assets_rejects_asset_backed_mesh_nodes",
            "prepare_with_assets_sorts_blend_meshes_back_to_front_before_render",
            "prepare_with_assets_renders_line_material_as_screen_space_stroke",
            "prepare_with_assets_renders_wireframe_material_triangle_edges",
            "prepare_with_assets_renders_edge_material_without_coplanar_internal_edges",
            "headless_gpu_renders_technical_material_primitives_when_available",
            "prepare_with_assets_rejects_unsupported_mesh_inputs_structurally",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "docs/specs/public-api.md",
        &["pub fn prepare_with_assets<F>"],
    );
}

fn check_environment_lifecycle_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render.rs",
        &[
            "environment: Option<EnvironmentHandle>",
            "environment_revision: u64",
            "PrepareError::EnvironmentAssetsRequired",
            "PrepareError::EnvironmentNotFound",
            "NotPreparedReason::EnvironmentChanged",
            "ChangeKind::Environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render/settings.rs",
        &[
            "pub fn environment(&self) -> Option<EnvironmentHandle>",
            "pub fn set_environment(&mut self, environment: EnvironmentHandle)",
            "pub fn clear_environment(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/diagnostics.rs",
        &[
            "EnvironmentAssetsRequired",
            "EnvironmentNotFound",
            "EnvironmentChanged",
            "Environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "tests/m1_geometry_materials.rs",
        &[
            "renderer_environment_is_structural_and_validated_during_prepare",
            "m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare",
            "renderer.clear_environment()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "tests/m1_visual_proof.rs",
        &[
            "render_default_cube_with_default_environment",
            "validate_default_cube_luminance_and_silhouette",
        ],
    );
}

fn check_equirectangular_hdr_environment_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets/environment.rs",
        &[
            "EnvironmentSourceKind::EquirectangularHdr",
            "pub fn from_equirectangular_hdr_path",
            "from_equirectangular_hdr_bytes",
            "is_equirectangular_hdr_path",
            "parse_equirectangular_hdr_dimensions",
            "parse_radiance_hdr_preview",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets.rs",
        &[
            "AssetError::UnsupportedEnvironmentFormat",
            "embedded_environment_bytes",
            "only base64 Radiance HDR data URIs",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/lib.rs",
        &["EnvironmentSourceKind"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "equirectangular_hdr_environment_loading_records_source_contract",
            "EnvironmentSourceKind::EquirectangularHdr",
            "UnsupportedEnvironmentFormat",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_environment_hdr_lights_pbr_preview_pixels",
            "m8_environment_hdr_data_uri_lights_pbr_preview_pixels",
            "tiny_radiance_hdr_rgbe",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Equirectangular HDR environment loading",
            "EnvironmentSourceKind",
        ],
    );
}

fn check_environment_ibl_prepare_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) struct PreparedEnvironmentStats",
            "cubemaps: 1",
            "prefilter_passes: 1",
            "brdf_luts: 1",
            "environment.is_equirectangular_hdr()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render.rs",
        &[
            "prepare::collect_environment_prepare_stats(environment_desc.as_ref())",
            "prepare::collect_environment_lighting(environment_desc.as_ref())",
            "self.stats.environment_cubemaps = environment_prepare_stats.cubemaps",
            "self.stats.environment_prefilter_passes = environment_prepare_stats.prefilter_passes",
            "self.stats.environment_brdf_luts = environment_prepare_stats.brdf_luts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/environment.rs",
        &[
            "pub(in crate::render) struct PreparedEnvironmentLighting",
            "EnvironmentDesc::preview_irradiance_rgb",
            "gpu_diffuse_intensity",
            "gpu_specular_intensity",
            "pbr_contribution",
            "collect_environment_lighting",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "equirectangular_environment_prepare_generates_ibl_resources",
            "environment_cubemaps",
            "environment_prefilter_passes",
            "environment_brdf_luts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Cubemap conversion", "ARCH-ENV-IBL-PREP"],
    );
}

fn check_scene_light_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene.rs",
        &[
            "pub struct LightKey",
            "mod lights;",
            "pub use lights::{DirectionalLight, Light, LightBuilder, PointLight, SpotLight}",
            "NodeKind::Light",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene/lights.rs",
        &[
            "pub enum Light",
            "pub struct DirectionalLight",
            "pub struct PointLight",
            "pub struct SpotLight",
            "casts_shadows: bool",
            "pub fn directional_light(&mut self, light: DirectionalLight) -> LightBuilder<'_>",
            "pub fn point_light(&mut self, light: PointLight) -> LightBuilder<'_>",
            "pub fn spot_light(&mut self, light: SpotLight) -> LightBuilder<'_>",
            "pub fn light(&self, light: LightKey) -> Option<&Light>",
            "pub const fn casts_shadows",
            "pub const fn with_shadows",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/lib.rs",
        &[
            "DirectionalLight",
            "LightBuilder",
            "LightKey",
            "PointLight",
            "SpotLight",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "scene_light_components_are_typed_and_node_owned",
            ".directional_light",
            ".point_light",
            ".spot_light",
            "NodeKind::Light",
        ],
    );
}

fn check_direct_light_shading_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/scene.rs",
        &[
            "impl Iterator<Item = (NodeKey, LightKey, Light, Transform)>",
            "self.world_transform(node_key)",
            "map(|transform| (node_key, light_key, light, transform))",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare.rs",
        &[
            "mod lighting;",
            "use self::lighting::{MaterialShadingInput, PreparedLights, material_color};",
            "let lights = PreparedLights::from_scene(scene, origin_shift)",
            "material_color(",
            "MaterialShadingInput {",
            ".map(CameraProjection::camera_position)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare/lighting.rs",
        &[
            "pub(super) struct MaterialShadingInput",
            "pub(super) struct PreparedLights",
            "pub(super) fn from_scene(scene: &Scene, origin_shift: Vec3) -> Self",
            "lights.has_direct_lights() || input.environment.is_active()",
            "shade_pbr_base_color",
            "pbr_light_contribution",
            "input.environment",
            ".pbr_contribution(",
            "input.metallic_roughness_texture",
            "input.occlusion_texture",
            "input.emissive_texture",
            "fresnel_schlick",
            "geometry_schlick_ggx",
            "material.metallic_factor()",
            "material.roughness_factor()",
            "light_direction(transform)",
            "light.illuminance_lux()",
            "light.intensity_candela()",
            "spot_cone_attenuation",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "direct_lights_tint_pbr_mesh_output",
            "MaterialDesc::pbr_metallic_roughness",
            "with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))",
            "red-dominant PBR preview output",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "direct_lights_tint_pbr_mesh_output",
            "ARCH-DIRECT-LIGHT-SHADING",
        ],
    );
}

fn check_directional_shadow_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) fn collect_lighting_stats(",
            "backend: Backend",
            "Capabilities::for_backend(backend)",
            "scene.light_nodes()",
            "light.casts_shadows()",
            "PrepareError::MultipleShadowedDirectionalLights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/shadows.rs",
        &[
            "pub(super) fn collect_shadow_occluders",
            "pub(super) fn directional_shadow_factor",
            "ray_intersects_triangle(",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/diagnostics.rs",
        &["MultipleShadowedDirectionalLights"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/diagnostics/display.rs",
        &["only one shadowed directional light"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/lighting.rs",
        &[
            "casts_shadows: bool",
            "input.directional_shadow_factor",
            "primary_shadow_ray_direction",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/geometry.rs",
        &["shadow_visibility: f32"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare.rs",
        &[
            "shadow_visibility_a",
            "shadow_visibility_b",
            "shadow_visibility_c",
            "directional_shadow_factor(position_a",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/gpu/output.rs",
        &[
            "@location(5) shadow_visibility: f32",
            "* shadow_visibility",
            "triangle_shader_consumes_prepared_directional_shadow_visibility",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/gpu/webgl2_program.rs",
        &[
            "in float shadow_visibility",
            "* shadow_visibility",
            "webgl2_fragment_shader_consumes_prepared_directional_shadow_visibility",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "shadowed_directional_light_is_opt_in_and_single_owner",
            "directional_shadow_receiver_pixels_are_darkened_by_caster",
            "headless_gpu_directional_shadow_visibility_darkens_receiver_when_available",
            "with_shadows(true)",
            "MultipleShadowedDirectionalLights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &["pbr-shadow-visibility", "assertShadowVisibilityProof"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "One opt-in shadowed directional light",
            "with_shadows(true)",
        ],
    );
}

fn check_shadow_map_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/diagnostics.rs",
        &[
            "pub shadow_maps: u64",
            "pub directional_shadow_map_resolution: Option<u32>",
            "pub directional_shadow_pcf_kernel: Option<u8>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/diagnostics/capabilities.rs",
        &[
            "pub directional_shadows: CapabilityStatus",
            "pub point_shadows: CapabilityStatus",
            "pub spot_shadows: CapabilityStatus",
            "const fn directional_shadow_status",
            "const fn punctual_shadow_status",
            "DiagnosticCode::DirectionalShadowsDegraded",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "pub directional_shadow_map_default_size: u32",
            "pub directional_shadow_map_max_size: u32",
            "pub directional_shadow_pcf_kernel: u8",
            "pub bloom: CapabilityStatus",
            "pub screen_space_ambient_occlusion: CapabilityStatus",
            "DiagnosticCode::BloomDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
            "pub reversed_z_depth: CapabilityStatus",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/prepare/stats.rs",
        &[
            "Capabilities::for_backend(backend)",
            "capabilities.directional_shadow_map_default_size",
            "DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3",
            "pub(in crate::render) struct PreparedLightingStats",
            "shadow_maps: 1",
            "directional_shadow_pcf_kernel: Some(DIRECTIONAL_SHADOW_PCF_KERNEL)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu.rs",
        &[
            "shadow_texture: Option<wgpu::Texture>",
            "shadow_view: Option<wgpu::TextureView>",
            "ARCH-SHADOW-MAP: M2 allocates shadow resources before the shadow render pass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/shadow.rs",
        &[
            "pub(super) fn create_shadow_texture",
            "wgpu::TextureFormat::Depth32Float",
            "wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING",
            "scena.m2.directional_shadow_map",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/stats.rs",
        &[
            "shadow_maps: u64",
            "shadow_map_resolution: Option<u32>",
            "depth_prepass_passes: u64",
            "textures: 1 + material_texture_count + shadow_maps + depth_prepass_passes",
            "render_targets: 1 + shadow_maps + depth_prepass_passes",
            "estimates_single_shadow_map_resource_counters",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "single_shadow_map_records_pcf3x3_prepare_stats",
            "directional_shadow_map_default_size",
            "stats.shadow_maps",
            "directional_shadow_pcf_kernel",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "tests/m4_performance_platform.rs",
        &[
            "headless.point_shadows",
            "headless.spot_shadows",
            "headless.bloom",
            "headless.screen_space_ambient_occlusion",
            "DiagnosticCode::PointShadowsDisabled",
            "DiagnosticCode::SpotShadowsDisabled",
            "DiagnosticCode::BloomDisabled",
            "DiagnosticCode::AmbientOcclusionDisabled",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Single shadow map with PCF 3x3", "ARCH-SHADOW-MAP"],
    );
}

fn check_depth_prepass_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/diagnostics.rs",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) struct PreparedDepthStats",
            "pub(in crate::render) fn collect_depth_prepass_stats(",
            "backend: Backend",
            "DEPTH_PREPASS_MIN_PRIMITIVES: usize = 2",
            "fn depth_prepass_benefits",
            "passes: 1",
            "draws: primitives.len() as u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render.rs",
        &[
            "let depth_stats = prepare::collect_depth_prepass_stats(&primitives, self.target.backend)",
            "self.stats.depth_prepass_passes = depth_stats.passes",
            "self.stats.depth_prepass_draws = depth_stats.draws",
            "backend_material_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu.rs",
        &[
            "mod depth;",
            "PreparedDepthStats",
            "depth_prepass: Option<depth::DepthPrepassResources>",
            "depth::create_depth_prepass_resources",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/draw.rs",
        &[
            "depth::encode_depth_prepass",
            "depth_view",
            "scena.headless_gpu.render_pass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/depth.rs",
        &[
            "pub(super) struct DepthPrepassResources",
            "wgpu::TextureFormat::Depth32Float",
            "scena.m2.depth_prepass",
            "clear_depth",
            "pub(super) fn encode_depth_prepass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/stats.rs",
        &[
            "depth_prepass_passes: u64",
            "depth_prepass_bytes",
            "estimates_depth_prepass_resource_counters",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "depth_prepass_is_skipped_for_trivial_single_primitive_scene",
            "depth_prepass_is_prepared_when_multiple_opaque_primitives_benefit",
            "near_far_precision_fixture_keeps_depth_order_for_small_and_large_scenes",
            "exposure_change_rerenders_on_change_and_changes_nonflat_pixels",
            "depth_prepass_passes",
            "depth_prepass_draws",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Depth pre-pass", "ARCH-DEPTH-PREPASS"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "docs/specs/public-api.md",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
            "M2 also prepares a depth pre-pass",
        ],
    );
}

fn check_reversed_z_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum CapabilityStatus",
            "Supported",
            "FeatureDisabled",
            "pub reversed_z_depth: CapabilityStatus",
            "const fn reversed_z_depth_status",
            "Backend::WebGl2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/lib.rs",
        &["CapabilityStatus"],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/render/prepare/stats.rs",
        &[
            "reversed_z: bool",
            "capabilities.reversed_z_depth == CapabilityStatus::Supported",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/render/gpu/depth.rs",
        &[
            "reversed_z: bool",
            "wgpu::CompareFunction::GreaterEqual",
            "clear_depth: if reversed_z { 0.0 } else { 1.0 }",
            "wgpu::LoadOp::Clear(resources.clear_depth)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "capability_matrix_reports_reversed_z_depth_support_and_webgl2_fallback",
            "CapabilityStatus::Supported",
            "CapabilityStatus::FeatureDisabled",
            "Backend::WebGl2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "docs/specs/public-api.md",
        &[
            "pub reversed_z_depth: CapabilityStatus",
            "Capabilities::reversed_z_depth",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Reversed-Z support", "ARCH-REVERSED-Z"],
    );
}

fn check_webgl2_depth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics/capabilities.rs",
        &[
            "pub fn diagnostics(self) -> Vec<Diagnostic>",
            "self.backend == Backend::WebGl2",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "near/far ranges",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics/diagnostic.rs",
        &["WebGl2DepthCompatibility"],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "webgl2_depth_capability_reports_structured_compatibility_warning",
            "Capabilities::for_attached_gpu_backend(Backend::WebGl2).diagnostics()",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "near/far",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "docs/specs/public-api.md",
        &[
            "Capabilities::diagnostics()",
            "DiagnosticCode::WebGl2DepthCompatibility",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["WebGL2 depth compatibility warnings", "ARCH-WEBGL2-DEPTH"],
    );
}

fn check_m2_leak_stats_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M2-LEAK-STATS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "environment_cubemaps",
            "environment_prefilter_passes",
            "environment_brdf_luts",
            "shadow_maps",
            "depth_prepass_passes",
            "depth_prepass_draws",
            "released.textures, baseline.textures",
            "released.pending_destructions, baseline.pending_destructions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M2-LEAK-STATS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "ARCH-M2-LEAK-STATS",
        ],
    );
}

fn check_camera_depth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene.rs",
        &[
            "mod camera;",
            "pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera}",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene/camera.rs",
        &[
            "pub enum Camera",
            "pub struct PerspectiveCamera",
            "pub struct OrthographicCamera",
            "pub struct DepthRange",
            "aspect: 0.0",
            "pub const fn with_aspect(mut self, aspect: f32) -> Self",
            "pub const fn new(near: f32, far: f32) -> Self",
            "pub const fn fit_sphere(center_distance: f32, radius: f32) -> Self",
            "pub const fn contains_interval(self, near: f32, far: f32) -> bool",
            "pub const fn with_depth_range(mut self, range: DepthRange) -> Self",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/lib.rs",
        &["DepthRange", "PerspectiveCamera", "OrthographicCamera"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "camera_depth_fit_helpers_cover_unit_cube_reference_distances",
            "DepthRange::fit_sphere",
            "with_depth_range",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Camera depth-range and depth-fit helpers",
            "DepthRange::fit_sphere",
        ],
    );
}

fn check_clipping_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/scene.rs",
        &["pub struct ClippingPlaneKey"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/scene/clipping.rs",
        &[
            "pub struct ClippingPlane",
            "pub struct ClippingPlaneSet",
            "pub fn add_clipping_plane",
            "pub fn set_clipping_planes",
            "pub(crate) fn active_clipping_plane_values",
            "pub fn contains(self, point: Vec3) -> bool",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/diagnostics.rs",
        &["ClippingPlaneNotFound"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/lib.rs",
        &["ClippingPlane", "ClippingPlaneKey", "ClippingPlaneSet"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render.rs",
        &[
            "clipping_planes: Vec<ClippingPlane>",
            "scene.active_clipping_plane_values().collect()",
            "let clipping_planes = self.prepared_state(scene)?.clipping_planes.clone()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render/cpu.rs",
        &[
            "clipping_planes: &[ClippingPlane]",
            "mix_position",
            "is_clipped",
            "plane.contains(position)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "clipping_plane_set_clips_rendered_output_half_space",
            "ClippingPlane::new",
            "ClippingPlaneSet::new().with_plane",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "docs/specs/public-api.md",
        &[
            "pub struct ClippingPlaneKey",
            "dot(normal, position) + distance >= 0",
            "ClippingPlaneNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["ClippingPlane", "ARCH-CLIPPING"],
    );
}

fn check_origin_shift_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene.rs",
        &["origin_shift: Vec3"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene/origin.rs",
        &[
            "pub fn set_origin_shift",
            "pub fn origin_shift(&self) -> Vec3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/render/prepare.rs",
        &[
            "let origin_shift = scene.origin_shift()",
            "prepared_primitive",
            "transform_position",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/render/prepare/diagnostics.rs",
        &["subtract_vec3", "relative_translation"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "origin_shift_keeps_large_offset_renderable_visible_without_precision_warning",
            "scene.set_origin_shift",
            "DiagnosticCode::LargeScenePrecisionRisk",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "docs/specs/public-api.md",
        &["pub fn set_origin_shift", "large-world"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Camera-relative rendering or origin-shift support",
            "ARCH-ORIGIN-SHIFT",
        ],
    );
}

fn check_m3a_scene_import_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "Cargo.toml",
        &[
            "base64",
            "serde_json",
            "wasm-bindgen-futures",
            "Response",
            "obj = []",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets.rs",
        &[
            "mod fetch;",
            "mod gltf;",
            "mod obj;",
            "pub use fetch::{AssetFetcher, DefaultAssetFetcher}",
            "pub use gltf::{",
            "SceneAssetMesh",
            "scene_lookup: BTreeMap<AssetPath, SceneAsset>",
            "pub async fn load_scene",
            "pub async fn reload_scene",
            "RetainPolicy::Always",
            "ReloadRequiresRetain",
            "retained_source_bytes()",
            "with_retained_source_bytes",
            "SceneAsset::from_gltf_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/fetch.rs",
        &[
            "pub trait AssetFetcher",
            "pub type DefaultAssetFetcher",
            "pub struct FileAssetFetcher",
            "pub struct BrowserAssetFetcher",
            "window.fetch_with_str",
            "wasm_bindgen_futures::JsFuture",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/obj.rs",
        &[
            "pub async fn load_geometry",
            "parse_obj_geometry",
            "mtllib",
            "GeometryTopology::Triangles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf.rs",
        &[
            "pub struct SceneAsset",
            "pub struct SceneAssetClip",
            "pub struct SceneAssetLight",
            "pub struct SceneAssetNode",
            "pub struct SceneAssetMesh",
            "pub(super) fn from_gltf_bytes",
            "pub(super) fn from_gltf_bytes_with_external_resources",
            "pub(super) fn external_buffer_paths",
            "pub(super) fn external_image_paths",
            "pub(super) fn from_gltf_source",
            "parse_glb",
            "pub fn mesh_count",
            "pub fn retained_source_bytes_len",
            "pub(super) fn retained_source_bytes",
            "pub(super) fn with_retained_source_bytes",
            "pub fn transform(&self)",
            "pub fn mesh(&self)",
            "pub fn meshes(&self)",
            "pub fn anchors(&self)",
            "pub fn connectors(&self)",
            "pub fn clips(&self)",
            "pub fn light(&self)",
            "pub const fn bounds",
            "pub const fn uses_vertex_colors",
            "parse_punctual_lights",
            "parse_gltf_clips",
            "parse_node_anchors",
            "parse_node_connectors",
            "parse_node_transform",
            "UnsupportedRequiredExtension",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/anchors.rs",
        &[
            "pub struct SceneAssetAnchor",
            "pub(crate) fn invalid_reason",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/connectors.rs",
        &[
            "pub struct SceneAssetConnector",
            "pub(crate) fn invalid_reason",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/extensions.rs",
        &[
            "pub enum GltfExtensionStatus",
            "pub struct GltfExtensionDiagnostic",
            "pub(super) fn is_v1_required_gltf_extension",
            "pub(super) fn collect_extension_diagnostics",
            "KHR_lights_punctual",
            "KHR_materials_unlit",
            "KHR_materials_emissive_strength",
            "KHR_texture_transform",
            "KHR_mesh_quantization",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/transform.rs",
        &[
            "pub(super) fn parse_node_transform",
            "\"matrix\"",
            "matrix_transform",
            "quat_from_rotation_columns",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/anchors.rs",
        &[
            "pub(super) fn parse_node_anchors",
            "validate_anchor_extras",
            "validate_number_array",
            "anchor rotation quaternion must be normalized",
            "anchor scale components must not be zero",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/connectors.rs",
        &[
            "pub(super) fn parse_node_connectors",
            "\"connectors\"",
            "\"kind\"",
            "validate_connector_extras",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/glb.rs",
        &["pub(super) fn parse_glb", "GLB_BIN_CHUNK", "GLB_JSON_CHUNK"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/external.rs",
        &[
            "pub(super) fn external_buffer_paths",
            "resolve_relative_path",
            "!uri.starts_with(\"data:\")",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/accessor.rs",
        &[
            "parse_buffers",
            "binary_chunk",
            "external_buffers",
            "parse_buffer_views",
            "parse_accessors",
            "read_color_accessor",
            "read_normalized_components",
            "normalized",
            "GL_SHORT",
            "GL_UNSIGNED_SHORT",
            "base64::engine::general_purpose::STANDARD",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/read.rs",
        &[
            "parse_materials",
            "parse_meshes",
            "let primitives = mesh",
            "parse_mesh_primitive",
            "parse_texture_transform",
            "TextureTransform::new",
            "GeometryDesc::try_new_with_vertex_colors",
            "TextureColorSpace::Srgb",
            "KHR_materials_unlit",
            "KHR_materials_emissive_strength",
            "KHR_texture_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/material.rs",
        &[
            "pub struct TextureTransform",
            "pub const fn base_color_texture_transform",
            "pub const fn with_base_color_texture_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/geometry.rs",
        &["try_new_with_vertex_colors", "pub fn vertex_colors"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene.rs",
        &[
            "mod import;",
            "mod instances;",
            "mod labels;",
            "mod materials;",
            "mod picking;",
            "mod view;",
            "ImportOptions",
            "InstanceSetKey",
            "LabelKey",
            "SceneImport",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/materials.rs",
        &[
            "pub fn set_mesh_material",
            "NodeIsNotMesh",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/picking.rs",
        &[
            "pub fn pick(",
            "pickable_renderables",
            "pick_scene",
            "pub fn interaction(&self)",
            "pub fn interaction_mut(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/instances.rs",
        &[
            "pub struct InstanceId",
            "pub enum InstanceCullingPolicy",
            "CpuBoundingBoxFallback",
            "pub struct InstanceSet",
            "pub fn add_instance_set",
            "pub fn reserve_instances",
            "pub fn push_instance",
            "pub fn remove_instance",
            "pub fn clear_instances",
            "pub fn instances(&self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/labels.rs",
        &[
            "pub struct LabelDesc",
            "pub enum LabelRasterization",
            "pub enum LabelBillboard",
            "pub fn sdf",
            "pub fn msdf",
            "pub fn add_label",
            "pub fn set_label_text",
            "LabelNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/picking.rs",
        &[
            "pub struct CursorPosition",
            "pub struct Viewport",
            "pub enum HitTarget",
            "pub struct Hit",
            "pub struct InteractionContext",
            "pub struct InteractionStyle",
            "set_hover",
            "set_primary_selection",
            "pub(crate) const fn revision",
            "pub(crate) fn pick_scene",
            "HitTarget::Node",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare.rs",
        &[
            "scene.instance_set_nodes()",
            "labels::append_label_primitives",
            "compose_transform",
            "instance_set.geometry()",
            "instance_set.material()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare/labels.rs",
        &[
            "pub(super) fn append_label_primitives",
            "scene.label_nodes()",
            "LabelBillboard::ScreenAligned",
            "Primitive::triangle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render.rs",
        &[
            "mod offscreen;",
            "hover_style: InteractionStyle",
            "selection_style: InteractionStyle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/offscreen.rs",
        &[
            "pub struct OffscreenTarget",
            "pub struct PixelReadback",
            "pub fn offscreen",
            "pub fn read_pixels",
            "pub fn into_rgba8",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/settings.rs",
        &["pub fn set_hover_style", "pub fn set_selection_style"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/view.rs",
        &[
            "pub fn camera_node",
            "pub fn frame(&mut self, camera: CameraKey, bounds: Aabb)",
            "pub fn frame_all",
            "pub fn frame_node",
            "pub fn look_at(&mut self, camera: CameraKey, target: NodeKey)",
            "DepthRange::fit_sphere",
            "set_node_transform_and_mark_changed",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import.rs",
        &[
            "pub struct ImportOptions",
            "pub enum SourceUnits",
            "pub enum SourceCoordinateSystem",
            "Centimeters",
            "Inches",
            "Feet",
            "ZUpRightHanded",
            "pub struct SceneImport",
            "pub struct ImportAnchor",
            "pub struct ImportConnector",
            "pub struct ImportClip",
            "pub struct ImportPivot",
            "pub fn instantiate(",
            "pub fn instantiate_with(",
            "pub async fn import<",
            "pub async fn import_with<",
            "pub fn replace_import(",
            "mark_stale",
            "node_bounds",
            "source_node.meshes()",
            "scene_asset: &SceneAsset",
            "InvalidAnchorExtras",
            "convert_marker_units(",
            "placement_node",
            "placement_transform",
            "root_from_node",
            "convert_transform(source_node.transform())",
            "ImportDiagnosticOverlayKind::Origin",
            "ImportDiagnosticOverlayKind::Axes",
            "ImportDiagnosticOverlayKind::Bounds",
            "ImportDiagnosticOverlayKind::Anchor",
            "ImportDiagnosticOverlayKind::Connector",
            "ImportDiagnosticOverlayKind::Pivot",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/handedness.rs",
        &[
            "reject_unproven_left_handed_mesh_import",
            "has_negative_determinant",
            "UnsupportedCoordinateSystem",
            "left-handed mesh imports require explicit winding and normal correction proof",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/types.rs",
        &["ImportBuild", "NodeKind::Mesh", "mesh_node_kind"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/options.rs",
        &[
            "pub const fn gltf_default() -> Self",
            "pub const fn with_source_units",
            "pub const fn with_source_coordinate_system",
            "pub(super) fn convert_transform",
            "convert_connector_transform(transform)",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/accessors.rs",
        &[
            "pub const fn placement_node",
            "self.placement_transform",
            "pub fn channels(&self)",
            "pub const fn duration_seconds",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/lookups.rs",
        &[
            "LookupError::StaleImport",
            "pub fn node(&self, name: &str)",
            "pub fn first_node(&self, name: &str)",
            "pub fn nodes_named",
            "pub fn path(&self, path: &str)",
            "pub fn bounds_local",
            "pub fn bounds_world",
            "scene.world_transform(record.node)",
            "pub fn pivot(&self",
            "pub fn diagnostic_overlays",
            "pub fn anchor(&self",
            "pub fn replacement_anchor",
            "pub fn connector(&self",
            "pub fn replacement_connector",
            "pub fn connectors_named",
            "pub fn first_anchor",
            "pub fn anchors_named",
            "pub fn clip(&self",
            "pub fn first_clip",
            "pub fn clips_named",
            "AmbiguousAnchorName",
            "AmbiguousClipName",
            "fn path_segments(path: &str)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/animation.rs",
        &["pub struct AnimationClipKey", "pub(crate) fn fresh"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/diagnostics.rs",
        &[
            "InstantiateError",
            "ImportError",
            "StaleImport",
            "NodeNameNotFound",
            "AmbiguousNodeName",
            "AnchorNotFound",
            "AmbiguousAnchorName",
            "ClipNotFound",
            "AmbiguousClipName",
            "PathNotFound",
            "NodeIsNotMesh",
            "pub struct ImportDiagnosticOverlay",
            "pub enum ImportDiagnosticOverlayKind",
            "pub const fn source_units",
            "pub const fn source_coordinate_system",
            "Connector",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "tests/m3a_app_features.rs",
        &[
            "assets_load_scene_caches_gltf_asset_and_rejects_required_extensions",
            "scene_instantiate_creates_import_hierarchy_and_name_lookups",
            "scene_import_convenience_uses_gltf_default_options",
            "replace_import_returns_fresh_import_and_stales_old_lookups",
            "scene_import_reports_duplicate_names_and_escaped_paths",
            "camera_frame_and_look_at_helpers_update_view_and_require_prepare",
            "assets_load_scene_uses_fetcher_trait_and_deduplicates_by_asset_path",
            "gltf_loader_creates_geometry_material_texture_and_vertex_color_contracts",
            "glb_loader_reads_binary_chunk_mesh_materials_and_instantiates",
            "gltf_loader_fetches_external_buffers_relative_to_scene_path",
            "gltf_loader_preserves_multi_primitive_meshes_as_child_mesh_nodes",
            "reload_scene_requires_retain_and_reprepare_after_replace_import",
            "import_options_apply_gltf_node_transforms_and_source_units",
            "source_coordinate_conversion_preserves_right_handed_basis_for_winding",
            "scene_import_exposes_named_pivots_and_diagnostic_overlays",
            "prepared_render_requires_reprepare_after_transform_changes",
            "prepared_render_requires_reprepare_after_material_and_interaction_changes",
            "scene_import_reports_local_and_world_bounds_for_imported_meshes",
            "scene_import_anchor_lookups_parse_gltf_extras_and_stale",
            "scene_import_rejects_duplicate_anchor_names_on_same_host",
            "scene_import_rejects_invalid_anchor_extras_data",
            "scene_import_clip_lookups_are_import_local_and_stale",
            "gltf_required_punctual_lights_instantiate_as_scene_lights",
            "gltf_required_texture_transform_and_mesh_quantization_are_realized",
            "obj_feature_load_geometry_parses_triangle_faces",
            "scene_pick_returns_typed_hit_target_for_renderable_triangle",
            "interaction_context_and_renderer_styles_are_explicit",
            "instance_sets_have_stable_ids_mutations_and_cpu_fallback",
            "offscreen_target_readback_is_explicit_and_owned",
            "m3a_resource_lifetime_counters_return_to_baseline_for_imports_targets_and_instances",
            "labels_use_sdf_msdf_descriptors_and_billboard_render_path",
            "InstanceCullingPolicy::CpuBoundingBoxFallback",
            "LabelRasterization::Msdf",
            "mesh_material_vertex_color_scene.gltf",
            "transform_options_scene.gltf",
            "ImportOptions::gltf_default",
            "ImportDiagnosticOverlayKind::Pivot",
            "Root/A\\\\/B",
            "UnsupportedRequiredExtension",
            "import.path(\"Root/Child\")",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "tests/m3a_visual_proof.rs",
        &[
            "m3a_headless_visual_artifacts_cover_import_interaction_instances_labels_and_readback",
            "target/gate-artifacts/m3a-visual",
            "m3a-glb-model-viewer",
            "m3a-picking-selection",
            "m3a-instancing",
            "m3a-labels",
            "m3a-offscreen-readback",
            "write_ppm_artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "tests/m3a_browser_rendered_output.rs",
        &[
            "wasm_bindgen_test_configure!(run_in_browser)",
            "m3a_browser_wasm_renders_import_and_interaction_paths_to_canvas",
            "render_glb_import_frame",
            "render_interaction_frame",
            "browser_canvas_roundtrip",
            "minimal_glb_triangle_scene",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "docs/checklists/m3a-app-features.md",
        &[
            "assets_load_scene_caches_gltf_asset_and_rejects_required_extensions",
            "scene_instantiate_creates_import_hierarchy_and_name_lookups",
            "ARCH-M3A-SCENE-IMPORT",
        ],
    );
}

fn check_m3b_animation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/animation.rs",
        &[
            "pub struct AnimationMixerKey",
            "pub enum AnimationPlaybackState",
            "pub enum AnimationLoopMode",
            "pub enum AnimationTarget",
            "pub enum AnimationInterpolation",
            "pub struct AnimationClip",
            "pub struct AnimationSourceClip",
            "pub struct AnimationChannel",
            "pub struct AnimationSourceChannel",
            "pub struct AnimationMixer",
            "pub enum AnimationOutput",
            "pub fn rebind",
            "pub fn sample_vec3",
            "pub fn sample_quat",
            "pub fn sample_weights",
            "pub(crate) fn play",
            "pub(crate) fn pause",
            "pub(crate) fn stop",
            "pub(crate) fn seek",
            "pub(crate) fn advance",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/animation/sampling.rs",
        &[
            "sample_cubic_vec3",
            "sample_cubic_quat",
            "sample_cubic_weights",
            "slerp_quat",
            "cubic_scalar",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/animation.rs",
        &[
            "pub(super) fn parse_gltf_clips",
            "parse_animation_channel",
            "\"translation\"",
            "\"rotation\"",
            "\"scale\"",
            "\"weights\"",
            "read_f32_accessor",
            "read_vec3_accessor",
            "read_vec4_accessor",
            "AnimationInterpolation::CubicSpline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import.rs",
        &[
            "clip.clip().rebind",
            "resolve_import_skin_bindings",
            "SceneSkinBinding::new",
            "convert_animation_vec3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/accessors.rs",
        &["pub(crate) fn live_flag"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/options.rs",
        &[
            "convert_animation_vec3",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/accessors.rs",
        &["pub fn channels(&self)", "pub const fn duration_seconds"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/mixers.rs",
        &[
            "pub fn create_animation_mixer",
            "pub fn animation_mixer",
            "pub fn play_animation",
            "pub fn pause_animation",
            "pub fn stop_animation",
            "pub fn seek_animation",
            "pub fn set_animation_speed",
            "pub fn set_animation_loop_mode",
            "pub fn update_animation",
            "AnimationError::StaleMixer",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/skinning.rs",
        &[
            "pub struct SceneSkinBinding",
            "pub fn skin_binding",
            "pub fn skin_matrices",
            "set_initial_skin_binding",
            "world_transform",
            "SkinningMatrix::inverse_from_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/morphs.rs",
        &[
            "pub fn morph_weights",
            "pub fn set_morph_weights",
            "set_initial_morph_weights",
            "set_morph_weights_unchecked",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry.rs",
        &[
            "InvalidMorphTargetVertexCount",
            "InvalidSkinJointVertexCount",
            "InvalidSkinWeightVertexCount",
            "InvalidSkinJointIndex",
            "GeometryMorphTarget",
            "GeometrySkin",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry/morph.rs",
        &[
            "pub struct GeometryMorphTarget",
            "pub fn with_morph_targets",
            "pub fn morphed_vertices",
            "InvalidMorphTargetVertexCount",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry/skinning.rs",
        &[
            "pub struct GeometrySkin",
            "pub struct SkinningMatrix",
            "pub fn with_skin",
            "pub fn skinned_vertices",
            "from_gltf_column_major",
            "inverse_from_transform",
            "pub fn then",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/read.rs",
        &[
            "\"targets\"",
            "GeometryMorphTarget::new",
            "\"weights\"",
            "\"JOINTS_0\"",
            "\"WEIGHTS_0\"",
            "GeometrySkin::new",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/accessor/skin.rs",
        &[
            "read_mat4_accessor",
            "read_joints_accessor",
            "read_weights_accessor",
            "SkinningMatrix::from_gltf_column_major",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf.rs",
        &[
            "pub use self::skins::SceneAssetSkin",
            "parse_skins",
            "pub fn skins(&self)",
            "pub const fn skin(&self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/skins.rs",
        &[
            "pub struct SceneAssetSkin",
            "parse_skins",
            "inverseBindMatrices",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/render/prepare.rs",
        &[
            "scene.skin_matrices(node)",
            "skinned_vertices",
            "InvalidSkinGeometry",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/diagnostics.rs",
        &[
            "pub enum AnimationError",
            "StaleMixer",
            "InvalidSkinIndex",
            "InvalidSkinJointIndex",
            "InvalidSkinGeometry",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_gltf_animation.rs",
        &[
            "mixer_controls_rebind_translation_channels_to_import_local_nodes",
            "playing_paused_and_seek_animation_dirty_prepared_render_state",
            "replace_import_invalidates_animation_mixers_with_stale_error",
            "gltf_animation_supports_rotation_scale_weights_and_normalizes_quaternions",
            "morph_target_weights_channel_updates_scene_morph_weights",
            "skinning_rebinds_joints_and_deforms_vertices_from_skeleton_hierarchy",
            "combined_morph_and_skinning_deforms_morphed_vertices_through_joint_matrices",
            "interpolation_handles_step_cubic_spline_and_quaternion_slerp",
            "khronos_sample_assets_load_instantiate_and_cover_animation_contracts",
            "steady_animation_update_reprepare_keeps_resource_counts_stable",
            "AnimationLoopMode::Repeat",
            "AnimationPlaybackState::Playing",
            "AnimationError::StaleMixer",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/assets/gltf/khronos/manifest.toml",
        &[
            "https://github.com/KhronosGroup/glTF-Sample-Assets",
            "2bac6f8c57bf471df0d2a1e8a8ec023c7801dddf",
            "RiggedSimple",
            "SimpleSkin",
            "SimpleMorph",
            "MorphCube",
            "RiggedFigure",
            "BrainStem",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_visual_proof.rs",
        &[
            "m3b_headless_visual_artifacts_cover_khronos_skin_morph_and_animation",
            "m3b-khronos-simple-skin",
            "m3b-khronos-simple-morph",
            "m3b-khronos-rigged-simple",
            "write_ppm_artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_browser_rendered_output.rs",
        &[
            "m3b_browser_wasm_renders_morph_animation_to_canvas",
            "browser_canvas_roundtrip",
            "render_morph_animation_frame",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/visual/fixtures/m3b-headless-animation.toml",
        &[
            "m3b-headless-animation",
            "max_abs_diff = 0",
            "m3b-khronos-simple-skin",
            "m3b-khronos-simple-morph",
            "m3b-khronos-rigged-simple",
        ],
    );
}

fn check_material_desc_fields_private(root: &Path, findings: &mut Vec<Finding>) {
    let path = root.join("src/material.rs");
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };

    for field in public_fields_in_struct(&text, "MaterialDesc") {
        findings.push(Finding::new(
            "ARCH-ASSET-API",
            format!("src/material.rs MaterialDesc exposes public field '{field}'"),
        ));
    }
}

fn check_m4_platform_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/scene/dirty.rs",
        &[
            "pub struct SceneDirtyState",
            "transform_revision",
            "pub fn dirty_state",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum HardwareTier",
            "pub hardware_tier: HardwareTier",
            "pub gpu_frustum_culling: CapabilityStatus",
            "pub per_instance_culling: CapabilityStatus",
            "pub texture_compression_basisu: CapabilityStatus",
            "pub hardware_instancing: CapabilityStatus",
            "pub fragment_high_precision: CapabilityStatus",
            "pub uniform_buffers: CapabilityStatus",
            "pub uniform_buffer_max_bytes: u32",
            "pub compute_shaders: CapabilityStatus",
            "pub storage_buffers: CapabilityStatus",
            "uniform_buffer_max_bytes",
            "HardwareTier::Medium",
            "Backend::WebGl2 => 128",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/settings.rs",
        &[
            "pub enum Profile",
            "pub enum Quality",
            "pub enum RenderMode",
            "pub struct RendererOptions",
            "OnChange",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render.rs",
        &[
            "render_generation",
            "skipped_frames",
            "culling::cull_cpu_frustum",
            "gpu_culling_dispatches",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/build.rs",
        &[
            "headless_with_options",
            "from_surface_with_options",
            "RenderMode::OnChange",
            "resolve_quality",
            "resolve_render_mode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/surface.rs",
        &[
            "handle_surface_event",
            "recover_surface",
            "recover_context",
            "RetainPolicy::Never",
            "loss_error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/platform.rs",
        &[
            "ScaleFactorChanged",
            "Occluded",
            "Lost",
            "ContextLost",
            "ContextRestored",
            "DeviceLost",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/culling.rs",
        &["cull_cpu_frustum", "outside_camera_clip_box", "culled"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/gpu/culling.rs",
        &[
            "create_culling_pipeline",
            "encode_culling_dispatch",
            "@compute @workgroup_size(64)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/controls.rs",
        &[
            "pub struct OrbitControls",
            "pub struct PointerEvent",
            "pub enum PointerButton",
            "pub enum OrbitControlAction",
            "handle_pointer",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "Cargo.toml",
        &[
            "controls = []",
            "controls-winit = [\"controls\"]",
            "controls-web = [\"controls\"]",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/m4_performance_platform.rs",
        &[
            "capability_matrix_reports_hardware_tier_and_backend_feature_states",
            "texture_compression_basisu",
            "screen_space_ambient_occlusion",
            "BloomDisabled",
            "AmbientOcclusionDisabled",
            "hardware_instancing",
            "fragment_high_precision",
            "uniform_buffer_max_bytes",
            "transform_dirty_state_propagates_through_world_transform_queries",
            "renderer_options_apply_profile_quality_and_render_mode_precedence",
            "on_change_render_static_idle_records_skipped_frame_stats",
            "render_on_change_static_idle_skip_has_zero_allocations",
            "cpu_frustum_culling_drops_offscreen_renderables_before_draw",
            "per_instance_cpu_culling_keeps_visible_instances_and_counts_culled_ones",
            "gpu_capable_renderer_records_compute_culling_dispatch_when_available",
            "surface_loss_requires_recovery_and_prepare_before_render",
            "dpr_change_marks_surface_state_dirty_until_prepare",
            "context_recovery_rejects_assets_without_retained_cpu_data",
            "public_threading_contract_is_statically_enforced",
            "orbit_controls_are_platform_neutral_pointer_actions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m4_platform_smoke.html",
        &[
            "scena.capabilities.v1",
            "linux-webgpu-chromium",
            "linux-webgl2-chromium",
            "gpu_frustum_culling",
            "per_instance_culling",
            "texture_compression_basisu",
            "screen_space_ambient_occlusion",
            "bloom",
            "hardware_instancing",
            "fragment_high_precision",
            "uniform_buffers",
            "event_sequence",
            "recover_context",
            "webglcontextlost",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m4_platform_smoke.js",
        &[
            "m4-platform-browser-smoke",
            "webgl2",
            "webgpu",
            "capabilities",
            "loss",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "docs/checklists/m4-performance-platform.md",
        &[
            "m4_performance_platform",
            "m4-platform-browser-smoke.json",
            "m4-wasm-size.json",
            "brotli_q11_bytes",
            "ARCH-M4-PLATFORM",
        ],
    );
}

fn check_m5_release_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-M5-RELEASE", REQUIRED_EXAMPLES);
    require_files(
        root,
        findings,
        "ARCH-M5-RELEASE",
        REQUIRED_M5_GATE_ARTIFACTS,
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "Cargo.toml",
        &[
            "version = \"1.0.0-rc.0\"",
            "rust-version = ",
            "documentation = \"https://docs.rs/scena\"",
            "keywords = [",
            "categories = [",
            "include = [",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/diagnostics.rs",
        &[
            "pub enum DebugOverlay",
            "RendererChanged",
            "DebugOverlay",
            "pub struct RendererStats",
            "pub enum BuildError",
            "pub enum AssetError",
            "pub enum ImportError",
            "pub enum InstantiateError",
            "pub enum PrepareError",
            "pub enum RenderError",
            "pub enum LookupError",
            "pub enum AnimationError",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/render/settings.rs",
        &[
            "pub fn debug_overlay",
            "pub fn set_debug",
            "pub fn set_debug_overlay",
            "debug_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/render.rs",
        &[
            "debug_revision",
            "NotPreparedReason::RendererChanged",
            "ChangeKind::DebugOverlay",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/bin/scena-convert.rs",
        &[
            "scena-convert",
            "FBX to glTF",
            "FBX2glTF",
            "--dry-run",
            "planned",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/api/m5-public-api-baseline.txt",
        &[
            "Renderer::prepare",
            "Renderer::render",
            "Renderer::set_debug",
            "Renderer::set_debug_overlay",
            "Renderer::capability_report",
            "Renderer::gpu_adapter_report",
            "CapabilityReport",
            "DebugOverlay",
            "RendererStats",
            "GpuAdapterReport",
            "AdapterLimitsReport",
            "BuildError",
            "RenderError",
            "SceneImport",
            "AnchorFrame",
            "ConnectorFrame",
            "ConnectorMetadata",
            "ConnectionAlignment",
            "ConnectionRoll",
            "ConnectionLineOverlay",
            "ConnectorRollPolicy",
            "ConnectorPolarity",
            "Scene::connect_import_connectors",
            "AnchorKey",
            "ConnectorKey",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/api/m5-semver-baseline.toml",
        &[
            "version = \"1.0.0-rc.0\"",
            "api_baseline = \"cargo run -p xtask -- doctor --full\"",
            "BuildError",
            "AnimationError",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "tests/m5_release.rs",
        &[
            "m5_debug_overlay_api_is_public_and_requires_prepare_after_change",
            "m5_public_api_baseline_names_frozen_contracts",
            "m5_benchmark_report_writes_required_scene_rows",
            "scena_convert_cli_reports_fbx_to_gltf_plan",
            "m5-benchmarks",
            "m5-public-api-freeze",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/specs/release-gates.md",
        &[
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/checklists/m5-v1-release.md",
        &[
            "m5_release",
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
            "cargo publish --dry-run",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/checklists/acceptance-index.md",
        &[
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
            "cargo publish --dry-run",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "target/gate-artifacts/m5-benchmarks.json",
        &[
            "\"gate\": \"m5-benchmarks\"",
            "\"status\": \"passed\"",
            "static-viewer",
            "standard-model-viewer-gltf",
            "larger-industrial-gltf",
            "high-instance",
            "headless-4k",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "target/gate-artifacts/m5-public-api-freeze.json",
        &[
            "\"gate\":\"m5-public-api-freeze\"",
            "\"status\":\"passed\"",
            "docs/api/m5-public-api-baseline.txt",
        ],
    );
}

const REQUIRED_EXAMPLES: &[&str] = &[
    "examples/primitive_shapes.rs",
    "examples/glb_model_viewer.rs",
    "examples/picking_selection_hover.rs",
    "examples/instancing.rs",
    "examples/labels_helpers.rs",
    "examples/animation.rs",
    "examples/native_window.rs",
    "examples/browser_canvas.rs",
    "examples/headless_ci.rs",
    "examples/industrial_static_scene.rs",
    "examples/industrial_connector_assembly.rs",
    "examples/coordinate_connector_repair.rs",
];

const REQUIRED_M5_GATE_ARTIFACTS: &[&str] = &[
    "target/gate-artifacts/m5-benchmarks.json",
    "target/gate-artifacts/m5-public-api-freeze.json",
];

fn public_fields_in_struct(text: &str, struct_name: &str) -> Vec<String> {
    let Some(body) = braced_body_after(text, &format!("struct {struct_name}")) else {
        return Vec::new();
    };

    body.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub ") || line.starts_with("pub("))
        .map(|line| line.trim_end_matches(',').to_string())
        .collect()
}

fn braced_body_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let marker_start = text.find(marker)?;
    let search_start = marker_start + marker.len();
    let brace_start = text[search_start..].find('{')? + search_start;
    let mut depth = 0usize;

    for (offset, character) in text[brace_start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&text[brace_start + 1..brace_start + offset]);
                }
            }
            _ => {}
        }
    }

    None
}

fn check_solid_kiss(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SOLID-KISS-DOCS",
        "docs/specs/module-boundaries.md",
        &[
            "## SOLID/KISS Gate",
            "Every public feature must name exactly one owner module",
            "No catch-all `Manager`, `Engine`, `World`, or broad `Context` type",
        ],
    );

    for rel in source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        let significant_lines = significant_line_count(&text);
        if significant_lines > MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
            findings.push(Finding::new(
                "ARCH-KISS-SIZE",
                format!(
                    "{} has {significant_lines} significant lines; split before exceeding {MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE}",
                    rel.display()
                ),
            ));
        }

        for (line_index, type_name) in declared_type_names(&text) {
            if is_catch_all_type_name(&type_name) {
                findings.push(Finding::new(
                    "ARCH-SOLID-CATCH-ALL",
                    format!(
                        "{}:{} declares catch-all type '{}'; use an owner-specific type or add an ADR-backed doctor allowlist",
                        rel.display(),
                        line_index + 1,
                        type_name
                    ),
                ));
            }
        }
    }
}

fn significant_line_count(text: &str) -> usize {
    text.lines()
        .take_while(|line| !line.trim_start().starts_with("#[cfg(test)]"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("//"))
        .count()
}

fn declared_type_names(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| declared_type_name(line).map(|name| (index, name)))
        .collect()
}

fn declared_type_name(line: &str) -> Option<String> {
    let line = line.trim_start();
    let line = line.strip_prefix("pub ").unwrap_or(line);
    let line = line.strip_prefix("crate ").unwrap_or(line);
    let line = line
        .strip_prefix("struct ")
        .or_else(|| line.strip_prefix("enum "))
        .or_else(|| line.strip_prefix("type "))
        .or_else(|| line.strip_prefix("trait "))?;
    let name = line
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .next()
        .unwrap_or_default();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn is_catch_all_type_name(name: &str) -> bool {
    if ALLOWED_CONTEXT_TYPES.contains(&name) {
        return false;
    }
    CATCH_ALL_TYPE_NAMES.contains(&name)
        || CATCH_ALL_TYPE_SUFFIXES
            .iter()
            .any(|suffix| name.ends_with(suffix))
        || name == "Context"
        || (name.ends_with("Context") && name.len() > "Context".len())
}

fn forbid_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    forbid_contains_path(root, findings, rule, Path::new(rel), needles);
}

fn forbid_contains_path(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &Path,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };

    for needle in needles {
        if text.contains(needle) {
            findings.push(Finding::new(
                rule,
                format!(
                    "{} contains forbidden boundary text '{}'",
                    rel.display(),
                    needle
                ),
            ));
        }
    }
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_source_files(&root.join("src"), Path::new("src"), &mut files);
    files.sort();
    files
}

fn collect_source_files(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_source_files(&path, &rel, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(rel);
        }
    }
}

fn contains_scope_term(lower_text: &str, term: &str) -> bool {
    if term.contains(' ') {
        return lower_text.contains(term);
    }

    lower_text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| token == term)
}

fn check_unit_test_first_governance(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "TEST-FIRST-AGENTS",
        "AGENTS.md",
        &[
            "## Unit Test First Rule",
            "Run the focused test and confirm it fails for the expected reason",
            "Do not mark a checklist implementation item complete without naming the test-first proof",
        ],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-SKILL-QUALITY",
        ".codex/skills/scena-renderer-quality/SKILL.md",
        &[
            "## Unit Test First Workflow",
            "Run the focused test and verify the failure is the expected failure",
        ],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-SKILL-ARCH",
        ".codex/skills/scena-renderer-architecture/SKILL.md",
        &["Before production implementation"],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-DOCTOR-CONTRACT",
        "docs/specs/doctor-contract.md",
        &["unit-test-first governance"],
    );

    for rel in MILESTONE_CHECKLISTS {
        require_contains(
            root,
            findings,
            "TEST-FIRST-CHECKLIST",
            rel,
            &["Unit-test-first evidence"],
        );
    }
}

const MILESTONE_CHECKLISTS: &[&str] = &[
    "docs/checklists/m0-foundation.md",
    "docs/checklists/m1-geometry-materials.md",
    "docs/checklists/m2-lighting-depth-clipping.md",
    "docs/checklists/m3a-app-features.md",
    "docs/checklists/m3b-gltf-animation.md",
    "docs/checklists/m4-performance-platform.md",
    "docs/checklists/m5-v1-release.md",
    "docs/checklists/acceptance-index.md",
];

fn check_backend_vocabulary(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/platform.rs",
        &["browser_webgpu_canvas", "browser_webgl2_canvas"],
    );
    require_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/diagnostics/capabilities.rs",
        &["WebGpu", "WebGl2"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/diagnostics.rs",
        &["BrowserSurface"],
    );
}

fn check_agent_validation(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "AGENTS-VALIDATION",
        "AGENTS.md",
        &["cargo run -p xtask -- doctor --full", "Use `scena-doctor`"],
    );
    require_contains(
        root,
        findings,
        "SKILL-DOCTOR",
        ".codex/skills/scena-doctor/SKILL.md",
        &["cargo run -p xtask -- doctor --full"],
    );
}

fn check_default_environment_manifest(root: &Path, findings: &mut Vec<Finding>) {
    let manifest_rel = "tests/assets/environment/default-environment.toml";
    let manifest_path = root.join(manifest_rel);
    let Ok(text) = fs::read_to_string(&manifest_path) else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("missing required default environment manifest {manifest_rel}"),
        ));
        return;
    };

    require_manifest_value(findings, manifest_rel, &text, "name", "neutral-studio");
    require_manifest_value(findings, manifest_rel, &text, "license", "CC0-1.0");
    require_manifest_value(findings, manifest_rel, &text, "wasm_delivery", "bundled");
    require_manifest_value(
        findings,
        manifest_rel,
        &text,
        "status",
        "text-fixture-not-ibl-proof",
    );
    require_manifest_u32(findings, manifest_rel, &text, "cubemap_resolution", 256);
    require_manifest_u32(findings, manifest_rel, &text, "brdf_lut_size", 256);

    let Some(source_path) = quoted_manifest_assignment(&text, "source_path") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing source_path"),
        ));
        return;
    };
    let Some(source_sha256) = quoted_manifest_assignment(&text, "source_sha256") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing source_sha256"),
        ));
        return;
    };
    let Some(generator) = quoted_manifest_assignment(&text, "generator") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing generator"),
        ));
        return;
    };
    if !generator.contains(&source_path) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} generator does not reference source_path"),
        ));
    }
    if binary_render_asset_extension(Path::new(&source_path)) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} source_path {source_path} must not use a binary render asset extension unless real binary bytes are committed"),
        ));
    }
    check_manifest_file_hash(root, findings, manifest_rel, &source_path, &source_sha256);

    let derivatives = derivative_manifest_entries(&text);
    if derivatives.len() < 2 {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} must declare at least cubemap and BRDF LUT derivatives"),
        ));
    }
    for (path, sha256) in derivatives {
        if path.contains("placeholder") {
            findings.push(Finding::new(
                "VISUAL-DEFAULT-ENV",
                format!("{manifest_rel} derivative {path} still points at a placeholder file"),
            ));
        }
        if binary_render_asset_extension(Path::new(&path)) {
            findings.push(Finding::new(
                "VISUAL-DEFAULT-ENV",
                format!("{manifest_rel} derivative {path} must not use a binary render asset extension unless real binary bytes are committed"),
            ));
        }
        check_default_environment_derivative_payload(root, findings, manifest_rel, &path);
        check_manifest_file_hash(root, findings, manifest_rel, &path, &sha256);
    }
}

fn check_default_environment_derivative_payload(
    root: &Path,
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    path: &str,
) {
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        return;
    };
    if text.contains("not a renderer-consumable") {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} derivative {path} declares itself non-consumable"),
        ));
    }
    let valid_magic =
        text.starts_with("SCENA_CUBEMAP_V1\n") || text.starts_with("SCENA_BRDF_LUT_V1\n");
    if !valid_magic {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} derivative {path} is missing a scena environment magic header"),
        ));
    }
}

fn check_visual_fixture_metadata(root: &Path, findings: &mut Vec<Finding>) {
    check_ndc_smoke_fixture_classification(
        root,
        findings,
        "tests/visual/fixtures/m1-headless-core.toml",
        &[
            "primitive-fullscreen",
            "unlit-asset-mesh",
            "pbr-asset-mesh",
            "transparent-blend",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/fixtures/m1-headless-core.toml",
        &[
            "[suite]",
            "name = \"m1-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
            "artifact_dir = \"target/gate-artifacts/m1-visual\"",
            "reference = \"tests/visual/references/m1-headless-core.toml\"",
            "reference_mode = \"sampled-rgba\"",
            "max_abs_diff = 0",
            "name = \"primitive-fullscreen\"",
            "name = \"unlit-asset-mesh\"",
            "name = \"pbr-asset-mesh\"",
            "name = \"transparent-blend\"",
            "name = \"line-material\"",
            "name = \"wire-edge-materials\"",
            "name = \"default-cube\"",
            "luminance_gate = \"center-nonblack\"",
            "silhouette_gate = \"corner-black\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/references/m1-headless-core.toml",
        &[
            "[suite]",
            "status = \"reference\"",
            "max_abs_diff = 0",
            "center_rgba = [119, 177, 204, 255]",
            "nonblack_pixels = 109",
            "rgba_hash = \"fnv1a64:1b305a55001a2b13\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/m1_visual_proof.rs",
        &[
            "m1_headless_visual_artifacts_cover_core_material_paths",
            "m1_headless_reference_tolerances_match_current_fixtures",
            "write_ppm_artifact",
            "target/gate-artifacts/m1-visual",
            "include_str!(\"visual/fixtures/m1-headless-core.toml\")",
            "include_str!(\"visual/references/m1-headless-core.toml\")",
            "rgba_within_tolerance",
            "rgba_fnv1a64",
        ],
    );
}

fn check_m2_visual_fixture_metadata(root: &Path, findings: &mut Vec<Finding>) {
    check_ndc_smoke_fixture_classification(
        root,
        findings,
        "tests/visual/fixtures/m2-headless-core.toml",
        &[
            "direct-lights-pbr",
            "shadowed-directional-light",
            "ibl-environment",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/visual/fixtures/m2-headless-core.toml",
        &[
            "[suite]",
            "name = \"m2-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
            "artifact_dir = \"target/gate-artifacts/m2-visual\"",
            "reference = \"tests/visual/references/m2-headless-core.toml\"",
            "reference_mode = \"sampled-rgba\"",
            "max_abs_diff = 0",
            "name = \"direct-lights-pbr\"",
            "name = \"shadowed-directional-light\"",
            "name = \"ibl-environment\"",
            "name = \"fxaa-edge\"",
            "name = \"clipping-half-space\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/visual/references/m2-headless-core.toml",
        &[
            "[suite]",
            "status = \"reference\"",
            "max_abs_diff = 0",
            "center_rgba = [116, 0, 1, 255]",
            "center_rgba = [68, 68, 68, 255]",
            "nonblack_pixels = 148",
            "rgba_hash = \"fnv1a64:b7fab3e0ab0ca5ff\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/m2_visual_proof.rs",
        &[
            "m2_headless_visual_artifacts_cover_lighting_depth_and_clipping",
            "m2_headless_reference_tolerances_match_current_fixtures",
            "write_ppm_artifact",
            "target/gate-artifacts/m2-visual",
            "include_str!(\"visual/fixtures/m2-headless-core.toml\")",
            "include_str!(\"visual/references/m2-headless-core.toml\")",
            "validate_shadowed_directional_light",
            "validate_ibl_environment",
            "validate_clipping_half_space",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "m2_headless_visual_artifacts_cover_lighting_depth_and_clipping",
            "m2-headless-core.toml",
            "VISUAL-M2-FIXTURE-METADATA",
        ],
    );
}

fn check_ndc_smoke_fixture_classification(
    root: &Path,
    findings: &mut Vec<Finding>,
    fixture_rel: &str,
    fixture_names: &[&str],
) {
    let Ok(text) = fs::read_to_string(root.join(fixture_rel)) else {
        findings.push(Finding::new(
            "VISUAL-HARNESS-SMOKE-P0",
            format!("could not read {fixture_rel}"),
        ));
        return;
    };

    for name in fixture_names {
        let Some(block) = fixture_block(&text, name) else {
            findings.push(Finding::new(
                "VISUAL-HARNESS-SMOKE-P0",
                format!("{fixture_rel} is missing fixture '{name}'"),
            ));
            continue;
        };
        for required in [
            "proof_class = \"harness-smoke\"",
            "production_claim = false",
        ] {
            if !block.contains(required) {
                findings.push(Finding::new(
                    "VISUAL-HARNESS-SMOKE-P0",
                    format!(
                        "{fixture_rel} fixture '{name}' must contain {required} so NDC/fullscreen smoke cannot satisfy production proof"
                    ),
                ));
            }
        }
    }
}

fn fixture_block<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("name = \"{name}\"");
    let start = text.find(&needle)?;
    let rest = &text[start..];
    let next_fixture = rest.find("\n[[fixture]]").unwrap_or(rest.len());
    Some(&rest[..next_fixture])
}

fn check_m1_browser_rendered_output(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "Cargo.toml",
        &[
            "wasm-bindgen",
            "wasm-bindgen-test",
            "CanvasRenderingContext2d",
            "ImageData",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "tests/m1_browser_rendered_output.rs",
        &[
            "wasm_bindgen_test_configure!(run_in_browser)",
            "fn m1_browser_wasm_renders_color_and_alpha_to_canvas",
            "fn m1_browser_wasm_renders_technical_materials_to_canvas",
            "Renderer::headless(4, 4)",
            "MaterialDesc::line",
            "MaterialDesc::wireframe",
            "MaterialDesc::edge",
            "put_image_data",
            "get_image_data",
            "[158, 0, 159, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "docs/checklists/m1-geometry-materials.md",
        &[
            "m1_browser_rendered_output",
            "Rust/WASM browser rendered-output proof",
        ],
    );
}

fn check_m2_browser_rendered_output(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "tests/browser/m2_browser_lighting_clipping_smoke.js",
        &[
            "m2_browser_lighting_clipping_smoke.html",
            "scenaM2BrowserLightingClippingSmoke",
            "webgl2",
            "webgpu",
            "m2-browser-lighting-clipping-smoke.json",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "tests/browser/m2_browser_lighting_clipping_smoke.html",
        &[
            "runWebGpuScene",
            "runWebGl2Scene",
            "directLightPassed",
            "clippingPassed",
            "vec4<f32>(1.0, 0.0, 0.0, 1.0)",
            "vec4(1.0, 0.0, 0.0, 1.0)",
            "directCenter",
            "clippingLeft",
            "clippingRight",
            "clippingNonBlackPixels",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "node tests/browser/m2_browser_lighting_clipping_smoke.js",
            "m2-browser-lighting-clipping-smoke.json",
            "VISUAL-BROWSER-M2",
        ],
    );
}

fn check_m6_browser_renderer_probe(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "Cargo.toml",
        &[
            "browser-probe",
            "WebGl2RenderingContext",
            "WebGlProgram",
            "WebGlShader",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe.rs",
        &[
            "m6RenderWebgl2Probe",
            "m6RenderWebgpuProbe",
            "m6RenderWorkflowProbe",
            "m6RenderSurfaceLifecycleProbe",
            "m6RenderBenchmarkProbe",
            "m6RenderStateLifecycleProbe",
            "Renderer::from_surface_async",
            "prepare_with_assets",
            "Renderer::render",
            "scena.m6.browser_renderer_probe.v1",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/probes.rs",
        &[
            "scena.m6.browser_surface_lifecycle_probe.v1",
            "build_workflow_scene(\"material-textures\")",
            "RetainPolicy::OnContextLossOnly",
            "material_texture_bindings",
            "scena.m6.browser_benchmark_probe.v1",
            "surface-context-lifecycle",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/probes/state_lifecycle.rs",
        &[
            "scena.m6.browser_state_lifecycle_probe.v1",
            "dirty-transform",
            "dirty-material",
            "dirty-instance",
            "dirty-camera",
            "dirty-resize-dpr",
            "dirty-hover-selection",
            "dirty-animation-mixer",
            "context-recovery",
            "resource-lifetime",
            "idle-render-skipped",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/workflows.rs",
        &[
            "model-viewer",
            "non_ndc_camera_scene.gltf",
            "camera-framed-non-ndc",
            "depth-overlap",
            "depth-overlap-near-wins",
            "pbr-point-light",
            "pbr-spot-light",
            "pbr-normal-map",
            "pbr-environment",
            "instancing",
            "picking-selection",
            "animation",
            "labels-helpers",
            "industrial-static-scene",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/workflows/pbr.rs",
        &[
            "browser-pbr-punctual-light",
            "browser-pbr-normal-map",
            "browser-pbr-environment-light",
            "inline-radiance-hdr",
            "radiance_hdr_data_uri",
            "pbr-metallic-roughness",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/workflows/ergonomics/viewer.rs",
        &[
            "textured-connector-viewer",
            "pick_and_select_with_assets",
            "frame_all_with_assets",
            "with_base_color_texture",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/render/gpu/build.rs",
        &[
            "create_browser_canvas_surface",
            "WebCanvasWindowHandle",
            "WebDisplayHandle",
            "raw_display_handle: Some(raw_display_handle)",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/report.rs",
        &[
            "material_bindings",
            "material_texture_bindings",
            "material_sampler_bindings",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "m6-rust-wasm-renderer-probe",
            "scenaM6RustWasmRendererProbe",
            "scenaM6RustWasmWorkflowProbe",
            "scenaM6RustWasmLifecycleProbe",
            "scenaM6RustWasmBenchmarkProbe",
            "scenaM6RustWasmStateLifecycleProbe",
            "/fixtures/",
            "webgl2",
            "webgpu",
            "SCENA_BROWSER_BACKENDS",
            "SCENA_BROWSER_ALLOW_UNAVAILABLE",
            "NoAdapter",
            "m6-rust-wasm-renderer-probe.json",
            "assertModelViewerProof",
            "crypto.createHash",
            "fixture_sha256",
            "camera-framed-non-ndc",
            "non_ndc_camera_scene.gltf",
            "canvas_data_url",
            "screenshot_metadata",
            "assertDepthOverlapProof",
            "assertPunctualLightProof",
            "assertNormalMapProof",
            "assertEnvironmentLightProof",
            "assertMaterialTextureProof",
            "assertTexturedConnectorViewerProof",
            "assertSurfaceLifecycleProbe",
            "assertNoScenaGpuValidationErrors",
            "scena wgpu uncaptured error",
            "material_texture_bindings < 5",
            "decoded_base_color_texture",
            "decoded_normal_texture",
            "decoded_emissive_texture",
            "depth-overlap",
            "depth-overlap-near-wins",
            "pbr-point-light",
            "pbr-spot-light",
            "pbr-normal-map",
            "pbr-environment",
            "browser-pbr-punctual-light",
            "browser-pbr-normal-map",
            "browser-pbr-environment-light",
            "material-textures",
            "textured-connector-viewer",
            "model-viewer",
            "instancing",
            "picking-selection",
            "animation",
            "labels-helpers",
            "industrial-static-scene",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &[
            "m6RenderWebgl2Probe",
            "m6RenderWebgpuProbe",
            "m6RenderWorkflowProbe",
            "m6RenderSurfaceLifecycleProbe",
            "m6RenderBenchmarkProbe",
            "m6RenderStateLifecycleProbe",
            "readWebGl2Pixels",
            "readCanvasPixels",
            "readRenderedPixels",
            "getImageData",
            "scenaM6RustWasmWorkflowProbe",
            "scenaM6RustWasmLifecycleProbe",
            "scenaM6RustWasmBenchmarkProbe",
            "scenaM6RustWasmStateLifecycleProbe",
            "screenshot_metadata",
            "device_pixel_ratio",
            "pixel_statistics",
            "nonblack",
        ],
    );
    if let Ok(page_source) =
        fs::read_to_string(root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"))
        && page_source.contains("backend === \"webgpu\" ||")
    {
        findings.push(Finding::new(
            "VISUAL-BROWSER-M6",
            "tests/browser/m6_rust_wasm_renderer_probe_page.js must not auto-pass WebGPU \
             workflows without nonblack pixel evidence",
        ));
    }
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/render/gpu/webgl2.rs",
        &[
            "WEBGL2_RENDER_CACHE",
            "struct WebGl2RenderCache",
            "last_vertex_hash",
            "buffer_sub_data_with_i32_and_array_buffer_view",
            "DEPTH_TEST",
            "DEPTH_BUFFER_BIT",
            "depth_func",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/render/gpu/webgl2_program.rs",
        &["aces_tonemap", "rrt_and_odt_fit", "fn vertex_stream_hash"],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "docs/checklists/m6-browser-renderer-parity.md",
        &[
            "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe",
            "node tests/browser/m6_rust_wasm_renderer_probe.js",
            "VISUAL-BROWSER-M6",
            "dirty-transform",
            "resource-lifetime",
            "idle-render-skipped",
        ],
    );
}

fn check_m9_ci_release_lanes(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/ci.yml",
        &[
            "linux-native-vulkan",
            "linux-browser-webgl2",
            "linux-browser-webgpu",
            "wasm32",
            "macos-metal",
            "windows-dx12",
            "headless-4k-performance",
            "dtolnay/rust-toolchain@1.93.1",
            "components: rustfmt, clippy",
            "node-version: \"20.20.0\"",
            "PLAYWRIGHT_VERSION: \"1.59.1\"",
            "BINARYEN_VERSION: \"129.0.0\"",
            "BROTLI_CLI_VERSION: \"2.1.1\"",
            "npm ci",
            "npx playwright install chromium --with-deps",
            "cargo install wasm-pack --version 0.14.0",
            "wasm-pack build --release --target web --out-dir target/m9-browser-pkg . --features browser-probe",
            "npm run wasm:size",
            "Platform parity gates",
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test --test m9_platform_release",
            "m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact",
            "SCENA_BROWSER_BACKENDS: webgl2",
            "SCENA_BROWSER_BACKENDS: webgpu",
            "SCENA_BROWSER_ALLOW_UNAVAILABLE: \"1\"",
            "cargo run -p xtask -- doctor --full",
            "premerge-release-readiness",
            "Download release lane artifacts",
            "Release readiness drift check",
            "actions/download-artifact@v4",
            "SCENA_RELEASE_ARTIFACT_ROOT",
            "target/release-artifacts",
            "cargo run -p xtask -- release-readiness",
            "RUSTDOCFLAGS: \"-D warnings\"",
            "cargo doc --no-deps --all-features",
            "release-lane-artifact",
            "target/gate-artifacts/**",
            "if-no-files-found: error",
        ],
    );
    forbid_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/ci.yml",
        &["if-no-files-found: ignore"],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/release.yml",
        &[
            "linux-native-vulkan",
            "linux-browser-webgl2",
            "linux-browser-webgpu",
            "wasm32-unknown-unknown",
            "macos-metal",
            "windows-dx12",
            "headless-4k-performance",
            "cargo publish --dry-run",
            "cargo publish",
            "gh release create",
            "cargo run -p xtask -- release-readiness",
            "actions/download-artifact@v4",
            "SCENA_RELEASE_ARTIFACT_ROOT",
            "SCENA_BROWSER_BACKENDS: webgl2",
            "SCENA_BROWSER_BACKENDS: webgpu",
            "components: rustfmt, clippy",
            "needs:",
            "release-lane-artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "package.json",
        &[
            "\"node\": \"20.20.0\"",
            "\"npm\": \"10.8.2\"",
            "\"playwright\": \"1.59.1\"",
            "\"binaryen\": \"129.0.0\"",
            "\"brotli-cli\": \"2.1.1\"",
            "\"browser:m6\"",
            "\"wasm:size\"",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "tests/release/m9_wasm_size_gate.js",
        &[
            "required_features: [\"browser-probe\"]",
            "m6RenderWebgl2Probe",
            "m6RenderWebgpuProbe",
            "feature_enabled_probe_exports_present",
            "WASM size gate must measure the browser-probe renderer bundle",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "package-lock.json",
        &["\"playwright\": \"1.59.1\"", "\"version\": \"1.59.1\""],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "Cargo.toml",
        &[
            "repository = \"https://github.com/johannesPettersson80/scena\"",
            "authors = [\"Johannes Pettersson <johannes_salomon@hotmail.com>\"]",
            "documentation = \"https://docs.rs/scena\"",
            "license = \"MIT OR Apache-2.0\"",
            "readme = \"README.md\"",
            "keywords = [\"renderer\", \"scene-graph\", \"gltf\", \"webgpu\", \"wasm\"]",
            "categories = [\"graphics\", \"rendering\", \"wasm\"]",
            "CHANGELOG.md",
            "docs/api/m5-semver-baseline.toml",
            "[package.metadata.docs.rs]",
            "wasm32-unknown-unknown",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "crates/xtask/src/main.rs",
        &[
            "release-lane-artifact",
            "scena.release_lane.v1",
            "content_ok",
            "command_records",
            "release_lane_command_records",
            "release_lane_measured_command_records",
            "require_release_lane_artifact_evidence",
            "duration_ms",
            "measurement_source",
            "failure_log_path",
            "failure_log_sha256",
            "release_lane_content_ok",
            "REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES",
            "require_native_gpu_render_proof",
            "native_gpu_render_proof_passes",
            "pbr_light_render_proof_passes",
            "native-pbr-punctual-light",
            "pbr-directional-red.ppm",
            "pbr-point-green.ppm",
            "pbr-spot-blue.ppm",
            "CPU fallback artifacts cannot satisfy GPU release claims",
            "release_readiness_rejects_cpu_fallback_native_render_artifact",
            "release_readiness_rejects_native_render_artifact_without_pbr_light_proof",
            "release_readiness_accepts_native_render_artifact_with_pbr_light_proof",
            "release_readiness_rejects_stale_timestamped_artifact",
            "release_readiness_rejects_constant_ppm_visual_artifact",
            "release_readiness_rejects_factory_contract_capability_rows",
            "release_readiness_rejects_benchmark_artifact_without_stored_baseline_comparison",
            "release_readiness_rejects_benchmark_regression_against_stored_baseline",
            "release_readiness_accepts_benchmark_artifact_with_passed_baseline_comparison",
            "reject_stale_json_timestamp",
            "reject_constant_ppm_artifact",
            "reject_unmeasured_capability_matrix_rows",
            "REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES",
            "m9-benchmarks-4k.json",
            "require_benchmark_baseline_comparison",
            "REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES",
            "require_rendered_output_screenshot_metadata",
            "require_screenshot_metadata_entry",
            "release_readiness_rejects_rendered_output_without_screenshot_metadata",
            "release_readiness_accepts_rendered_output_with_screenshot_metadata",
            "release_readiness_rejects_release_lane_artifact_without_measured_command_duration",
            "release_readiness_rejects_release_lane_artifact_with_failed_command_record",
            "MIN_BENCHMARK_SAMPLE_COUNT",
            "release_lane_artifact_status_requires_native_gpu_content_proof",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "scripts/release_lane_command.sh",
        &[
            "target/gate-artifacts/release-lanes",
            "duration_ms",
            "failure_log_sha256",
            "ci-wrapper",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/ci.yml",
        &["scripts/release_lane_command.sh", "release-lane-artifact"],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/release.yml",
        &["scripts/release_lane_command.sh", "release-lane-artifact"],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "tests/m9_platform_release.rs",
        &[
            "m9_platform_rendered_output_suite_writes_release_artifacts",
            "m9_capability_matrix_artifact_covers_required_lanes",
            "m9_surface_context_loss_artifact_records_required_sequence",
            "m9-benchmarks.json",
            "m9-benchmarks-4k.json",
            "m9-capability-matrix.json",
            "scena.m9.platform_render.v1",
            "scena.capabilities.v1",
            "linux-native-vulkan",
            "macos-metal",
            "windows-dx12",
            "default-scene.ppm",
            "static-gltf.ppm",
            "pbr-directional-red.ppm",
            "pbr-point-green.ppm",
            "pbr-spot-blue.ppm",
            "render_pbr_light_suite_platform",
            "native-pbr-punctual-light",
            "PbrLightKind::DirectionalRed",
            "PbrLightKind::PointGreen",
            "PbrLightKind::SpotBlue",
            "m9_asset_provenance_records_source_path_and_hash",
            "m9_static_gltf_proof_uses_non_ndc_camera_framed_asset",
            "STATIC_GLTF_PROOF_FIXTURE",
            "non_ndc_camera_scene.gltf",
            "\"proof_class\": \"harness-smoke\"",
            "\"production_claim\": false",
            "production_claim_for_gpu",
            "static_gltf_proof_class",
            "cpu-fallback-camera-framed-non-ndc",
            "cpu fallback is diagnostic only and never satisfies GPU rendered-output claims",
            "\"gpu_proof\"",
            "\"asset_provenance\"",
            "\"source_hash\"",
            "m9_cpu_fallback_artifacts_do_not_claim_gpu_rendered_output",
            "m9_screenshot_metadata_records_renderer_color_and_tolerance_contract",
            "screenshot_renderer_settings",
            "screenshot_color_management",
            "screenshot_tolerance_metadata",
            "\"renderer_settings\"",
            "\"color_management\"",
            "\"tolerance\"",
            "GpuAdapterReport",
            "AdapterLimitsReport",
            "m9_adapter_metadata_records_actual_gpu_identity_when_available",
            "gpu_adapter_report",
            "adapter_metadata",
            "\"adapter\"",
            "\"features\"",
            "\"limits\"",
            "\"driver\"",
            "\"commit\"",
            "\"timestamp_unix_seconds\"",
            "\"test_names\"",
            "\"artifact_paths\"",
            "native-rendered-output-smoke",
            "srgb8-after-aces",
            "BENCHMARK_BASELINE_PATH",
            "BENCHMARK_SAMPLE_COUNT: usize = 100",
            "m9_benchmark_rows_use_distribution_not_single_sample",
            "m9_benchmark_rows_record_stored_baseline_comparison",
            "m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact",
            "m9_benchmark_baseline_comparison_fails_significant_regressions",
            "apply_benchmark_baselines",
            "benchmark_baseline_for_row",
            "docs/benchmarks/m9-baselines.json",
            "\"baseline_comparison\"",
            "\"baseline_sha256\"",
            "deferred-to-dedicated-performance-lane",
            "\"sample_count\"",
            "\"p50_frame_ms\"",
            "\"p95_frame_ms\"",
            "\"min_frame_ms\"",
            "\"max_frame_ms\"",
            "\"stddev_frame_ms\"",
            "\"fixture\"",
            "\"sample_count_policy\"",
            "\"status\": \"incomplete\"",
            "measurement_source",
            "capability_matrix_row",
            "lane_capability_from_artifact",
            "lane-renderer-runtime",
            "missing_lane_capability",
            "missing-lane-artifact",
            "no factory capability constants are accepted as platform proof",
        ],
    );
    forbid_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "tests/m9_platform_release.rs",
        &["factory-contract"],
    );
    forbid_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "tests/m9_platform_release.rs",
        &["\"status\": \"passed\",\n        \"lanes\": [\n            lane_capability"],
    );
    forbid_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "tests/m9_platform_release.rs",
        &["\"production_claim\": true"],
    );
}

fn check_m10_claim_audit_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "crates/xtask/src/main.rs",
        &[
            "claim-audit",
            "m10-claim-audit.json",
            "scena.m10.claim_audit.v1",
            "required_final_gates",
            "release-readiness",
            "REQUIRED_RELEASE_ARTIFACT_SUFFIXES",
        ],
    );
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "docs/checklists/m10-threejs-replacement-acceptance.md",
        &["m10-claim-audit.json", "claim audit"],
    );
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "docs/api/m10-public-api-diff.md",
        &[
            "M10 Public API Diff From M5 Baseline",
            "Renderer::diagnose_scene",
            "AssetLoadControl",
            "AssetError::UnsupportedTextureFormat",
            "Semver Decision",
        ],
    );
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "docs/release-notes/v1.0.0-rc.md",
        &[
            "Release Candidate Notes",
            "Remaining Release Blockers",
            "does not claim to replace game engines",
        ],
    );
}

fn check_state_of_art_checklist_links(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CHECKLIST-STATE-OF-ART",
        "docs/checklists/acceptance-index.md",
        &[
            "state-of-art-threejs-replacement-plan.md",
            "State Of The Art Three.js Replacement Plan",
        ],
    );
    require_contains(
        root,
        findings,
        "CHECKLIST-STATE-OF-ART",
        "docs/checklists/m10-threejs-replacement-acceptance.md",
        &[
            "state-of-art-threejs-replacement-plan.md",
            "State Of The Art Three.js Replacement Plan",
        ],
    );
}

fn check_m7_ergonomics_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_files(
        root,
        findings,
        "ERGONOMICS-M7",
        &[
            "docs/guides/place-and-connect-objects.md",
            "docs/guides/units-axes-handedness.md",
            "docs/guides/authoring-gltf-anchors-connectors.md",
            "docs/guides/migrating-from-threejs.md",
            "docs/guides/troubleshooting-misplaced-assets.md",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/controls.rs",
        &[
            "with_damping",
            "focus",
            "apply_to_scene",
            "damping_factor",
            "TouchEvent",
            "pub const fn wheel",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/platform.rs",
        &["SurfaceViewport", "ViewportChanged", "device_pixel_ratio"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/picking.rs",
        &["pick_pointer", "pick_and_select", "InvalidViewport"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/visibility.rs",
        &[
            "set_camera_layer_mask",
            "camera_layer_mask",
            "visible_for_active_camera",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/import.rs",
        &[
            "ImportAnchorDebugMetadata",
            "YUpLeftHanded",
            "ZUpLeftHanded",
            "source_units",
            "source_coordinate_system",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/import/options.rs",
        &[
            "meters_per_unit",
            "convert_position",
            "convert_connector_transform",
            "has_negative_determinant",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/connectors.rs",
        &["source_coordinate_system", "connection_transform"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/connectors/error.rs",
        &["HandednessMismatch", "ConnectorHostNotPrepared"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/connectors/validation.rs",
        &[
            "validate_connector_handedness",
            "validate_connector_host_prepared",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/inspection.rs",
        &[
            "pub struct SceneInspectionReport",
            "pub struct SceneNodeInspection",
            "pub struct SceneDrawInspection",
            "pub fn inspect(&self)",
            "pub fn draw_list",
            "visible_drawable_count",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/assets.rs",
        &[
            "create_static_batch",
            "create_static_batch_with_report",
            "assets.material(texture)",
            "assets.geometry(material)",
            "assets.texture(material)",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &["scene.mesh(geometry, texture)"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/static_batch.rs",
        &["pub fn static_batch", "transform_point"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/static_batch.rs",
        &[
            "pub struct StaticBatchReport",
            "pub fn static_batch_report",
            "requires_prepare_after_rebuild",
            "picking_debug_instances",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/helpers.rs",
        &[
            "pub fn bounding_box",
            "pub fn camera_frustum",
            "pub fn light_helper",
            "pub fn anchor_marker",
            "pub fn normal_lines",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/help.rs",
        &["add_default_camera", "anchors_named", "recover_context"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/render.rs",
        &[
            "pub fn diagnose_scene",
            "pub fn diagnose_scene_with_assets",
            "pub fn capability_report",
            "MissingActiveCamera",
            "InvisibleScene",
            "MissingLightingOrEnvironment",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/capabilities.rs",
        &[
            "pub struct CapabilityReport",
            "pub fn new(capabilities: Capabilities, adapter: Option<GpuAdapterReport>)",
            "pub const fn backend",
            "pub fn adapter",
            "pub fn diagnostics",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "README.md",
        &[
            "## Happy Path",
            "examples/camera_framing.rs",
            "examples/anchor_alignment.rs",
            "examples/connect_objects.rs",
            "examples/imported_anchor_connection.rs",
            "examples/industrial_connector_assembly.rs",
            "examples/coordinate_connector_repair.rs",
            "examples/coordinate_units.rs",
            "examples/static_batching.rs",
            "examples/layers_visibility.rs",
            "examples/beginner_diagnostics.rs",
        ],
    );
    if let Ok(entries) = fs::read_dir(root.join("examples")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                let rel = Path::new("examples").join(entry.file_name());
                forbid_contains_path(
                    root,
                    findings,
                    "ERGONOMICS-M7",
                    &rel,
                    &["Primitive::unlit_triangle()", "add_renderable("],
                );
            }
        }
    }
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/first_visible_render.rs",
        &["add_default_camera", "render_active"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/industrial_connector_assembly.rs",
        &[
            "ConnectorFrame::from_import_connector",
            "ConnectionAlignment::ForwardToBack",
            "lock_node_for_connections",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/coordinate_connector_repair.rs",
        &[
            "ConnectionError::HandednessMismatch",
            "SourceCoordinateSystem::YUpLeftHanded",
            "SourceCoordinateSystem::ZUpRightHanded",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/controls.rs",
        &["apply_to_scene", "ensure_camera_depth_reaches"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls.rs",
        &[
            "OrbitControls",
            "with_damping",
            "apply_to_scene",
            "TouchEvent",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls_native_adapter.rs",
        &[
            "NativeMouseButton",
            "native_press",
            "PointerEvent::released",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls_browser_adapter.rs",
        &["browser_pointer_drag", "browser_wheel", "browser_pinch"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/camera_framing.rs",
        &["scene.frame(", "scene.look_at(", "render_active"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/anchor_alignment.rs",
        &[
            "snap_anchor",
            "anchor(\"inspection\")",
            "anchor_debug_metadata",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/connect_objects.rs",
        &["add_connector", "connect_by_key", "ConnectorFrame::new"],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/connect_objects.rs",
        &["Mat4", "from_matrix"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/imported_anchor_connection.rs",
        &[
            "ConnectorFrame::from_import_anchor",
            "connect_by_key",
            "anchor(\"inspection\")",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/imported_anchor_connection.rs",
        &["Mat4", "from_matrix"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/coordinate_units.rs",
        &[
            "SourceUnits::Millimeters",
            "SourceCoordinateSystem::ZUpRightHanded",
            "meters_per_unit",
            "convert_position",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/static_batching.rs",
        &[
            "create_static_batch_with_report",
            "requires_prepare_after_rebuild",
            "picking_debug_instances",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/layers_visibility.rs",
        &[
            "set_layer_mask",
            "set_camera_layer_mask",
            "set_render_group",
            "set_helper_on_top",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/beginner_diagnostics.rs",
        &[
            "diagnose_scene",
            "DiagnosticSeverity::Error",
            "add_default_camera",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/diagnostic.rs",
        &[
            "pub fn code(&self) -> DiagnosticCode",
            "pub fn severity(&self) -> DiagnosticSeverity",
            "pub fn message(&self) -> &str",
            "pub fn help(&self) -> Option<&str>",
            "pub fn suggested_fix(&self) -> Option<&str>",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/glb_model_viewer.rs",
        &["first_render_gltf_headless", "first.import.roots()"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/viewer.rs",
        &[
            "pub struct FirstRender",
            "pub struct HeadlessGltfViewer",
            "pub struct HeadlessGltfViewerBuilder",
            "pub fn headless_gltf_viewer",
            "pub const fn size",
            "pub const fn with_default_light",
            "pub const fn with_default_environment",
            "pub const fn with_render_mode",
            "pub const fn on_change",
            "pub async fn build",
            "pub async fn render",
            "pub fn render_next_frame",
            "pub fn prepare",
            "pub async fn first_render_gltf_headless",
            "assets.load_scene",
            "assets.default_environment",
            "scene.instantiate",
            "scene.frame_import",
            "renderer.set_environment",
            "renderer.prepare_with_assets",
            "renderer.render_active",
            "renderer.diagnostics().to_vec()",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/render/offscreen.rs",
        &["pub fn screenshot_rgba8", "self.read_pixels()"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/inspection.rs",
        &[
            "pub struct SceneInspectionReport",
            "pub struct SceneNodeInspection",
            "pub struct SceneDrawInspection",
            "pub struct SceneCameraFrustumInspection",
            "pub struct SceneMaterialInspection",
            "pub struct SceneTextureInspection",
            "pub struct SceneNormalInspection",
            "pub fn inspect_with_assets",
            "pub fn draw_list",
            "pub fn camera_frustums",
            "pub fn normal_overlays",
            "pub const fn camera_count",
            "pub const fn light_count",
            "pub const fn anchor_count",
            "pub const fn connector_count",
            "pub const fn mesh_geometry",
            "pub const fn mesh_material",
            "pub const fn material_preview",
            "world_transform: self.world_transform(node_key).unwrap_or(node.transform)",
            "pub const fn base_color_texture",
            "pub const fn source_format",
            "pub const fn decoded_dimensions",
            "pub const fn primitive_count",
            "pub const fn world_transform",
            "pub const fn corners",
            "pub fn segments",
            "pub const fn has_base_color_texture",
            "pub const fn camera",
            "pub const fn light",
            "pub const fn bounds",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/picking.rs",
        &[
            "pub fn pick(",
            "pub fn pick_with_assets",
            "pub fn pick_pointer",
            "pub fn pick_and_select",
            "pub fn pick_and_select_with_assets",
            "pub fn pick_and_hover",
            "pub fn pick_and_hover_with_assets",
            "pub fn set_hover_target",
            "pub fn set_primary_selection_target",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/first_render_api.rs",
        &[
            "first_render_gltf_headless_loads_frames_prepares_and_renders",
            "headless_gltf_viewer_builder_loads_frames_lights_and_renders",
            "headless_gltf_viewer_builder_can_attach_environment_and_report_diagnostics",
            "headless_gltf_viewer_builder_can_build_on_change_render_loop",
            "with_default_light",
            "with_default_environment",
            "on_change",
            "render_next_frame",
            "first.outcome.draw_calls > 0",
            "first.import.roots()",
            "screenshot_rgba8",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/view.rs",
        &[
            "pub fn frame_all_with_assets",
            "pub fn frame_node_with_assets",
            "asset_backed_scene_bounds_world",
            "asset_backed_node_subtree_bounds_world",
            "world_transform(target)",
            "world_transform(camera_node)",
            "inverse_unit_quat",
            "local_transform_for_world",
            "local_transform_from_world",
            "NonInvertibleParentTransform",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "m7_frame_all_with_assets_frames_direct_mesh_bounds_without_manual_bounds_math",
            "m7_frame_all_with_assets_frames_instance_bounds_without_manual_bounds_math",
            "m7_frame_node_with_assets_frames_direct_mesh_without_manual_bounds_math",
            "m7_pick_with_assets_hits_direct_mesh_without_legacy_renderable",
            "m7_pick_with_assets_hits_instance_set_without_manual_triangles",
            "m7_pick_and_select_with_assets_updates_interaction_for_direct_mesh",
            "m7_pick_with_assets_hits_imported_gltf_mesh_without_manual_triangles",
            "m7_orbit_controls_keep_framed_asset_visible_after_camera_distance_change",
            "m7_scene_inspection_with_assets_reports_material_preview_metadata",
            "m7_scene_inspection_with_assets_reports_draw_list_entries",
            "m7_scene_inspection_reports_camera_frustum_debug_geometry",
            "m7_scene_inspection_reports_normal_debug_segments",
            "m7_scene_inspection_reports_local_and_world_transforms_for_nested_nodes",
            "m7_frame_nested_camera_preserves_requested_world_camera_pose",
            "texture_preview.decoded_dimensions()",
            "frame_all_with_assets",
            "frame_node_with_assets",
            "pick_with_assets",
            "pick_and_select_with_assets",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/scene_inspection.rs",
        &[
            "scene.inspect",
            "scene.inspect_with_assets",
            "visible_drawable_count",
            "camera_count",
            "camera_frustums",
            "normal_overlays",
            "connector_count",
            "draw_list",
            "mesh_geometry",
            "mesh_material",
            "material_preview",
            "primitive_count",
            "node.world_transform()",
            "world_transform",
            "node.kind()",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "create_static_batch",
            "pick_and_select",
            "pick_and_hover",
            "set_hover_target",
            "set_primary_selection_target",
            "set_camera_layer_mask",
            "SurfaceViewport",
            "ImportAnchorDebugMetadata",
            "with_damping",
            "m7_beginner_scene_diagnostics_explain_invisible_setups",
            "GeometryDesc::normal_lines",
            "m7_error_display_snapshots_cover_beginner_recovery_paths",
            "m7_diagnostics_expose_typed_actionable_suggested_fixes",
            "m7_renderer_capability_report_exports_backend_adapter_and_diagnostics",
            "suggested_fix",
            "m7_viewer_operations_dirty_prepare_without_persistent_resource_growth",
            "m7_benchmark_artifact_writes_required_viewer_workflow_rows",
            "m7-workflow-benchmarks.json",
            "scena.m7.workflow_benchmarks.v1",
            "create_static_batch_with_report",
            "picking_debug_instances",
            "m7_scene_inspection_feature_reports_reproducible_metadata",
            "m7_scene_inspection_reports_local_and_world_transforms_for_nested_nodes",
            "camera_count",
            "mesh_geometry",
            "mesh_material",
            "node.camera()",
            "m7_import_exposes_source_units_and_coordinate_system_for_placement_diagnostics",
            "m7_import_diagnostic_overlays_expose_source_units_and_coordinate_system",
            "m7_replacement_import_rebinds_stable_anchor_and_connector_names",
            "m7_z_up_import_connector_basis_converts_before_connection_solving",
            "m7_left_handed_import_connector_rejects_instead_of_mirroring_silently",
            "m7_left_handed_mesh_import_fails_closed_until_winding_policy_exists",
            "m7_three_imported_objects_connect_into_assembly_without_raw_matrix_math",
            "m7_renderable_parent_world_transform_drives_rendered_placement",
            "m7_mesh_parent_world_transform_drives_rendered_placement",
            "m7_camera_look_at_nested_target_uses_world_transform",
            "m7_camera_look_at_point_nested_camera_uses_world_position",
            "m7_bounds_helpers_on_nested_nodes_preserve_requested_world_placement",
            "renderable_scene_with_parent_transform",
            "mesh_scene_with_parent_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/render_nodes.rs",
        &[
            "self.world_transform(node_key)",
            "map(|transform| (renderable, transform))",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/render_nodes.rs",
        &["Some((renderable, node.transform))"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &[
            ".world_transform(key)",
            "map(|transform| (key, mesh, transform))",
            "self.world_transform(node_key)",
            "map(|transform| (node_key, instance_set, transform))",
            "map(|transform| (node_key, label, label_desc, transform))",
            "map(|transform| (node_key, light_key, light, transform))",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &[
            "Some((key, mesh, node.transform))",
            "map(|instance_set| (node_key, instance_set, node.transform))",
            "map(|label_desc| (node_key, label, label_desc, node.transform))",
            "map(|light| (node_key, light_key, light, node.transform))",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_visual_proof.rs",
        &["Primitive::unlit_triangle()", "add_renderable("],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/assets/gltf/connector_zup_scene.gltf",
        &["z-up-mount", "0.70710677"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_visual_proof.rs",
        &[
            "m7_headless_visual_artifacts_cover_ergonomics_workflows",
            "target/gate-artifacts/m7-visual",
            "m7-first-render",
            "m7-first-glb",
            "m7-camera-frame",
            "m7-picking-selection",
            "m7-helpers",
            "m7-labels",
            "m7-controls",
            "m7-layers-helper-on-top",
            "m7-static-batching",
            "m7-anchor-alignment",
            "m7-connector-before",
            "m7-connector-after",
            "connector before/after proof",
            "m7-coordinate-units",
            "m7-industrial-static-scene",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/browser_probe/workflows/ergonomics.rs",
        &[
            "camera-framing",
            "anchor-alignment",
            "connector-before",
            "connector-after",
            "ConnectorFrame::new",
            "ConnectOptions::default",
            "coordinate-units",
            "static-batching",
            "layers-helper-on-top",
            "beginner-diagnostics",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "camera-framing",
            "anchor-alignment",
            "connector-before",
            "connector-after",
            "connector before/after workflow",
            "coordinate-units",
            "static-batching",
            "layers-helper-on-top",
            "beginner-diagnostics",
        ],
    );
}

fn check_m8_assets_materials_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/read.rs",
        &[
            "normalTexture",
            "metallicRoughnessTexture",
            "occlusionTexture",
            "emissiveTexture",
            "with_normal_texture_transform",
            "with_metallic_roughness_texture_transform",
            "with_occlusion_texture_transform",
            "with_emissive_texture_transform",
            "TextureColorSpace::Linear",
            "TextureColorSpace::Srgb",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/read/textures.rs",
        &[
            "AssetError::MissingTexture",
            "validate_texture_source_format",
            "basisu_texture_source_format",
            "KHR_texture_basisu",
            "TextureSourceFormat::Ktx2Basisu",
            "decode_missing_pixels_from_bytes",
            "parse_sampler",
            "TextureWrap::MirroredRepeat",
            "TextureFilter::LinearMipmapLinear",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets.rs",
        &[
            "TextureSourceFormat",
            "source_format",
            "load_scene_with_progress",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/texture.rs",
        &[
            "validate_texture_source_format",
            "UnsupportedTextureFormat",
            "TextureSourceFormat",
            "source_format",
            "TextureSourceFormat::Jpeg",
            "decode_jpeg_rgba8",
            "jpeg_decoder::Decoder",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/load.rs",
        &[
            "pub struct AssetLoadControl",
            "pub struct AssetLoadReport",
            "pub enum AssetLoadProgress",
            "progress_events",
            "emit_progress",
            "AssetError::Cancelled",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets.rs",
        &["load_scene_with_report", "load_scene_controlled"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/extensions.rs",
        &[
            "pub enum GltfDecoderPolicy",
            "decoder_policy",
            "basis-universal",
            "feature: \"ktx2\"",
            "KHR_materials_clearcoat",
            "KHR_materials_transmission",
            "KHR_materials_ior",
            "KHR_materials_volume",
            "KHR_materials_sheen",
            "KHR_materials_specular",
            "KHR_materials_iridescence",
            "EXT_texture_webp",
            "KHR_texture_basisu",
            "KHR_draco_mesh_compression",
            "EXT_meshopt_compression",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_missing_texture_slots_fail_with_actionable_asset_error",
            "m8_modern_optional_extensions_have_explicit_v1x_defer_metadata",
            "m8_metallic_roughness_factors_affect_cpu_preview_pixels",
            "GltfDecoderPolicy::FeatureFlag",
            "GltfDecoderPolicy::External",
            "normal_texture",
            "normal_texture_transform",
            "metallic_roughness_texture",
            "occlusion_texture",
            "emissive_texture",
            "emissive_texture_transform",
            "TextureWrap::MirroredRepeat",
            "TextureFilter::LinearMipmapLinear",
            "GltfExtensionStatus::Degraded",
            "m8_unsupported_texture_formats_fail_before_silent_handles_are_created",
            "m8_scene_load_reports_cache_fetch_and_external_buffer_metadata",
            "m8_cancelled_scene_load_does_not_cache_partial_asset_state",
            "m8_scene_load_progress_reports_fetch_parse_cache_and_external_buffers",
            "m8_gltf_data_uri_image_texture_descriptor_is_preserved",
            "m8_gltf_texcoord0_is_preserved_for_material_texture_sampling_contract",
            "m8_gltf_tangent_attribute_is_preserved_with_handedness",
            "m8_data_uri_base_color_texture_affects_cpu_preview_pixels",
            "m8_external_png_base_color_texture_affects_cpu_preview_pixels",
            "m8_reload_promotes_cached_texture_descriptor_when_external_png_arrives",
            "m8_retained_scene_source_bytes_allow_reload_when_fetcher_goes_offline",
            "m8_emissive_png_texture_affects_cpu_preview_pixels",
            "m8_normal_png_texture_affects_cpu_preview_pixels",
            "m8_metallic_roughness_png_texture_affects_cpu_preview_pixels",
            "m8_occlusion_png_texture_affects_cpu_preview_pixels",
            "m8_direct_load_texture_decodes_png_for_cpu_preview_pixels",
            "m8_direct_load_texture_decodes_jpeg_for_cpu_preview_pixels",
            "m8_checked_asset_lookups_report_typed_missing_handles",
            "m8_prepare_rejects_material_texture_handles_from_wrong_assets",
            "m8_texture_sampler_clamp_to_edge_affects_cpu_preview_pixels",
            "m8_headless_gpu_samples_multiple_base_color_material_slots_when_available",
            "m8_headless_gpu_applies_base_color_texture_transform_when_available",
            "m8_headless_gpu_directional_light_uniform_tints_pbr_output_when_available",
            "m8_headless_gpu_point_light_uniform_tints_pbr_output_when_available",
            "m8_headless_gpu_spot_light_uniform_tints_pbr_output_when_available",
            "m8_headless_gpu_tangent_space_normal_map_changes_pbr_lighting_when_available",
            "MutableMemoryFetcher",
            "png_rgba8",
            "TEXCOORD_0",
            "TANGENT",
            "tex_coords0",
            "tangents",
            "data:image/png",
            "TextureSourceFormat::Png",
            "m8_ktx2_basisu_texture_requires_feature_or_explicit_decoder_policy",
            "m8_ktx2_basisu_feature_loads_compressed_texture_descriptor",
            "m8_asset_resource_lifetime_counters_return_to_baseline_after_reload_cycle",
            "material_bindings",
            "material_texture_bindings",
            "material_sampler_bindings",
            "m8_khronos_material_texture_samples_cover_promoted_extensions",
            "m8_khronos_jpeg_textures_decode_for_degraded_material_preview",
            "m8_real_world_fixture_matrix_covers_asset_edge_cases",
            "m8_native_fetcher_cache_dedup_reload_retain_and_external_buffers_are_explicit",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_visual_proof.rs",
        &[
            "m8-unlit-textured-asset",
            "m8-metallic-roughness-asset",
            "m8-normal-mapped-asset",
            "m8-emissive-asset",
            "m8-alpha-mask",
            "m8-alpha-blend",
            "m8-texture-slots",
            "m8-environment-color-management",
            "(256, 256)",
            "png_rgba8",
            "TextureColorSpace::Srgb",
            "TextureColorSpace::Linear",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/read.rs",
        &[
            "TEXCOORD_0",
            "TANGENT",
            "read_vec2_accessor",
            "read_vec4_accessor",
            "tex_coords0",
            "with_tangents",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets.rs",
        &[
            "sample_texture",
            "fetch_optional_texture_bytes",
            "cached_texture_if_decoded",
            "texture_format_has_cpu_decoder",
            "external_image_paths",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/texture.rs",
        &[
            "decode_png_rgba8",
            "decode_jpeg_rgba8",
            "has_decoded_pixels",
            "decode_missing_pixels_from_bytes",
            "wrap_texture_coordinate",
            "TextureWrap::ClampToEdge",
            "TextureWrap::MirroredRepeat",
            "png::Decoder",
            "jpeg_decoder::Decoder",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/materials.rs",
        &[
            "base_color_texture_sample",
            "backend_sampled_base_color_texture",
            "backend_sampled_base_color_texture_is_not_baked_twice",
            "emissive_texture_sample",
            "normal_texture_sample",
            "metallic_roughness_texture_sample",
            "occlusion_texture_sample",
            "transform_texture_uv",
            "sample_texture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "docs/assets/gltf-asset-matrix.md",
        &[
            "Khronos `AlphaBlendModeTest`",
            "Khronos `TextureSettingsTest`",
            "Khronos `TextureTransformTest`",
            "Khronos `UnlitTest`",
            "`memory://real-world/material-degradation.gltf`",
            "`memory://real-world/external/scene.gltf`",
            "`memory://real-world/embedded.glb`",
            "`memory://embedded-texture.gltf`",
            "`memory://texcoord0.gltf`",
            "`memory://red-texture.gltf`",
            "`memory://external-texture/scene.gltf`",
            "`memory://reload-texture/scene.gltf`",
            "`memory://emissive-texture.gltf`",
            "`memory://sampler-clamp/scene.gltf`",
            "`memory://real-world/missing-texture.gltf`",
            "`memory://modern-optional-extensions.gltf`",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_visual_proof.rs",
        &[
            "m8_headless_visual_artifacts_cover_material_texture_environment_paths",
            "m8_visual_reference_sensitivity_covers_camera_transform_depth_material_texture_and_lighting",
            "assert_visual_change",
            "render_depth_sensitivity_scene",
            "target/gate-artifacts/m8-visual",
            "m8-alpha-blend",
            "m8-alpha-mask",
            "m8-texture-slots",
            "m8-environment-color-management",
            "decoded-texture-pixels-256",
            "memory://m8-visual-textures/scene.gltf",
            "center_pixel",
            "(256, 256)",
            "proof_class",
            "source_hash",
            "backend =",
            "adapter =",
            "renderer_settings =",
            "color_management =",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/materials.rs",
        &[
            "create_material_bind_group_layout",
            "create_material_resources",
            "material_texture_byte_len",
            "Vec<MaterialTextureResources>",
            "MaterialTextureUpload",
            "MaterialUniformUpload",
            "MATERIAL_UNIFORM_BYTE_LEN",
            "from_base_color_texture",
            "from_linear_texture",
            "decoded_base_color_texture_becomes_backend_upload",
            "binding: 2",
            "NORMAL_BINDINGS",
            "METALLIC_ROUGHNESS_BINDINGS",
            "OCCLUSION_BINDINGS",
            "EMISSIVE_BINDINGS",
            "SamplerBindingType::Filtering",
            "TextureSampleType::Float { filterable: true }",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/material_uniform.rs",
        &[
            "MaterialUniformUpload",
            "MATERIAL_UNIFORM_BYTE_LEN",
            "material_uniform_upload_encodes_base_color_texture_transform",
            "material_uniform_upload_encodes_material_factors",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/resources.rs",
        &[
            "collect_backend_base_color_textures",
            "collect_backend_material_slots",
            "backend_material_slots_preserve_all_texture_roles_and_material_only_slots",
            "base_color_texture",
            "normal_texture",
            "metallic_roughness_texture",
            "occlusion_texture",
            "emissive_texture",
            "base_color_texture_transform",
            "backend_base_color_texture_selection_keeps_multiple_decoded_textures",
            "backend_base_color_texture_selection_preserves_texture_transform_uniforms",
            "primary_base_color_texture_selection_defers_texture_transforms_to_cpu_bake",
            "has_decoded_pixels",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/texture.rs",
        &["decoded_rgba8", "rgba8.as_slice()"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/vertices.rs",
        &[
            "VERTEX_BYTE_LEN: usize = 17",
            "primitive.vertex_attributes()",
            "attributes.normal.x",
            "attributes.tex_coord0[0]",
            "attributes.tangent.x",
            "attributes.tangent_handedness",
            "attributes.shadow_visibility",
            "gpu_vertex_stream_carries_normals_and_texcoord0",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/webgl2.rs",
        &[
            "bind_material_texture",
            "PrimitiveDrawBatch",
            "draw_batch_hash",
            "WebGl2MaterialTextureSet",
            "upload_webgl2_material_texture_set",
            "configure_vertex_attributes",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/webgl2_program.rs",
        &[
            "uniform sampler2D base_color_texture",
            "uniform sampler2D normal_texture",
            "uniform sampler2D metallic_roughness_texture",
            "uniform sampler2D occlusion_texture",
            "uniform sampler2D emissive_texture",
            "uniform vec4 base_color_uv_offset_scale",
            "uniform vec4 base_color_uv_rotation",
            "uniform vec4 base_color_factor",
            "uniform vec4 emissive_strength",
            "uniform vec4 metallic_roughness_alpha",
            "in vec2 tex_coord0",
            "in vec2 v_tex_coord0",
            "texture(base_color_texture, transformed_uv)",
            "texture(normal_texture",
            "texture(metallic_roughness_texture",
            "texture(occlusion_texture",
            "texture(emissive_texture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/webgl2_materials.rs",
        &[
            "create_material_texture",
            "upload_material_texture_if_dirty",
            "MaterialTextureUpload::from_base_color_texture",
            "webgl2_wrap_mode",
            "webgl2_filter_mode",
            "tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/gpu/webgl2_texture_set.rs",
        &[
            "WebGl2MaterialTextureSet",
            "upload_webgl2_material_texture_set",
            "MaterialTextureUpload::from_normal_texture",
            "MaterialTextureUpload::from_metallic_roughness_texture",
            "MaterialTextureUpload::from_occlusion_texture",
            "MaterialTextureUpload::from_emissive_texture",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/browser_probe/workflows/ergonomics.rs",
        &[
            "material-textures",
            "asset-cache-reload",
            "decoded_base_color_texture",
            "decoded_normal_texture",
            "decoded_emissive_texture",
            "data:image/png;base64",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "material-textures",
            "asset-cache-reload",
            "assertMaterialTextureProof",
        ],
    );
}

fn check_binary_render_asset_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "BINARY-ASSET-TRUTH-P9";
    for asset_root in ["tests/assets", "docs/assets"] {
        collect_text_binary_asset_findings(root, Path::new(asset_root), findings, RULE);
    }
}

fn collect_text_binary_asset_findings(
    root: &Path,
    rel: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
) {
    let full = root.join(rel);
    let Ok(metadata) = fs::metadata(&full) else {
        return;
    };
    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(&full) else {
            return;
        };
        for entry in entries.flatten() {
            collect_text_binary_asset_findings(root, &rel.join(entry.file_name()), findings, rule);
        }
        return;
    }
    if !binary_render_asset_extension(rel) {
        return;
    }
    let Ok(bytes) = fs::read(&full) else {
        findings.push(Finding::new(
            rule,
            format!("could not read binary render asset {}", rel.display()),
        ));
        return;
    };
    if looks_like_text_fixture(&bytes) {
        findings.push(Finding::new(
            rule,
            format!(
                "{} uses a binary render asset extension but contains text fixture data; rename it to a fixture format or replace it with real binary bytes",
                rel.display()
            ),
        ));
    }
}

fn binary_render_asset_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "hdr" | "ktx2" | "rgba16f" | "glb"
            )
        })
        .unwrap_or(false)
}

fn looks_like_text_fixture(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && std::str::from_utf8(bytes)
            .map(|text| {
                text.contains("placeholder")
                    || text.contains("text-fixture")
                    || text
                        .chars()
                        .all(|ch| ch == '\n' || ch == '\r' || ch == '\t' || !ch.is_control())
            })
            .unwrap_or(false)
}

fn check_gltf_asset_matrix_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSET-MATRIX-M8";
    let matrix_rel = "docs/assets/gltf-asset-matrix.md";
    let manifest_rel = "tests/assets/gltf/khronos/manifest.toml";

    let Ok(matrix) = fs::read_to_string(root.join(matrix_rel)) else {
        findings.push(Finding::new(RULE, format!("could not read {matrix_rel}")));
        return;
    };

    for required in [
        "Source/License",
        "Expected Diagnostics",
        "Rendered Output Reference",
        "fail with a structured error",
        "silent fallback",
    ] {
        if !matrix.contains(required) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} is missing asset-matrix contract text '{required}'"),
            ));
        }
    }

    let rows = gltf_asset_matrix_rows(&matrix);
    if rows.is_empty() {
        findings.push(Finding::new(
            RULE,
            format!("{matrix_rel} has no asset rows"),
        ));
        return;
    }

    let mut listed_local_fixtures = BTreeSet::new();
    let mut listed_khronos_assets = BTreeSet::new();

    for row in rows {
        if row.len() != 7 {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{}' must have 7 columns: asset, source/license, features, expected result, expected diagnostics, rendered-output reference, evidence",
                    row.join(" | ")
                ),
            ));
            continue;
        }

        let asset = row[0].trim();
        let source_license = row[1].trim();
        let features = row[2].trim();
        let expected = row[3].trim();
        let diagnostics = row[4].trim();
        let rendered_output = row[5].trim();
        let evidence = row[6].trim();
        let row_text = row.join(" | ");

        if contains_placeholder(&row_text) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row contains placeholder text: {row_text}"),
            ));
        }
        if source_license.is_empty()
            || !(source_license.to_ascii_lowercase().contains("license")
                || source_license.to_ascii_lowercase().contains("test-only"))
        {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row '{asset}' must name source and license/test-only status"),
            ));
        }
        if features.is_empty() {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row '{asset}' must name covered features"),
            ));
        }
        if !expected_result_is_explicit(expected) {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{asset}' must expect pass, degrade, fail, or defer explicitly"
                ),
            ));
        }
        if diagnostics.is_empty() || diagnostics.eq_ignore_ascii_case("none") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{asset}' must record expected diagnostics or the explicit 'none expected' contract"
                ),
            ));
        }
        if rendered_output.is_empty()
            || rendered_output.eq_ignore_ascii_case("n/a")
            || contains_placeholder(rendered_output)
        {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{asset}' must name rendered-output proof or an explicit structured non-visual/deferred reason"
                ),
            ));
        }
        if evidence.is_empty() || contains_placeholder(evidence) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row '{asset}' must link executable evidence"),
            ));
        }

        for evidence_path in backtick_values(evidence) {
            if is_local_evidence_path(&evidence_path) && !root.join(&evidence_path).is_file() {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} row '{asset}' links missing evidence {evidence_path}"),
                ));
            }
        }

        for rendered_path in backtick_values(rendered_output) {
            if is_local_evidence_path(&rendered_path) && !root.join(&rendered_path).exists() {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{matrix_rel} row '{asset}' links missing rendered-output reference {rendered_path}"
                    ),
                ));
            }
        }

        if let Some(local_fixture) = first_backtick_value(asset)
            && local_fixture.starts_with("tests/assets/gltf/")
            && !local_fixture.contains("/khronos/")
        {
            listed_local_fixtures.insert(local_fixture.clone());
            if !root.join(&local_fixture).is_file() {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} lists missing fixture {local_fixture}"),
                ));
            }
        }

        if let Some(memory_fixture) = first_backtick_value(asset)
            && memory_fixture.starts_with("memory://")
        {
            let source_lower = source_license.to_ascii_lowercase();
            if !(source_lower.contains("generated") && source_lower.contains("test-only")) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{matrix_rel} memory fixture '{memory_fixture}' must be marked generated test-only"
                    ),
                ));
            }
        }

        if asset.starts_with("Khronos ") {
            if let Some(name) = first_backtick_value(asset) {
                listed_khronos_assets.insert(name);
            } else {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} Khronos row '{asset}' must backtick the asset name"),
                ));
            }
            let source_lower = source_license.to_ascii_lowercase();
            if !(source_lower.contains("khronos") && source_lower.contains("license")) {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} Khronos row '{asset}' must name Khronos license source"),
                ));
            }
        }
    }

    for fixture in direct_gltf_fixture_paths(root) {
        if !listed_local_fixtures.contains(&fixture) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} is missing direct glTF fixture row for {fixture}"),
            ));
        }
    }

    let Ok(manifest) = fs::read_to_string(root.join(manifest_rel)) else {
        findings.push(Finding::new(RULE, format!("could not read {manifest_rel}")));
        return;
    };

    for required in ["repository = ", "commit = ", "license_reference = "] {
        if !manifest.contains(required) {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} is missing required source metadata '{required}'"),
            ));
        }
    }

    for name in khronos_manifest_asset_names(&manifest) {
        if !listed_khronos_assets.contains(&name) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} is missing Khronos asset row for {name}"),
            ));
        }
    }

    let file_hashes = khronos_manifest_file_hashes(&manifest);
    for rel in khronos_manifest_file_paths(&manifest) {
        let full_rel = format!("tests/assets/gltf/khronos/{rel}");
        if !root.join(&full_rel).is_file() {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} references missing Khronos fixture file {full_rel}"),
            ));
            continue;
        }

        let Some(expected_sha256) = file_hashes.get(&rel) else {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} must record a SHA-256 hash for {full_rel}"),
            ));
            continue;
        };

        if !is_lower_hex_sha256(expected_sha256) {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} records invalid SHA-256 for {full_rel}"),
            ));
            continue;
        }

        match sha256_hex(&root.join(&full_rel)) {
            Ok(actual) if actual == *expected_sha256 => {}
            Ok(actual) => findings.push(Finding::new(
                RULE,
                format!(
                    "{manifest_rel} SHA-256 mismatch for {full_rel}: got {actual}, expected {expected_sha256}"
                ),
            )),
            Err(error) => findings.push(Finding::new(
                RULE,
                format!("could not hash {full_rel}: {error}"),
            )),
        }
    }

    for rel in file_hashes.keys() {
        let full_rel = format!("tests/assets/gltf/khronos/{rel}");
        if !root.join(&full_rel).is_file() {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{manifest_rel} records a hash for missing Khronos fixture file {full_rel}"
                ),
            ));
        }
    }
}

fn gltf_asset_matrix_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|') && line.ends_with('|'))
        .filter_map(|line| {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect::<Vec<_>>();
            match cells.first().map(String::as_str) {
                Some("Asset/Fixture") => None,
                Some(value) if value.chars().all(|ch| ch == '-') => None,
                Some(_) => Some(cells),
                None => None,
            }
        })
        .collect()
}

fn direct_gltf_fixture_paths(root: &Path) -> Vec<String> {
    let dir = root.join("tests/assets/gltf");
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut fixtures = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("gltf") {
                return None;
            }
            Some(format!(
                "tests/assets/gltf/{}",
                entry.file_name().to_string_lossy()
            ))
        })
        .collect::<Vec<_>>();
    fixtures.sort();
    fixtures
}

fn khronos_manifest_asset_names(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| quoted_assignment(line, "name"))
        .collect()
}

fn khronos_manifest_file_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut in_file_hashes = false;

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_file_hashes = line == "[file_hashes]";
            continue;
        }
        if in_file_hashes {
            continue;
        }
        if let Some(path) = quoted_assignment(line, "path").or_else(|| quoted_array_item(line)) {
            paths.push(path);
        }
    }

    paths
}

fn khronos_manifest_file_hashes(text: &str) -> BTreeMap<String, String> {
    let mut hashes = BTreeMap::new();
    let mut in_file_hashes = false;

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_file_hashes = line == "[file_hashes]";
            continue;
        }
        if !in_file_hashes || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let Some(path) = key.strip_prefix('"').and_then(|key| key.strip_suffix('"')) else {
            continue;
        };
        let Some(sha256) = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            continue;
        };
        hashes.insert(path.to_string(), sha256.to_string());
    }

    hashes
}

fn quoted_array_item(line: &str) -> Option<String> {
    line.strip_prefix('"')
        .and_then(|value| value.trim_end_matches(',').strip_suffix('"'))
        .map(str::to_string)
}

fn expected_result_is_explicit(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("pass")
        || lower.starts_with("degrade")
        || lower.starts_with("fail")
        || lower.starts_with("defer")
}

fn contains_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("todo")
        || lower.contains("tbd")
        || lower.contains("placeholder")
        || lower.contains("unknown")
}

fn is_local_evidence_path(value: &str) -> bool {
    value.starts_with("tests/")
        || value.starts_with("docs/")
        || value.starts_with("examples/")
        || value.starts_with("target/gate-artifacts/")
}

fn first_backtick_value(value: &str) -> Option<String> {
    backtick_values(value).into_iter().next()
}

fn backtick_values(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remainder = value;
    while let Some(start) = remainder.find('`') {
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        values.push(after_start[..end].to_string());
        remainder = &after_start[end + 1..];
    }
    values
}

fn require_manifest_value(
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    text: &str,
    key: &str,
    expected: &str,
) {
    match quoted_manifest_assignment(text, key) {
        Some(value) if value == expected => {}
        Some(value) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} {key} is '{value}', expected '{expected}'"),
        )),
        None => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing {key}"),
        )),
    }
}

fn require_manifest_u32(
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    text: &str,
    key: &str,
    expected: u32,
) {
    match u32_manifest_assignment(text, key) {
        Some(value) if value == expected => {}
        Some(value) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} {key} is {value}, expected {expected}"),
        )),
        None => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing {key}"),
        )),
    }
}

fn check_manifest_file_hash(
    root: &Path,
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    rel: &str,
    expected_sha256: &str,
) {
    if !is_lower_hex_sha256(expected_sha256) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} records invalid SHA-256 for {rel}"),
        ));
        return;
    }

    let path = root.join(rel);
    if !path.is_file() {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} references missing file {rel}"),
        ));
        return;
    }

    match sha256_hex(&path) {
        Ok(actual) if actual == expected_sha256 => {}
        Ok(actual) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} SHA-256 mismatch for {rel}: got {actual}"),
        )),
        Err(error) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("could not hash {rel}: {error}"),
        )),
    }
}

fn derivative_manifest_entries(text: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut current_path = None;
    for line in text.lines().map(str::trim) {
        if line == "[[derivative]]" {
            current_path = None;
            continue;
        }
        if let Some(path) = quoted_assignment(line, "path") {
            current_path = Some(path);
            continue;
        }
        if let Some(sha256) = quoted_assignment(line, "sha256")
            && let Some(path) = current_path.take()
        {
            entries.push((path, sha256));
        }
    }
    entries
}

fn quoted_manifest_assignment(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| quoted_assignment(line, key))
}

fn u32_manifest_assignment(text: &str, key: &str) -> Option<u32> {
    let prefix = format!("{key} = ");
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|value| value.parse().ok())
}

fn quoted_assignment(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    line.strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_string)
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let digest = Sha256::digest(fs::read(path)?);
    Ok(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_doctor_modes() {
        assert_eq!(
            parse_command(vec!["doctor".into(), "--docs".into()]),
            Ok(Command::Doctor(DoctorMode::Docs))
        );
        assert_eq!(
            parse_command(vec!["doctor".into(), "--architecture".into()]),
            Ok(Command::Doctor(DoctorMode::Architecture))
        );
        assert_eq!(
            parse_command(vec!["doctor".into(), "--full".into()]),
            Ok(Command::Doctor(DoctorMode::Full))
        );
        assert_eq!(
            parse_command(vec!["doctor".into()]),
            Ok(Command::Doctor(DoctorMode::Full))
        );
        assert_eq!(
            parse_command(vec!["claim-audit".into()]),
            Ok(Command::ClaimAudit)
        );
        assert_eq!(
            parse_command(vec!["release-lane-artifact".into(), "macos-metal".into()]),
            Ok(Command::ReleaseLaneArtifact("macos-metal".into()))
        );
        assert_eq!(
            parse_command(vec!["release-readiness".into()]),
            Ok(Command::ReleaseReadiness)
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse_command(vec!["check".into()]).is_err());
    }

    #[test]
    fn release_lane_artifacts_use_release_schema() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact =
            release_lane_artifact(&root, "linux-webgpu-chromium").expect("known lane is accepted");

        assert_eq!(artifact["schema"], "scena.release_lane.v1");
        assert_eq!(artifact["lane"], "linux-webgpu-chromium");
        assert_eq!(artifact["backend"], "WebGpu");
        assert!(
            artifact["generated_at_unix_seconds"].as_u64().is_some(),
            "release lane artifacts must be timestamped"
        );
        assert!(
            artifact["commit"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "release lane artifacts must record the source revision"
        );
        let command_records = artifact["command_records"]
            .as_array()
            .expect("command records are present");
        assert!(
            !command_records.is_empty(),
            "release lane artifacts must include structured command records"
        );
        let first = &command_records[0];
        assert!(first["command"].as_str().is_some());
        assert!(first["duration_ms"].is_null());
        assert_eq!(first["duration_source"], "ci-step-summary-or-wrapper");
        assert!(
            first["failure_log_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("linux-webgpu-chromium.log")),
            "command record must reserve a lane-specific failure log path"
        );
        assert!(
            first["artifact_checksums"].is_array(),
            "command record must carry checksum references for produced artifacts"
        );
        assert!(release_lane_artifact(&root, "unknown").is_err());
    }

    #[test]
    fn claim_categories_map_to_evidence_links() {
        let categories = claim_categories("Scene Renderer glTF WebGPU benchmark doctor");

        assert!(categories.contains(&"public-api"));
        assert!(categories.contains(&"assets-gltf"));
        assert!(categories.contains(&"browser-platform"));
        assert!(categories.contains(&"performance"));
        assert!(categories.contains(&"doctor"));
        assert!(
            evidence_links_for_category("assets-gltf")
                .iter()
                .any(|link| link.ends_with("m8_assets_materials_ecosystem.rs"))
        );
    }

    #[test]
    fn extracts_markdown_links() {
        let links = markdown_link_targets("A [doc](docs/a.md) and [external](https://x.test).");
        assert_eq!(links, vec!["docs/a.md", "https://x.test"]);
    }

    #[test]
    fn extracts_declared_type_names() {
        assert_eq!(
            declared_type_name("pub struct Renderer;"),
            Some("Renderer".into())
        );
        assert_eq!(declared_type_name("enum Backend {"), Some("Backend".into()));
        assert_eq!(declared_type_name("// pub struct Ignored;"), None);
    }

    #[test]
    fn catches_broad_type_names() {
        assert!(is_catch_all_type_name("SceneManager"));
        assert!(is_catch_all_type_name("World"));
        assert!(is_catch_all_type_name("AppContext"));
        assert!(!is_catch_all_type_name("InteractionContext"));
        assert!(!is_catch_all_type_name("Renderer"));
    }

    #[test]
    fn doctor_findings_include_contract_reference() {
        let finding = Finding::new("ARCH-RENDER-TRUTH", "shader bypassed camera projection");

        assert!(
            finding
                .message
                .contains("docs/checklists/state-of-art-threejs-replacement-plan.md"),
            "doctor findings must point maintainers at the governing checklist or spec"
        );
    }

    #[test]
    fn unit_test_first_governance_is_declared() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_unit_test_first_governance(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn browser_backend_vocabulary_is_explicit() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_backend_vocabulary(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn module_boundaries_guard_render_phase_side_effects() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_module_boundaries(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn source_files_include_renderer_submodules() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let files = source_files(&root);

        assert!(
            files
                .iter()
                .any(|path| path == Path::new("src/render/gpu.rs"))
        );
    }

    #[test]
    fn source_scope_terms_match_whole_tokens() {
        assert!(contains_scope_term("a robot module", "robot"));
        assert!(!contains_scope_term("a roboticist module", "robot"));
    }

    #[test]
    fn asset_api_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_asset_api_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn render_alpha_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_render_alpha_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn output_stage_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_output_stage_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn fxaa_output_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_fxaa_output_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn diagnostics_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_diagnostics_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn renderer_stats_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_renderer_stats_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn renderer_truth_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_renderer_truth_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn doctor_rejects_shader_clip_position_passthrough_regression() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/shader-passthrough");
        let shader_path = fixture_root.join("src/render/gpu/output.rs");
        fs::create_dir_all(shader_path.parent().expect("shader parent")).expect("fixture dir");
        fs::write(
            &shader_path,
            "fn vs_main() { out.position = vec4<f32>(in.position, 1.0); }\n",
        )
        .expect("shader fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-TRUTH",
            "src/render/gpu/output.rs",
            &["out.position = vec4<f32>(in.position, 1.0);"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-TRUTH"
                    && finding.message.contains("out.position = vec4")
            }),
            "doctor must reject production shaders that bypass camera projection: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_supported_forward_pbr_regression() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/supported-pbr");
        let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
        fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
            .expect("fixture dir");
        fs::write(
            &capabilities_path,
            "const fn forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported\n}\n",
        )
        .expect("capability fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-TRUTH",
            "src/diagnostics/capabilities.rs",
            &[
                "forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-TRUTH"
                    && finding.message.contains("CapabilityStatus::Supported")
            }),
            "doctor must reject false forward_pbr support claims: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_meshless_model_viewer_regression() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/meshless-viewer");
        let example_path = fixture_root.join("examples/glb_model_viewer.rs");
        fs::create_dir_all(example_path.parent().expect("example parent")).expect("fixture dir");
        fs::write(
            &example_path,
            "fn main() { let _path = \"tests/assets/gltf/minimal_scene.gltf\"; }\n",
        )
        .expect("example fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-TRUTH",
            "examples/glb_model_viewer.rs",
            &["minimal_scene.gltf"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-TRUTH"
                    && finding.message.contains("minimal_scene.gltf")
            }),
            "doctor must reject model-viewer examples backed by meshless fixtures: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_oversized_source_module_regression() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/oversized-module");
        let source_path = fixture_root.join("src/render/too_large.rs");
        fs::create_dir_all(source_path.parent().expect("source parent")).expect("fixture dir");
        let mut source = String::new();
        for index in 0..=MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
            source.push_str(&format!("pub fn oversized_fixture_{index}() {{}}\n"));
        }
        fs::write(&source_path, source).expect("oversized source fixture");
        let mut findings = Vec::new();

        check_solid_kiss(&fixture_root, &mut findings);

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-KISS-SIZE"
                    && finding.message.contains("src/render/too_large.rs")
            }),
            "doctor must reject source modules above the KISS size threshold: {findings:?}",
        );
    }

    #[test]
    fn prepare_asset_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_prepare_asset_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn render_world_bake_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_render_world_bake_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn doctor_rejects_renderer_asset_fetch_regression() {
        // ARCH-RENDER: nothing under src/render/** may name asset fetcher entry points.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/renderer-asset-fetch");
        let render_path = fixture_root.join("src/render/build.rs");
        fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
        fs::write(
            &render_path,
            "fn build_renderer() { let _bytes = fetcher.fetch(\"asset\"); }\n",
        )
        .expect("render fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER",
            "src/render/build.rs",
            &["fetch("],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER" && finding.message.contains("fetch(")
            }),
            "doctor must reject renderer modules that call fetcher entry points: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_render_phase_pipeline_creation_regression() {
        // ARCH-RENDER-LIFECYCLE: render-phase modules must not allocate shaders or pipelines.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root =
            root.join("target/xtask-doctor-regressions/render-phase-pipeline-creation");
        let draw_path = fixture_root.join("src/render/gpu/draw.rs");
        fs::create_dir_all(draw_path.parent().expect("draw parent")).expect("fixture dir");
        fs::write(
            &draw_path,
            "fn render() { device.create_render_pipeline(&desc); }\n",
        )
        .expect("draw fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-LIFECYCLE",
            "src/render/gpu/draw.rs",
            &["create_render_pipeline"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-LIFECYCLE"
                    && finding.message.contains("create_render_pipeline")
            }),
            "doctor must reject GPU render-phase modules that create render pipelines: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_platform_renderer_pass_regression() {
        // ARCH-PLATFORM: platform stays an adapter layer; pass type names belong in
        // render/**. The canonical forbidden terms are `wgpu::`, `ForwardPass`, `ShadowPass`,
        // and `PostProcessPass`.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/platform-render-pass");
        let platform_path = fixture_root.join("src/platform.rs");
        fs::create_dir_all(platform_path.parent().expect("platform parent")).expect("fixture dir");
        fs::write(
            &platform_path,
            "pub struct ForwardPass; pub fn run(_pass: &mut ForwardPass) {}\n",
        )
        .expect("platform fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-PLATFORM",
            "src/platform.rs",
            &["ForwardPass"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-PLATFORM" && finding.message.contains("ForwardPass")
            }),
            "doctor must reject platform.rs that owns renderer pass types: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_assets_wgpu_dependency_regression() {
        // ARCH-ASSETS: assets owns fetch/parse/cache and must not consume wgpu surface types.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/assets-wgpu-dependency");
        let assets_path = fixture_root.join("src/assets.rs");
        fs::create_dir_all(assets_path.parent().expect("assets parent")).expect("fixture dir");
        fs::write(
            &assets_path,
            "fn upload(device: &wgpu::Device) { let _texture = device.create_texture(&desc); }\n",
        )
        .expect("assets fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ASSETS",
            "src/assets.rs",
            &["wgpu::"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ASSETS" && finding.message.contains("wgpu::")
            }),
            "doctor must reject assets.rs that pulls in wgpu types: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_output_stage_missing_aces_tonemap_regression() {
        // ARCH-OUTPUT-STAGE: the renderer output stage must implement ACES; a stub
        // src/render/output.rs that drops the tonemap helpers regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/output-stage-no-aces");
        let output_path = fixture_root.join("src/render/output.rs");
        fs::create_dir_all(output_path.parent().expect("output parent")).expect("fixture dir");
        fs::write(
            &output_path,
            "// no aces helpers here\npub fn passthrough() {}\n",
        )
        .expect("output fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-OUTPUT-STAGE",
            "src/render/output.rs",
            &[
                "fn aces_tonemap",
                "fn rrt_and_odt_fit",
                "ACES_INPUT_MATRIX",
                "ACES_OUTPUT_MATRIX",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-OUTPUT-STAGE" && finding.message.contains("fn aces_tonemap")
            }),
            "doctor must reject output stages that drop ACES tonemap helpers: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_render_alpha_missing_linear_source_over_regression() {
        // ARCH-RENDER-ALPHA: capabilities.rs must expose AlphaPipelineStatus with the
        // LinearSourceOver and BackendPassthrough variants. A stub that drops them
        // regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/render-alpha-stub");
        let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
        fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
            .expect("fixture dir");
        fs::write(&capabilities_path, "pub struct Capabilities {}\n")
            .expect("capabilities fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-ALPHA",
            "src/diagnostics/capabilities.rs",
            &[
                "pub enum AlphaPipelineStatus",
                "LinearSourceOver",
                "BackendPassthrough",
                "pub alpha_pipeline: AlphaPipelineStatus",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-ALPHA" && finding.message.contains("LinearSourceOver")
            }),
            "doctor must reject capabilities that drop the alpha-pipeline contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_diagnostics_missing_typed_code_regression() {
        // ARCH-DIAGNOSTICS: diagnostic.rs must expose Diagnostic with code, severity,
        // and message. A stub without typed code regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/diagnostics-untyped");
        let diagnostic_path = fixture_root.join("src/diagnostics/diagnostic.rs");
        fs::create_dir_all(diagnostic_path.parent().expect("diagnostic parent"))
            .expect("fixture dir");
        fs::write(
            &diagnostic_path,
            "pub struct Diagnostic { pub message: String }\n",
        )
        .expect("diagnostic fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-DIAGNOSTICS",
            "src/diagnostics/diagnostic.rs",
            &[
                "pub struct Diagnostic",
                "pub code: DiagnosticCode",
                "pub severity: DiagnosticSeverity",
                "pub message: String",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-DIAGNOSTICS" && finding.message.contains("DiagnosticCode")
            }),
            "doctor must reject Diagnostic types that drop the typed code/severity \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_renderer_stats_missing_required_counters_regression() {
        // ARCH-RENDER-STATS: diagnostics.rs must expose RendererStats with the required
        // resource-lifetime counters. A stub that drops them regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/renderer-stats-stub");
        let diagnostics_path = fixture_root.join("src/diagnostics.rs");
        fs::create_dir_all(diagnostics_path.parent().expect("diagnostics parent"))
            .expect("fixture dir");
        fs::write(
            &diagnostics_path,
            "pub struct RendererStats { pub frames_rendered: u64 }\n",
        )
        .expect("diagnostics fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-STATS",
            "src/diagnostics.rs",
            &[
                "pub struct RendererStats",
                "pub buffers: u64",
                "pub textures: u64",
                "pub materials: u64",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-STATS" && finding.message.contains("pub buffers: u64")
            }),
            "doctor must reject RendererStats that drops the resource-lifetime counter \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_camera_depth_missing_perspective_camera_regression() {
        // ARCH-CAMERA-DEPTH: src/scene/camera.rs must expose Camera/PerspectiveCamera/
        // OrthographicCamera/DepthRange. A stub that drops them regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/camera-depth-stub");
        let camera_path = fixture_root.join("src/scene/camera.rs");
        fs::create_dir_all(camera_path.parent().expect("camera parent")).expect("fixture dir");
        fs::write(&camera_path, "pub struct CameraStub {}\n").expect("camera fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-CAMERA-DEPTH",
            "src/scene/camera.rs",
            &[
                "pub enum Camera",
                "pub struct PerspectiveCamera",
                "pub struct OrthographicCamera",
                "pub struct DepthRange",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-CAMERA-DEPTH" && finding.message.contains("PerspectiveCamera")
            }),
            "doctor must reject camera modules that drop the typed-camera contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_clipping_missing_clipping_plane_key_regression() {
        // ARCH-CLIPPING: src/scene.rs must expose ClippingPlaneKey for typed clipping
        // plane handles.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/clipping-stub");
        let scene_path = fixture_root.join("src/scene.rs");
        fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
        fs::write(&scene_path, "pub struct Scene {}\n").expect("scene fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-CLIPPING",
            "src/scene.rs",
            &["pub struct ClippingPlaneKey"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-CLIPPING" && finding.message.contains("ClippingPlaneKey")
            }),
            "doctor must reject scene modules that drop the typed clipping-plane handle: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_depth_prepass_missing_counters_regression() {
        // ARCH-DEPTH-PREPASS: diagnostics.rs must expose the depth-prepass counter
        // contract so the doctor can prove the prepass actually executed.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/depth-prepass-stub");
        let diagnostics_path = fixture_root.join("src/diagnostics.rs");
        fs::create_dir_all(diagnostics_path.parent().expect("diagnostics parent"))
            .expect("fixture dir");
        fs::write(
            &diagnostics_path,
            "pub struct RendererStats { pub frames_rendered: u64 }\n",
        )
        .expect("diagnostics fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-DEPTH-PREPASS",
            "src/diagnostics.rs",
            &[
                "pub depth_prepass_passes: u64",
                "pub depth_prepass_draws: u64",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-DEPTH-PREPASS"
                    && finding.message.contains("depth_prepass_passes")
            }),
            "doctor must reject diagnostics.rs that drops the depth-prepass counter \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_origin_shift_missing_field_regression() {
        // ARCH-ORIGIN-SHIFT: src/scene.rs must expose origin_shift as a Vec3 field so
        // large-scene precision shifts stay observable.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/origin-shift-stub");
        let scene_path = fixture_root.join("src/scene.rs");
        fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
        fs::write(&scene_path, "pub struct Scene { pub root: NodeKey }\n").expect("scene fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ORIGIN-SHIFT",
            "src/scene.rs",
            &["origin_shift: Vec3"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ORIGIN-SHIFT"
                    && finding.message.contains("origin_shift: Vec3")
            }),
            "doctor must reject scene.rs that drops the origin_shift Vec3 contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_environment_lifecycle_missing_revision_regression() {
        // ARCH-ENVIRONMENT-LIFECYCLE: src/render.rs must track the bound environment plus
        // its revision so reload/dirty propagation stays observable.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/environment-lifecycle-stub");
        let render_path = fixture_root.join("src/render.rs");
        fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
        fs::write(&render_path, "pub struct Renderer {}\n").expect("render fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ENVIRONMENT-LIFECYCLE",
            "src/render.rs",
            &[
                "environment: Option<EnvironmentHandle>",
                "environment_revision: u64",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ENVIRONMENT-LIFECYCLE"
                    && finding.message.contains("environment_revision")
            }),
            "doctor must reject Renderer types that drop the environment revision \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_scene_lights_missing_typed_key_regression() {
        // ARCH-SCENE-LIGHTS: src/scene.rs must expose the typed LightKey handle plus
        // the lights submodule so light entries do not become string lookups.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/scene-lights-stub");
        let scene_path = fixture_root.join("src/scene.rs");
        fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
        fs::write(&scene_path, "pub struct Scene { pub root: NodeKey }\n").expect("scene fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-SCENE-LIGHTS",
            "src/scene.rs",
            &["pub struct LightKey", "mod lights;"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-SCENE-LIGHTS" && finding.message.contains("LightKey")
            }),
            "doctor must reject scene.rs that drops the typed light-key contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_shadow_map_missing_counter_regression() {
        // ARCH-SHADOW-MAP: diagnostics.rs must expose shadow_maps and the directional
        // shadow-map resolution metadata so missing shadow infrastructure stays visible.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/shadow-map-stub");
        let diagnostics_path = fixture_root.join("src/diagnostics.rs");
        fs::create_dir_all(diagnostics_path.parent().expect("diagnostics parent"))
            .expect("fixture dir");
        fs::write(
            &diagnostics_path,
            "pub struct RendererStats { pub frames_rendered: u64 }\n",
        )
        .expect("diagnostics fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-SHADOW-MAP",
            "src/diagnostics.rs",
            &[
                "pub shadow_maps: u64",
                "pub directional_shadow_map_resolution: Option<u32>",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-SHADOW-MAP" && finding.message.contains("shadow_maps: u64")
            }),
            "doctor must reject diagnostics.rs that drops the shadow-map counter \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_m3b_animation_missing_typed_keys_regression() {
        // ARCH-M3B-ANIMATION: src/animation.rs must expose the typed animation handle and
        // playback-state enums so animation lookups stay typed instead of stringly.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/m3b-animation-stub");
        let animation_path = fixture_root.join("src/animation.rs");
        fs::create_dir_all(animation_path.parent().expect("animation parent"))
            .expect("fixture dir");
        fs::write(&animation_path, "pub struct AnimationMixer {}\n").expect("animation fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-M3B-ANIMATION",
            "src/animation.rs",
            &[
                "pub struct AnimationMixerKey",
                "pub enum AnimationPlaybackState",
                "pub enum AnimationLoopMode",
                "pub enum AnimationTarget",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-M3B-ANIMATION"
                    && finding.message.contains("AnimationMixerKey")
            }),
            "doctor must reject animation.rs that drops the typed mixer-key contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_m4_platform_missing_dirty_state_regression() {
        // ARCH-M4-PLATFORM: src/scene/dirty.rs must expose SceneDirtyState plus the
        // transform_revision counter so dirty propagation stays observable to render-
        // on-change consumers.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/m4-platform-stub");
        let dirty_path = fixture_root.join("src/scene/dirty.rs");
        fs::create_dir_all(dirty_path.parent().expect("dirty parent")).expect("fixture dir");
        fs::write(&dirty_path, "pub struct DirtyState {}\n").expect("dirty fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-M4-PLATFORM",
            "src/scene/dirty.rs",
            &[
                "pub struct SceneDirtyState",
                "transform_revision",
                "pub fn dirty_state",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-M4-PLATFORM" && finding.message.contains("SceneDirtyState")
            }),
            "doctor must reject scene/dirty.rs that drops the SceneDirtyState contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_release_ci_silent_artifact_upload_regression() {
        // RELEASE-CI-M9: CI workflows must use `if-no-files-found: error` on artifact
        // upload so a silent missing-artifacts upload doesn't pretend the lane passed.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/release-ci-silent-upload");
        let workflow_path = fixture_root.join(".github/workflows/ci.yml");
        fs::create_dir_all(workflow_path.parent().expect("workflow parent")).expect("fixture dir");
        fs::write(
            &workflow_path,
            "jobs:\n  some-lane:\n    steps:\n      - uses: actions/upload-artifact@v4\n        with:\n          name: gate-artifacts\n          path: target/gate-artifacts/**\n          if-no-files-found: ignore\n",
        )
        .expect("workflow fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "RELEASE-CI-M9",
            ".github/workflows/ci.yml",
            &["if-no-files-found: ignore"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "RELEASE-CI-M9"
                    && finding.message.contains("if-no-files-found: ignore")
            }),
            "doctor must reject CI workflows that silently ignore missing artifacts: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_fxaa_missing_pass_counter_regression() {
        // ARCH-FXAA-OUTPUT: diagnostics.rs must expose fxaa_passes: u64 so the FXAA
        // pass invocation count stays observable to release-readiness.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/fxaa-output-stub");
        let diagnostics_path = fixture_root.join("src/diagnostics.rs");
        fs::create_dir_all(diagnostics_path.parent().expect("diagnostics parent"))
            .expect("fixture dir");
        fs::write(
            &diagnostics_path,
            "pub struct RendererStats { pub frames_rendered: u64 }\n",
        )
        .expect("diagnostics fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-FXAA-OUTPUT",
            "src/diagnostics.rs",
            &["pub fxaa_passes: u64"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-FXAA-OUTPUT" && finding.message.contains("fxaa_passes: u64")
            }),
            "doctor must reject diagnostics.rs that drops the FXAA pass counter: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_reversed_z_missing_capability_field_regression() {
        // ARCH-REVERSED-Z: capabilities.rs must expose reversed_z_depth as a typed
        // CapabilityStatus and the const status helper that downgrades on WebGL2.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/reversed-z-stub");
        let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
        fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
            .expect("fixture dir");
        fs::write(&capabilities_path, "pub struct Capabilities {}\n")
            .expect("capabilities fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-REVERSED-Z",
            "src/diagnostics/capabilities.rs",
            &[
                "pub reversed_z_depth: CapabilityStatus",
                "const fn reversed_z_depth_status",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-REVERSED-Z" && finding.message.contains("reversed_z_depth")
            }),
            "doctor must reject capabilities.rs that drops the reversed_z_depth typed \
             status contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_backend_vocabulary_missing_browser_canvas_regression() {
        // ARCH-BACKEND-VOCAB: src/platform.rs must expose browser_webgpu_canvas /
        // browser_webgl2_canvas constructors so the descriptor and attached-canvas
        // backends share a stable vocabulary.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/backend-vocab-stub");
        let platform_path = fixture_root.join("src/platform.rs");
        fs::create_dir_all(platform_path.parent().expect("platform parent")).expect("fixture dir");
        fs::write(&platform_path, "pub struct PlatformSurface {}\n").expect("platform fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-BACKEND-VOCAB",
            "src/platform.rs",
            &["browser_webgpu_canvas", "browser_webgl2_canvas"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-BACKEND-VOCAB"
                    && finding.message.contains("browser_webgpu_canvas")
            }),
            "doctor must reject platform.rs that drops the browser canvas backend \
             vocabulary: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_webgl2_depth_missing_diagnostic_regression() {
        // ARCH-WEBGL2-DEPTH: capabilities.rs must emit the WebGL2 depth-compatibility
        // diagnostic so users see the reduced near/far precision warning.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/webgl2-depth-stub");
        let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
        fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
            .expect("fixture dir");
        fs::write(
            &capabilities_path,
            "pub struct Capabilities { pub backend: Backend }\n",
        )
        .expect("capabilities fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-WEBGL2-DEPTH",
            "src/diagnostics/capabilities.rs",
            &[
                "pub fn diagnostics(self) -> Vec<Diagnostic>",
                "self.backend == Backend::WebGl2",
                "DiagnosticCode::WebGl2DepthCompatibility",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-WEBGL2-DEPTH"
                    && finding.message.contains("WebGl2DepthCompatibility")
            }),
            "doctor must reject capabilities.rs that drops the WebGL2 depth diagnostic: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_solid_kiss_docs_missing_gate_regression() {
        // ARCH-SOLID-KISS-DOCS: docs/specs/module-boundaries.md must enumerate the
        // SOLID/KISS gate so the design rules stay anchored to a doc the doctor reads.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/solid-kiss-docs-stub");
        let module_path = fixture_root.join("docs/specs/module-boundaries.md");
        fs::create_dir_all(module_path.parent().expect("module boundaries parent"))
            .expect("fixture dir");
        fs::write(
            &module_path,
            "# Module Boundaries\n\nNo SOLID/KISS gate text here.\n",
        )
        .expect("module-boundaries fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-SOLID-KISS-DOCS",
            "docs/specs/module-boundaries.md",
            &[
                "## SOLID/KISS Gate",
                "Every public feature must name exactly one owner module",
                "No catch-all `Manager`, `Engine`, `World`, or broad `Context` type",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-SOLID-KISS-DOCS"
                    && finding.message.contains("SOLID/KISS Gate")
            }),
            "doctor must reject module-boundaries.md that drops the SOLID/KISS gate \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_direct_light_shading_missing_world_transform_iter_regression() {
        // ARCH-DIRECT-LIGHT-SHADING: scene.rs must expose the world-transform light
        // iterator so direct-light shading uses composed world transforms instead of
        // local node transforms.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/direct-light-shading-stub");
        let scene_path = fixture_root.join("src/scene.rs");
        fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
        fs::write(&scene_path, "pub struct Scene { pub root: NodeKey }\n").expect("scene fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-DIRECT-LIGHT-SHADING",
            "src/scene.rs",
            &[
                "impl Iterator<Item = (NodeKey, LightKey, Light, Transform)>",
                "self.world_transform(node_key)",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-DIRECT-LIGHT-SHADING"
                    && finding.message.contains("world_transform")
            }),
            "doctor must reject scene.rs that drops the world-transform light iteration \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_environment_hdr_missing_loader_regression() {
        // ARCH-ENV-HDR: src/assets/environment.rs must expose the equirectangular HDR
        // loader so HDR fixtures can be parsed into PreparedEnvironmentLighting.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/environment-hdr-stub");
        let environment_path = fixture_root.join("src/assets/environment.rs");
        fs::create_dir_all(environment_path.parent().expect("environment parent"))
            .expect("fixture dir");
        fs::write(&environment_path, "pub struct EnvironmentDesc {}\n")
            .expect("environment fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ENV-HDR",
            "src/assets/environment.rs",
            &[
                "EnvironmentSourceKind::EquirectangularHdr",
                "pub fn from_equirectangular_hdr_path",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ENV-HDR" && finding.message.contains("EquirectangularHdr")
            }),
            "doctor must reject assets/environment.rs that drops the equirectangular \
             HDR loader contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_environment_ibl_prepare_missing_stats_regression() {
        // ARCH-ENV-IBL-PREP: prepare/stats.rs must expose PreparedEnvironmentStats so
        // the IBL prepare path stays observable through structured stats.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/env-ibl-prepare-stub");
        let stats_path = fixture_root.join("src/render/prepare/stats.rs");
        fs::create_dir_all(stats_path.parent().expect("stats parent")).expect("fixture dir");
        fs::write(&stats_path, "pub struct LightingStats {}\n").expect("stats fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ENV-IBL-PREP",
            "src/render/prepare/stats.rs",
            &["pub(in crate::render) struct PreparedEnvironmentStats"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ENV-IBL-PREP"
                    && finding.message.contains("PreparedEnvironmentStats")
            }),
            "doctor must reject prepare/stats.rs that drops the PreparedEnvironmentStats \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_directional_shadow_missing_multiple_lights_error_regression() {
        // ARCH-DIRECTIONAL-SHADOW: prepare/stats.rs must expose MultipleShadowedDirectionalLights
        // so a scene with two shadow-casting directional lights fails closed instead of
        // silently picking one.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/directional-shadow-stub");
        let stats_path = fixture_root.join("src/render/prepare/stats.rs");
        fs::create_dir_all(stats_path.parent().expect("stats parent")).expect("fixture dir");
        fs::write(&stats_path, "pub struct LightingStats {}\n").expect("stats fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-DIRECTIONAL-SHADOW",
            "src/render/prepare/stats.rs",
            &[
                "pub(in crate::render) fn collect_lighting_stats(",
                "PrepareError::MultipleShadowedDirectionalLights",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-DIRECTIONAL-SHADOW"
                    && finding
                        .message
                        .contains("MultipleShadowedDirectionalLights")
            }),
            "doctor must reject prepare/stats.rs that drops the directional-shadow \
             multiple-lights error: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_m3a_scene_import_missing_dependencies_regression() {
        // ARCH-M3A-SCENE-IMPORT: Cargo.toml must keep the base64/serde_json/wasm-bindgen
        // -futures/Response/obj feature-flag dependencies that the M3a scene importer
        // relies on. A stub Cargo.toml without them regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/m3a-scene-import-stub");
        let cargo_path = fixture_root.join("Cargo.toml");
        fs::create_dir_all(cargo_path.parent().expect("cargo parent")).expect("fixture dir");
        fs::write(
            &cargo_path,
            "[package]\nname = \"scena\"\nversion = \"0.0.0\"\n",
        )
        .expect("cargo fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-M3A-SCENE-IMPORT",
            "Cargo.toml",
            &[
                "base64",
                "serde_json",
                "wasm-bindgen-futures",
                "Response",
                "obj = []",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-M3A-SCENE-IMPORT" && finding.message.contains("base64")
            }),
            "doctor must reject Cargo.toml that drops the M3a scene-import dependency \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_assets_m8_missing_texture_role_imports_regression() {
        // ASSETS-M8: src/assets/gltf/read.rs must parse all five glTF material texture
        // roles plus their KHR_texture_transform variants. A stub that drops them
        // regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/assets-m8-stub");
        let read_path = fixture_root.join("src/assets/gltf/read.rs");
        fs::create_dir_all(read_path.parent().expect("read parent")).expect("fixture dir");
        fs::write(&read_path, "pub fn read_baseColorTexture() {}\n").expect("read fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ASSETS-M8",
            "src/assets/gltf/read.rs",
            &[
                "normalTexture",
                "metallicRoughnessTexture",
                "occlusionTexture",
                "emissiveTexture",
                "with_normal_texture_transform",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ASSETS-M8" && finding.message.contains("normalTexture")
            }),
            "doctor must reject assets/gltf/read.rs that drops the five glTF texture \
             role imports: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_solid_catch_all_type_regression() {
        // ARCH-SOLID-CATCH-ALL: source modules must not declare catch-all types like
        // Manager, Engine, World, or broad Context. A stub that names one regresses
        // the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/solid-catch-all-stub");
        let scope_path = fixture_root.join("src/scope.rs");
        fs::create_dir_all(scope_path.parent().expect("scope parent")).expect("fixture dir");
        // Use a simple needle the rule will reject; the rule's source scan in
        // check_solid_kiss looks for Manager/Engine/World/broad Context names.
        fs::write(
            &scope_path,
            "pub struct GlobalManager {}\npub struct WorldEngine {}\n",
        )
        .expect("scope fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-SOLID-CATCH-ALL",
            "src/scope.rs",
            &["GlobalManager"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-SOLID-CATCH-ALL" && finding.message.contains("GlobalManager")
            }),
            "doctor must reject source modules that name catch-all Manager/Engine \
             types: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_visual_fixture_metadata_missing_suite_regression() {
        // VISUAL-FIXTURE-METADATA: tests/visual/fixtures/m1-headless-core.toml must
        // declare the [suite] block with the name/format/encoding contract so the
        // doctor can compare each rendered fixture against it.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root =
            root.join("target/xtask-doctor-regressions/visual-fixture-metadata-stub");
        let toml_path = fixture_root.join("tests/visual/fixtures/m1-headless-core.toml");
        fs::create_dir_all(toml_path.parent().expect("toml parent")).expect("fixture dir");
        fs::write(&toml_path, "# placeholder fixture metadata\n").expect("toml fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "VISUAL-FIXTURE-METADATA",
            "tests/visual/fixtures/m1-headless-core.toml",
            &[
                "[suite]",
                "name = \"m1-headless-core\"",
                "format = \"ppm\"",
                "encoding = \"srgb8\"",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "VISUAL-FIXTURE-METADATA" && finding.message.contains("[suite]")
            }),
            "doctor must reject m1 visual fixture TOML missing the suite contract: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_release_ci_missing_lane_regression() {
        // RELEASE-CI-M9: ci.yml must list every release lane name. A workflow that
        // drops e.g. macos-metal regresses the contract that release-readiness can
        // expect lane artifacts on every push.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/release-ci-missing-lane");
        let workflow_path = fixture_root.join(".github/workflows/ci.yml");
        fs::create_dir_all(workflow_path.parent().expect("workflow parent")).expect("fixture dir");
        fs::write(
            &workflow_path,
            "jobs:\n  linux-native-vulkan:\n    runs-on: ubuntu-24.04\n",
        )
        .expect("workflow fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "RELEASE-CI-M9",
            ".github/workflows/ci.yml",
            &[
                "linux-native-vulkan",
                "linux-browser-webgl2",
                "macos-metal",
                "windows-dx12",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "RELEASE-CI-M9" && finding.message.contains("macos-metal")
            }),
            "doctor must reject CI workflows that drop a required release lane: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_ergonomics_m7_missing_controls_contract_regression() {
        // ERGONOMICS-M7: src/controls.rs must expose the orbit-controls contract terms
        // (with_damping, focus, apply_to_scene, damping_factor, TouchEvent, wheel) so
        // controls keep the ergonomic shape examples expect.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/ergonomics-m7-stub");
        let controls_path = fixture_root.join("src/controls.rs");
        fs::create_dir_all(controls_path.parent().expect("controls parent")).expect("fixture dir");
        fs::write(&controls_path, "pub struct OrbitControls {}\n").expect("controls fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ERGONOMICS-M7",
            "src/controls.rs",
            &[
                "with_damping",
                "focus",
                "apply_to_scene",
                "damping_factor",
                "TouchEvent",
                "pub const fn wheel",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ERGONOMICS-M7" && finding.message.contains("apply_to_scene")
            }),
            "doctor must reject controls.rs that drops the orbit-controls ergonomic \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_assets_m8_missing_color_space_regression() {
        // ASSETS-M8 (color space): src/assets/gltf/read.rs must mention both linear and
        // sRGB texture color spaces so glTF imports tag every material texture's color
        // pipeline correctly.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/assets-m8-color-space-stub");
        let read_path = fixture_root.join("src/assets/gltf/read.rs");
        fs::create_dir_all(read_path.parent().expect("read parent")).expect("fixture dir");
        fs::write(
            &read_path,
            "pub fn baseColorTexture() {}\npub fn normalTexture() {}\n\
             pub fn metallicRoughnessTexture() {}\npub fn occlusionTexture() {}\n\
             pub fn emissiveTexture() {}\npub fn with_normal_texture_transform() {}\n\
             pub fn with_metallic_roughness_texture_transform() {}\n\
             pub fn with_occlusion_texture_transform() {}\n\
             pub fn with_emissive_texture_transform() {}\n",
        )
        .expect("read fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ASSETS-M8",
            "src/assets/gltf/read.rs",
            &["TextureColorSpace::Linear", "TextureColorSpace::Srgb"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ASSETS-M8" && finding.message.contains("TextureColorSpace")
            }),
            "doctor must reject assets/gltf/read.rs that drops the texture color-space \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_asset_api_missing_color_space_parameter_regression() {
        // ARCH-ASSET-API: src/assets.rs must keep the explicit
        // load_texture(color_space: TextureColorSpace) signature so callers cannot
        // accidentally load a texture into the wrong color pipeline.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/asset-api-stub");
        let assets_path = fixture_root.join("src/assets.rs");
        fs::create_dir_all(assets_path.parent().expect("assets parent")).expect("fixture dir");
        fs::write(
            &assets_path,
            "pub struct Assets {}\nimpl Assets { pub async fn load_texture(&self) {} }\n",
        )
        .expect("assets fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ASSET-API",
            "src/assets.rs",
            &[
                "pub async fn load_texture",
                "color_space: TextureColorSpace",
                "Result<TextureHandle, AssetError>",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ASSET-API" && finding.message.contains("color_space")
            }),
            "doctor must reject assets.rs that drops the explicit color_space parameter: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_prepare_assets_missing_collect_call_regression() {
        // ARCH-PREPARE-ASSETS: src/render.rs must route prepare_with_assets through
        // prepare::collect_prepared_primitives so the prepare phase stays the single
        // place that owns asset-aware primitive collection.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/prepare-assets-stub");
        let render_path = fixture_root.join("src/render.rs");
        fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
        fs::write(
            &render_path,
            "pub struct Renderer {}\nimpl Renderer { pub fn prepare_with_assets(&self) {} }\n",
        )
        .expect("render fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-PREPARE-ASSETS",
            "src/render.rs",
            &[
                "pub fn prepare_with_assets",
                "prepare::collect_prepared_primitives",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-PREPARE-ASSETS"
                    && finding.message.contains("collect_prepared_primitives")
            }),
            "doctor must reject render.rs that drops the prepare::collect_prepared_primitives \
             routing: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_visual_browser_m6_missing_probe_exports_regression() {
        // VISUAL-BROWSER-M6: src/browser_probe.rs must expose the wasm_bindgen probe
        // entry points (m6Render*Probe) plus the Renderer::from_surface_async +
        // prepare_with_assets + Renderer::render shape that distinguishes Rust/WASM
        // probe proof from JavaScript-only smoke tests.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/visual-browser-m6-stub");
        let probe_path = fixture_root.join("src/browser_probe.rs");
        fs::create_dir_all(probe_path.parent().expect("browser probe parent"))
            .expect("fixture dir");
        fs::write(
            &probe_path,
            "//! Stub browser probe.\npub fn m6_passthrough() {}\n",
        )
        .expect("browser probe fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "VISUAL-BROWSER-M6",
            "src/browser_probe.rs",
            &[
                "m6RenderWebgl2Probe",
                "m6RenderWebgpuProbe",
                "m6RenderWorkflowProbe",
                "Renderer::from_surface_async",
                "scena.m6.browser_renderer_probe.v1",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "VISUAL-BROWSER-M6"
                    && finding.message.contains("from_surface_async")
            }),
            "doctor must reject browser_probe.rs that drops the Rust/WASM Renderer \
             attached-canvas contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_environment_lifecycle_missing_handle_regression() {
        // ARCH-ENVIRONMENT-LIFECYCLE (handle): src/render.rs must store the bound
        // EnvironmentHandle alongside the revision counter so reload propagation
        // stays observable. The earlier batch covered the revision counter; this
        // fixture ensures the typed-handle field cannot disappear silently.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root =
            root.join("target/xtask-doctor-regressions/environment-lifecycle-handle-stub");
        let render_path = fixture_root.join("src/render.rs");
        fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
        fs::write(
            &render_path,
            "pub struct Renderer { pub environment_revision: u64 }\n",
        )
        .expect("render fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-ENVIRONMENT-LIFECYCLE",
            "src/render.rs",
            &["environment: Option<EnvironmentHandle>"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-ENVIRONMENT-LIFECYCLE"
                    && finding.message.contains("EnvironmentHandle")
            }),
            "doctor must reject Renderer types that drop the typed EnvironmentHandle \
             field: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_test_first_agents_governance_regression() {
        // TEST-FIRST-AGENTS: AGENTS.md must keep the Unit-Test-First Rule + the
        // "fail for the expected reason" + the "name the test-first proof" governance
        // text so contributors do not regress to write-then-test.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/test-first-agents-stub");
        let agents_path = fixture_root.join("AGENTS.md");
        fs::create_dir_all(fixture_root.as_path()).expect("fixture dir");
        fs::write(&agents_path, "# AGENTS\n\nNo test-first rule here.\n").expect("agents fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "TEST-FIRST-AGENTS",
            "AGENTS.md",
            &[
                "## Unit Test First Rule",
                "Run the focused test and confirm it fails for the expected reason",
                "Do not mark a checklist implementation item complete without naming the test-first proof",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "TEST-FIRST-AGENTS"
                    && finding.message.contains("Unit Test First Rule")
            }),
            "doctor must reject AGENTS.md that drops the unit-test-first governance \
             contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_default_environment_manifest_missing_field_regression() {
        // VISUAL-DEFAULT-ENV: tests/assets/environment/default-environment.toml must
        // declare name = "neutral-studio" (and the rest of the manifest contract).
        // A stub without the canonical name regresses the rule.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/default-environment-stub");
        let manifest_path = fixture_root.join("tests/assets/environment/default-environment.toml");
        fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture dir");
        fs::write(&manifest_path, "# placeholder default environment\n").expect("manifest fixture");
        let mut findings = Vec::new();

        check_default_environment_manifest(&fixture_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule == "VISUAL-DEFAULT-ENV"),
            "doctor must reject the default-environment manifest when its required \
             fields are missing: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_m2_leak_stats_missing_counters_regression() {
        // ARCH-M2-LEAK-STATS: tests/m2_lighting_depth_clipping.rs must keep the
        // resource-lifetime baseline test that watches environment cubemaps,
        // shadow maps, depth pre-pass counters, and pending destructions.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/m2-leak-stats-stub");
        let m2_path = fixture_root.join("tests/m2_lighting_depth_clipping.rs");
        fs::create_dir_all(m2_path.parent().expect("m2 parent")).expect("fixture dir");
        fs::write(&m2_path, "// no leak baseline test here\n").expect("m2 fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-M2-LEAK-STATS",
            "tests/m2_lighting_depth_clipping.rs",
            &[
                "m2_resource_counters_return_to_baseline_after_empty_prepare",
                "environment_cubemaps",
                "shadow_maps",
                "depth_prepass_passes",
                "released.pending_destructions, baseline.pending_destructions",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-M2-LEAK-STATS"
                    && finding.message.contains("environment_cubemaps")
            }),
            "doctor must reject m2 lighting test that drops the resource-lifetime \
             counter contract: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_camera_depth_missing_module_export_regression() {
        // ARCH-CAMERA-DEPTH (module): src/scene.rs must keep the public
        // `mod camera;` declaration plus the `pub use camera::{Camera, ...}`
        // re-export so the typed cameras stay accessible without leaking the
        // module path.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/camera-depth-module-stub");
        let scene_path = fixture_root.join("src/scene.rs");
        fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
        fs::write(&scene_path, "pub struct Scene {}\n").expect("scene fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-CAMERA-DEPTH",
            "src/scene.rs",
            &[
                "mod camera;",
                "pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera}",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-CAMERA-DEPTH" && finding.message.contains("pub use camera")
            }),
            "doctor must reject scene.rs that drops the camera module re-export: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_module_boundaries_missing_renderer_no_fetch_clause_regression() {
        // ARCH-MODULES: module-boundaries.md must keep the "no hidden asset fetch,
        // shader compile, or first-time GPU upload inside render()" clause so the
        // module ownership rules stay anchored in the spec.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/module-boundaries-stub");
        let module_path = fixture_root.join("docs/specs/module-boundaries.md");
        fs::create_dir_all(module_path.parent().expect("module-boundaries parent"))
            .expect("fixture dir");
        fs::write(
            &module_path,
            "# Module Boundaries\n\nNo render-fetch clause here.\n",
        )
        .expect("module-boundaries fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-MODULES",
            "docs/specs/module-boundaries.md",
            &[
                "`scene`",
                "`assets`",
                "`render`",
                "No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-MODULES" && finding.message.contains("No hidden asset fetch")
            }),
            "doctor must reject module-boundaries.md that drops the render-no-fetch \
             clause: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_render_alpha_capabilities_field_regression() {
        // ARCH-RENDER-ALPHA (capability field): capabilities.rs must keep
        // pub alpha_pipeline: AlphaPipelineStatus on Capabilities so backends can
        // structurally distinguish LinearSourceOver from BackendPassthrough.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/render-alpha-field-stub");
        let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
        fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
            .expect("fixture dir");
        fs::write(
            &capabilities_path,
            "pub enum AlphaPipelineStatus { LinearSourceOver, BackendPassthrough }\npub struct Capabilities {}\n",
        )
        .expect("capabilities fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-ALPHA",
            "src/diagnostics/capabilities.rs",
            &["pub alpha_pipeline: AlphaPipelineStatus"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-ALPHA"
                    && finding.message.contains("pub alpha_pipeline")
            }),
            "doctor must reject Capabilities that drops the alpha_pipeline field: \
             {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_render_alpha_missing_linear_frame_path_regression() {
        // ARCH-RENDER-ALPHA (CPU path): src/render.rs must keep the
        // linear_frame: Option<Vec<Color>> field plus the cpu::clear_cpu and
        // cpu::draw_primitive_cpu calls so CPU-rasterised alpha blending happens
        // in linear space before the output stage. A stub Renderer that drops
        // the field regresses the contract.
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root =
            root.join("target/xtask-doctor-regressions/render-alpha-linear-frame-stub");
        let render_path = fixture_root.join("src/render.rs");
        fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
        fs::write(&render_path, "pub struct Renderer { pub frame: Vec<u8> }\n")
            .expect("render fixture");
        let mut findings = Vec::new();

        require_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-ALPHA",
            "src/render.rs",
            &[
                "linear_frame: Option<Vec<Color>>",
                "cpu::clear_cpu",
                "cpu::draw_primitive_cpu",
            ],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-ALPHA" && finding.message.contains("linear_frame")
            }),
            "doctor must reject Renderer that drops the linear_frame CPU alpha-blend \
             path: {findings:?}",
        );
    }

    #[test]
    fn doctor_rejects_world_baked_prepare_regression() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/world-baked-prepare");
        let prepare_path = fixture_root.join("src/render/prepare.rs");
        fs::create_dir_all(prepare_path.parent().expect("prepare parent")).expect("fixture dir");
        fs::write(
            &prepare_path,
            "fn collect() { let _ = transform_primitive(primitive, transform, origin_shift); }\n",
        )
        .expect("prepare fixture");
        let mut findings = Vec::new();

        forbid_contains(
            &fixture_root,
            &mut findings,
            "ARCH-RENDER-WORLD-BAKE",
            "src/render/prepare.rs",
            &["transform_primitive(primitive, transform, origin_shift)"],
        );

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-WORLD-BAKE"
                    && finding.message.contains("transform_primitive")
            }),
            "doctor must reject prepare.rs that bakes per-renderable world transforms into \
             vertex positions instead of stamping them through prepared_primitive(...): \
             {findings:?}",
        );
    }

    #[test]
    fn environment_lifecycle_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_environment_lifecycle_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn equirectangular_hdr_environment_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_equirectangular_hdr_environment_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn environment_ibl_prepare_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_environment_ibl_prepare_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn scene_light_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_scene_light_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn direct_light_shading_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_direct_light_shading_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn directional_shadow_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_directional_shadow_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn shadow_map_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_shadow_map_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn depth_prepass_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_depth_prepass_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn reversed_z_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_reversed_z_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn webgl2_depth_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_webgl2_depth_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m2_leak_stats_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m2_leak_stats_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn camera_depth_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_camera_depth_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn origin_shift_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_origin_shift_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn clipping_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_clipping_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m3a_scene_import_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m3a_scene_import_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn default_environment_manifest_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_default_environment_manifest(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn visual_fixture_metadata_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_visual_fixture_metadata(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m2_visual_fixture_metadata_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m2_visual_fixture_metadata(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m1_browser_rendered_output_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m1_browser_rendered_output(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m2_browser_rendered_output_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m2_browser_rendered_output(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m6_browser_renderer_probe_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m6_browser_renderer_probe(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m9_release_metadata_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m9_ci_release_lanes(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn state_of_art_checklist_links_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_state_of_art_checklist_links(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m9_release_artifact_uploads_fail_closed() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))
            .expect("CI workflow must be readable");

        assert!(
            !workflow.contains("if-no-files-found: ignore"),
            "release artifact uploads must fail when required evidence is missing"
        );
        for artifact_name in [
            "linux-native-vulkan-gate-artifacts",
            "linux-browser-webgl2-gate-artifacts",
            "linux-browser-webgpu-gate-artifacts",
            "macos-metal-gate-artifacts",
            "windows-dx12-gate-artifacts",
        ] {
            assert!(
                workflow.contains(artifact_name),
                "missing release artifact upload {artifact_name}"
            );
        }
    }

    #[test]
    fn release_readiness_blocks_open_release_deferrals() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_release_readiness(&root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("ADR-0005 still records blocking")),
            "open ADR-0005 deferrals must block release readiness"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("m10-threejs-replacement-acceptance.md")),
            "open M10 checklist gates must block release readiness"
        );
    }

    #[test]
    fn release_readiness_reports_missing_downloaded_artifacts() {
        let mut findings = Vec::new();

        check_release_artifact_bundle(
            Path::new("target/xtask-release-readiness-test/missing"),
            &mut findings,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("missing release artifact root")),
            "downloaded release artifact root must be required when configured"
        );
    }

    #[test]
    fn release_readiness_rejects_unavailable_browser_artifact() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/unavailable-browser");
        let artifact_dir = artifact_root.join("browser");
        fs::create_dir_all(&artifact_dir).expect("test artifact dir");
        fs::write(
            artifact_dir.join("m6-rust-wasm-renderer-probe.json"),
            r#"{"status":"unavailable"}"#,
        )
        .expect("test artifact write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("does not have status 'passed'")),
            "release readiness must reject unavailable browser proof artifacts"
        );
    }

    #[test]
    fn release_readiness_rejects_command_recorded_release_lane_artifact() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/command-recorded");
        let lane_dir = artifact_root.join("release-lanes");
        fs::create_dir_all(&lane_dir).expect("lane artifact dir");
        fs::write(
            lane_dir.join("linux-native-vulkan.json"),
            r#"{"schema":"scena.release_lane.v1","status":"command-recorded"}"#,
        )
        .expect("lane artifact write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("only records a command") }),
            "release readiness must reject command-recorded lane artifacts: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_release_lane_artifact_without_measured_command_duration() {
        let artifact = json!({
            "schema": "scena.release_lane.v1",
            "status": "passed",
            "command_records": [
                {
                    "command": "cargo test --test m9_platform_release",
                    "status": "artifact-evidence-present",
                    "duration_ms": null,
                    "failure_log_path": "target/gate-artifacts/release-lanes/linux-native-vulkan.log",
                    "artifact_checksums": []
                }
            ]
        });
        let mut findings = Vec::new();

        require_release_lane_artifact_evidence(
            &artifact,
            "release-lanes/linux-native-vulkan.json",
            &mut findings,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("measured command duration")),
            "release readiness must reject lane artifacts without measured durations: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_release_lane_artifact_with_failed_command_record() {
        let artifact = json!({
            "schema": "scena.release_lane.v1",
            "status": "passed",
            "command_records": [
                {
                    "command": "cargo test --test m9_platform_release",
                    "status": "failed",
                    "duration_ms": 42,
                    "failure_log_path": "target/gate-artifacts/release-lanes/linux-native-vulkan.log",
                    "failure_log_sha256": "fnv1a64:0000000000000001",
                    "artifact_checksums": []
                }
            ]
        });
        let mut findings = Vec::new();

        require_release_lane_artifact_evidence(
            &artifact,
            "release-lanes/linux-native-vulkan.json",
            &mut findings,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("did not pass")),
            "release readiness must reject failed lane command records: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_stale_timestamped_artifact() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/stale-artifact");
        let platform_dir = artifact_root.join("m9-platform");
        fs::create_dir_all(&platform_dir).expect("platform artifact dir");
        fs::write(
            platform_dir.join("m9-capability-matrix.json"),
            r#"{
                "schema": "scena.m9.capability_matrix.v1",
                "status": "passed",
                "timestamp_unix_seconds": 1,
                "lanes": []
            }"#,
        )
        .expect("capability matrix write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("is stale") }),
            "release readiness must reject stale timestamped artifacts: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_constant_ppm_visual_artifact() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/constant-ppm");
        let lane_dir = artifact_root.join("m9-platform/linux-native-vulkan");
        fs::create_dir_all(&lane_dir).expect("lane artifact dir");
        fs::write(
            lane_dir.join("default-scene.ppm"),
            b"P6\n2 2\n255\n\x20\x40\x60\x20\x40\x60\x20\x40\x60\x20\x40\x60",
        )
        .expect("constant ppm write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("constant-color") }),
            "release readiness must reject constant-color visual artifacts: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_factory_contract_capability_rows() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/factory-capability");
        let platform_dir = artifact_root.join("m9-platform");
        fs::create_dir_all(&platform_dir).expect("platform artifact dir");
        fs::write(
            platform_dir.join("m9-capability-matrix.json"),
            format!(
                r#"{{
                    "schema": "scena.m9.capability_matrix.v1",
                    "status": "passed",
                    "timestamp_unix_seconds": {},
                    "lanes": [
                        {{
                            "lane": "macos-metal",
                            "measurement_source": "factory-contract"
                        }}
                    ]
                }}"#,
                current_unix_seconds()
            ),
        )
        .expect("capability matrix write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("factory-contract rows") }),
            "release readiness must reject factory-contract capability rows: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_missing_lane_capability_rows() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/missing-capability");
        let platform_dir = artifact_root.join("m9-platform");
        fs::create_dir_all(&platform_dir).expect("platform artifact dir");
        fs::write(
            platform_dir.join("m9-capability-matrix.json"),
            format!(
                r#"{{
                    "schema": "scena.m9.capability_matrix.v1",
                    "status": "passed",
                    "timestamp_unix_seconds": {},
                    "lanes": [
                        {{
                            "lane": "macos-metal",
                            "measurement_source": "missing-lane-artifact"
                        }}
                    ]
                }}"#,
                current_unix_seconds()
            ),
        )
        .expect("capability matrix write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("missing-lane-artifact rows") }),
            "release readiness must reject missing-lane capability rows: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_benchmark_artifact_without_stored_baseline_comparison() {
        let artifact = json!({
            "schema": "scena.m9.benchmarks.v1",
            "lane": "linux-native-vulkan",
            "rows": [
                {
                    "scene": "static-viewer",
                    "backend": "Headless",
                    "sample_count": 100,
                    "p95_frame_ms": 12.0
                }
            ]
        });
        let mut findings = Vec::new();

        require_benchmark_baseline_comparison(
            &artifact,
            "m9-platform/m9-benchmarks.json",
            &mut findings,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("stored baseline comparison")),
            "release readiness must reject benchmark artifacts without stored-baseline comparison: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_benchmark_regression_against_stored_baseline() {
        let artifact = json!({
            "schema": "scena.m9.benchmarks.v1",
            "lane": "linux-native-vulkan",
            "baseline_comparison": {
                "status": "failed",
                "baseline_path": "docs/benchmarks/m9-baselines.json",
                "baseline_sha256": "fnv1a64:0000000000000001",
                "metric": "p95_frame_ms"
            },
            "rows": [
                {
                    "scene": "static-viewer",
                    "backend": "Headless",
                    "sample_count": 100,
                    "p95_frame_ms": 12.0,
                    "baseline_comparison": {
                        "status": "failed",
                        "baseline_p95_frame_ms": 10.0,
                        "allowed_regression_percent": 5.0,
                        "regression_percent": 20.0
                    }
                }
            ]
        });
        let mut findings = Vec::new();

        require_benchmark_baseline_comparison(
            &artifact,
            "m9-platform/m9-benchmarks.json",
            &mut findings,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("benchmark regression")),
            "release readiness must reject statistically significant p95 benchmark regressions: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_accepts_benchmark_artifact_with_passed_baseline_comparison() {
        let artifact = json!({
            "schema": "scena.m9.benchmarks.v1",
            "lane": "linux-native-vulkan",
            "baseline_comparison": {
                "status": "passed",
                "baseline_path": "docs/benchmarks/m9-baselines.json",
                "baseline_sha256": "fnv1a64:0000000000000001",
                "metric": "p95_frame_ms"
            },
            "rows": [
                {
                    "scene": "static-viewer",
                    "backend": "Headless",
                    "sample_count": 100,
                    "p95_frame_ms": 10.2,
                    "baseline_comparison": {
                        "status": "passed",
                        "baseline_p95_frame_ms": 10.0,
                        "allowed_regression_percent": 5.0,
                        "regression_percent": 2.0
                    }
                },
                {
                    "scene": "headless-4k",
                    "status": "deferred-to-dedicated-performance-lane",
                    "sample_count": 0,
                    "baseline_comparison": {
                        "status": "deferred"
                    }
                }
            ]
        });
        let mut findings = Vec::new();

        require_benchmark_baseline_comparison(
            &artifact,
            "m9-platform/m9-benchmarks.json",
            &mut findings,
        );

        assert_eq!(
            findings,
            Vec::new(),
            "passed stored-baseline benchmark comparison should not block release readiness"
        );
    }

    #[test]
    fn release_readiness_rejects_rendered_output_without_screenshot_metadata() {
        let artifact = json!({
            "schema": "scena.m9.platform_render.v1",
            "default_scene": {
                "backend": "Headless",
                "screenshot": "target/default.ppm",
                "width": 96,
                "height": 64
            },
            "static_gltf": {
                "backend": "Headless",
                "screenshot": "target/static.ppm",
                "width": 96,
                "height": 64,
                "asset_provenance": { "path": "tests/assets/gltf/non_ndc_camera_scene.gltf" }
            }
        });
        let mut findings = Vec::new();

        require_rendered_output_screenshot_metadata(
            &artifact,
            "m9-platform/headless-cpu/rendered-output.json",
            &mut findings,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("screenshot metadata")),
            "release readiness must reject rendered-output artifacts without full screenshot metadata: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_accepts_rendered_output_with_screenshot_metadata() {
        let screenshot = json!({
            "backend": "Headless",
            "adapter": { "available": false },
            "renderer_settings": { "width": 96, "height": 64 },
            "color_management": { "output_encoding": "srgb8-after-aces" },
            "tolerance": { "policy": "native-rendered-output-smoke" },
            "screenshot": "target/default.ppm",
            "width": 96,
            "height": 64
        });
        let artifact = json!({
            "schema": "scena.m9.platform_render.v1",
            "default_scene": screenshot,
            "static_gltf": {
                "backend": "Headless",
                "adapter": { "available": false },
                "renderer_settings": { "width": 96, "height": 64 },
                "color_management": { "output_encoding": "srgb8-after-aces" },
                "tolerance": { "policy": "native-rendered-output-smoke" },
                "screenshot": "target/static.ppm",
                "width": 96,
                "height": 64,
                "asset_provenance": {
                    "path": "tests/assets/gltf/non_ndc_camera_scene.gltf",
                    "hash": "fnv1a64:0000000000000001"
                }
            },
            "pbr_lights": {
                "lights": [
                    {
                        "light_type": "directional",
                        "backend": "Headless",
                        "adapter": { "available": false },
                        "renderer_settings": { "width": 96, "height": 64 },
                        "color_management": { "output_encoding": "srgb8-after-aces" },
                        "tolerance": { "policy": "native-rendered-output-smoke" },
                        "screenshot": "target/light.ppm",
                        "width": 96,
                        "height": 64
                    }
                ]
            }
        });
        let mut findings = Vec::new();

        require_rendered_output_screenshot_metadata(
            &artifact,
            "m9-platform/headless-cpu/rendered-output.json",
            &mut findings,
        );

        assert_eq!(
            findings,
            Vec::new(),
            "complete rendered-output screenshot metadata should satisfy release readiness"
        );
    }

    #[test]
    fn release_readiness_rejects_cpu_fallback_native_render_artifact() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact_root = root.join("target/xtask-release-readiness-test/cpu-fallback-render");
        let lane_dir = artifact_root.join("m9-platform/linux-native-vulkan");
        fs::create_dir_all(&lane_dir).expect("lane artifact dir");
        fs::write(
            lane_dir.join("rendered-output.json"),
            r#"{
                "schema": "scena.m9.platform_render.v1",
                "gpu_proof": false,
                "host_gpu_available": false,
                "fallback_policy": "cpu fallback is diagnostic only and never satisfies GPU rendered-output claims",
                "static_gltf": {
                    "proof_class": "cpu-fallback-camera-framed-non-ndc",
                    "production_claim": false,
                    "gpu_proof": false
                }
            }"#,
        )
        .expect("fallback rendered-output write");
        let mut findings = Vec::new();

        check_release_artifact_bundle(&artifact_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("does not prove GPU output") }),
            "release readiness must reject native GPU artifacts that are CPU fallback only: {findings:?}",
        );
    }

    #[test]
    fn release_readiness_rejects_native_render_artifact_without_pbr_light_proof() {
        let artifact = json!({
            "schema": "scena.m9.platform_render.v1",
            "gpu_proof": true,
            "host_gpu_available": true,
            "static_gltf": {
                "proof_class": "camera-framed-non-ndc",
                "production_claim": true,
                "gpu_proof": true
            }
        });

        assert!(
            !native_gpu_render_proof_passes(&artifact),
            "native release proof must include PBR punctual-light rendered-output evidence"
        );
    }

    #[test]
    fn release_readiness_accepts_native_render_artifact_with_pbr_light_proof() {
        let artifact = json!({
            "schema": "scena.m9.platform_render.v1",
            "gpu_proof": true,
            "host_gpu_available": true,
            "static_gltf": {
                "proof_class": "camera-framed-non-ndc",
                "production_claim": true,
                "gpu_proof": true
            },
            "pbr_lights": {
                "proof_class": "native-pbr-punctual-light",
                "production_claim": true,
                "gpu_proof": true,
                "lights": [
                    {
                        "light_type": "directional",
                        "production_claim": true,
                        "gpu_proof": true,
                        "color_assertion_passed": true,
                        "nonblack_pixels": 1200
                    },
                    {
                        "light_type": "point",
                        "production_claim": true,
                        "gpu_proof": true,
                        "color_assertion_passed": true,
                        "nonblack_pixels": 1200
                    },
                    {
                        "light_type": "spot",
                        "production_claim": true,
                        "gpu_proof": true,
                        "color_assertion_passed": true,
                        "nonblack_pixels": 1200
                    }
                ]
            }
        });

        assert!(
            native_gpu_render_proof_passes(&artifact),
            "native release proof should pass when camera-framed glTF and all PBR punctual-light proofs pass"
        );
    }

    #[test]
    fn release_lane_artifact_uses_required_file_evidence_not_command_recorded_status() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let artifact = release_lane_artifact(&root, "linux-native-vulkan")
            .expect("release lane artifact builds");

        assert_ne!(artifact["status"], "command-recorded");
        assert!(
            artifact["required_artifacts"]
                .as_array()
                .expect("required artifacts array")
                .iter()
                .any(|entry| entry["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("rendered-output.json"))),
            "release-lane artifact must name required proof files instead of only recording a command",
        );
    }

    #[test]
    fn release_lane_artifact_consumes_measured_command_records() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-release-lane-command-record-test");
        let lane = "linux-native-vulkan";
        let command_dir = fixture_root.join("target/gate-artifacts/release-lanes");
        fs::create_dir_all(&command_dir).expect("command record dir");
        fs::write(
            command_dir.join(format!("{lane}.log")),
            b"focused lane command output\n",
        )
        .expect("failure log");
        let log_sha = sha256_hex(&command_dir.join(format!("{lane}.log"))).expect("log sha");
        fs::write(
            command_dir.join(format!("{lane}.commands.jsonl")),
            format!(
                r#"{{"command":"cargo test --test m9_platform_release","status":"passed","duration_ms":1234,"duration_source":"ci-wrapper","failure_log_path":"target/gate-artifacts/release-lanes/{lane}.log","failure_log_sha256":"{log_sha}"}}"#
            ),
        )
        .expect("command record jsonl");

        let artifact = release_lane_artifact(&fixture_root, lane)
            .expect("release lane artifact builds with measured command records");
        let records = artifact["command_records"]
            .as_array()
            .expect("command records");
        let measured = records
            .iter()
            .find(|record| record["command"] == "cargo test --test m9_platform_release")
            .expect("measured test command record");

        assert_eq!(measured["duration_ms"], 1234);
        assert_eq!(measured["duration_source"], "ci-wrapper");
        assert_eq!(measured["failure_log_sha256"], log_sha);
        assert_eq!(
            measured["measurement_source"],
            "target/gate-artifacts/release-lanes/linux-native-vulkan.commands.jsonl"
        );
    }

    #[test]
    fn release_lane_artifact_status_requires_native_gpu_content_proof() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-release-lane-content-test");
        let lane = "linux-native-vulkan";
        let lane_dir = fixture_root.join(format!("target/gate-artifacts/m9-platform/{lane}"));
        fs::create_dir_all(&lane_dir).expect("lane artifact dir");
        fs::write(
            lane_dir.join("rendered-output.json"),
            r#"{
                "schema": "scena.m9.platform_render.v1",
                "gpu_proof": false,
                "host_gpu_available": false,
                "static_gltf": {
                    "proof_class": "cpu-fallback-camera-framed-non-ndc",
                    "production_claim": false,
                    "gpu_proof": false
                }
            }"#,
        )
        .expect("rendered output artifact");
        for file in [
            "capabilities.json",
            "surface-context-loss.json",
            "default-scene.ppm",
            "static-gltf.ppm",
        ] {
            fs::write(lane_dir.join(file), b"fixture").expect("lane file");
        }
        let platform_dir = fixture_root.join("target/gate-artifacts/m9-platform");
        fs::write(platform_dir.join("m9-benchmarks.json"), b"{}").expect("benchmarks");

        let artifact =
            release_lane_artifact(&fixture_root, lane).expect("release lane artifact builds");

        assert_eq!(artifact["content_ok"], false);
        assert_eq!(artifact["status"], "incomplete");
    }

    #[test]
    fn release_lane_artifact_supports_separate_headless_cpu_proof_lane() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-headless-cpu-lane-test");
        let lane_dir = fixture_root.join("target/gate-artifacts/m9-platform/headless-cpu");
        fs::create_dir_all(&lane_dir).expect("headless lane artifact dir");
        fs::write(
            lane_dir.join("rendered-output.json"),
            r#"{
                "schema": "scena.m9.platform_render.v1",
                "lane": "headless-cpu",
                "backend": "Headless",
                "headless_cpu_proof": true,
                "static_gltf": {
                    "proof_class": "cpu-camera-framed-non-ndc",
                    "production_claim": true,
                    "nonblack_pixels": 42
                }
            }"#,
        )
        .expect("headless rendered-output write");
        for file in ["capabilities.json", "default-scene.ppm", "static-gltf.ppm"] {
            fs::write(lane_dir.join(file), b"fixture").expect("headless lane file");
        }
        let platform_dir = fixture_root.join("target/gate-artifacts/m9-platform");
        fs::write(platform_dir.join("m9-benchmarks.json"), b"{}").expect("benchmarks");

        let artifact = release_lane_artifact(&fixture_root, "headless-cpu")
            .expect("headless release lane artifact builds");

        assert_eq!(artifact["status"], "passed");
        assert_eq!(artifact["content_ok"], true);
        assert!(
            artifact["required_artifacts"]
                .as_array()
                .expect("required artifacts array")
                .iter()
                .any(|entry| entry["path"]
                    .as_str()
                    .is_some_and(|path| path.contains("headless-cpu/rendered-output.json"))),
            "headless CPU lane must have its own rendered-output artifact",
        );
    }

    #[test]
    fn m8_gltf_asset_matrix_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_gltf_asset_matrix_contract(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m8_gltf_asset_matrix_rejects_unhashed_sample_files() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-gltf-asset-matrix-hash-test");
        let matrix_dir = fixture_root.join("docs/assets");
        let sample_dir = fixture_root.join("tests/assets/gltf/khronos/Sample");
        fs::create_dir_all(&matrix_dir).expect("matrix dir");
        fs::create_dir_all(&sample_dir).expect("sample dir");
        fs::write(
            sample_dir.join("Sample.gltf"),
            br#"{"asset":{"version":"2.0"}}"#,
        )
        .expect("sample glTF");
        fs::write(
            matrix_dir.join("gltf-asset-matrix.md"),
            r#"# glTF Asset Matrix

This matrix catches fail with a structured error and silent fallback rows.

| Asset/Fixture | Source/License | Features | Expected Result | Expected Diagnostics | Rendered Output Reference | Evidence |
|---|---|---|---|---|---|---|
| Khronos `Sample` | Khronos sample / upstream sample license | mesh | pass | none expected | deferred structured non-visual proof | `tests/m8_assets_materials_ecosystem.rs` |
"#,
        )
        .expect("matrix write");
        fs::write(
            fixture_root.join("tests/assets/gltf/khronos/manifest.toml"),
            r#"[source]
repository = "https://github.com/KhronosGroup/glTF-Sample-Assets"
commit = "sample"
license_reference = "Upstream LICENSES directory in glTF-Sample-Assets"

[[asset]]
name = "Sample"
path = "Sample/Sample.gltf"
contract = "hash guard"
"#,
        )
        .expect("manifest write");
        let mut findings = Vec::new();

        check_gltf_asset_matrix_contract(&fixture_root, &mut findings);

        assert!(
            findings.iter().any(|finding| finding.message.contains(
                "must record a SHA-256 hash for tests/assets/gltf/khronos/Sample/Sample.gltf"
            )),
            "sample assets must not be accepted without source hashes: {findings:?}",
        );
    }

    #[test]
    fn m8_assets_materials_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m8_assets_materials_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn binary_render_asset_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_binary_render_asset_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn binary_render_asset_contracts_reject_text_fixtures_with_binary_extensions() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-binary-asset-contract-test");
        let fixture_dir = fixture_root.join("tests/assets/environment/generated");
        fs::create_dir_all(&fixture_dir).expect("fixture dir");
        fs::write(
            fixture_dir.join("fake.ktx2"),
            b"SCENA_CUBEMAP_V1\nencoding = rgba16f-text-fixture\n",
        )
        .expect("fixture write");
        let mut findings = Vec::new();

        check_binary_render_asset_contracts(&fixture_root, &mut findings);

        assert!(
            findings.iter().any(|finding| {
                finding.rule == "BINARY-ASSET-TRUTH-P9"
                    && finding.message.contains("fake.ktx2")
                    && finding.message.contains("text fixture data")
            }),
            "text fixtures must not be allowed to masquerade as binary render assets: {findings:?}",
        );
    }

    #[test]
    fn m7_ergonomics_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m7_ergonomics_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn public_fields_in_struct_detects_material_desc_visibility_regressions() {
        let source = r#"
            pub struct MaterialDesc {
                kind: MaterialKind,
                pub base_color: Color,
                pub(crate) roughness_factor: f32,
            }
        "#;

        assert_eq!(
            public_fields_in_struct(source, "MaterialDesc"),
            vec!["pub base_color: Color", "pub(crate) roughness_factor: f32"]
        );
    }
}
