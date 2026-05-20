use crate::app::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssetGuidanceSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetGuidanceFinding {
    pub(crate) extension: String,
    pub(crate) required: bool,
    pub(crate) severity: AssetGuidanceSeverity,
    pub(crate) status: &'static str,
    pub(crate) message: String,
    pub(crate) fix: String,
}

pub(crate) fn run_asset_doctor(input: &str) -> Result<(), Vec<Finding>> {
    let root =
        repo_root().map_err(|message| vec![Finding::new("ASSET-VALIDATION-DOCTOR", message)])?;
    let input_path = Path::new(input);
    let path = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        root.join(input_path)
    };
    let report = asset_doctor_report(&path);
    let body = serde_json::to_string_pretty(&report.json)
        .unwrap_or_else(|error| format!(r#"{{"status":"failed","error":"{error}"}}"#));
    println!("{body}");

    if report.passed {
        Ok(())
    } else {
        Err(vec![Finding::new(
            "ASSET-VALIDATION-DOCTOR",
            report.failure_message,
        )])
    }
}

pub(crate) fn official_gltf_validator_args(path: &Path) -> Vec<String> {
    vec!["-o".to_string(), path.to_string_lossy().into_owned()]
}

pub(crate) fn scena_native_asset_guidance(
    path: &Path,
) -> Result<Vec<AssetGuidanceFinding>, String> {
    let document = load_gltf_json(path)?;
    Ok(scena_native_guidance_from_document(&document))
}

struct AssetDoctorReport {
    json: Value,
    passed: bool,
    failure_message: String,
}

fn asset_doctor_report(path: &Path) -> AssetDoctorReport {
    let official = run_official_gltf_validator(path);
    let guidance_result = scena_native_asset_guidance(path);
    let (guidance, native_error) = match guidance_result {
        Ok(guidance) => (guidance, None),
        Err(error) => (Vec::new(), Some(error)),
    };
    let native_has_errors = guidance
        .iter()
        .any(|finding| finding.severity == AssetGuidanceSeverity::Error);
    let passed = official.passed && native_error.is_none() && !native_has_errors;
    let status = if passed { "passed" } else { "failed" };
    let mut failure_parts = Vec::new();
    if !official.passed {
        failure_parts.push("official Khronos glTF Validator did not pass".to_string());
    }
    if let Some(error) = &native_error {
        failure_parts.push(format!(
            "scena-native validation failed to read asset: {error}"
        ));
    }
    if native_has_errors {
        failure_parts.push("scena-native renderer guidance found errors".to_string());
    }
    let failure_message = if failure_parts.is_empty() {
        "asset validation passed".to_string()
    } else {
        failure_parts.join("; ")
    };
    let guidance_json: Vec<Value> = guidance.iter().map(asset_guidance_json).collect();

    AssetDoctorReport {
        json: json!({
            "schema": "scena.asset_doctor.v1",
            "status": status,
            "asset": path.to_string_lossy(),
            "official_validator": official.json,
            "scena_guidance": guidance_json,
            "scena_native_error": native_error,
        }),
        passed,
        failure_message,
    }
}

struct OfficialValidatorOutcome {
    passed: bool,
    json: Value,
}

fn run_official_gltf_validator(path: &Path) -> OfficialValidatorOutcome {
    let tool = env::var("SCENA_GLTF_VALIDATOR").unwrap_or_else(|_| "gltf_validator".to_string());
    let args = official_gltf_validator_args(path);
    let output = ProcessCommand::new(&tool).args(&args).output();
    match output {
        Ok(output) => {
            let exit_code = output.status.code();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout_json = serde_json::from_str::<Value>(&stdout).ok();
            let passed = output.status.success() && stdout_json.is_some();
            OfficialValidatorOutcome {
                passed,
                json: json!({
                    "tool": tool,
                    "args": args,
                    "status": if passed { "passed" } else { "failed" },
                    "exit_code": exit_code,
                    "stdout_json": stdout_json,
                    "stderr": stderr,
                    "fix": if output.status.success() {
                        "The official Khronos glTF Validator did not emit parseable stdout JSON; verify the executable and stdout mode before trusting the asset report."
                    } else {
                        "Fix glTF specification errors reported by the official Khronos glTF Validator before debugging scena renderer behavior."
                    },
                }),
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => OfficialValidatorOutcome {
            passed: false,
            json: json!({
                "tool": tool,
                "args": args,
                "status": "unavailable",
                "exit_code": null,
                "stdout_json": null,
                "stderr": "",
                "fix": "Install the official Khronos glTF Validator CLI as `gltf_validator`, or set SCENA_GLTF_VALIDATOR to the validator executable path.",
            }),
        },
        Err(error) => OfficialValidatorOutcome {
            passed: false,
            json: json!({
                "tool": tool,
                "args": args,
                "status": "failed-to-run",
                "exit_code": null,
                "stdout_json": null,
                "stderr": error.to_string(),
                "fix": "Make the official Khronos glTF Validator executable runnable, then rerun the asset doctor.",
            }),
        },
    }
}

fn scena_native_guidance_from_document(document: &Value) -> Vec<AssetGuidanceFinding> {
    let mut used = string_array(document.get("extensionsUsed"));
    collect_nested_extension_keys(document, &mut used);
    let required = string_array(document.get("extensionsRequired"));
    let mut findings = Vec::new();
    for extension in used {
        if let Some(finding) = extension_guidance(&extension, required.contains(&extension)) {
            findings.push(finding);
        }
    }
    findings
}

fn extension_guidance(extension: &str, required: bool) -> Option<AssetGuidanceFinding> {
    match extension {
        "KHR_lights_punctual"
        | "KHR_materials_unlit"
        | "KHR_materials_emissive_strength"
        | "KHR_texture_transform"
        | "KHR_mesh_quantization"
        | "KHR_materials_variants"
        | "EXT_mesh_gpu_instancing" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: AssetGuidanceSeverity::Info,
            status: "supported",
            message: format!("{extension} is supported by scena's glTF importer."),
            fix: "No action needed for this extension.".to_string(),
        }),
        "KHR_texture_basisu" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "feature-gated",
            message: "KTX2/Basis textures need scena's decoder-backed KTX2 path.".to_string(),
            fix: "Enable the production-assets or ktx2 feature, or export PNG/JPEG/WebP fallback textures.".to_string(),
        }),
        "EXT_meshopt_compression" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "feature-gated",
            message: "Meshopt-compressed buffers need scena's meshopt decoder path.".to_string(),
            fix: "Enable the production-assets or meshopt feature, or export an uncompressed buffer fallback.".to_string(),
        }),
        "KHR_draco_mesh_compression" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "unsupported",
            message: "Draco mesh compression is not part of scena's v1.4 production path.".to_string(),
            fix: "Re-export the asset uncompressed or with EXT_meshopt_compression; revisit Draco when a maintained decoder is adopted.".to_string(),
        }),
        "KHR_materials_clearcoat" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "degraded",
            message: "Clearcoat factors plus clearcoat, roughness, and normal texture slots are CPU/reference-supported and wired through the GPU shader path, but required clearcoat can still depend on approved GPU/browser rendered-output proof that is not release-proven.".to_string(),
            fix: "If the look depends on backend parity, export a fallback material without clearcoat or keep KHR_materials_clearcoat optional until approved backend screenshots or readback proof cover the target lane.".to_string(),
        }),
        "KHR_materials_sheen" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "degraded",
            message: "Sheen factors plus sheen color and roughness texture slots are CPU/reference-supported and wired through the GPU shader path, but required sheen can still depend on approved GPU/browser rendered-output proof that is not release-proven.".to_string(),
            fix: "If the look depends on backend parity, export a fallback material without sheen or keep KHR_materials_sheen optional until approved backend screenshots or readback proof cover the target lane.".to_string(),
        }),
        "KHR_materials_anisotropy" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "degraded",
            message: "Anisotropy strength, rotation, and direction/strength texture slots are CPU/reference-supported and wired through the GPU shader path, but required anisotropy can still depend on approved GPU/browser rendered-output proof that is not release-proven.".to_string(),
            fix: "If the look depends on backend parity, export a fallback material without anisotropy or keep KHR_materials_anisotropy optional until approved backend screenshots or readback proof cover the target lane.".to_string(),
        }),
        "KHR_materials_iridescence" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "degraded",
            message: "Iridescence factor, IOR, thickness range, and factor/thickness texture slots are CPU/reference-supported and wired through the GPU shader path, but required iridescence can still depend on approved GPU/browser rendered-output proof that is not release-proven.".to_string(),
            fix: "If the look depends on backend parity, export a fallback material without iridescence or keep KHR_materials_iridescence optional until approved backend screenshots or readback proof cover the target lane.".to_string(),
        }),
        "KHR_materials_transmission"
        | "KHR_materials_ior"
        | "KHR_materials_volume"
        | "KHR_materials_specular"
        | "KHR_materials_dispersion" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "degraded",
            message: format!(
                "{extension} changes material appearance and currently renders as a structured fallback in scena."
            ),
            fix: format!(
                "If the look depends on {extension}, export a fallback material without {extension} or wait for the matching renderer feature before making it required."
            ),
        }),
        "EXT_texture_webp" => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "deferred",
            message: "EXT_texture_webp texture-source rebinding is deferred even though plain .webp image paths are supported.".to_string(),
            fix: "Re-export with PNG/JPEG/WebP image URIs outside EXT_texture_webp, or use KTX2 through the ktx2 feature.".to_string(),
        }),
        _ => Some(AssetGuidanceFinding {
            extension: extension.to_string(),
            required,
            severity: if required {
                AssetGuidanceSeverity::Error
            } else {
                AssetGuidanceSeverity::Warning
            },
            status: "unknown",
            message: format!("{extension} is not in scena's supported extension policy table."),
            fix: "Make the extension optional with a visual fallback, or add an explicit scena decoder/support policy before relying on it.".to_string(),
        }),
    }
}

