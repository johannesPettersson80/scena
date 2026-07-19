use crate::app::prelude::*;
use crate::app::tests_19::{
    assert_m5_provenance_mutations_rejected, stamp_stage_json_fixtures, write_stage_review_fixture,
    write_stage_waterbottle_result,
};

pub(crate) const STAGE_TEST_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
pub(crate) fn stage_release_artifacts_generates_canonical_release_evidence() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-readiness-test/stage-input");
    let output_root = root.join("target/xtask-release-readiness-test/stage-output");
    let _ = fs::remove_dir_all(&fixture_root);
    let _ = fs::remove_dir_all(&output_root);
    fs::create_dir_all(&fixture_root).expect("fixture root");

    write_stage_test_json(
        &fixture_root.join("release-webgl2/m6-rust-wasm-renderer-probe.json"),
        &browser_probe_fixture("webgl2"),
    );
    write_stage_test_json(
        &fixture_root.join("release-webgpu/m6-rust-wasm-renderer-probe.json"),
        &browser_probe_fixture("webgpu"),
    );
    write_stage_test_json(
        &fixture_root.join("release-wasm/m9-wasm-size.json"),
        &json!({"schema":"scena.m9.wasm_size.v1","status":"passed"}),
    );
    for lane in [
        "linux-native-vulkan",
        "headless-cpu",
        "macos-metal",
        "windows-dx12",
    ] {
        let lane_dir = fixture_root.join(format!("release-{lane}/m9-platform/{lane}"));
        fs::create_dir_all(&lane_dir).expect("lane dir");
        write_stage_test_json(
            &lane_dir.join("capabilities.json"),
            &json!({
                "schema": "scena.capabilities.v1",
                "lane": lane,
                "backend": lane,
                "adapter": { "available": lane != "headless-cpu" },
                "features": {},
                "diagnostics": [],
                "timestamp_unix_seconds": current_unix_seconds()
            }),
        );
        write_stage_test_json(
            &lane_dir.join("rendered-output.json"),
            &native_render_fixture(lane != "headless-cpu"),
        );
    }
    let waterbottle = fixture_root.join("release-macos-metal/m8-real-asset/waterbottle_gpu.png");
    fs::create_dir_all(waterbottle.parent().expect("waterbottle parent")).expect("waterbottle dir");
    fs::write(
        &waterbottle,
        [&[0x89, b'P', b'N', b'G'][..], &[1u8; 2048][..]].concat(),
    )
    .expect("waterbottle fixture");
    crate::app::tests_24::write_q01_cpu_proof_fixture(
        &fixture_root.join("release-linux-native-vulkan"),
        STAGE_TEST_COMMIT,
    );
    crate::app::tests_26::write_q02_release_proof_fixtures(
        &fixture_root.join("release-linux-native-vulkan"),
        STAGE_TEST_COMMIT,
    );
    for suffix in [
        "release-lanes/linux-native-vulkan.json",
        "release-lanes/headless-cpu.json",
        "release-lanes/linux-webgl2-chromium.json",
        "release-lanes/linux-webgpu-chromium.json",
        "release-lanes/wasm32-unknown-unknown.json",
        "release-lanes/macos-metal.json",
        "release-lanes/windows-dx12.json",
        "m5-benchmarks.json",
        "m5-public-api-freeze.json",
        "m9-platform/m9-benchmarks.json",
        "m9-platform/m9-benchmarks-4k.json",
        "m9-platform/m9-benchmarks-feature-matrix.json",
    ] {
        write_stage_test_json(
            &fixture_root.join(format!("release-linux-native-vulkan/{suffix}")),
            &json!({
                "status": "passed",
                "command_records": [{
                    "command": "fixture",
                    "status": "passed",
                    "duration_ms": 1,
                    "failure_log_path": "fixture.log",
                    "artifact_checksums": [{"path":"fixture","sha256":"x","bytes":1}]
                }],
                "baseline_comparison": {"status":"passed"},
                "rows": []
            }),
        );
    }
    for suffix in [
        "examples-visual/camera_framing_frame_bounds.ppm",
        "examples-visual/cad_plate_drawing_import.ppm",
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
    ] {
        write_stage_test_ppm(&fixture_root.join(format!("release-linux-native-vulkan/{suffix}")));
    }
    write_stage_test_json(
        &fixture_root
            .join("release-linux-native-vulkan/examples-visual/camera_framing_frame_bounds.json"),
        &json!({"schema":"fixture","status":"passed"}),
    );
    for suffix in [
        "m9-platform/linux-native-vulkan/surface-context-loss.json",
        "m9-platform/macos-metal/surface-context-loss.json",
        "m9-platform/windows-dx12/surface-context-loss.json",
    ] {
        write_stage_test_json(
            &fixture_root.join(format!("release-linux-native-vulkan/{suffix}")),
            &json!({"schema":"fixture","status":"passed"}),
        );
    }

    let local_checkout_error = stage_release_artifacts(&root, &fixture_root, &output_root)
        .expect_err("release staging must reject an unattributed local checkout");
    assert!(
        local_checkout_error.contains("local-checkout"),
        "local release provenance must fail before artifact validation: {local_checkout_error}",
    );

    let missing_provenance_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("staging must not synthesize missing source provenance");
    assert!(
        missing_provenance_error.contains("provenance")
            || missing_provenance_error.contains("commit"),
        "missing source provenance must fail explicitly: {missing_provenance_error}",
    );

    stamp_stage_json_fixtures(&fixture_root, STAGE_TEST_COMMIT, current_unix_seconds());

    let benchmark_source = fixture_root.join("release-linux-native-vulkan/m5-benchmarks.json");
    assert_m5_provenance_mutations_rejected(
        &fixture_root,
        &output_root,
        &benchmark_source,
        STAGE_TEST_COMMIT,
    );
    crate::app::tests_24::write_q01_cpu_proof_fixture(
        &fixture_root.join("release-linux-native-vulkan"),
        STAGE_TEST_COMMIT,
    );

    let missing_waterbottle_result =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("WaterBottle PNG alone must not satisfy release evidence");
    assert!(
        missing_waterbottle_result.contains("waterbottle_gpu_result.json"),
        "missing WaterBottle companion result must fail explicitly: {missing_waterbottle_result}",
    );
    write_stage_test_png(&waterbottle);
    write_stage_waterbottle_result(&fixture_root, &waterbottle);

    let missing_review_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("staging must not manufacture missing release-review evidence");
    assert!(
        missing_review_error.contains("RELEASE-REVIEWS-MISSING")
            && missing_review_error.contains("review"),
        "missing reviewer evidence must fail for an explicit review reason: \
         {missing_review_error}",
    );

    let expected_review_files = write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);
    let expected_benchmark = fs::read(&benchmark_source).expect("benchmark source reads");

    let first_role = REQUIRED_REVIEW_ROLES[0];
    let first_report = fixture_root.join(format!(
        "release-review-evidence/reviews/{first_role}/{STAGE_TEST_COMMIT}.md"
    ));
    let report_text = fs::read_to_string(&first_report).expect("review fixture reads");
    fs::write(
        &first_report,
        report_text.replace(
            "github:independent-reviewer-0",
            "github:scena-release-automation",
        ),
    )
    .expect("automation-authored review fixture writes");
    let automation_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("synthetic automation must not count as an independent reviewer");
    assert!(
        automation_error.contains("automation") && automation_error.contains("reviewer"),
        "automation-authored review must fail for reviewer identity: {automation_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    fs::remove_file(&first_report).expect("required review removal");
    let missing_role_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("every required review role must be present");
    assert!(
        missing_role_error.contains(first_role) && missing_role_error.contains("missing"),
        "missing review role must fail explicitly: {missing_role_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    let report_text = fs::read_to_string(&first_report).expect("review fixture reads");
    fs::write(
        &first_report,
        report_text.replace(
            STAGE_TEST_COMMIT,
            "ffffffffffffffffffffffffffffffffffffffff",
        ),
    )
    .expect("commit-mismatched review fixture writes");
    let review_commit_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("review commit mismatch must fail closed");
    assert!(
        review_commit_error.contains("reviewed_commit")
            && review_commit_error.contains(STAGE_TEST_COMMIT),
        "review commit mismatch must fail for the exact commit contract: {review_commit_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    let second_role = REQUIRED_REVIEW_ROLES[1];
    let second_report = fixture_root.join(format!(
        "release-review-evidence/reviews/{second_role}/{STAGE_TEST_COMMIT}.md"
    ));
    let second_text = fs::read_to_string(&second_report).expect("second review reads");
    fs::write(
        &second_report,
        second_text.replace(
            "github:independent-reviewer-1",
            "github:independent-reviewer-0",
        ),
    )
    .expect("duplicate reviewer fixture writes");
    let duplicate_reviewer_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("one identity must not satisfy multiple required roles");
    assert!(
        duplicate_reviewer_error.contains("reused")
            && duplicate_reviewer_error.contains("review roles"),
        "duplicate reviewer must fail distinct-role policy: {duplicate_reviewer_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    let findings_path = fixture_root.join("release-review-evidence/reviews/findings.json");
    write_stage_test_json(
        &findings_path,
        &json!({
            "schema": "scena.release.findings.v1",
            "reviewed_commit": STAGE_TEST_COMMIT,
            "generated_at": "2026-07-16T00:00:00Z",
            "findings": [{
                "id": "D01-BLOCKER",
                "role": first_role,
                "summary": "release evidence is not trustworthy",
                "severity": "blocker",
                "status": "open",
                "evidence": ["crates/xtask/src/app/release/stage_reviews.rs"],
                "notes": "must be fixed before release",
                "deferral_target": null,
                "history": [{"status":"open", "at":"2026-07-16T00:00:00Z"}]
            }]
        }),
    );
    let blocker_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("open blocker must fail release review validation");
    assert!(
        blocker_error.contains("D01-BLOCKER") && blocker_error.contains("remains open"),
        "open blocker must fail for its status: {blocker_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    let mut report_bytes = fs::read(&first_report).expect("review fixture reads");
    report_bytes.extend_from_slice(b"\nTampered after maintainer sign-off.\n");
    fs::write(&first_report, report_bytes).expect("tampered review writes");
    let tamper_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("report bytes changed after sign-off must fail");
    assert!(
        tamper_error.contains("tampered") && tamper_error.contains(first_role),
        "tampered review report must fail its sign-off hash: {tamper_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    let signoff_path = fixture_root.join("release-review-evidence/reviews/maintainer-signoff.toml");
    fs::remove_file(&signoff_path).expect("maintainer sign-off removal");
    let missing_signoff_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("maintainer sign-off is mandatory");
    assert!(
        missing_signoff_error.contains("maintainer-signoff.toml")
            && missing_signoff_error.contains("missing"),
        "absent maintainer sign-off must fail explicitly: {missing_signoff_error}",
    );
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);

    fs::write(
        &waterbottle,
        [&[0x89, b'P', b'N', b'G'][..], &[1u8; 2048][..]].concat(),
    )
    .expect("fake WaterBottle fixture rewrites");
    write_stage_waterbottle_result(&fixture_root, &waterbottle);
    let fake_png_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("PNG magic bytes plus arbitrary payload must not pass staging");
    assert!(
        fake_png_error.contains("RELEASE-VISUAL-PROOF")
            && fake_png_error.contains("WaterBottle")
            && fake_png_error.contains("PNG"),
        "fake WaterBottle PNG must fail for an explicit decode/contract reason: {fake_png_error}",
    );
    write_stage_test_png(&waterbottle);
    let substituted_png_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("substituting the PNG after result generation must fail");
    assert!(
        substituted_png_error.contains("PNG") && substituted_png_error.contains("hash"),
        "substituted WaterBottle PNG must fail its result binding: {substituted_png_error}",
    );
    write_stage_waterbottle_result(&fixture_root, &waterbottle);

    let webgpu_probe = fixture_root.join("release-webgpu/m6-rust-wasm-renderer-probe.json");
    set_browser_readback_source(&webgpu_probe, "canvas-readback");
    let canvas_error =
        stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
            .expect_err("canvas-only browser pixels must not pass as renderer GPU evidence");
    assert!(
        canvas_error.contains("RELEASE-BROWSER-PROOF")
            && canvas_error.contains("webgpu")
            && canvas_error.contains("renderer-owned-gpu-copy"),
        "canvas-only browser proof must fail for the renderer-owned readback contract: \
         {canvas_error}",
    );
    set_browser_readback_source(&webgpu_probe, "renderer-owned-gpu-copy");

    stage_release_artifacts_for_commit(&fixture_root, &output_root, STAGE_TEST_COMMIT)
        .expect("stage succeeds");

    for (relative, expected) in expected_review_files {
        let staged = fs::read(output_root.join(&relative))
            .unwrap_or_else(|error| panic!("staged review {} reads: {error}", relative.display()));
        assert_eq!(
            staged,
            expected,
            "staging must preserve independently authored review evidence byte-for-byte: {}",
            relative.display(),
        );
    }
    assert_eq!(
        fs::read(output_root.join("m5-benchmarks.json")).expect("staged benchmark reads"),
        expected_benchmark,
        "staging must preserve source artifact provenance byte-for-byte",
    );
    let staging_metadata = fs::read_to_string(output_root.join("staging-metadata.json"))
        .expect("separate staging metadata reads");
    assert!(staging_metadata.contains("\"schema\": \"scena.release.staging.v1\""));
    assert!(staging_metadata.contains("\"staged_at\""));
    assert!(staging_metadata.contains("\"staging_checkout\""));
    assert!(staging_metadata.contains("\"staging_tool_version\""));
    assert!(staging_metadata.contains(STAGE_TEST_COMMIT));
    let merged_browser: Value = serde_json::from_str(
        &fs::read_to_string(output_root.join("m6-rust-wasm-renderer-probe.json"))
            .expect("merged browser probe reads"),
    )
    .expect("merged browser probe parses");
    let merged_results = merged_browser["results"]
        .as_array()
        .expect("merged browser release results are an array");
    assert_eq!(
        merged_browser["schema"],
        "scena.m6.rust_wasm_renderer_probe.aggregate.v1"
    );
    assert_eq!(
        merged_browser["producer"],
        "cargo run -p xtask -- stage-release-artifacts"
    );
    assert_eq!(merged_browser["evidence_phase"], "staging-aggregation");
    assert_eq!(
        merged_browser["source_checksums"]
            .as_array()
            .expect("merged browser sources are checksummed")
            .len(),
        2
    );
    assert_eq!(
        merged_results.len(),
        2,
        "diagnostic workflow results must not become ambiguous release headlines"
    );
    assert!(
        merged_results
            .iter()
            .all(|result| result["workflow"] == "triangle")
    );
    let mut typed_visual_findings = Vec::new();
    for suffix in REQUIRED_VISUAL_PROOF_ARTIFACT_SUFFIXES {
        require_visual_proof_artifact_file(
            &output_root.join(suffix),
            suffix,
            &mut typed_visual_findings,
        );
    }
    assert_eq!(
        typed_visual_findings,
        Vec::new(),
        "staging must emit complete typed visual proof contracts"
    );
    assert!(
        output_root
            .join("visual-proof/browser-webgpu.json")
            .is_file()
    );
    assert!(
        output_root
            .join("visual-proof/waterbottle-gpu.json")
            .is_file()
    );
    assert!(
        output_root
            .join("visual-proof/waterbottle-cpu.json")
            .is_file()
    );
    let matrix_text = fs::read_to_string(output_root.join("m9-platform/m9-capability-matrix.json"))
        .expect("matrix reads");
    assert!(matrix_text.contains("\"status\": \"passed\""));
    let matrix: Value = serde_json::from_str(&matrix_text).expect("matrix parses");
    assert_eq!(
        matrix["producer"],
        "cargo run -p xtask -- stage-release-artifacts"
    );
    assert_eq!(matrix["evidence_phase"], "staging-aggregation");
    assert!(
        matrix["source_checksums"]
            .as_array()
            .is_some_and(|sources| !sources.is_empty()),
        "capability aggregation must bind its source lane artifacts"
    );
    assert!(!matrix_text.contains("missing-lane-artifact"));
}

#[test]
pub(crate) fn stage_release_artifact_timestamp_format_is_rfc3339_utc() {
    assert_eq!(utc_rfc3339_from_unix(0), "1970-01-01T00:00:00Z");
    assert_eq!(utc_rfc3339_from_unix(1_688_212_096), "2023-07-01T11:48:16Z");
}

fn browser_probe_fixture(backend: &str) -> serde_json::Value {
    let mut release_result = json!({
        "schema": "scena.m6.browser_renderer_probe.v1",
        "backend": backend,
        "workflow": "triangle",
        "status": "passed",
        "pixels": { "nonblack": 42 },
        "capabilities": { "backend": backend },
        "renderer_readback": {
            "source": "renderer-owned-gpu-copy",
            "width": 64,
            "height": 64,
            "pixel_statistics": { "nonblack": 42 },
            "rgba8_fnv1a64": "0000000000000001"
        }
    });
    if backend.eq_ignore_ascii_case("webgl2") {
        release_result["parity"] = crate::app::tests_20::browser_webgl2_parity();
        release_result["renderer_readback"]["width"] = json!(1);
        release_result["renderer_readback"]["height"] = json!(1);
        release_result["renderer_readback"]["rgba8_fnv1a64"] = json!("fedcba9876543210");
    }
    json!({
        "gate": "m6-rust-wasm-renderer-probe",
        "status": "passed",
        "release_results": [release_result.clone()],
        "results": [release_result, {
            "schema": "scena.m6.browser_renderer_probe.v1",
            "backend": backend,
            "workflow": "diagnostic-workflow",
            "status": "passed",
            "pixels": { "nonblack": 42 },
            "capabilities": { "backend": backend },
            "renderer_readback": {
                "source": "renderer-owned-gpu-copy",
                "width": 64,
                "height": 64,
                "pixel_statistics": { "nonblack": 42 },
                "rgba8_fnv1a64": "0000000000000001"
            }
        }]
    })
}

pub(crate) fn write_stage_test_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).expect("fixture serializes")
        ),
    )
    .expect("json fixture");
}

