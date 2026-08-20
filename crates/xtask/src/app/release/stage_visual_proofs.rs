use crate::app::prelude::*;

mod output;
use output::{
    valid_sha256, visual_proof_base, write_browser_visual_proof, write_native_gpu_visual_proof,
};

const WATERBOTTLE_REFERENCE_SHA256: &str =
    "4db449cdacf2340f8fa53937c28e5c4b5e2c7deaea73cbe0987dcd51eb93c751";

pub(super) fn write_visual_proof_artifacts(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    write_waterbottle_visual_proof(output, files, expected_commit)?;
    write_waterbottle_cpu_visual_proof(output, files, expected_commit)?;
    write_browser_visual_proof(output, files, expected_commit, "webgl2", "browser-webgl2")?;
    write_browser_visual_proof(output, files, expected_commit, "webgpu", "browser-webgpu")?;
    write_native_gpu_visual_proof(output, expected_commit)?;
    Ok(())
}

fn write_waterbottle_cpu_visual_proof(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    let Some(source) =
        super::stage_artifacts::select_stage_source(files, "q01-waterbottle-cpu/live.png")
    else {
        return Err("missing WaterBottle CPU PNG for visual proof".to_string());
    };
    let bytes = fs::read(&source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    let live_sha256 = sha256_hex(&source).map_err(|error| error.to_string())?;
    let result_path = output.join("q01-waterbottle-cpu/result.json");
    let result_text = fs::read_to_string(&result_path)
        .map_err(|error| format!("failed to read {}: {error}", result_path.display()))?;
    let result = serde_json::from_str::<Value>(&result_text)
        .map_err(|error| format!("failed to parse {}: {error}", result_path.display()))?;
    validate_waterbottle_cpu_result(output, &result, expected_commit, &live_sha256)?;

    let decoded =
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).map_err(|error| {
            format!(
                "WaterBottle CPU visual proof {} is not a decodable PNG: {error}",
                source.display()
            )
        })?;
    if decoded.width() != 256
        || decoded.height() != 256
        || decoded.color() != image::ColorType::Rgba8
    {
        return Err(format!(
            "WaterBottle CPU visual proof {} must be 256x256 RGBA8, got {}x{} {:?}",
            source.display(),
            decoded.width(),
            decoded.height(),
            decoded.color()
        ));
    }
    let staged_png = output.join("q01-waterbottle-cpu/live.png");
    let proof = visual_proof_base(
        "waterbottle-cpu",
        expected_commit,
        "cpu-waterbottle-reference",
    )
    .with_source(&staged_png, "q01-waterbottle-cpu/live.png")?
    .with_extra(json!({
        "artifact": "q01-waterbottle-cpu/live.png",
        "live_png_sha256": live_sha256,
        "reference_path": result.get("reference_path").cloned().unwrap_or(Value::Null),
        "reference_sha256": result.get("reference_sha256").cloned().unwrap_or(Value::Null),
        "result_artifact": "q01-waterbottle-cpu/result.json",
        "result_sha256": sha256_hex(&result_path).map_err(|error| error.to_string())?,
        "byte_len": bytes.len(),
        "width": 256,
        "height": 256,
        "color_type": "rgba8",
        "color_space": "srgb-output",
        "row_orientation": "top-to-bottom",
        "alpha_contract": "opaque",
        "source_producer": result.get("producer").cloned().unwrap_or(Value::Null),
        "test_name": result.get("test_name").cloned().unwrap_or(Value::Null),
        "backend": result.get("backend").cloned().unwrap_or(Value::Null),
        "adapter": result.get("adapter").cloned().unwrap_or(Value::Null),
        "command_record_sha256": result
            .get("command_record_sha256")
            .cloned()
            .unwrap_or(Value::Null),
        "command_record_artifact": "release-lanes/headless-cpu.commands.jsonl",
        "metrics": result.get("metrics").cloned().unwrap_or(Value::Null),
        "mutations": result.get("mutations").cloned().unwrap_or(Value::Null),
        "rust_test_command": true,
        "rust_test_output_observed": true,
    }))
    .finish();
    super::stage_artifacts::write_stage_json(
        &output.join("visual-proof/waterbottle-cpu.json"),
        &proof,
    )
}

