use crate::app::prelude::*;

#[test]
pub(crate) fn workflow_action_policy_rejects_mutable_or_unreviewable_references() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/workflow-action-pins");
    let workflow = fixture_root.join(".github/workflows/ci.yml");
    let dependabot = fixture_root.join(".github/dependabot.yml");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(workflow.parent().expect("workflow parent")).expect("workflow dir");
    fs::write(
        dependabot,
        "version: 2\nupdates:\n  - package-ecosystem: \"github-actions\"\n    directory: \"/\"\n    schedule:\n      interval: \"weekly\"\n",
    )
    .expect("Dependabot fixture writes");

    fs::write(
        &workflow,
        "jobs:\n  test:\n    steps:\n      - uses: actions/checkout@v4\n",
    )
    .expect("mutable workflow fixture writes");
    let mut findings = Vec::new();
    check_m9_ci_release_lanes(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "WORKFLOW-ACTION-PIN"
                && finding.message.contains("actions/checkout@v4")
                && finding.message.contains("40-hex")
        }),
        "mutable action references must fail explicitly: {findings:?}",
    );

    fs::write(
        &workflow,
        "jobs:\n  test:\n    steps:\n      - uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5\n",
    )
    .expect("uncommented workflow fixture writes");
    let mut findings = Vec::new();
    check_m9_ci_release_lanes(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "WORKFLOW-ACTION-PIN" && finding.message.contains("version comment")
        }),
        "immutable references without a reviewable release comment must fail: {findings:?}",
    );

    fs::write(
        &workflow,
        "jobs:\n  test:\n    steps:\n      - uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4.3.1\n",
    )
    .expect("pinned workflow fixture writes");
    let mut findings = Vec::new();
    check_m9_ci_release_lanes(&fixture_root, &mut findings);
    assert!(
        findings
            .iter()
            .all(|finding| finding.rule != "WORKFLOW-ACTION-PIN"),
        "immutable action references with release comments must pass policy: {findings:?}",
    );
}

#[test]
pub(crate) fn cli_output_policy_rejects_raw_println_and_hand_written_json() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/cli-output-policy");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("src/bin/scena")).expect("fixture directories create");
    fs::write(
        fixture_root.join("src/bin/scena/process_output_shared.rs"),
        "fn write_stdout_line() {}\n// BrokenPipe\n// serde_json::json!\n\
         const FALLBACK: &str = \"{\\\"schema\\\":\\\"manual\\\"}\";\n",
    )
    .expect("shared output fixture writes");
    fs::write(
        fixture_root.join("src/bin/scena.rs"),
        "fn main() { println!(\"{}\", outcome.stdout); }\n",
    )
    .expect("scena fixture writes");
    fs::write(
        fixture_root.join("src/bin/scena-convert.rs"),
        "fn json_escape(value: &str) -> String { value.to_owned() }\n\
         fn main() { println!(\"{{\\\"status\\\":\\\"planned\\\"}}\"); }\n",
    )
    .expect("convert fixture writes");
    let mut findings = Vec::new();

    check_cli_output_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "CLI-MACHINE-OUTPUT"
                && finding.message.contains("println!")
                && finding.message.contains("src/bin/scena.rs")
        }),
        "raw scena machine println must fail: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "CLI-MACHINE-OUTPUT"
                && finding.message.contains("json_escape")
                && finding.message.contains("src/bin/scena-convert.rs")
        }),
        "hand-written conversion JSON escaping must fail: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "CLI-MACHINE-OUTPUT"
                && finding.message.contains("\\\"schema\\\"")
                && finding
                    .message
                    .contains("src/bin/scena/process_output_shared.rs")
        }),
        "hand-written shared output JSON must fail: {findings:?}",
    );
}

