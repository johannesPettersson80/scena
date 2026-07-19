use crate::app::prelude::*;

pub(super) fn require_typed_visual_proof_contract(
    path: &Path,
    suffix: &str,
    value: &serde_json::Value,
    findings: &mut Vec<Finding>,
) {
    let (expected_lane, expected_class, expected_backend) = match suffix {
        "visual-proof/waterbottle-gpu.json" => ("waterbottle-gpu", "native-waterbottle-gpu", None),
        "visual-proof/waterbottle-cpu.json" => {
            ("waterbottle-cpu", "cpu-waterbottle-reference", None)
        }
        "visual-proof/browser-webgpu.json" => (
            "browser-webgpu",
            "browser-rust-wasm-rendered-output",
            Some("webgpu"),
        ),
        "visual-proof/browser-webgl2.json" => (
            "browser-webgl2",
            "browser-rust-wasm-rendered-output",
            Some("webgl2"),
        ),
        "visual-proof/native-gpu.json" => ("native-gpu", "native-gpu-rendered-output", None),
        _ => return,
    };
    for (field, expected) in [
        ("schema", "scena.visual_proof.v1"),
        ("lane", expected_lane),
        ("proof_class", expected_class),
        ("producer", "cargo run -p xtask -- stage-release-artifacts"),
    ] {
        if value.get(field).and_then(serde_json::Value::as_str) != Some(expected) {
            findings.push(Finding::new(
                "VISUAL-PROOF",
                format!("visual proof artifact {suffix} {field} must be {expected:?}"),
            ));
        }
    }
    if value
        .get("release_evidence")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} must declare release_evidence=true"),
        ));
    }
    let commit = value
        .get("commit_sha")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} must carry an exact commit"),
        ));
    }
    if value
        .get("timestamp_unix_seconds")
        .and_then(serde_json::Value::as_u64)
        .is_none()
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} must carry a generation timestamp"),
        ));
    }
    require_visual_source_hash(path, suffix, value, findings);
    if let Some(backend) = expected_backend {
        require_browser_visual_proof(suffix, value, backend, findings);
    }
    match suffix {
        "visual-proof/waterbottle-gpu.json" => {
            require_waterbottle_visual_proof(path, suffix, value, findings)
        }
        "visual-proof/waterbottle-cpu.json" => {
            require_waterbottle_cpu_visual_proof(path, suffix, value, findings)
        }
        "visual-proof/native-gpu.json" => {
            require_native_visual_proof(path, suffix, value, findings)
        }
        _ => {}
    }
}