fn validate_waterbottle_cpu_result(
    output: &Path,
    result: &Value,
    expected_commit: &str,
    live_sha256: &str,
) -> Result<(), String> {
    for (field, expected) in [
        ("schema", "scena.q01.waterbottle_cpu_reference.v1"),
        ("status", "passed"),
        (
            "test_name",
            "q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders",
        ),
        ("commit_sha", expected_commit),
        ("backend", "Headless"),
        ("adapter", "software-rasterizer"),
        ("color_type", "rgba8"),
        ("color_space", "srgb-output"),
        ("row_orientation", "top-to-bottom"),
        ("alpha_contract", "opaque"),
        ("live_png_sha256", live_sha256),
    ] {
        if result.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "WaterBottle CPU result {field} must be {expected:?}"
            ));
        }
    }
    if result.get("release_evidence").and_then(Value::as_bool) != Some(true)
        || result
            .get("rust_test_output_observed")
            .and_then(Value::as_bool)
            != Some(true)
        || result.get("width").and_then(Value::as_u64) != Some(256)
        || result.get("height").and_then(Value::as_u64) != Some(256)
        || result
            .get("timestamp_unix_seconds")
            .and_then(Value::as_u64)
            .is_none()
        || result.pointer("/metrics/passed").and_then(Value::as_bool) != Some(true)
        || result
            .pointer("/metrics/alpha_mismatch_pixels")
            .and_then(Value::as_u64)
            != Some(0)
        || result
            .pointer("/metrics/rgb_chebyshev_tolerance")
            .and_then(Value::as_u64)
            != Some(4)
        || result
            .pointer("/metrics/within_tolerance_fraction")
            .and_then(Value::as_f64)
            .is_none_or(|fraction| fraction < 0.995)
        || result
            .pointer("/metrics/rgb_rmse")
            .and_then(Value::as_f64)
            .is_none_or(|rmse| rmse > 2.0)
    {
        return Err(
            "WaterBottle CPU result comparison metrics are incomplete or failed".to_string(),
        );
    }
    if result.get("reference_sha256").and_then(Value::as_str)
        != Some("8bbaa66e23a3dea4a9efcb53f9157226b666cd77bd020dea30cd89277d3037b5")
    {
        return Err(
            "WaterBottle CPU result does not bind the approved committed reference".to_string(),
        );
    }
    let determinism = result
        .get("determinism")
        .ok_or_else(|| "WaterBottle CPU result is missing determinism evidence".to_string())?;
    let hashes = determinism
        .get("rgba8_sha256")
        .and_then(Value::as_array)
        .ok_or_else(|| "WaterBottle CPU determinism hashes must be an array".to_string())?;
    if determinism.get("comparison_order").and_then(Value::as_str)
        != Some("independent-render-before-committed-reference")
        || determinism.get("repeat_count").and_then(Value::as_u64) != Some(2)
        || determinism.get("byte_identical").and_then(Value::as_bool) != Some(true)
        || hashes.len() != 2
        || hashes[0] != hashes[1]
    {
        return Err(
            "WaterBottle CPU result must prove two byte-identical independent renders before reference comparison"
                .to_string(),
        );
    }
    let command_sha = result
        .get("command_record_sha256")
        .and_then(Value::as_str)
        .unwrap_or("");
    let command_path = output.join("release-lanes/headless-cpu.commands.jsonl");
    if !valid_sha256(command_sha) || sha256_hex(&command_path).ok().as_deref() != Some(command_sha)
    {
        return Err(
            "WaterBottle CPU result command-record hash is not bound to the staged command record"
                .to_string(),
        );
    }
    let mutations = result
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
        super::waterbottle_results::validate_waterbottle_mutation_provenance(mutation, name)?;
        let sha = mutation.get("sha256").and_then(Value::as_str).unwrap_or("");
        if mutation.get("path").and_then(Value::as_str) != Some(relative)
            || mutation.get("oracle_rejected").and_then(Value::as_bool) != Some(true)
            || mutation.pointer("/metrics/passed").and_then(Value::as_bool) != Some(false)
            || !valid_sha256(sha)
            || sha256_hex(&output.join(relative)).ok().as_deref() != Some(sha)
        {
            return Err(format!(
                "WaterBottle CPU mutation {name} is not bound to a rejected staged output"
            ));
        }
    }
    Ok(())
}

