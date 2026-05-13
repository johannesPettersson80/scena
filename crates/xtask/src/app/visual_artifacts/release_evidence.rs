use crate::app::prelude::*;

pub(crate) fn require_rendered_output_screenshot_metadata(
    value: &serde_json::Value,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
    for section in ["default_scene", "static_gltf"] {
        let Some(entry) = value.get(section) else {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!("rendered-output artifact {suffix} is missing screenshot metadata section {section}"),
            ));
            continue;
        };
        require_screenshot_metadata_entry(entry, suffix, section, findings);
    }

    let Some(lights) = value
        .get("pbr_lights")
        .and_then(|pbr| pbr.get("lights"))
        .and_then(serde_json::Value::as_array)
    else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("rendered-output artifact {suffix} is missing screenshot metadata section pbr_lights.lights"),
        ));
        return;
    };
    if lights.is_empty() {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "rendered-output artifact {suffix} has no PBR light screenshot metadata entries"
            ),
        ));
    }
    for (index, entry) in lights.iter().enumerate() {
        require_screenshot_metadata_entry(
            entry,
            suffix,
            &format!("pbr_lights.lights[{index}]"),
            findings,
        );
    }
}

pub(crate) fn require_screenshot_metadata_entry(
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

pub(crate) fn require_release_lane_artifact_file(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
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

pub(crate) fn require_release_lane_artifact_evidence(
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
        if record
            .get("artifact_checksums")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|checksums| checksums.is_empty())
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

pub(crate) fn require_native_gpu_render_proof(
    path: &Path,
    suffix: &str,
    findings: &mut Vec<Finding>,
) {
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

pub(crate) fn native_gpu_render_proof_passes(value: &serde_json::Value) -> bool {
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

pub(crate) fn pbr_light_render_proof_passes(value: &serde_json::Value) -> bool {
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

pub(crate) fn headless_cpu_render_proof_passes(value: &serde_json::Value) -> bool {
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

pub(crate) fn path_ends_with(path: &Path, suffix: &str) -> bool {
    path.to_string_lossy().replace('\\', "/").ends_with(suffix)
}

pub(crate) fn build_claim_audit(root: &Path) -> Result<serde_json::Value, String> {
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

pub(crate) fn claim_audit_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
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

pub(crate) fn collect_files_with_extensions(
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

pub(crate) fn claim_categories(source: &str) -> Vec<&'static str> {
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