#[test]
pub(crate) fn scene_import_transaction_policy_rejects_premature_stale_replacement() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/import-transaction-policy");
    let load = fixture_root.join("src/scene/import/load.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(load.parent().expect("load parent")).expect("fixture directories create");
    fs::write(
        &load,
        r#"pub fn replace_import() {
    let _foreign = ForeignReplacementImport;
    let mut transaction = SceneTransaction::new(self);
    import.mark_stale();
    let replacement = transaction.scene().instantiate_with(scene_asset, options)?;
    transaction.scene().remove_nodes_unchecked(&removed);
    transaction.commit();
    Ok(replacement)
}
"#,
    )
    .expect("premature-stale fixture writes");
    let mut findings = Vec::new();

    check_m3a_scene_import_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-IMPORT-TRANSACTION"
                && finding.message.contains("replace_import lifecycle order")
        }),
        "premature stale marking must fail import transaction policy: {findings:?}",
    );
}

fn browser_result(backend: &str) -> Value {
    let mut result = json!({
        "schema": "scena.m6.browser_renderer_probe.v1",
        "backend": backend,
        "workflow": "triangle",
        "status": "passed",
        "renderer_readback": {
            "source": "renderer-owned-gpu-copy",
            "width": 64,
            "height": 64,
            "rgba8_fnv1a64": "0123456789abcdef",
            "pixel_statistics": {"nonblack": 42}
        }
    });
    if backend.eq_ignore_ascii_case("webgl2") {
        result["parity"] = browser_webgl2_parity();
        result["renderer_readback"]["width"] = json!(1);
        result["renderer_readback"]["height"] = json!(1);
        result["renderer_readback"]["rgba8_fnv1a64"] = json!("fedcba9876543210");
    }
    result
}

pub(crate) fn browser_webgl2_parity() -> Value {
    json!({
        "schema": "scena.m6.cpu_webgl2_parity.v1",
        "status": "passed",
        "failure_codes": [],
        "normalization": {
            "row_origin": "top-left",
            "transfer": "srgb8",
            "alpha": "straight-opaque",
            "dimensions": "exact",
            "width": 1,
            "height": 1,
            "comparison_channels": "rgb",
            "ssim_domain": "srgb8-luma"
        },
        "thresholds": {
            "rmse_max": 0.08,
            "ssim_min": 0.93,
            "p95_channel_delta_max": 24,
            "mean_channel_delta_max": 6.0,
            "foreground_iou_min": 0.90,
            "foreground_region_rmse_max": 0.13,
            "alpha_deviations_max": 0
        },
        "cpu_frame": {
            "source": "renderer-owned-cpu-frame",
            "width": 1,
            "height": 1,
            "rgba8_fnv1a64": "0123456789abcdef",
            "rgba8_base64": "AAAA/w==",
            "alpha_deviations": 0
        },
        "gpu_frame": {
            "source": "renderer-owned-gpu-copy",
            "width": 1,
            "height": 1,
            "rgba8_fnv1a64": "fedcba9876543210",
            "rgba8_base64": "AQID/w==",
            "alpha_deviations": 0
        },
        "metrics": {
            "rmse": 0.01,
            "ssim": 0.99,
            "max_channel_delta": 3,
            "p95_channel_delta": 3,
            "mean_channel_delta": 2.0,
            "foreground_iou": 0.99,
            "foreground_region_rmse": 0.02,
            "foreground_bounds": [0, 0, 1, 1],
            "compared_pixels": 1
        },
        "known_bad_mutation": {
            "kind": "gpu-center-channel-perturbation",
            "rejected": true,
            "failure_codes": ["rmse", "ssim"],
            "metrics": {"rmse": 0.5, "ssim": 0.4}
        }
    })
}