fn write_waterbottle_visual_proof(
    output: &Path,
    files: &[PathBuf],
    expected_commit: &str,
) -> Result<(), String> {
    let Some(source) =
        super::stage_artifacts::select_stage_source(files, "m8-real-asset/waterbottle_gpu.png")
    else {
        return Err("missing WaterBottle GPU PNG for visual proof".to_string());
    };
    let bytes = fs::read(&source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    let png_sha256 = sha256_hex(&source).map_err(|error| error.to_string())?;
    let diff_source =
        super::stage_artifacts::select_stage_source(files, "m8-real-asset/waterbottle_diff.png")
            .ok_or_else(|| {
                "missing WaterBottle GPU full-frame diff PNG for visual proof".to_string()
            })?;
    let diff_sha256 = sha256_hex(&diff_source).map_err(|error| error.to_string())?;
    let result_path = output.join("m8-real-asset/waterbottle_gpu_result.json");
    let result_text = fs::read_to_string(&result_path)
        .map_err(|error| format!("failed to read {}: {error}", result_path.display()))?;
    let result = serde_json::from_str::<Value>(&result_text)
        .map_err(|error| format!("failed to parse {}: {error}", result_path.display()))?;
    validate_waterbottle_result(&result, expected_commit, &png_sha256, &diff_sha256)?;
    let decoded =
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).map_err(|error| {
            format!(
                "WaterBottle GPU visual proof {} is not a decodable PNG: {error}",
                source.display()
            )
        })?;
    if decoded.width() != 512 || decoded.height() != 512 {
        return Err(format!(
            "WaterBottle GPU visual proof {} must be 512x512, got {}x{}",
            source.display(),
            decoded.width(),
            decoded.height()
        ));
    }
    if decoded.color() != image::ColorType::Rgba8 {
        return Err(format!(
            "WaterBottle GPU visual proof {} must decode as RGBA8, got {:?}",
            source.display(),
            decoded.color()
        ));
    }
    let rgba = decoded.into_rgba8();
    let nonblack_pixels = rgba
        .pixels()
        .filter(|pixel| pixel.0[..3] != [0, 0, 0])
        .count();
    let mut colors = rgba
        .pixels()
        .map(|pixel| u32::from_be_bytes(pixel.0))
        .collect::<Vec<_>>();
    colors.sort_unstable();
    colors.dedup();
    if nonblack_pixels <= 5_000 || colors.len() < 64 {
        return Err(format!(
            "WaterBottle GPU visual proof {} has trivial pixel content: {nonblack_pixels} \
             nonblack pixels and {} distinct RGBA values",
            source.display(),
            colors.len()
        ));
    }
    let staged_png = output.join("m8-real-asset/waterbottle_gpu.png");
    let proof = visual_proof_base("waterbottle-gpu", expected_commit, "native-waterbottle-gpu")
        .with_source(&staged_png, "m8-real-asset/waterbottle_gpu.png")?
        .with_extra(json!({
            "artifact": "m8-real-asset/waterbottle_gpu.png",
            "png_sha256": png_sha256,
            "result_artifact": "m8-real-asset/waterbottle_gpu_result.json",
            "result_sha256": sha256_hex(&result_path).map_err(|error| error.to_string())?,
            "diff_artifact": "m8-real-asset/waterbottle_diff.png",
            "diff_sha256": diff_sha256,
            "reference_path": result.get("reference_path").cloned().unwrap_or(Value::Null),
            "reference_sha256": result.get("reference_sha256").cloned().unwrap_or(Value::Null),
            "byte_len": bytes.len(),
            "width": 512,
            "height": 512,
            "color_type": "rgba8",
            "nonblack_pixels": nonblack_pixels,
            "distinct_rgba_values": colors.len(),
            "source_producer": result.get("producer").cloned().unwrap_or(Value::Null),
            "test_name": result.get("test_name").cloned().unwrap_or(Value::Null),
            "backend": result.get("backend").cloned().unwrap_or(Value::Null),
            "adapter": result.get("adapter").cloned().unwrap_or(Value::Null),
            "command_record_sha256": result
                .get("command_record_sha256")
                .cloned()
                .unwrap_or(Value::Null),
            "metrics": result.get("metrics").cloned().unwrap_or(Value::Null),
            "rust_test_output_observed": result
                .get("rust_test_output_observed")
                .cloned()
                .unwrap_or(Value::Bool(false)),
        }))
        .finish();
    super::stage_artifacts::write_stage_json(
        &output.join("visual-proof/waterbottle-gpu.json"),
        &proof,
    )
}

