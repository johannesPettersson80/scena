use crate::app::prelude::*;
use crate::app::release::round_e_material_results::q02_material_result_passes;

mod readiness;
pub(crate) use readiness::{
    check_release_readiness, check_release_readiness_adr, check_release_readiness_checklists,
    run_claim_audit,
};

pub(crate) fn run_release_lane_artifact(lane: &str) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("RELEASE-LANE-ROOT", message)])?;
    if lane == "macos-metal" {
        finalize_waterbottle_gpu_result(&root)
            .map_err(|message| vec![Finding::new("RELEASE-LANE", message)])?;
    } else if lane == "headless-cpu" {
        finalize_waterbottle_cpu_result(&root)
            .map_err(|message| vec![Finding::new("RELEASE-LANE", message)])?;
    }
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
    if env::var("SCENA_REQUIRE_PARITY").as_deref() == Ok("1")
        && matches!(lane, "linux-native-vulkan" | "linux-webgpu-chromium")
        && artifact.get("status").and_then(Value::as_str) != Some("passed")
    {
        return Err(vec![Finding::new(
            "RELEASE-LANE-REQUIRED-PARITY",
            format!(
                "required GPU lane {lane} is incomplete; see {}",
                artifact_path.display()
            ),
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
    let evidence_class = release_lane_evidence_class(lane);
    let content_ok = if evidence_class == "hardware-release" {
        release_lane_content_ok(root, lane)?
    } else {
        release_lane_content_ok_for_class(root, lane, evidence_class)?
    };
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
    let generated_at = current_unix_seconds();
    let source_checksums = release_lane_source_checksums(root, &evidence)?;
    Ok(json!({
        "schema": "scena.release_lane.v1",
        "lane": lane,
        "os": os,
        "backend": backend,
        "evidence_class": evidence_class,
        "rustc": "1.93.1",
        "producer": format!("cargo run -p xtask -- release-lane-artifact {lane}"),
        "generated_at_unix_seconds": generated_at,
        "timestamp_unix_seconds": generated_at,
        "commit": release_artifact_commit_label(root),
        "commit_sha": release_artifact_commit_label(root),
        "source_checksums": source_checksums,
        "status": status,
        "required_artifacts": evidence,
        "content_ok": content_ok,
        "commands_ok": commands_ok,
        "commands": commands,
        "command_records": command_records,
        "note": "Lane status is passed only when the required local gate artifacts exist, are checksummed, and native GPU rendered-output proof is not CPU fallback. CI may populate command duration and failure-log fields through the same command_records schema."
    }))
}

fn release_lane_source_checksums(root: &Path, evidence: &[Value]) -> Result<Vec<Value>, String> {
    let mut checksums = evidence
        .iter()
        .filter(|entry| entry.get("exists").and_then(Value::as_bool) == Some(true))
        .filter_map(|entry| {
            Some(json!({
                "path": entry.get("path")?.clone(),
                "sha256": entry.get("sha256")?.clone(),
            }))
        })
        .collect::<Vec<_>>();
    for relative in [
        "Cargo.lock",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ] {
        let path = root.join(relative);
        checksums.push(json!({
            "path": relative,
            "sha256": sha256_hex(&path).map_err(|error| {
                format!("failed to hash release-lane source {relative}: {error}")
            })?,
        }));
    }
    Ok(checksums)
}

pub(crate) fn release_lane_content_ok(root: &Path, lane: &str) -> Result<bool, String> {
    release_lane_content_ok_for_class(root, lane, "hardware-release")
}

fn release_lane_content_ok_for_class(
    root: &Path,
    lane: &str,
    evidence_class: &str,
) -> Result<bool, String> {
    if lane == "headless-cpu" {
        let path = root.join("target/gate-artifacts/m9-platform/headless-cpu/rendered-output.json");
        if !path.is_file() {
            return Ok(false);
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let value = serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        let q01_path = root.join("target/gate-artifacts/q01-waterbottle-cpu/result.json");
        if !q01_path.is_file() {
            return Ok(false);
        }
        let q01_text = fs::read_to_string(&q01_path)
            .map_err(|error| format!("failed to read {}: {error}", q01_path.display()))?;
        let q01 = serde_json::from_str::<Value>(&q01_text)
            .map_err(|error| format!("failed to parse {}: {error}", q01_path.display()))?;
        let q11_path = root.join("target/gate-artifacts/q11-reference-stability/linux-x86_64.json");
        let q11 = fs::read_to_string(&q11_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        return Ok(headless_cpu_render_proof_passes(&value)
            && validate_waterbottle_cpu_result(&root.join("target/gate-artifacts"), &q01).is_ok()
            && q11.as_ref().is_some_and(|result| {
                validate_q11_reference_stability_result(result, "linux", "x86_64").is_ok()
            })
            && q02_material_result_passes(
                root,
                "target/gate-artifacts/round-e-cpu-material-proof.json",
                "scena.q02.round_e_cpu_material_proof.v1",
                "live-cpu-headless",
                "live-cpu-round-e-shared-threshold-evaluation",
                "target/gate-artifacts/round-e-cpu-material-proof/live-frame.png",
            )?);
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
        let (result, schema, surface, proof_class, live_frame) = match lane {
            "linux-webgl2-chromium" => (
                "target/gate-artifacts/round-e-cloudflare-material-proof.json",
                "scena.q02.round_e_webgl2_material_proof.v1",
                "live-webgl2-chromium",
                "round-e-cloudflare-material-proof",
                "target/gate-artifacts/round-e-cloudflare-material-proof/canvas.png",
            ),
            "linux-webgpu-chromium" => (
                "target/gate-artifacts/round-e-webgpu-material-proof/result.json",
                "scena.q02.round_e_webgpu_material_proof.v1",
                "live-webgpu-chromium",
                "required-live-webgpu-round-e-shared-threshold-evaluation",
                "target/gate-artifacts/round-e-webgpu-material-proof/live-frame.png",
            ),
            _ => unreachable!("browser lane was matched above"),
        };
        let browser_proof_passes = if evidence_class == "hardware-release" {
            browser_probe_release_proof_passes(&value, lane)
        } else {
            browser_probe_release_proof_passes_for_class(&value, lane, evidence_class)
        };
        return Ok(browser_proof_passes
            && q02_material_result_passes(
                root,
                result,
                schema,
                surface,
                proof_class,
                live_frame,
            )?);
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
    if !native_gpu_render_proof_passes(&value) {
        return Ok(false);
    }
    if lane == "linux-native-vulkan" {
        return Ok(true);
    }
    let (q11_suffix, q11_os, q11_arch) = if lane == "macos-metal" {
        ("macos-aarch64.json", "macos", "aarch64")
    } else {
        ("windows-x86_64.json", "windows", "x86_64")
    };
    let q11_path = root.join(format!(
        "target/gate-artifacts/q11-reference-stability/{q11_suffix}"
    ));
    let q11 = fs::read_to_string(&q11_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if !q11.as_ref().is_some_and(|result| {
        validate_q11_reference_stability_result(result, q11_os, q11_arch).is_ok()
    }) {
        return Ok(false);
    }
    if lane != "macos-metal" {
        return Ok(true);
    }
    let lifecycle_path =
        root.join("target/gate-artifacts/c09-gpu-resource-lifecycle/required-result.json");
    let lifecycle = fs::read_to_string(&lifecycle_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    Ok(lifecycle
        .as_ref()
        .is_some_and(required_gpu_resource_lifecycle_proof_passes))
}

pub(crate) fn release_lane_required_artifacts(lane: &str) -> Vec<String> {
    match lane {
        "headless-cpu" => [
            "target/gate-artifacts/m9-platform/headless-cpu/rendered-output.json".to_string(),
            "target/gate-artifacts/m9-platform/headless-cpu/capabilities.json".to_string(),
            "target/gate-artifacts/m9-platform/headless-cpu/default-scene.ppm".to_string(),
            "target/gate-artifacts/m9-platform/headless-cpu/static-gltf.ppm".to_string(),
            "target/gate-artifacts/m9-platform/m9-benchmarks.json".to_string(),
            "target/gate-artifacts/m9-platform/m9-benchmarks-feature-matrix.json".to_string(),
            "target/gate-artifacts/q01-waterbottle-cpu/live.png".to_string(),
            "target/gate-artifacts/q01-waterbottle-cpu/known_bad_flattened_chrome.png".to_string(),
            "target/gate-artifacts/q01-waterbottle-cpu/known_bad_wrong_material.png".to_string(),
            "target/gate-artifacts/q01-waterbottle-cpu/known_bad_wrong_camera.png".to_string(),
            "target/gate-artifacts/q01-waterbottle-cpu/result.json".to_string(),
            "target/gate-artifacts/q11-reference-stability/linux-x86_64.json".to_string(),
            "target/gate-artifacts/round-e-cpu-material-proof/live-frame.png".to_string(),
            "target/gate-artifacts/round-e-cpu-material-proof/live-cpu-frame.json".to_string(),
            "target/gate-artifacts/round-e-cpu-material-proof.json".to_string(),
        ]
        .into_iter()
        .collect(),
        "linux-native-vulkan" | "macos-metal" | "windows-dx12" => {
            let mut artifacts = [
                format!("target/gate-artifacts/m9-platform/{lane}/rendered-output.json"),
                format!("target/gate-artifacts/m9-platform/{lane}/capabilities.json"),
                format!("target/gate-artifacts/m9-platform/{lane}/surface-context-loss.json"),
                format!("target/gate-artifacts/m9-platform/{lane}/default-scene.ppm"),
                format!("target/gate-artifacts/m9-platform/{lane}/static-gltf.ppm"),
                format!("target/gate-artifacts/m9-platform/{lane}/pbr-directional-red.ppm"),
                format!("target/gate-artifacts/m9-platform/{lane}/pbr-point-green.ppm"),
                format!("target/gate-artifacts/m9-platform/{lane}/pbr-spot-blue.ppm"),
                "target/gate-artifacts/m9-platform/m9-benchmarks.json".to_string(),
                "target/gate-artifacts/m9-platform/m9-benchmarks-feature-matrix.json".to_string(),
            ]
            .into_iter()
            .collect::<Vec<_>>();
            if lane == "macos-metal" {
                artifacts.push(
                    "target/gate-artifacts/c09-gpu-resource-lifecycle/required-result.json"
                        .to_string(),
                );
                artifacts.extend([
                    "target/gate-artifacts/q07-antialiasing-effect/result.json".to_string(),
                    "target/gate-artifacts/q07-antialiasing-effect/none.ppm".to_string(),
                    "target/gate-artifacts/q07-antialiasing-effect/fxaa.ppm".to_string(),
                    "target/gate-artifacts/q07-antialiasing-effect/msaa4.ppm".to_string(),
                    "target/gate-artifacts/q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json".to_string(),
                    "target/gate-artifacts/q08-required-parity/close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json".to_string(),
                    "target/gate-artifacts/q08-required-parity/dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json".to_string(),
                    "target/gate-artifacts/q08-required-parity/z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json".to_string(),
                    "target/gate-artifacts/q08-required-parity/core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json".to_string(),
                    "target/gate-artifacts/q08-required-parity/pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json".to_string(),
                ]);
                artifacts.push(
                    "target/gate-artifacts/q11-reference-stability/macos-aarch64.json".to_string(),
                );
            } else if lane == "windows-dx12" {
                artifacts.push(
                    "target/gate-artifacts/q11-reference-stability/windows-x86_64.json".to_string(),
                );
            }
            artifacts
        }
        "linux-webgl2-chromium" => [
            "target/gate-artifacts/m6-rust-wasm-renderer-probe.json",
            "target/gate-artifacts/round-e-cloudflare-material-proof.json",
            "target/gate-artifacts/round-e-cloudflare-material-proof/canvas.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/matte.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/plastic.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/metal.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/rough_metal.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/chrome.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/brushed_steel.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/clearcoat_plastic.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/satin.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/leather.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/clear_glass.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/frosted_glass.png",
            "target/gate-artifacts/round-e-cloudflare-material-proof/rubber.png",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        "linux-webgpu-chromium" => [
            "target/gate-artifacts/m6-rust-wasm-renderer-probe.json",
            "target/gate-artifacts/round-e-webgpu-material-proof/live-frame.png",
            "target/gate-artifacts/round-e-webgpu-material-proof/result.json",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        "wasm32-unknown-unknown" => {
            vec!["target/gate-artifacts/m9-wasm-size.json".to_string()]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn release_lane_expected_commands(lane: &str) -> Vec<&'static str> {
    match lane {
        "headless-cpu" => vec![
            "cargo test --test m9_platform_release",
            "cargo test --test q01_waterbottle_cpu_reference q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact",
            "cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
            "cargo test --test examples_visual_proof q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact",
            "node scripts/evaluate_round_e_cpu_materials.cjs",
        ],
        "linux-native-vulkan" => vec![
            "cargo test --test m9_platform_release",
            "cargo check --examples --all-features",
        ],
        "windows-dx12" => vec![
            "cargo test --test m9_platform_release",
            "cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
            "cargo check --examples --all-features",
        ],
        "macos-metal" => vec![
            "cargo test --test m9_platform_release",
            "cargo check --examples --all-features",
            "cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
            "cargo test --test c09_gpu_resource_lifecycle required_hardware_gpu_resource_lifecycle_executes_complete_cycle -- --exact --nocapture",
            "cargo test --test q07_antialiasing_effect q07_required_native_antialiasing_modes_have_pixel_effect -- --exact",
            "cargo test --test transmission_parity physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep -- --exact",
            "cargo test --test c13_depth_clipping_parity close_camera_near_clip_matches_cpu_and_gpu_rendered_output -- --exact",
            "cargo test --test dynamic_transform_parity dynamic_transform_motion_matches_cpu_and_gpu_for_authored_animation_and_imports -- --exact",
            "cargo test --test dynamic_transform_parity z_up_imported_rotation_frame_matches_cpu_and_gpu_after_basis_conversion -- --exact",
            "cargo test --test pbr_brdf_parity core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep -- --exact",
            "cargo test --test pf08_texture_bake_parity pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu -- --exact",
        ],
        "linux-webgl2-chromium" => vec![
            "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe",
            "npm run browser:m6",
            "npm run cloudflare:materials -- http://127.0.0.1:18104/proof/?sample=material-presets",
        ],
        "linux-webgpu-chromium" => vec![
            "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe",
            "npm run browser:q02-materials",
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