#[test]
pub(crate) fn browser_release_headline_validation_fails_closed_by_contract_dimension() {
    let valid = browser_result("webgpu");
    validate_browser_backend_result(std::slice::from_ref(&valid), "webgpu")
        .expect("complete renderer-owned browser result passes");

    let absent = validate_browser_backend_result(&[], "webgpu")
        .expect_err("absent backend result must fail");
    assert!(absent.contains("missing backend webgpu"));

    let wrong_backend = validate_browser_backend_result(&[browser_result("metal")], "webgpu")
        .expect_err("wrong backend must fail");
    assert!(wrong_backend.contains("missing backend webgpu"));

    let ambiguous = validate_browser_backend_result(&[valid.clone(), valid.clone()], "webgpu")
        .expect_err("duplicate headline results must fail");
    assert!(ambiguous.contains("ambiguous"));

    for (pointer, replacement, expected) in [
        ("/status", json!("failed"), "did not pass"),
        (
            "/renderer_readback/source",
            json!("canvas-readback"),
            "renderer-owned-gpu-copy",
        ),
        ("/renderer_readback/width", json!(0), "width and height"),
        (
            "/renderer_readback/pixel_statistics/nonblack",
            json!(0),
            "zero nonblack",
        ),
        (
            "/renderer_readback/rgba8_fnv1a64",
            json!("0000000000000000"),
            "invalid or zero",
        ),
        (
            "/renderer_readback/rgba8_fnv1a64",
            json!("fnv1a64:0123456789abcdef"),
            "invalid or zero",
        ),
    ] {
        let mut malformed = valid.clone();
        *malformed
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("fixture pointer {pointer} exists")) = replacement;
        let error = validate_browser_backend_result(&[malformed], "webgpu")
            .expect_err("malformed browser proof must fail");
        assert!(
            error.contains(expected),
            "{pointer} must fail for {expected}: {error}"
        );
    }
}

#[test]
pub(crate) fn webgl2_release_headline_requires_complete_cpu_gpu_parity_evidence() {
    let valid = browser_result("webgl2");
    validate_browser_backend_result(std::slice::from_ref(&valid), "webgl2")
        .expect("complete CPU/WebGL2 parity evidence passes");

    for (pointer, replacement, expected) in [
        ("/parity", Value::Null, "CPU/WebGL2 parity"),
        ("/parity/status", json!("failed"), "did not pass"),
        (
            "/parity/cpu_frame/source",
            json!("committed-reference"),
            "renderer-owned-cpu-frame",
        ),
        (
            "/parity/gpu_frame/source",
            json!("canvas-readback"),
            "renderer-owned-gpu-copy",
        ),
        ("/parity/gpu_frame/width", json!(2), "matching dimensions"),
        (
            "/parity/cpu_frame/rgba8_base64",
            json!(""),
            "RGBA8 frame input",
        ),
        (
            "/parity/normalization/row_origin",
            json!("bottom-left"),
            "normalization",
        ),
        ("/parity/failure_codes", json!(["rmse"]), "failure_codes"),
        (
            "/parity/known_bad_mutation/rejected",
            json!(false),
            "known-bad mutation",
        ),
    ] {
        let mut malformed = valid.clone();
        *malformed
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("fixture pointer {pointer} exists")) = replacement;
        let error = validate_browser_backend_result(&[malformed], "webgl2")
            .expect_err("incomplete CPU/WebGL2 parity proof must fail");
        assert!(
            error.contains(expected),
            "{pointer} must fail for {expected}: {error}"
        );
    }
}

#[test]
pub(crate) fn visual_proof_rejects_wrong_typed_browser_contract() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-proof-typed-browser");
    let artifact_path = fixture_root.join("visual-proof/browser-webgpu.json");
    fs::create_dir_all(artifact_path.parent().expect("visual proof parent"))
        .expect("visual proof fixture dir");
    fs::write(
        &artifact_path,
        r#"{
          "schema": "untyped",
          "lane": "browser-webgl2",
          "status": "passed",
          "preview_only": false,
          "rust_test_command": false,
          "rust_test_output_observed": false,
          "skip_marker_observed": false,
          "release_evidence": true,
          "proof_class": "native-gpu-rendered-output",
          "backend": "webgl2",
          "commit_sha": "local-checkout",
          "timestamp_unix_seconds": 1,
          "source_artifact_path": "missing.json",
          "source_artifact_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
          "renderer_readback": {
            "source": "canvas-readback",
            "width": 0,
            "height": 0,
            "rgba8_fnv1a64": "0000000000000000",
            "pixel_statistics": {"nonblack": 0}
          }
        }"#,
    )
    .expect("visual proof fixture");
    let mut findings = Vec::new();

    require_visual_proof_artifact_file(
        &artifact_path,
        "visual-proof/browser-webgpu.json",
        &mut findings,
    );

    for expected in [
        "schema",
        "lane",
        "proof_class",
        "producer",
        "exact commit",
        "source artifact hash",
        "backend",
        "renderer-owned-gpu-copy",
        "dimensions",
        "nonblack",
        "rgba8_fnv1a64",
    ] {
        assert!(
            findings
                .iter()
                .any(|finding| finding.rule == "VISUAL-PROOF" && finding.message.contains(expected)),
            "typed visual proof must reject {expected}: {findings:?}"
        );
    }
}