fn validate_waterbottle_result(
    result: &Value,
    expected_commit: &str,
    png_sha256: &str,
    diff_sha256: &str,
) -> Result<(), String> {
    for (field, expected) in [
        ("schema", "scena.m8.waterbottle_gpu_result.v1"),
        ("status", "passed"),
        ("test_name", "m8_real_asset_waterbottle_gpu_headline"),
        ("commit_sha", expected_commit),
        ("png_sha256", png_sha256),
        ("diff_sha256", diff_sha256),
        ("reference_sha256", WATERBOTTLE_REFERENCE_SHA256),
        (
            "reference_path",
            "tests/assets/gltf/khronos/WaterBottle/reference_512.png",
        ),
        ("diff_path", "m8-real-asset/waterbottle_diff.png"),
    ] {
        if result.get(field).and_then(Value::as_str) != Some(expected) {
            if field == "png_sha256" {
                return Err(format!(
                    "WaterBottle GPU result PNG hash does not match the staged PNG; expected \
                     {expected}"
                ));
            }
            return Err(format!(
                "WaterBottle GPU result {field} must be {expected:?}"
            ));
        }
    }
    for (field, expected) in [
        ("release_evidence", true),
        ("software_adapter", false),
        ("skip_marker_observed", false),
        ("fallback_observed", false),
        ("rust_test_output_observed", true),
    ] {
        if result.get(field).and_then(Value::as_bool) != Some(expected) {
            return Err(format!("WaterBottle GPU result {field} must be {expected}"));
        }
    }
    let backend = result.get("backend").and_then(Value::as_str).unwrap_or("");
    if !matches!(backend, "Metal" | "Vulkan" | "Dx12") {
        return Err(format!(
            "WaterBottle GPU result backend {backend:?} is not an approved native GPU backend"
        ));
    }
    let adapter = result
        .get("adapter")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if adapter.is_empty()
        || ["software", "llvmpipe", "swiftshader", "basic render driver"]
            .iter()
            .any(|marker| adapter.contains(marker))
    {
        return Err(format!(
            "WaterBottle GPU result adapter {adapter:?} is missing or disallowed"
        ));
    }
    let adapter_key = result
        .get("adapter_key")
        .ok_or_else(|| "WaterBottle GPU result is missing adapter_key".to_string())?;
    if adapter_key.get("schema").and_then(Value::as_str) != Some("scena.gpu_adapter_key.v1")
        || adapter_key.get("backend").and_then(Value::as_str) != Some(backend)
        || adapter_key.get("vendor").and_then(Value::as_u64).is_none()
        || adapter_key.get("device").and_then(Value::as_u64).is_none()
        || adapter_key
            .get("device_type")
            .and_then(Value::as_str)
            .is_none()
        || adapter_key.get("driver").and_then(Value::as_str).is_none()
        || adapter_key
            .get("driver_info")
            .and_then(Value::as_str)
            .is_none()
    {
        return Err(
            "WaterBottle GPU result adapter_key must be a structured scena.gpu_adapter_key.v1 matching the backend"
                .to_string(),
        );
    }
    super::waterbottle_results::validate_waterbottle_adapter_expectation(result)?;
    let command_sha = result
        .get("command_record_sha256")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !valid_sha256(command_sha) {
        return Err(
            "WaterBottle GPU result command_record_sha256 must be a nonzero 64-hex checksum"
                .to_string(),
        );
    }
    for metric in [
        "nonblack_passed",
        "region_checks_passed",
        "color_family_histograms_passed",
    ] {
        if result.pointer(&format!("/metrics/{metric}")) != Some(&Value::Bool(true)) {
            return Err(format!(
                "WaterBottle GPU result metrics.{metric} must be true"
            ));
        }
    }
    if result
        .pointer("/metrics/reference_diff")
        .and_then(Value::as_str)
        != Some("passed")
        || result
            .pointer("/metrics/full_frame/compared_pixels")
            .and_then(Value::as_u64)
            != Some(512 * 512)
        || result
            .pointer("/metrics/full_frame/within_tolerance_fraction")
            .and_then(Value::as_f64)
            .is_none_or(|fraction| fraction < 0.95)
        || result
            .pointer("/metrics/full_frame/worst_region_bbox")
            .and_then(Value::as_array)
            .is_none_or(|bbox| bbox.len() != 4)
        || result
            .pointer("/metrics/thresholds/rgb_chebyshev_max")
            .and_then(Value::as_u64)
            != Some(16)
        || result
            .pointer("/metrics/thresholds/within_tolerance_fraction_min")
            .and_then(Value::as_f64)
            != Some(0.95)
    {
        return Err(
            "WaterBottle GPU result must carry the passed 512x512 full-frame reference-diff metrics and fixed thresholds"
                .to_string(),
        );
    }
    let mirror_rejected = result
        .get("known_bad_mutations")
        .and_then(Value::as_array)
        .is_some_and(|mutations| {
            mutations.iter().any(|mutation| {
                mutation.get("name").and_then(Value::as_str) == Some("horizontal_mirror")
                    && mutation.get("rejected").and_then(Value::as_bool) == Some(true)
            })
        });
    if !mirror_rejected {
        return Err(
            "WaterBottle GPU result must prove the full-frame oracle rejected horizontal_mirror"
                .to_string(),
        );
    }
    let checksums = result
        .get("source_checksums")
        .and_then(Value::as_array)
        .ok_or_else(|| "WaterBottle GPU result is missing source_checksums".to_string())?;
    let bound_png = checksums.iter().any(|entry| {
        entry.get("path").and_then(Value::as_str) == Some("m8-real-asset/waterbottle_gpu.png")
            && entry.get("sha256").and_then(Value::as_str) == Some(png_sha256)
    });
    let bound_command = checksums.iter().any(|entry| {
        entry
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.ends_with("macos-metal.commands.jsonl"))
            && entry.get("sha256").and_then(Value::as_str) == Some(command_sha)
    });
    let bound_diff = checksums.iter().any(|entry| {
        entry.get("path").and_then(Value::as_str) == Some("m8-real-asset/waterbottle_diff.png")
            && entry.get("sha256").and_then(Value::as_str) == Some(diff_sha256)
    });
    let bound_reference = checksums.iter().any(|entry| {
        entry.get("path").and_then(Value::as_str)
            == Some("tests/assets/gltf/khronos/WaterBottle/reference_512.png")
            && entry.get("sha256").and_then(Value::as_str) == Some(WATERBOTTLE_REFERENCE_SHA256)
    });
    if !bound_png || !bound_diff || !bound_reference || !bound_command {
        return Err(
            "WaterBottle GPU result source_checksums must bind the render, diff, committed reference, and command record".to_string(),
        );
    }
    Ok(())
}