fn require_waterbottle_cpu_visual_proof(
    path: &Path,
    suffix: &str,
    value: &serde_json::Value,
    findings: &mut Vec<Finding>,
) {
    if value.get("width").and_then(Value::as_u64) != Some(256)
        || value.get("height").and_then(Value::as_u64) != Some(256)
        || value.get("color_type").and_then(Value::as_str) != Some("rgba8")
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} CPU dimensions/type must be 256x256 RGBA8"),
        ));
    }
    if value.get("color_space").and_then(Value::as_str) != Some("srgb-output")
        || value.get("row_orientation").and_then(Value::as_str) != Some("top-to-bottom")
        || value.get("alpha_contract").and_then(Value::as_str) != Some("opaque")
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "visual proof artifact {suffix} CPU color/orientation contract must be \
                 sRGB output, top-to-bottom, opaque RGBA8"
            ),
        ));
    }
    if value.get("backend").and_then(Value::as_str) != Some("Headless")
        || value.get("adapter").and_then(Value::as_str) != Some("software-rasterizer")
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "visual proof artifact {suffix} CPU backend/adapter must be \
                 Headless/software-rasterizer"
            ),
        ));
    }
    if value.pointer("/metrics/passed").and_then(Value::as_bool) != Some(true)
        || value
            .pointer("/metrics/alpha_mismatch_pixels")
            .and_then(Value::as_u64)
            != Some(0)
        || value
            .pointer("/metrics/rgb_chebyshev_tolerance")
            .and_then(Value::as_u64)
            != Some(4)
        || value
            .pointer("/metrics/within_tolerance_fraction")
            .and_then(Value::as_f64)
            .is_none_or(|fraction| fraction < 0.995)
        || value
            .pointer("/metrics/rgb_rmse")
            .and_then(Value::as_f64)
            .is_none_or(|rmse| rmse > 2.0)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} CPU comparison metrics must pass"),
        ));
    }
    let mutations = value
        .get("mutations")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let artifact_root = path.parent().and_then(Path::parent);
    for (name, expected_path) in [
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
        let valid = mutations
            .iter()
            .find(|mutation| mutation.get("name").and_then(Value::as_str) == Some(name))
            .is_some_and(|mutation| {
                let hash = mutation.get("sha256").and_then(Value::as_str).unwrap_or("");
                mutation.get("path").and_then(Value::as_str) == Some(expected_path)
                    && mutation.get("oracle_rejected").and_then(Value::as_bool) == Some(true)
                    && mutation.pointer("/metrics/passed").and_then(Value::as_bool) == Some(false)
                    && valid_visual_sha256(hash)
                    && artifact_root
                        .map(|root| root.join(expected_path))
                        .and_then(|artifact| sha256_hex(&artifact).ok())
                        .as_deref()
                        == Some(hash)
            });
        if !valid {
            findings.push(Finding::new(
                "VISUAL-PROOF",
                format!(
                    "visual proof artifact {suffix} CPU mutation oracle must bind and reject \
                     {name}"
                ),
            ));
        }
    }
    if value.get("test_name").and_then(Value::as_str)
        != Some("q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders")
        || value.get("rust_test_command").and_then(Value::as_bool) != Some(true)
        || value
            .get("rust_test_output_observed")
            .and_then(Value::as_bool)
            != Some(true)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} CPU exact Rust test output is missing"),
        ));
    }
    let live_hash = value
        .get("live_png_sha256")
        .and_then(Value::as_str)
        .unwrap_or("");
    let source_hash = value
        .get("source_artifact_sha256")
        .and_then(Value::as_str)
        .unwrap_or("");
    if live_hash != source_hash || !valid_visual_sha256(live_hash) {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} CPU live PNG hash binding is invalid"),
        ));
    }
    if value.get("reference_sha256").and_then(Value::as_str)
        != Some("922cc35e0c6420d2b3f8e533891291a9d4f9396697ae366f0b93de3c15973da4")
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} CPU committed reference hash is invalid"),
        ));
    }
    require_related_visual_artifact_hash(
        path,
        suffix,
        value,
        "command_record_artifact",
        "command_record_sha256",
        "CPU command-record hash",
        findings,
    );
    require_related_visual_artifact_hash(
        path,
        suffix,
        value,
        "result_artifact",
        "result_sha256",
        "CPU result artifact hash",
        findings,
    );
}

fn require_visual_source_hash(
    path: &Path,
    suffix: &str,
    value: &serde_json::Value,
    findings: &mut Vec<Finding>,
) {
    let relative = value
        .get("source_artifact_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let expected = value
        .get("source_artifact_sha256")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let valid_hash = expected.len() == 64
        && expected.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !expected.bytes().all(|byte| byte == b'0');
    let artifact_root = path.parent().and_then(Path::parent);
    let source = artifact_root.map(|root| root.join(relative));
    let source_is_bound = !relative.is_empty()
        && !Path::new(relative).is_absolute()
        && !Path::new(relative)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        && valid_hash
        && source.as_ref().is_some_and(|source| source.is_file())
        && source
            .as_ref()
            .and_then(|source| sha256_hex(source).ok())
            .as_deref()
            == Some(expected);
    if !source_is_bound {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "visual proof artifact {suffix} source artifact hash must bind an existing bundle file"
            ),
        ));
    }
}

fn require_browser_visual_proof(
    suffix: &str,
    value: &serde_json::Value,
    expected_backend: &str,
    findings: &mut Vec<Finding>,
) {
    if value.get("backend").and_then(serde_json::Value::as_str) != Some(expected_backend) {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} backend must be {expected_backend}"),
        ));
    }
    let readback = value.get("renderer_readback");
    if readback
        .and_then(|readback| readback.get("source"))
        .and_then(serde_json::Value::as_str)
        != Some("renderer-owned-gpu-copy")
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} must use renderer-owned-gpu-copy readback"),
        ));
    }
    let width = readback
        .and_then(|readback| readback.get("width"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let height = readback
        .and_then(|readback| readback.get("height"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if width == 0 || height == 0 {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} renderer dimensions must be positive"),
        ));
    }
    let nonblack = readback
        .and_then(|readback| readback.pointer("/pixel_statistics/nonblack"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if nonblack == 0 {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} must contain nonblack renderer pixels"),
        ));
    }
    let checksum = readback
        .and_then(|readback| readback.get("rgba8_fnv1a64"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if checksum.len() != 16
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || checksum.bytes().all(|byte| byte == b'0')
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "visual proof artifact {suffix} rgba8_fnv1a64 must be a nonzero lowercase checksum"
            ),
        ));
    }
}