fn asset_guidance_json(finding: &AssetGuidanceFinding) -> Value {
    json!({
        "extension": finding.extension,
        "required": finding.required,
        "severity": finding.severity.as_str(),
        "status": finding.status,
        "message": finding.message,
        "fix": finding.fix,
    })
}

impl AssetGuidanceSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

fn string_array(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn collect_nested_extension_keys(value: &Value, extensions: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::Object(extension_object)) = object.get("extensions") {
                extensions.extend(extension_object.keys().cloned());
            }
            for child in object.values() {
                collect_nested_extension_keys(child, extensions);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_nested_extension_keys(item, extensions);
            }
        }
        _ => {}
    }
}

fn load_gltf_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.starts_with(b"glTF") {
        load_glb_json(&bytes)
    } else {
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
    }
}

fn load_glb_json(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() < 20 {
        return Err("GLB is too short to contain a JSON chunk".to_string());
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 2 {
        return Err(format!("unsupported GLB version {version}; expected 2"));
    }
    let total_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if total_len > bytes.len() {
        return Err("GLB declared length exceeds file length".to_string());
    }
    let chunk_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    let chunk_type = &bytes[16..20];
    if chunk_type != b"JSON" {
        return Err("first GLB chunk is not JSON".to_string());
    }
    let chunk_end = 20usize
        .checked_add(chunk_len)
        .ok_or_else(|| "GLB JSON chunk length overflowed".to_string())?;
    if chunk_end > bytes.len() {
        return Err("GLB JSON chunk exceeds file length".to_string());
    }
    let mut json_bytes = bytes[20..chunk_end].to_vec();
    while json_bytes
        .last()
        .is_some_and(|byte| *byte == 0 || *byte == b' ')
    {
        json_bytes.pop();
    }
    serde_json::from_slice(&json_bytes).map_err(|error| format!("GLB JSON chunk: {error}"))
}