#[test]
pub(crate) fn visual_proof_rejects_incomplete_waterbottle_and_native_contracts() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-proof-typed-native");
    let visual_root = fixture_root.join("visual-proof");
    fs::create_dir_all(&visual_root).expect("visual proof fixture dir");

    let water_source = fixture_root.join("m8-real-asset/waterbottle_gpu.png");
    fs::create_dir_all(water_source.parent().expect("WaterBottle source parent"))
        .expect("WaterBottle source dir");
    fs::write(&water_source, b"fixture PNG bytes").expect("WaterBottle source writes");
    let water_hash = sha256_hex(&water_source).expect("WaterBottle source hash");
    let water_path = visual_root.join("waterbottle-gpu.json");
    fs::write(
        &water_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.visual_proof.v1",
            "producer": "cargo run -p xtask -- stage-release-artifacts",
            "lane": "waterbottle-gpu",
            "status": "passed",
            "preview_only": false,
            "rust_test_command": false,
            "rust_test_output_observed": false,
            "skip_marker_observed": false,
            "release_evidence": true,
            "proof_class": "native-waterbottle-gpu",
            "commit_sha": "0123456789abcdef0123456789abcdef01234567",
            "timestamp_unix_seconds": current_unix_seconds(),
            "source_artifact_path": "m8-real-asset/waterbottle_gpu.png",
            "source_artifact_sha256": water_hash,
            "width": 1,
            "height": 1,
            "nonblack_pixels": 0,
            "distinct_rgba_values": 1,
            "metrics": {}
        }))
        .expect("WaterBottle fixture serializes"),
    )
    .expect("WaterBottle fixture writes");

    let native_source = fixture_root.join("m9-platform/macos-metal/rendered-output.json");
    fs::create_dir_all(native_source.parent().expect("native source parent"))
        .expect("native source dir");
    fs::write(&native_source, "{\"gpu_proof\":false}\n").expect("native source writes");
    let native_hash = sha256_hex(&native_source).expect("native source hash");
    let native_path = visual_root.join("native-gpu.json");
    fs::write(
        &native_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.visual_proof.v1",
            "producer": "cargo run -p xtask -- stage-release-artifacts",
            "lane": "native-gpu",
            "status": "passed",
            "preview_only": false,
            "rust_test_command": false,
            "rust_test_output_observed": false,
            "skip_marker_observed": false,
            "release_evidence": true,
            "proof_class": "native-gpu-rendered-output",
            "commit_sha": "0123456789abcdef0123456789abcdef01234567",
            "timestamp_unix_seconds": current_unix_seconds(),
            "source_artifact_path": "m9-platform/macos-metal/rendered-output.json",
            "source_artifact_sha256": native_hash,
            "source_lane": "macos-metal",
            "gpu_proof": true
        }))
        .expect("native fixture serializes"),
    )
    .expect("native fixture writes");

    let mut findings = Vec::new();
    require_visual_proof_artifact_file(
        &water_path,
        "visual-proof/waterbottle-gpu.json",
        &mut findings,
    );
    require_visual_proof_artifact_file(&native_path, "visual-proof/native-gpu.json", &mut findings);
    for expected in [
        "WaterBottle dimensions",
        "WaterBottle pixel distribution",
        "WaterBottle comparison metrics",
        "native GPU source artifact",
    ] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == "VISUAL-PROOF" && finding.message.contains(expected)
            }),
            "typed visual proof must reject {expected}: {findings:?}"
        );
    }
}