fn require_waterbottle_visual_proof(
    path: &Path,
    suffix: &str,
    value: &serde_json::Value,
    findings: &mut Vec<Finding>,
) {
    if value.get("width").and_then(serde_json::Value::as_u64) != Some(512)
        || value.get("height").and_then(serde_json::Value::as_u64) != Some(512)
        || value.get("color_type").and_then(serde_json::Value::as_str) != Some("rgba8")
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "visual proof artifact {suffix} WaterBottle dimensions/type must be 512x512 RGBA8"
            ),
        ));
    }
    let nonblack = value
        .get("nonblack_pixels")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let distinct = value
        .get("distinct_rgba_values")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if nonblack <= 5_000 || distinct < 64 {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} WaterBottle pixel distribution is trivial"),
        ));
    }
    if [
        "nonblack_passed",
        "region_checks_passed",
        "color_family_histograms_passed",
    ]
    .into_iter()
    .any(|metric| value.pointer(&format!("/metrics/{metric}")) != Some(&Value::Bool(true)))
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} WaterBottle comparison metrics must all pass"),
        ));
    }
    if value.get("test_name").and_then(serde_json::Value::as_str)
        != Some("m8_real_asset_waterbottle_gpu_headline")
        || value
            .get("rust_test_output_observed")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} WaterBottle exact Rust test output is missing"),
        ));
    }
    let backend = value
        .get("backend")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let adapter = value
        .get("adapter")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(backend, "Metal" | "Vulkan" | "Dx12")
        || adapter.is_empty()
        || ["software", "llvmpipe", "swiftshader", "basic render driver"]
            .iter()
            .any(|marker| adapter.contains(marker))
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} WaterBottle backend/adapter is not approved"),
        ));
    }
    let png_hash = value
        .get("png_sha256")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let source_hash = value
        .get("source_artifact_sha256")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if png_hash != source_hash || !valid_visual_sha256(png_hash) {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} WaterBottle PNG hash binding is invalid"),
        ));
    }
    if !valid_visual_sha256(
        value
            .get("command_record_sha256")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    ) {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} WaterBottle command-record hash is invalid"),
        ));
    }
    require_related_visual_artifact_hash(
        path,
        suffix,
        value,
        "result_artifact",
        "result_sha256",
        "WaterBottle result artifact hash",
        findings,
    );
}

fn require_native_visual_proof(
    path: &Path,
    suffix: &str,
    value: &serde_json::Value,
    findings: &mut Vec<Finding>,
) {
    let lane = value
        .get("source_lane")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let relative = value
        .get("source_artifact_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let approved_lane = matches!(lane, "macos-metal" | "windows-dx12" | "linux-native-vulkan");
    let expected_relative = format!("m9-platform/{lane}/rendered-output.json");
    let source_proves_gpu = path
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join(relative))
        .and_then(|source| fs::read_to_string(source).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .is_some_and(|source| native_gpu_render_proof_passes(&source));
    if !approved_lane
        || relative != expected_relative
        || value.get("gpu_proof").and_then(serde_json::Value::as_bool) != Some(true)
        || !source_proves_gpu
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!(
                "visual proof artifact {suffix} native GPU source artifact does not prove an approved GPU lane"
            ),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn require_related_visual_artifact_hash(
    path: &Path,
    suffix: &str,
    value: &serde_json::Value,
    path_field: &str,
    hash_field: &str,
    label: &str,
    findings: &mut Vec<Finding>,
) {
    let relative = value
        .get(path_field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let expected = value
        .get(hash_field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let actual = path
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join(relative))
        .and_then(|source| sha256_hex(&source).ok());
    if Path::new(relative).is_absolute()
        || Path::new(relative)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || !valid_visual_sha256(expected)
        || actual.as_deref() != Some(expected)
    {
        findings.push(Finding::new(
            "VISUAL-PROOF",
            format!("visual proof artifact {suffix} {label} must bind an existing bundle file"),
        ));
    }
}

fn valid_visual_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !value.bytes().all(|byte| byte == b'0')
}