fn set_browser_readback_source(path: &Path, source: &str) {
    let text = fs::read_to_string(path).expect("browser fixture reads");
    let mut value = serde_json::from_str::<Value>(&text).expect("browser fixture parses");
    value["release_results"][0]["renderer_readback"]["source"] = json!(source);
    value["results"][0]["renderer_readback"]["source"] = json!(source);
    write_stage_test_json(path, &value);
}

fn write_stage_test_ppm(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("ppm parent");
    }
    fs::write(path, b"P6\n2 1\n255\n\x00\x00\x00\xff\x80\x40").expect("ppm fixture");
}

fn write_stage_test_png(path: &Path) {
    let image = image::RgbaImage::from_fn(512, 512, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    });
    image
        .save_with_format(path, image::ImageFormat::Png)
        .expect("valid WaterBottle PNG fixture writes");
}

fn native_render_fixture(gpu: bool) -> serde_json::Value {
    json!({
        "schema": "scena.m9.platform_render.v1",
        "backend": if gpu { "Metal" } else { "Headless" },
        "host_gpu_available": gpu,
        "gpu_proof": gpu,
        "headless_cpu_proof": !gpu,
        "timestamp_unix_seconds": current_unix_seconds(),
        "default_scene": screenshot_metadata_fixture(),
        "static_gltf": {
            "production_claim": true,
            "gpu_proof": gpu,
            "proof_class": if gpu { "camera-framed-non-ndc" } else { "cpu-camera-framed-non-ndc" },
            "nonblack_pixels": 10,
            "asset_provenance": { "hash": "fnv1a64:0000000000000001" },
            "backend": "fixture",
            "adapter": {},
            "renderer_settings": {},
            "color_management": {},
            "tolerance": {},
            "screenshot": "fixture.ppm",
            "width": 2,
            "height": 1
        },
        "pbr_lights": {
            "proof_class": "native-pbr-punctual-light",
            "production_claim": gpu,
            "gpu_proof": gpu,
            "lights": [
                pbr_light_fixture("directional", gpu),
                pbr_light_fixture("point", gpu),
                pbr_light_fixture("spot", gpu)
            ]
        }
    })
}

fn screenshot_metadata_fixture() -> serde_json::Value {
    json!({
        "backend": "fixture",
        "adapter": {},
        "renderer_settings": {},
        "color_management": {},
        "tolerance": {},
        "screenshot": "fixture.ppm",
        "width": 2,
        "height": 1
    })
}

fn pbr_light_fixture(light_type: &str, gpu: bool) -> serde_json::Value {
    json!({
        "light_type": light_type,
        "gpu_proof": gpu,
        "production_claim": gpu,
        "color_assertion_passed": true,
        "nonblack_pixels": 10,
        "backend": "fixture",
        "adapter": {},
        "renderer_settings": {},
        "color_management": {},
        "tolerance": {},
        "screenshot": "fixture.ppm",
        "width": 2,
        "height": 1
    })
}
